"""Independent SHA-256/channel glyph fixtures from the normative slot table."""
from pathlib import Path
import hashlib,itertools,sys
ROOT=Path(__file__).resolve().parents[3]
s=(ROOT/'docs/mesh-chat-design.md').read_text(encoding='utf-8')
a=s.index('| melodic | techno | valley |');b=s.index('The combos read',a)
rows=[[v.strip() for v in l.split('|')[1:-1]] for l in s[a:b].splitlines() if l.startswith('|')]
assert len(rows)==20 and all(len(r)==3 for r in rows)
slots=list(zip(*rows));glyphs=['moon','sun','star','mushroom','crystal','spiral','comet','cactus','disco-ball']
entries=[];seen={}
for name,glyph in [('#general','wave'),('#confessions','fire'),('#event updates','lightning-bolt')]+[('|'.join(words),None) for words in itertools.product(*slots)]:
    digest=hashlib.sha256(('meshfest-v1|'+name).encode()).hexdigest()[:8]
    assert digest not in seen, f'COLLISION: {name} / {seen.get(digest)}; requires explicit decision'
    seen[digest]=name
    entries.append('\t'.join([name,digest,glyph or glyphs[int(digest,16)%9]]))
content='# canonical_name\tid_hex\tglyph\n'+'\n'.join(entries)+'\n'
p=Path(__file__).with_name('channels.tsv')
if '--write' in sys.argv:p.write_text(content,encoding='utf-8')
else:assert p.read_text(encoding='utf-8')==content,'channel vector drift'
print('8,003 unique IDs: 8,000 private triples, three public channels; no collisions')

# Public, deliberately unauthenticated binary proposals. Never persist staff seeds.
import base64,urllib.parse
bundle=b'\x01'+bytes([9])*64
event=b'\x01'+bytes([9])*100
b64=lambda b:base64.urlsafe_b64encode(b).rstrip(b'=').decode()
urls=[]
def add(name,url,valid):urls.append(f'{name}\t{str(valid).lower()}\t{url}')
for name,data,route in [('friend',bundle,'friend'),('event',event,'event')]:
    for scheme in ['meshfest://'+route+'/','https://meshfest.app/'+route+'/']:
        url=scheme+b64(data)+'/N'
        add(name+'-'+str(len(url)),url,True)
        for suffix in ['=','%3D','/','?x=1','#x']:
            add(name+'-suffix-'+str(len(url))+'-'+str(len(urls)),url+suffix,suffix=='%3D')
    add(name+'-unicode','meshfest://'+route+'/'+b64(data)+'/'+urllib.parse.quote('é🌻',safe=''),True)
    add(name+'-version','meshfest://'+route+'/'+b64(b'\x02'+data[1:])+'/N',False)
    add(name+'-length','meshfest://'+route+'/'+b64(data+b'\x00')+'/N',False)
content='# id\tvalid_structural_proposal\turl\n'+'\n'.join(urls)+'\n'
p=Path(__file__).with_name('public-links.tsv')
if '--write' in sys.argv:p.write_text(content,encoding='utf-8')
else:assert p.read_text(encoding='utf-8')==content,'public link vector drift'
print(f'{len(urls)} public link grammar vectors match; no staff seeds persisted')
