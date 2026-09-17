// This page only prepares a link. The native core validates and confirms trust.
export function proposal(path, search, hash, words) {
  if (search || hash || path.length > 2000 || !/^\/[\x21-\x7e]+$/.test(path)) return null;
  const parts = path.split('/');
  if (parts[1] === 'j' && parts.length === 3) {
    const triple = parts[2].toLowerCase().split('-');
    if (triple.length !== 3 || !triple.every((word, i) => words[i].includes(word))) return null;
    const name = triple.join('-');
    return {kind: 'Shared channel', name, uri: 'meshfest://j/' + name,
      warning: 'Anyone with these words can read this channel. It is not encrypted. Review the words and confirm inside the app.'};
  }
  if (parts[1] !== 'friend' || parts.length !== 4) return null;
  const bundle = parts[2];
  // 65 bytes -> 87 unpadded base64url characters, with two zero pad bits.
  if (!/^[A-Za-z0-9_-]{87}$/.test(bundle)) return null;
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';
  if ((alphabet.indexOf(bundle.at(-1)) & 3) !== 0) return null;
  const bytes = Uint8Array.from(atob(bundle.replace(/-/g, '+').replace(/_/g, '/') + '='), c => c.charCodeAt(0));
  if (bytes.length !== 65 || bytes[0] !== 1 || !/^(?:[A-Za-z0-9._~-]|%[A-Fa-f0-9]{2})+$/.test(parts[3])) return null;
  let name;
  try { name = decodeURIComponent(parts[3]); } catch { return null; }
  if (new TextEncoder().encode(name).length < 1 || new TextEncoder().encode(name).length > 20 || /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}]/u.test(name)) return null;
  return {kind: 'Shared friend code', name: 'They broadcast as “' + name + '”', uri: 'meshfest://friend/' + bundle + '/' + parts[3],
    warning: 'This name is a claim. A link received online may be someone else’s code. The app checks the keys; compare the full fingerprint in person before pinning. Nothing is pinned here.'};
}
