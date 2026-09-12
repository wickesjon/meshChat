"""Independent MC-010 outer-frame assembly, standard library only."""
from pathlib import Path
import struct
import sys
ROOT=Path(__file__).resolve().parent
logical={p[0]:bytes.fromhex(p[3]) for line in (ROOT/'logical.txt').read_text().splitlines() if not line.startswith('#') and (p:=line.split())[2]=='ok'}
def u16(n): return struct.pack('>H',n)
def outer(kind,body): return bytes([kind,0])+u16(len(body))+body
rows=[]
def add(id,frame): rows.append((id,frame.hex()))
add('reaction',outer(0,logical['reaction']))
add('terminal',outer(2,bytes.fromhex('010001000005000000000000')))
hello=bytes.fromhex('010000920200')+b'\x01'+bytes(15)+b'\x09'+bytes(31)
add('hello',outer(2,b'\x02'+hello))
add('proof',outer(2,bytes.fromhex('030101')+bytes(64)))
raw=logical['unknown-maximum']
for i in range(8):
    add(f'logical-{i}',outer(1,raw[4:12]+u16(99)+bytes([i,8])+u16(len(raw))+raw[i*128:(i+1)*128]))
raw=logical['max-chat'];body=bytes.fromhex('000100000000000000')+u16(len(raw))+raw
for i in range(3):
    add(f'sync-{i}',outer(3,b'\x01'+u16(88)+bytes([i,3])+u16(len(body))+bytes(5)+body[i*130:(i+1)*130]))
content='# id hex (capacity146, structural only)\n'+'\n'.join(' '.join(r) for r in rows)+'\n'
path=ROOT/'frames.txt'
if '--write' in sys.argv:path.write_text(content,encoding='utf-8')
else:assert path.read_text(encoding='utf-8')==content,'frame vector drift'
print(f'{len(rows)} independent outer frame vectors match')
