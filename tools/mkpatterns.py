import csv, sys, re
csv.field_size_limit(10**9)
dl, out = sys.argv[1], sys.argv[2]
def pem_col(row):
    for k, v in row.items():
        if v and 'BEGIN CERTIFICATE' in v:
            return v.strip().strip("'") + '\n'
roots = []
for r in csv.DictReader(open(f'{dl}/ccadb/moz-included.csv', newline='', encoding='utf-8')):
    p = pem_col(r)
    if p: roots.append(p)
for f in sys.argv[3:]:
    roots += [m + '\n' for m in re.findall(r'-----BEGIN (?:TRUSTED )?CERTIFICATE-----.*?-----END (?:TRUSTED )?CERTIFICATE-----', open(f).read(), re.S)]
roots = [r.replace('TRUSTED CERTIFICATE', 'CERTIFICATE') for r in roots]
open(f'{out}/roots.pem', 'w').write(''.join(dict.fromkeys(roots)))
watch = []
want_root = re.compile(r'^(emSign (Root CA - G1|Root CA - C1|ECC Root CA - G3|ECC Root CA - C3)|AAA Certificate Services)$')
for r in csv.DictReader(open(f'{dl}/ccadb/moz-included.csv', newline='', encoding='utf-8')):
    if want_root.match(r['Common Name or Certificate Name']): watch.append(pem_col(r))
for r in csv.DictReader(open(f'{dl}/ccadb/moz-intermediates.csv', newline='', encoding='utf-8')):
    if r['Certificate Subject Common Name'] in ('emSign Root TLS CA - G3', 'emSign Root CS CA - G3'): watch.append(pem_col(r))
for r in csv.DictReader(open(f'{dl}/ccadb/revoked.csv', newline='', encoding='utf-8')):
    if r['Date of Revocation'].startswith('2026 Oct') and 'cACompromise' in r['RFC 5280 Revocation Reason Code']: watch.append(pem_col(r))
watch = [w for w in watch if w]
open(f'{out}/watch.pem', 'w').write(''.join(watch))
print('roots', len(dict.fromkeys(roots)), 'watch', len(watch))
