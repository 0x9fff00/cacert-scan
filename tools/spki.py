import csv, sys, subprocess
csv.field_size_limit(10**9)
for r in csv.DictReader(open(sys.argv[1], newline='', encoding='utf-8')):
    if r['Date of Revocation'].startswith('2026 Oct') and 'cACompromise' in r['RFC 5280 Revocation Reason Code']:
        pem = r['PEM Info'].strip("'")
        der = subprocess.run(['openssl','x509','-pubkey','-noout'], input=pem.encode(), capture_output=True).stdout
        h = subprocess.run('openssl pkey -pubin -outform DER | openssl dgst -sha256 -r', shell=True, input=der, capture_output=True).stdout.decode().split()[0]
        print(h, r['CA Owner'][:12], r['Certificate Subject Common Name'], '<-', r['Certificate Issuer Common Name'])
