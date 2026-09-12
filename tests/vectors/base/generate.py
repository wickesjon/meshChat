"""Reproduce structural MC-006/008 vectors; no valid signatures/identity claims."""
from pathlib import Path
import struct
import sys

ROOT = Path(__file__).resolve().parent
SID = bytes.fromhex('1112131415161718')
ROOT_ID = bytes.fromhex('3132333435363738')
GENERAL = bytes.fromhex('bcf0eae3')
EVENT = bytes.fromhex('d91ae76a')

def u16(n): return struct.pack('>H', n)
def u32(n): return struct.pack('>I', n)
def sized(s): return bytes([len(s)]) + s
def packet(kind, payload, flags=0, ttl=None, channel=None):
    direct = kind in (2, 3, 7)
    ttl = (1 if direct else 7) if ttl is None else ttl
    channel = (bytes(4) if direct else EVENT if kind in (5, 8) or flags & 7 == 6 else GENERAL) if channel is None else channel
    return bytes([1, kind, flags, ttl]) + bytes.fromhex('0102030405060708') + SID + channel + u16(len(payload)) + payload

def chat(n=b'N', text=b'Hello'):
    return u32(123) + bytes([255]) + sized(n) + bytes.fromhex('83010203') + u16(len(text)) + text

def friend(included=1):
    return SID + bytes([included]) + (bytes([9])*32 if included == 1 else b'') + bytes(64)

def cred(label=b'Staff'):
    return b'\x01' + ROOT_ID + bytes([9])*32 + u32(10) + u32(1000) + sized(label) + bytes(64)

def organizer(included=1, label=b'Staff', pin=1, expiry=100):
    c = cred(label) if included else b''
    return bytes([pin]) + u32(expiry) + ROOT_ID + SID + bytes([included]) + u16(len(c)) + c + bytes(64)

def announce(digest=b''):
    return u32(123) + bytes([255, 4, 255]) + sized(b'N') + bytes.fromhex('83010203') + u16(len(digest)) + digest

def encrypted(size):
    return b'\x01' + bytes([7])*8 + b'\x09' + bytes(31) + bytes(size-41)

rows = []
def add(name, value, expected='ok', context='live'):
    rows.append((name, context, expected, value.hex() or '-'))

add('chat', packet(1, chat()))
add('friend-included', packet(1, chat()+friend(),2))
add('friend-omitted', packet(1, chat()+friend(0),2))
add('organizer-included', packet(1,chat()+organizer(),6))
add('organizer-omitted', packet(1,chat()+organizer(0,pin=0,expiry=0),6))
add('announce',packet(2,announce()))
add('signed-announce',packet(2,announce(bytes(256))+friend(),2))
add('sync-request',packet(3,u16(1)+u16(5000)+u32(0)+bytes(512)))
add('event-info',packet(5,ROOT_ID+sized(b'Event')))
add('reaction',packet(6,bytes.fromhex('21222324252627280000')))
add('reaction-unknown-palette',packet(6,bytes.fromhex('2122232425262728feff'),0x80))
add('credential-request',packet(7,ROOT_ID+SID))
add('credential-offer',packet(8,cred()))
add('credential-empty-label',packet(8,cred(b'')))
for size in (121,201,361): add('encrypted-chat-'+str(size),packet(1,encrypted(size),1))
add('encrypted-reaction',packet(6,encrypted(121),1))
add('chat-utf8',packet(1,chat('é'.encode(), '🌻'.encode())))
add('chat-raw-controls',packet(1,chat(text=b'a\x00b')))
add('unknown-empty',packet(9,b'',0xff))
add('unknown-maximum',packet(255,bytes(998),0xff))
add('chat-stored-ttl-zero',packet(1,chat(),ttl=0),context='stored')
add('flood-high-ttl',packet(6,bytes(10),ttl=255))
add('flood-ttl-one',packet(6,bytes(10),ttl=1))
add('max-chat',packet(1,chat(b'n'*20,b't'*280)))
add('max-friend',packet(1,chat(b'n'*20,b't'*280)+friend(),2))
add('max-organizer',packet(1,chat(b'n'*20,b't'*280)+organizer(label=b's'*16),6))
add('max-announce',packet(2,u32(1)+bytes(3)+sized(b'n'*20)+bytes(4)+u16(256)+bytes(256)+friend(),2))
add('max-event',packet(5,ROOT_ID+sized(b'e'*32)))
add('max-credential',packet(8,cred(b's'*16)))

