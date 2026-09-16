// Published/deterministic TEST seeds only. Node/OpenSSL is independent of dalek.
// node tests/vectors/crypto/organizer_vectors.cjs > .work/organizer-vectors.tsv
const {createPrivateKey,createPublicKey,createHash,sign,verify}=require('node:crypto');
const B=Buffer, cat=(...p)=>B.concat(p), hex=h=>B.from(h,'hex');
const u16=n=>{const b=B.alloc(2);b.writeUInt16BE(n);return b;};
const u32=n=>{const b=B.alloc(4);b.writeUInt32BE(n);return b;};
const u64=n=>{const b=B.alloc(8);b.writeBigUInt64BE(BigInt(n));return b;};
const D=s=>B.from(`meshfest/${s}/v1\0`);
function key(seed){const priv=createPrivateKey({key:cat(hex('302e020100300506032b657004220420'),seed),format:'der',type:'pkcs8'});const pub=createPublicKey(priv).export({format:'der',type:'spki'}).subarray(-32);return {priv,pub,id:createHash('sha256').update(pub).digest().subarray(0,8)};}
function signed(k,data){const sig=sign(null,data,k.priv);if(!verify(null,data,createPublicKey(k.priv),sig))throw Error('OpenSSL verification');return sig;}
const rfc=key(hex('9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60'));
if(rfc.pub.toString('hex')!=='d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'||signed(rfc,B.alloc(0)).toString('hex')!=='e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b')throw Error('RFC8032 section7.1 test1 failed');
const root=key(B.alloc(32,9)), own=key(B.alloc(32,2));
const bundleBody=cat(B.from([1]),root.pub,u32(202000));
const bundle=cat(bundleBody,signed(root,cat(D('event-root'),bundleBody)));
function credential(staff){const c=cat(B.from([1]),root.id,staff.pub,u32(199990),u32(201000),B.from([3]),B.from('Ops'));return cat(c,signed(root,cat(D('credential'),u16(c.length),c)));}
function chat(staff,id,include,pin,domain='staff-sign'){
 const cred=credential(staff);const body=cat(u32(200000),B.from([0,1,65,0,0,0,0]),u16(5),B.from('hello'),B.from([pin>0?1:0]),u32(pin),root.id,staff.id,B.from([include?1:0]),u16(include?cred.length:0),include?cred:B.alloc(0));
 const header=cat(B.from([1,1,6,7]),u64(id),own.id,hex('d91ae76a'),u16(body.length+64));
 return cat(header,body,signed(staff,cat(D(domain),header.subarray(0,3),header.subarray(4),u16(body.length),body)));
}
const a=key(B.alloc(32,7)),b=key(B.alloc(32,8));
const cases={root_bundle:bundle,credential_a:credential(a),credential_b:credential(b),chat_a:chat(a,1,true,0),chat_b:chat(b,2,true,0),omitted_a:chat(a,3,false,0),pinned_a:chat(a,4,true,200100),wrong_domain:chat(a,5,true,0,'friend-sign')};
for(const [name,bytes]of Object.entries(cases))console.log(name+'\t'+bytes.toString('hex'));
console.error(`RFC8032 and profile fixtures generated with Node ${process.versions.node}, OpenSSL ${process.versions.openssl}`);
