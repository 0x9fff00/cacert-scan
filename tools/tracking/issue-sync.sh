#!/usr/bin/env bash
# keeps the #576475 meta table in sync with the category issues and reports PR links, PR search hits, comments and issue edits; one line per event
set -u
R=NixOS/nixpkgs
PARENT=576475
declare -A CAT=([576541]="Vendored PEM bundle" [576542]="Bundle inside a wheel, jar or zip" [576543]="Java keystore" [576544]="Compiled into a binary")
WATCH="576475 576541 576542 576543 576544 576485"
S=${CACERT_TRACKING_STATE:-$HOME/.cache/cacert-tracking}
mkdir -p "$S"
JQ=$(command -v jq)

refs() {
  gh api -H "Accept: application/vnd.github+json" "repos/$R/issues/$1/timeline" --paginate -q '.[] | select(.event=="cross-referenced") | .source.issue | select(.pull_request) | "\(.repository_url | sub("https://api.github.com/repos/"; ""))#\(.number) [\(.state)\(if .pull_request.merged_at then "/merged" else "" end)] \(.user.login): \(.title)"' 2>/dev/null | sort -u
}
comments() {
  gh api "repos/$R/issues/$1/comments" --paginate -q '.[] | "\(.id) \(.user.login): \(.body | gsub("\\s+"; " ") | .[:160])"' 2>/dev/null
}

search_prs() {
  gh api -X GET search/issues -f q="repo:NixOS/nixpkgs is:pr 576541 OR 576542 OR 576543 OR 576544 updated:>=$(date -u -d '2 days ago' +%F)" -f per_page=100 -q '.items[] | "NixOS/nixpkgs#\(.number) [\(.state)] \(.user.login): \(.title)"' 2>/dev/null | sort -u
}

body_of() {
  gh api "repos/$R/issues/$1" -q '"title: " + .title + "\n" + .body' 2>/dev/null | tr -d '\r'
}
check_edits() {
  local i new
  for i in $WATCH; do
    new=$(body_of $i) || continue
    [[ -n $new ]] || continue
    if [[ ! -e $S/body-$i ]]; then printf '%s\n' "$new" > $S/body-$i; continue; fi
    if ! cmp -s $S/body-$i <(printf '%s\n' "$new"); then
      diff $S/body-$i <(printf '%s\n' "$new") | grep -E '^[<>] ' | head -6 | cut -c1-220 | sed "s/^</#$i edit: -/; s/^>/#$i edit: +/"
      printf '%s\n' "$new" > $S/body-$i
    fi
  done
}

