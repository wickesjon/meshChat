// TEST-ONLY RFC 9180 Auth reference using Node/OpenSSL primitives. Public,
// deterministic synthetic identities. Never import this into production.
const crypto=require('node:crypto'), fs=require('node:fs'), assert=require('node:assert/strict');
const B=x=>Buffer.from(x), hex=x=>Buffer.from(x,'hex'), cat=(...x)=>Buffer.concat(x);
const u16=n=>{const b=Buffer.alloc(2);b.writeUInt16BE(n);return b;};
const u32=n=>{const b=Buffer.alloc(4);b.writeUInt32BE(n);return b;};
const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64BE(BigInt(n));return b;};
const hash=b=>crypto.createHash('sha256').update(b).digest();
const hmac=(k,b)=>crypto.createHmac('sha256',k).update(b).digest();
const empty=Buffer.alloc(0), ks=cat(B('KEM'),u16(32)), suite=cat(B('HPKE'),u16(32),u16(1),u16(3));
function extract(suite,salt,label,input){return hmac(salt.length?salt:Buffer.alloc(32),cat(B('HPKE-v1'),suite,B(label),input));}
function expand(suite,prk,label,info,len){
 const input=cat(u16(len),B('HPKE-v1'),suite,B(label),info);let out=empty,previous=empty;
 for(let n=1;out.length<len;n++){previous=hmac(prk,cat(previous,input,Buffer.from([n])));out=cat(out,previous);}return out.subarray(0,len);
}
function key(seed,ed=false){return crypto.createPrivateKey({key:cat(hex(ed?'302e020100300506032b657004220420':'302e020100300506032b656e04220420'),seed),format:'der',type:'pkcs8'});}
const pub=k=>crypto.createPublicKey(k).export({format:'der',type:'spki'}).subarray(-32);
const publicX=p=>crypto.createPublicKey({key:cat(hex('302a300506032b656e032100'),p),format:'der',type:'spki'});
const dh=(sk,pk)=>crypto.diffieHellman({privateKey:key(sk),publicKey:publicX(pk)});
function setup(seed,skS,pkS,pkR,info,kciSkR){
 const skE=expand(ks,extract(ks,empty,'dkp_prk',seed),'sk',empty,32),enc=pub(key(skE));
 const dhs=cat(dh(skE,pkR),kciSkR?dh(kciSkR,pkS):dh(skS,pkR));
 const shared=expand(ks,extract(ks,empty,'eae_prk',dhs),'shared_secret',cat(enc,pkR,pkS),32);
 const context=cat(Buffer.from([2]),extract(suite,empty,'psk_id_hash',empty),extract(suite,empty,'info_hash',info));
 const secret=extract(suite,shared,'secret',empty);
 const aesKey=expand(suite,secret,'key',context,32),nonce=expand(suite,secret,'base_nonce',context,12);
 return {enc,nonce,seal(plain,aad){const c=crypto.createCipheriv('chacha20-poly1305',aesKey,nonce,{authTagLength:16});c.setAAD(aad,{plaintextLength:plain.length});return cat(c.update(plain),c.final(),c.getAuthTag());}};
}
const rfc=Object.fromEntries(fs.readFileSync(__dirname+'/hpke-auth-rfc9180.tsv','utf8').trim().split(/\r?\n/).map(l=>{const [k,v]=l.split('\t');return [k,hex(v)];}));
const r=setup(rfc.ikmE,rfc.skSm,rfc.pkSm,rfc.pkRm,rfc.info);
assert.deepEqual(r.enc,rfc.enc);assert.deepEqual(r.nonce,rfc.nonce);assert.deepEqual(r.seal(rfc.pt,rfc.aad),rfc.ct);
function tuple(n){const seed=Buffer.alloc(32,n);return cat(pub(key(seed,true)),pub(key(seed)));}
function packet(from,to,id,length,options={}){
 const s=tuple(from),recipient=tuple(to),seed=Buffer.alloc(32,7),skS=Buffer.alloc(32,from),skR=Buffer.alloc(32,to),stamp=200000;
 const type=options.reaction?6:1,pad=options.reaction?64:[64,144,304].find(n=>n>=6+length);
 let plain=Buffer.alloc(pad);u32(stamp).copy(plain);
 if(type===1){u16(length).copy(plain,4);plain.fill(0x61,6,6+length);}else{u64(1).copy(plain,4);plain[12]=0;plain[13]=7;}
 if(options.badPadding)plain[plain.length-1]=1;
 if(options.nonminimal){plain=cat(plain,Buffer.alloc(80));}
 if(options.badUtf8)plain[6]=255;
 const info=cat(B('meshfest/dm-info/v1\0'),hex('02002000010003'),s,recipient);
 const pair=dh(skS,recipient.subarray(32)),tag=hmac(pair,cat(B('meshfest-dmtag-v1'),u64(Math.floor(stamp/3600)))).subarray(0,4);
 const header=cat(Buffer.from([1,type,1,7]),u64(id),hash(s.subarray(0,32)).subarray(0,8),tag,u16(plain.length+57));
 const ctx=setup(seed,options.forge?Buffer.alloc(32,4):skS,s.subarray(32),recipient.subarray(32),info,options.kci?skR:undefined);
 const envelope=cat(Buffer.from([1]),hash(s.subarray(32)).subarray(0,8),ctx.enc);
 const aad=cat(B('meshfest/dm-aad/v1\0'),header.subarray(0,3),header.subarray(4),envelope);
 return cat(header,envelope,ctx.seal(plain,aad));
}
let id=1;
for(const [from,to] of [[2,3],[3,2]])for(const n of [1,58,59,138,139,280])console.log(`chat_${from}_${n}\t${packet(from,to,id++,n).toString('hex')}`);
for(const [name,opt] of Object.entries({reaction:{reaction:true},bad_padding:{badPadding:true},nonminimal:{nonminimal:true},bad_utf8:{badUtf8:true},sender_forgery:{forge:true}}))console.log(`${name}\t${packet(2,3,id++,1,opt).toString('hex')}`);
// Expected KCI limitation: the recipient can fabricate the same Auth transcript.
assert.deepEqual(packet(2,3,1,1,{kci:true}),packet(2,3,1,1));
console.error(`RFC Auth KAT and recipient-fabrication limitation reproduced; Node ${process.versions.node}, OpenSSL ${process.versions.openssl}`);
