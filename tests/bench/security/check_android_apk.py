"""Apply existing native alignment checks to the probe's SQLCipher/Rust/JNA set."""
import argparse
from pathlib import Path
import runpy
import zipfile

check_elf = runpy.run_path(str(Path(__file__).resolve().parents[2] /
                                'integration/ffi/check_android_apk.py'))['check_elf']

parser = argparse.ArgumentParser()
parser.add_argument('apks', nargs='+')
args = parser.parse_args()
for path in args.apks:
    with zipfile.ZipFile(path) as apk:
        names = {name for name in apk.namelist() if name.startswith('lib/') and name.endswith('.so')}
        expected = {f'lib/{abi}/{library}.so'
                    for abi in ['arm64-v8a', 'x86_64']
                    for library in ['libmeshchat_core', 'libjnidispatch', 'libsqlcipher']}
        if names != expected:
            raise ValueError(f'{path}: unexpected native library set: {sorted(names)}')
        for name in sorted(names):
            check_elf(apk.read(name), name)
    print(f'{path}: SQLCipher/Rust/JNA both ABIs have 16 KB LOAD and RELRO alignment')
