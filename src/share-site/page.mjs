import config from './config.mjs';
import {proposal} from './share.mjs';
const byId = id => document.getElementById(id);
const value = proposal(location.pathname, location.search, location.hash, config.words);
if (value) {
  byId('summary').textContent = 'Your code is ready to review in the app.';
  byId('kind').textContent = value.kind;
  byId('name').textContent = value.name;
  byId('warning').textContent = value.warning;
  byId('open').href = value.uri;
  byId('proposal').hidden = false;
} else {
  byId('summary').textContent = 'This is not a supported channel or friend link. Ask for the original QR or channel words. Nothing has been joined or pinned.';
}
let stores = 0;
for (const [id, key] of [['android', 'android_store_url'], ['ios', 'ios_store_url']]) {
  if (config[key]) {byId(id).href = config[key];byId(id).hidden = false;stores++;}
}
if (stores) byId('installation').textContent = 'Installation needs internet. After installing, return to this same link or scan the original QR; your code is not transferred through the store.';
