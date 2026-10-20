import csv, sys
csv.field_size_limit(10**9)
for r in csv.DictReader(open(sys.argv[1], newline='', encoding='utf-8')):
    n = r['Certificate Name']
    if 'emSign' in n or 'EM ' in n[:3] or 'eMudhra' in r['CA Owner']:
        print('\t'.join([r['Certificate Record Type'], n, '<-', r['Parent Certificate Name'], 'moz='+r['Mozilla Status'], 'chrome='+r['Chrome Status'], r['Revocation Status'], r['SHA-256 Fingerprint'][:16]]))
