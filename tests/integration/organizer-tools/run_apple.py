"""Run tool-generated organizer bundles through Swift and the canonical importer."""
import os
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[3]
if platform.system() != 'Darwin':
    raise RuntimeError('Requires Mac/Xcode')
out = ROOT / '.work/mc041/swift'
out.mkdir(parents=True, exist_ok=True)
cache = out / 'module-cache'
cache.mkdir(exist_ok=True)
ffi = ROOT / '.work/security-ffi/swift'
library = ROOT / '.work/security-target/debug'
args = ['xcrun', 'swiftc', '-swift-version', '6', '-warnings-as-errors', '-parse-as-library',
        '-module-cache-path', str(cache), '-I', str(ffi),
        '-Xcc', '-fmodule-map-file=' + str(ffi / 'meshchat_coreFFI.modulemap'),
        str(ffi / 'meshchat_core.swift'), str(Path(__file__).with_name('FixtureSql.swift')),
        str(Path(__file__).with_name('ImportMain.swift')), '-L', str(library), '-lmeshchat_core',
        '-Xlinker', '-rpath', '-Xlinker', str(library), '-o', str(out / 'imports')]
env = os.environ.copy()
env['TMPDIR'] = str(ROOT / '.work/tmp')
subprocess.run(args, cwd=ROOT, env=env, check=True)
subprocess.run([str(out / 'imports')], cwd=ROOT, env=env, check=True)
