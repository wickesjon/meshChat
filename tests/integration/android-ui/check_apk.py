"""Require the complete production app native set and actual 16 KiB ELF alignment."""
from pathlib import Path
import runpy
import sys
import zipfile

check = runpy.run_path(str(Path(__file__).resolve().parents[1]/'ffi/check_android_apk.py'))['check_elf']
for path in sys.argv[1:]:
    with zipfile.ZipFile(path) as apk:
        actual = {n for n in apk.namelist() if n.startswith('lib/') and n.endswith('.so')}
        expected = {f'lib/{abi}/{name}.so' for abi in ['arm64-v8a','x86_64']
                    for name in ['libmeshchat_core','libjnidispatch','libsqlcipher','libandroidx.graphics.path']}
        if actual != expected:
            raise ValueError(f'{path}: unexpected native set: {sorted(actual)}')
        for name in sorted(actual):
            check(apk.read(name),name)
    print(f'{path}: both ABIs include Rust/JNA/SQLCipher/Compose graphics with 16 KiB LOAD/RELRO alignment')
