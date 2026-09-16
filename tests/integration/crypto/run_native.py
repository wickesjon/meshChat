"""Generate test-only UniFFI bindings and execute public crypto vectors natively."""
from pathlib import Path
import argparse
import os
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[3]
WORK = ROOT / '.work/mc022'


def run(*args, env):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, env=env, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('language', choices=['kotlin', 'swift'])
    args = parser.parse_args()
    system = platform.system()
    if args.language == 'swift' and system != 'Darwin':
        raise RuntimeError('Swift vector validation requires a Mac/Xcode host')
    env = os.environ.copy()
    for key, folder in [('CARGO_HOME', '.work/cargo'), ('RUSTUP_HOME', '.work/rustup'),
                        ('CARGO_TARGET_DIR', '.work/mc022/target'), ('TMPDIR', '.work/tmp'),
                        ('TEMP', '.work/tmp'), ('TMP', '.work/tmp')]:
        env[key] = str(ROOT / folder)
        Path(env[key]).mkdir(parents=True, exist_ok=True)
    # Match the repository's existing pinned host bindgen and core version.
    run('cargo', 'build', '--manifest-path', ROOT / 'tests/integration/Cargo.toml',
        '--locked', env=env)
    target = WORK / 'target/debug'
    library = target / {'Windows': 'meshchat_wire_checks.dll', 'Darwin': 'libmeshchat_wire_checks.dylib',
                        'Linux': 'libmeshchat_wire_checks.so'}[system]
    bindings = WORK / 'bindings'
    run('cargo', 'run', '--manifest-path', ROOT / 'src/core/Cargo.toml',
        '--features', 'bindgen', '--locked', '--bin', 'uniffi-bindgen', '--',
        'generate', '--library', library, '--crate', 'meshchat_wire_checks',
        '--language', args.language, '--out-dir', bindings, '--no-format', env=env)
    if args.language == 'kotlin':
        wrapper = ROOT / 'src/android' / ('gradlew.bat' if system == 'Windows' else 'gradlew')
        command = [wrapper] if system == 'Windows' else ['bash', wrapper]
        env['GRADLE_USER_HOME'] = str(ROOT / '.work/gradle')
        run(*command, '-p', ROOT / 'tests/integration/crypto', '--no-daemon',
            '--max-workers=2', 'test', env=env)
    else:
        cache = WORK / 'swift-module-cache'
        cache.mkdir(parents=True, exist_ok=True)
        binary = WORK / 'swift-wire-checks'
        run('xcrun', 'swiftc', '-swift-version', '6', '-warnings-as-errors', '-parse-as-library',
            '-module-cache-path', cache, '-I', bindings, '-Xcc',
            '-fmodule-map-file=' + str(bindings / 'meshchat_wire_checksFFI.modulemap'),
            bindings / 'meshchat_wire_checks.swift', ROOT / 'tests/integration/crypto/WireMain.swift',
            '-L', target, '-lmeshchat_wire_checks', '-Xlinker', '-rpath', '-Xlinker', target,
            '-o', binary, env=env)
        run(binary, ROOT, env=env)


if __name__ == '__main__':
    main()