# PRs by $PR_AUTHOR updated since the last cycle: new comments, reviews, state and CI failures
PR_AUTHOR=${PR_AUTHOR:-zimbatm}
watch_prs() {
  local since now pr repo num title state merged sha c
  now=$(date -u +%FT%TZ)
  [[ -e $S/prs-since ]] || { echo "$now" > $S/prs-since; return; }
  since=$(cat $S/prs-since)
  gh api -X GET search/issues -f q="is:pr author:$PR_AUTHOR updated:>=$since" -f per_page=100 \
    -q '.items[] | "\(.repository_url | sub("https://api.github.com/repos/"; ""))\t\(.number)\t\(.title)"' 2>/dev/null |
  while IFS=$'\t' read -r repo num title; do
    pr="$repo#$num"
    gh api "repos/$repo/issues/$num/comments?since=$since" -q '.[] | select(.user.login != "'"$PR_AUTHOR"'" and (.user.type != "Bot")) | "\(.user.login): \(.body | gsub("\\s+"; " ") | .[:160])"' 2>/dev/null | sed "s|^|$pr comment: |"
    gh api "repos/$repo/pulls/$num/reviews" -q '.[] | select(.submitted_at >= "'"$since"'" and .user.login != "'"$PR_AUTHOR"'") | "\(.user.login) \(.state): \(.body | gsub("\\s+"; " ") | .[:160])"' 2>/dev/null | sed "s|^|$pr review: |"
    gh api "repos/$repo/pulls/$num/comments?since=$since" -q '.[] | select(.user.login != "'"$PR_AUTHOR"'") | "\(.user.login) on \(.path): \(.body | gsub("\\s+"; " ") | .[:160])"' 2>/dev/null | sed "s|^|$pr inline: |"
    read -r state merged sha < <(gh api "repos/$repo/pulls/$num" -q '"\(.state) \(.merged_at != null) \(.head.sha)"' 2>/dev/null)
    [[ $state == closed ]] && { [[ $merged == true ]] && echo "$pr merged: $title" || echo "$pr closed: $title"; }
    c=$(gh api "repos/$repo/commits/$sha/check-runs?per_page=100" -q '[.check_runs[] | select(.conclusion == "failure") | .name] | join(", ")' 2>/dev/null)
    [[ -n $c && $c != "$(cat "$S/ci-${repo//\//_}-$num" 2>/dev/null)" ]] && echo "$pr CI failed: $c"
    printf '%s' "$c" > "$S/ci-${repo//\//_}-$num"
  done
  echo "$now" > $S/prs-since
}

rest_body() {
  gh api "repos/$R/issues/$1" -q .body 2>/dev/null
}
# write only if the issue still has the text we started from; GitHub has no conditional update
cas_write() {
  local issue=$1 before=$2 after=$3 now
  now=$(rest_body $issue) || return 1
  [[ "$now" == "$before" ]] || { echo "#$issue changed during update, retrying next cycle" >&2; return 1; }
  gh api -X PATCH "repos/$R/issues/$issue" -F body=@<(printf '%s' "$after") >/dev/null
}
sync_table() {
  local body new total=0 ticked=0 n t k b
  body=$(rest_body $PARENT) || return
  [[ -n $body ]] || return
  new=$body
  for n in 576541 576542 576543 576544; do
    b=$(rest_body $n) || return
    [[ -n $b ]] || return
    t=$(grep -c '^- \[[ xX]\] ' <<<"$b")
    k=$(grep -c '^- \[[xX]\] ' <<<"$b")
    total=$((total + t)); ticked=$((ticked + k))
    new=$(awk -v name="${CAT[$n]}" -v t="$t" -v k="$k" -F' \\| ' 'BEGIN{OFS=" | "} index($0, "| " name " |")==1 && NF==5 && $2 ~ /^[0-9]+$/ && $3 ~ /^[0-9]+$/ {$2=t; $3=k} {print}' <<<"$new")
  done
  new=$(sed -E "s/^\| \*\*Total\*\* \| \*\*[0-9]+\*\* \| \*\*[0-9]+\*\* \|/| **Total** | **$total** | **$ticked** |/" <<<"$new")
  if [[ "$new" != "$body" ]]; then
    new=$(sed -E "s/as of [0-9]{4}-[0-9]{2}-[0-9]{2}\./as of $(date -u +%F)./" <<<"$new")
    cas_write $PARENT "$body" "$new" && echo "updated #$PARENT table: total=$total ticked=$ticked"
    body_of $PARENT > $S/body-$PARENT
  fi
}

for i in $WATCH; do [[ -e $S/refs-$i ]] || refs $i > $S/refs-$i; [[ -e $S/comments-$i ]] || comments $i > $S/comments-$i; done

while true; do
  sync_table
  check_edits
  watch_prs
  for i in $WATCH; do
    cur=$(refs $i) && [[ -n $cur ]] && { comm -13 <(grep -v '^$' $S/refs-$i | sort) <(grep -v '^$' <<<"$cur" | sort) | grep -E "^(NixOS/nixpkgs|numtide/|zimbatm/)" | sed "s/^/#$i pr: /"; echo "$cur" > $S/refs-$i; }
    cur=$(comments $i) && { comm -13 <(grep -v '^$' $S/comments-$i | sort) <(grep -v '^$' <<<"$cur" | sort) | sed "s/^/#$i comment: /"; echo "$cur" > $S/comments-$i; }
  done
  cur=$(search_prs) && [[ -n $cur ]] && { [[ -e $S/search ]] || echo "$cur" > $S/search; comm -13 <(sort $S/search) <(sort <<<"$cur") | sed "s/^/search pr: /"; sort -u $S/search <(echo "$cur") -o $S/search; }
  sleep 300
done
