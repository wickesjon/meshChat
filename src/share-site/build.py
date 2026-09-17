"""Build local sharing fixtures. This command never deploys or verifies a domain."""
import argparse
import json
from pathlib import Path
import re
import shutil
from string import Template
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve().parent


def build(config, output):
    output = Path(output).resolve()
    if not output.is_relative_to((ROOT / '.work').resolve()):
        raise ValueError('Generated output must remain inside repository .work/')
    domain = config['domain']
    if not re.fullmatch(r'[a-z0-9]+(?:[.-][a-z0-9]+)*\.[a-z]{2,}', domain):
        raise ValueError('Expected a plain lowercase domain without path or port')
    package, fingerprints, apple = (config.get(k) for k in
                                   ['android_package', 'android_sha256', 'apple_app_id'])
    if package is not None and not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+', package):
        raise ValueError('Invalid Android package')
    if not isinstance(fingerprints, list) or any(not isinstance(f, str) or not re.fullmatch(r'(?:[0-9A-F]{2}:){31}[0-9A-F]{2}', f) for f in fingerprints):
        raise ValueError('Expected SHA-256 certificate fingerprints')
    if apple is not None and not re.fullmatch(r'[A-Z0-9]{10}\.[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+', apple):
        raise ValueError('Expected Apple app prefix and bundle ID')
    for key, host, prefix in [('android_store_url', 'play.google.com', '/store/apps/details'),
                              ('ios_store_url', 'apps.apple.com', '/')]:
        value = config.get(key)
        if value is not None:
            url = urlsplit(value)
            if url.scheme != 'https' or url.netloc != host or not url.path.startswith(prefix) or url.fragment:
                raise ValueError('Invalid store URL')
    output.mkdir(parents=True, exist_ok=True)
    for file in ['index.html', 'style.css', 'share.mjs', 'page.mjs']:
        shutil.copyfile(SOURCE / file, output / file)
    # Word lists are generated from the frozen core, never a second maintained dictionary.
    source = (ROOT / 'src/core/src/channel.rs').read_text(encoding='utf-8')
    words = [re.findall(r'"([a-z]+)"', re.search(r'pub const '+name+r':.*?= \[(.*?)\];', source, re.S).group(1))
             for name in ['DESCRIPTORS', 'GENRES', 'LOCATIONS']]
    if any(len(group) != 20 for group in words):
        raise ValueError('Core word-list layout changed; review generator')
    public = {key: config.get(key) for key in ['domain', 'android_store_url', 'ios_store_url']}
    public['words'] = words
    (output / 'config.mjs').write_text('export default '+json.dumps(public).replace('<', '\\u003c')+';\n', encoding='utf-8')
    associations = output / '.well-known'
    associations.mkdir(exist_ok=True)
    values = {key: json.dumps(value) for key, value in config.items()}
    for template, target, ready, empty in [
        ('assetlinks.json.in', 'assetlinks.json', package and fingerprints, []),
        ('apple-app-site-association.in', 'apple-app-site-association', apple, {'applinks': {'apps': [], 'details': []}}),
    ]:
        content = json.loads(Template((SOURCE / 'templates' / template).read_text()).substitute(values)) if ready else empty
        (associations / target).write_text(json.dumps(content, indent=2)+'\n', encoding='utf-8')
    native = output / 'native-templates'
    native.mkdir(exist_ok=True)
    for template in ['AssociatedDomains.entitlements.in', 'AndroidAppLinks.xml.in']:
        (native / template.removesuffix('.in')).write_text(Template((SOURCE / 'templates' / template).read_text()).substitute(domain=domain), encoding='utf-8')
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, default=SOURCE / 'config.example.json')
    parser.add_argument('--output', type=Path, default=ROOT / '.work/mc031/site')
    args = parser.parse_args()
    build(json.loads(args.config.read_text(encoding='utf-8')), args.output)
    print('Local fixtures built. Production association and installation remain unverified.')
