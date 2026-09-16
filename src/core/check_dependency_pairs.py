"""Enforce the exact MC-008 approved duplicate pairs, not open-ended skips."""
from pathlib import Path
import os, re, subprocess
root = Path(__file__).resolve().parents[2]
approved = {
    'block-buffer': {'0.10.4', '0.12.1'},
    'cpufeatures': {'0.2.17', '0.3.1'},
    'crypto-common': {'0.1.7', '0.2.2'},
    'curve25519-dalek': {'4.1.3', '5.0.0'},
    'digest': {'0.10.7', '0.11.3'},
    'fiat-crypto': {'0.2.9', '0.3.0'},
    'rand_core': {'0.6.4', '0.10.1'},
    'sha2': {'0.10.9', '0.11.0'},
    'x25519-dalek': {'2.0.1', '3.0.0'},
}
result = subprocess.run([os.environ.get('CARGO', 'cargo'), 'tree', '--manifest-path', str(root/'src/core/Cargo.toml'),
    '--all-features', '--locked', '--target', 'all', '--prefix', 'none', '--format', '{p}'],
    cwd=root, check=True, capture_output=True, text=True)
versions = {}
for line in result.stdout.splitlines():
    match = re.match(r'^(\S+) v(\S+)', line)
    if match:
        versions.setdefault(match[1], set()).add(match[2])
actual = {name: values for name, values in versions.items() if len(values)>1}
if actual != approved:
    raise SystemExit(f'Dependency scope changed: expected {approved}, found {actual}')
print('Exact nine MC-008 duplicate-version pairs verified.')