add('empty',b'','length')
add('chat-trailing-payload',packet(1,chat()+b'!'),'length')
add('reaction-short',packet(6,bytes(9)),'length')
add('reaction-long',packet(6,bytes(11)),'length')
add('bad-nick-utf8',packet(1,chat(b'\xff')),'utf8')
add('bad-text-utf8',packet(1,chat(text=b'\xed\xa0\x80')),'utf8')
add('empty-nickname',packet(1,chat(b'')),'field')
add('long-nickname',packet(1,chat(b'x'*21)),'field')
add('empty-text',packet(1,chat(text=b'')),'field')
add('long-text',packet(1,chat(text=b'x'*281)),'field')
add('bad-digest',packet(2,announce(bytes(1))),'field')
add('announce-key-omitted',packet(2,announce()+friend(0),2),'field')
add('friend-included-invalid',packet(1,chat()+friend(2),2),'field')
add('friend-id-mismatch',packet(1,chat()+bytes(8)+friend()[8:],2),'field')
add('organizer-bad-pin',packet(1,chat()+organizer(pin=0,expiry=10),6),'field')
add('organizer-zero-expiry',packet(1,chat()+organizer(pin=1,expiry=0),6),'field')
add('organizer-bad-state',packet(1,chat()+organizer(pin=2),6),'field')
add('sync-count-over',packet(3,u16(1)+u16(5001)+u32(0)+bytes(512)),'field')
add('event-empty-name',packet(5,ROOT_ID+sized(b'')),'field')
add('event-long-name',packet(5,ROOT_ID+sized(b'e'*33)),'field')
add('event-utf8',packet(5,ROOT_ID+sized(b'\xff')),'utf8')
add('credential-label-long',packet(8,cred(b's'*17)),'length')
add('encrypted-old-length',packet(1,encrypted(122),1),'length')
add('encrypted-reaction-large',packet(6,encrypted(201),1),'length')
add('encrypted-bad-profile',packet(1,b'\x02'+encrypted(121)[1:],1),'field')
p = bytearray(encrypted(121)); p[9:41] = bytes.fromhex('ed'+'ff'*30+'7f')
add('encrypted-enc-prime',packet(1,p,1),'field')
p[40] = 0x80
add('encrypted-enc-high-bit',packet(1,p,1),'field')
c=bytearray(cred()); c[0]=2
add('credential-version',packet(8,c),'version')
c=bytearray(cred()); c[41:45]=u32(1001)
add('credential-reversed-time',packet(8,c),'field')
c=bytearray(cred()); c[50]=255
add('credential-utf8',packet(8,c),'utf8')
x=bytearray(organizer()); x[5+19+1]^=1
add('organizer-root-mismatch',packet(1,chat()+x,6),'field')
x=bytearray(organizer()); x[5+16]=0
add('organizer-included-length',packet(1,chat()+x,6),'field')
for kind in (0,4): add('reserved-'+str(kind),packet(kind,b''),'version')
for kind in (2,3,7):
    payload={2:announce(),3:u16(1)+u16(1)+u32(0)+bytes(512),7:ROOT_ID+SID}[kind]
    add('direct-ttl-'+str(kind),packet(kind,payload,ttl=7),'scope')
    add('direct-channel-'+str(kind),packet(kind,payload,channel=GENERAL),'scope')
    add('stored-control-'+str(kind),packet(kind,payload),'scope','stored')
for kind,payload in ((1,chat()),(6,bytes(10)),(9,b'')):
    add('live-zero-'+str(kind),packet(kind,payload,ttl=0),'scope')
add('stored-reaction',packet(6,bytes(10)),'scope','stored')
add('stored-unknown',packet(9,b''),'scope','stored')
for kind,payload,flags in ((1,chat()+organizer(),6),(5,ROOT_ID+sized(b'E'),0),(8,cred(),0)):
    add('wrong-event-channel-'+str(kind),packet(kind,payload,flags,channel=GENERAL),'scope')
for kind in (1,2,3,5,6,7,8):
    allowed={1:{0,1,2,6},2:{0,2},3:{0},5:{0},6:{0,1},7:{0},8:{0}}[kind]
    for flags in set(range(8))-allowed:
        for high in (0,0x80): add(f'flags-{kind}-{flags+high}',packet(kind,b'',flags+high),'flags')
assert len({r[0] for r in rows}) == len(rows)
content = '# id context result hex\n'+'\n'.join(' '.join(r) for r in rows)+'\n'
path = ROOT/'logical.txt'
if '--write' in sys.argv: path.write_text(content,encoding='utf-8')
else: assert path.read_text(encoding='utf-8') == content, 'vector drift; review before --write'
print(f'{len(rows)} structural vectors match MC-006/008 generator')
