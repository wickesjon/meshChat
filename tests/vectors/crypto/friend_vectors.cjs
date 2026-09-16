// Public, deterministic TEST seeds only; never use these identities outside tests.
// Run: node tests/vectors/crypto/friend_vectors.cjs > .work/friend-vectors.tsv
// Node crypto uses OpenSSL Ed25519, independently of Rust/dalek.
const { createPrivateKey, createPublicKey, createHash, sign, verify } = require('node:crypto');
const from = h => Buffer.from(h, 'hex');
const cat = (...b) => Buffer.concat(b);
const word = n => { const b = Buffer.alloc(2); b.writeUInt16BE(n); return b; };
function identity(seed) {
  const privateKey = createPrivateKey({ key: cat(from('302e020100300506032b657004220420'), seed), format: 'der', type: 'pkcs8' });
  const publicKey = createPublicKey(privateKey).export({ format: 'der', type: 'spki' }).subarray(-32);
  return { privateKey, publicKey, hint: createHash('sha256').update(publicKey).digest().subarray(0, 8) };
}
const own = identity(Buffer.alloc(32, 41));
const seed = Buffer.alloc(32); seed.writeUInt16BE(2);
const peer = identity(seed);
function signed(key, bytes) {
  const signature = sign(null, bytes, key.privateKey);
  if (!verify(null, bytes, createPublicKey(key.privateKey), signature)) throw Error('OpenSSL verification failed');
  return signature;
}
function chat(key) {
  const body = cat(from('00030d4000014100000000000568656c6c6f'), key.hint, from('01'), key.publicKey);
  const header = cat(from('010102070000000000000001'), key.hint, from('01020304'), word(body.length + 64));
  const transcript = cat(Buffer.from('meshfest/friend-sign/v1\0'), header.subarray(0,3), header.subarray(4), word(body.length), body);
  return cat(header, body, signed(key, transcript));
}
const local = cat(from('010002000200'), Buffer.alloc(16,1), own.publicKey);
const remote = cat(from('010102000200'), Buffer.alloc(16,2), peer.publicKey);
function proof(key, role) {
  const transcript = cat(Buffer.from('meshfest/link-proof/v1\0'), Buffer.from([role]), local, remote);
  return cat(Buffer.from([1, role]), signed(key, transcript));
}
for (const [name, bytes] of Object.entries({peer_public:peer.publicKey, own_public:own.publicKey,
  peer_chat:chat(peer), own_chat:chat(own), local_hello:local, remote_hello:remote,
  peer_proof:proof(peer,1), own_proof:proof(own,0)})) console.log(name+'\t'+bytes.toString('hex'));
console.error(`Generated with Node ${process.versions.node}, OpenSSL ${process.versions.openssl}`);
