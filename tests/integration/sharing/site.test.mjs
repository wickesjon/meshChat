import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {proposal} from '../../../src/share-site/share.mjs';
import config from '../../../.work/mc031/site/config.mjs';

for (const a of config.words[0]) for (const b of config.words[1]) for (const c of config.words[2]) {
  const words = `${a}-${b}-${c}`;
  assert.equal(proposal('/j/'+words, '', '', config.words).uri, 'meshfest://j/'+words);
}
const bytes = new Uint8Array(65).fill(31);bytes[0] = 1;
const bundle = Buffer.from(bytes).toString('base64url');
const path = '/friend/'+bundle+'/Bob%20%2F%20%C3%A9%25';
assert.equal(proposal(path, '', '', config.words).uri, 'meshfest:/'+path);
assert.equal(proposal(path, '', '', config.words).name, 'They broadcast as “Bob / é%”');
for (const bad of ['/staff/secret/seed', '/event/x/name', '/j/unknown-techno-valley', '/j/melodic-techno-valley/',
  '/J/melodic-techno-valley', '/j/melodic%2dtechno-valley', path+'/', '/friend/'+bundle+'=/Bob',
  '/friend/'+bundle+'/Bad%00', '/friend/'+bundle+'/%FF', '/friend/'+bundle+'/raw name',
  '/friend/'+bundle.slice(0,-1)+'9/Bob', '/friend/'+bundle+'/'+'x'.repeat(21)]) {
  assert.equal(proposal(bad, '', '', config.words), null, bad);
}
assert.equal(proposal('/j/melodic-techno-valley', '?join=1', '', config.words), null);
assert.equal(proposal(path, '', '#x', config.words), null);

// Execute the actual page module with a minimal DOM. No installed-app detection,
// automatic redirects, store transfer or network calls are supplied to this page.
async function page(pathname, fixture = {}) {
  const nodes = new Map();
  globalThis.document = {getElementById(id) {
    if (!nodes.has(id)) nodes.set(id, {hidden: true, textContent: '', href: ''});
    return nodes.get(id);
  }};
  globalThis.location = {pathname, search: '', hash: ''};
  const source = (await readFile(new URL('../../../src/share-site/page.mjs', import.meta.url), 'utf8'))
    .replace("import config from './config.mjs';", '')
    .replace("import {proposal} from './share.mjs';", '');
  new Function('config', 'proposal', source)({...config, ...fixture}, proposal);
  return nodes;
}
let nodes = await page('/j/melodic-techno-valley');
assert.equal(nodes.get('open').href, 'meshfest://j/melodic-techno-valley');
assert.equal(nodes.get('android'), undefined);
assert.equal(nodes.get('ios'), undefined);
assert.equal(nodes.get('proposal').hidden, false);
nodes = await page(path, {android_store_url: 'https://play.google.com/store/apps/details?id=org.example.meshchat'});
assert.equal(nodes.get('android').hidden, false);
assert.match(nodes.get('installation').textContent, /return to this same link/);
assert.equal(nodes.get('open').href, 'meshfest:/'+path);
nodes = await page('/staff/secret/seed');
assert.equal(nodes.get('open'), undefined);
assert.match(nodes.get('summary').textContent, /Nothing has been joined or pinned/);
console.log('PASS: 8000 channels, exact friend encoding, invalid inputs, installed-app proposal and uninstalled/store fallback fixtures');
