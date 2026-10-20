import csv, sys
csv.field_size_limit(10**9)
for r in csv.DictReader(open(sys.argv[1], newline='', encoding='utf-8')):
    if 'cACompromise' in r['RFC 5280 Revocation Reason Code']:
        print('\t'.join([r['Date of Revocation'], r['CA Owner'], r['Certificate Subject Common Name'], r['Certificate Issuer Common Name'], r['OneCRL Status'], r['SHA-256 Fingerprint'][:16]]))
