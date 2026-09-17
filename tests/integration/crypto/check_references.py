"""Reproduce public reference bytes and record a reviewable input inventory."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--node', default='node')
    args = parser.parse_args()
    for family in ('friend', 'dm', 'organizer'):
        output = subprocess.check_output(
            [args.node, str(ROOT / f'tests/vectors/crypto/{family}_vectors.cjs')], cwd=ROOT, text=True)
        expected = (ROOT / f'tests/vectors/crypto/{family}-v1.tsv').read_text()
        if output.splitlines() != expected.splitlines():
            raise RuntimeError(f'{family} independent reference differs from committed bytes')
    def packages(path):
        return {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                for p in tomllib.loads(path.read_text())['package']}
    additions = packages(ROOT / 'tests/integration/Cargo.lock') - packages(ROOT / 'Cargo.lock')
    if additions != {('meshchat-wire-checks', '0.1.0', None, None)}:
        raise RuntimeError(f'Test facade changed selected dependency packages: {additions}')
    paths = [ROOT / 'Cargo.lock', ROOT / 'tests/integration/Cargo.lock',
             ROOT / 'docs/decisions/MC-006-wire-contract.md',
             ROOT / 'docs/decisions/MC-008-crypto-contract.md']
    for folder, suffix in [('src/core/src', '.rs'), ('tests/vectors/crypto', '.tsv'),
                           ('tests/vectors/crypto', '.cjs')]:
        paths.extend((ROOT / folder).rglob('*' + suffix))
    inventory = {str(p.relative_to(ROOT)).replace('\\', '/'): hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in sorted(paths)}
    report = ROOT / '.work/mc022/reference-inputs.json'
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps({'revision': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'working_tree_changes': subprocess.check_output(
            ['git', 'status', '--porcelain'], cwd=ROOT, text=True).splitlines(),
        'node': subprocess.check_output([args.node, '-p',
            'process.version + " / OpenSSL " + process.versions.openssl'], text=True).strip(),
        'sha256': inventory}, indent=2) + '\n')
    print('Three independent reference generators match; test dependency packages preserved.')
    print(f'Input inventory: {report}')


if __name__ == '__main__':
    main()
