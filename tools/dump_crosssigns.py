import csv, sys, os
csv.field_size_limit(10**9)
for r in csv.DictReader(open(sys.argv[1], newline='', encoding='utf-8')):
    if r['Date of Revocation'].startswith('2026 Oct') and 'cACompromise' in r['RFC 5280 Revocation Reason Code']:
        n = ('sectigo-' if r['CA Owner']=='Sectigo' else 'emudhra-') + r['Certificate Subject Common Name'].replace(' ','_')
        open(os.path.join(sys.argv[2], n+'.pem'),'w').write(r['PEM Info'].strip("'")+'\n')
