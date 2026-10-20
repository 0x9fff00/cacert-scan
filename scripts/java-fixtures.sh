#!/usr/bin/env bash
# usage: java-fixtures.sh WORKDIR SCANNER PATTERNDIR — builds Java truststores in every format and scans them
set -eu
w=$1 scan=$2 pat=$3
rm -rf "$w"; mkdir -p "$w"/src "$w"/out "$w"/jar/certs; cd "$w"
B=$(nix build --no-link --print-out-paths nixpkgs#cacert)/etc/ssl/certs/ca-no-trust-rules-bundle.crt
awk '/BEGIN CERT/{n++; f=sprintf("src/c%03d.pem",n); on=1} on{print >> f} /END CERT/{on=0; close(f)}' "$B"
ls src | head -24 > pick
for f in src/*.pem; do openssl x509 -in "$f" -noout -subject | grep -q 'emSign ECC Root CA - G3' && basename "$f" >> pick; done
echo "certs per store: $(wc -l < pick)"
imp() {
  local kt=$1 st=$2 out=$3 pw=$4; shift 4
  for c in $(cat pick); do $kt "$@" -importcert -noprompt -storetype "$st" -keystore "$out" -storepass "$pw" -alias "$c" -file "src/$c" >/dev/null; done
}
imp "$JDK/bin/keytool" JKS out/trust.jks changeit
imp "$JDK/bin/keytool" JCEKS out/trust.jceks changeit
imp "$JDK/bin/keytool" PKCS12 out/trust-aes.p12 changeit
imp "$JDK/bin/keytool" PKCS12 out/trust-legacy.p12 changeit -J-Dkeystore.pkcs12.legacy
imp "$JDK/bin/keytool" PKCS12 out/trust-nopass.p12 changeit -J-Dkeystore.pkcs12.certProtectionAlgorithm=NONE -J-Dkeystore.pkcs12.macAlgorithm=NONE
imp "$JDK/bin/keytool" PKCS12 out/trust-unknownpw.p12 hunter22
imp "$JDK8/bin/keytool" JKS out/jdk8-cacerts changeit
cp out/trust-aes.p12 jar/certs/truststore.p12; cp out/trust.jks jar/certs/
(cd jar && zip -qr ../out/app.jar .)
"$scan" --roots "$pat/roots.pem" --watch "$pat/watch.pem" --path out > res.jsonl
jq -r '[(.file|sub(".*/out/";"")), (.magic//"-"), (.formats|join(",")), (.known_roots|length), (if (.watch|index("emSign ECC Root CA - G3")) then "eccG3" else "-" end), (.note//"")] | @tsv' res.jsonl | column -t -s $'\t'
