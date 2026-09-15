"""Build pinned Rust artifacts and generated bindings; all output stays in the repo."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / '.work' / 'ffi'


def run(*args, env=None):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, env=env, check=True)


def android_linker_env(sdk, system, target, compiler):
    hosts = {'Windows': 'windows-x86_64', 'Linux': 'linux-x86_64',
             'Darwin': 'darwin-x86_64'}
    if system not in hosts:
        raise ValueError(f'Unsupported Android build host: {system}')
    if not sdk:
        raise ValueError('Set ANDROID_HOME to the SDK containing NDK 27.3.13750724')
    ndk = Path(sdk) / 'ndk' / '27.3.13750724'
    compiler_dir = ndk / 'toolchains' / 'llvm' / 'prebuilt' / hosts[system] / 'bin'
    linker = compiler_dir / ('clang.exe' if system == 'Windows' else compiler)
    if not linker.is_file():
        raise ValueError(f'Missing pinned Android NDK compiler: {linker}')
    env = os.environ.copy()
    prefix = 'CARGO_TARGET_' + target.upper().replace('-', '_')
    env[prefix + '_LINKER'] = str(linker)
    flags = ('-C link-arg=-Wl,-z,max-page-size=16384 '
             '-C link-arg=-Wl,-z,common-page-size=16384')
    if system == 'Windows':
        # Direct Clang avoids batch-wrapper argument forwarding on Windows.
        # The API suffix is essential: otherwise Clang targets the Windows host.
        flags += ' -C link-arg=--target=' + compiler.removesuffix('-clang')
    env[prefix + '_RUSTFLAGS'] = flags
    return env


def main():
    global OUT
    parser = argparse.ArgumentParser()
    parser.add_argument('platform', choices=['host', 'android', 'ios'])
    parser.add_argument('--security-probe', action='store_true')
    args = parser.parse_args()
    if args.security_probe:
        OUT = ROOT / '.work' / 'security-ffi'
    target_dir = ROOT / ('.work/security-target' if args.security_probe else 'target')
    features = ['--features', 'security-probe'] if args.security_probe else []
    OUT.mkdir(parents=True, exist_ok=True)
    for name, suffix in [('CARGO_HOME', 'cargo'), ('RUSTUP_HOME', 'rustup'),
                         ('CARGO_TARGET_DIR', '../target'), ('TMPDIR', 'tmp'),
                         ('TEMP', 'tmp'), ('TMP', 'tmp')]:
        path = (ROOT / '.work' / suffix).resolve()
        path.mkdir(parents=True, exist_ok=True)
        os.environ[name] = str(path)
    os.environ['CARGO_TARGET_DIR'] = str(target_dir)
    run('cargo', 'build', '--locked', '-p', 'meshchat-core', '--features', 'bindgen', *features)
    system = platform.system()
    library = {'Windows': 'meshchat_core.dll', 'Darwin': 'libmeshchat_core.dylib',
               'Linux': 'libmeshchat_core.so'}[system]
    host_lib = target_dir / 'debug' / library
    for language in ['kotlin', 'swift']:
        run('cargo', 'run', '--locked', '-p', 'meshchat-core', '--features', 'bindgen', *features,
            '--bin', 'uniffi-bindgen', '--', 'generate', '--library', host_lib,
            '--language', language, '--out-dir', OUT / language,
            '--config', ROOT / 'src/core/uniffi.toml', '--no-format')
    if args.platform == 'android':
        for target, abi, compiler in [
            ('aarch64-linux-android', 'arm64-v8a', 'aarch64-linux-android29-clang'),
            ('x86_64-linux-android', 'x86_64', 'x86_64-linux-android29-clang'),
        ]:
            env = android_linker_env(os.environ.get('ANDROID_HOME'), system, target, compiler)
            run('rustup', 'target', 'add', target)
            run('cargo', 'build', '--locked', '--lib', '--release', '--target', target, *features, env=env)
            destination = OUT / 'android' / abi
            destination.mkdir(parents=True, exist_ok=True)
            shutil.copy2(target_dir / target / 'release' / 'libmeshchat_core.so', destination)
    elif args.platform == 'ios':
        for target in ['aarch64-apple-ios', 'aarch64-apple-ios-sim', 'x86_64-apple-ios']:
            run('rustup', 'target', 'add', target)
            env = os.environ.copy()
            env['IPHONEOS_DEPLOYMENT_TARGET'] = '15.0'
            run('cargo', 'build', '--locked', '--lib', '--release', '--target', target, *features, env=env)
        for sdk in ['iphoneos', 'iphonesimulator']:
            (OUT / sdk).mkdir(exist_ok=True)
        shutil.copy2(target_dir / 'aarch64-apple-ios/release/libmeshchat_core.a', OUT / 'iphoneos')
        run('xcrun', 'lipo', '-create',
            target_dir / 'aarch64-apple-ios-sim/release/libmeshchat_core.a',
            target_dir / 'x86_64-apple-ios/release/libmeshchat_core.a',
            '-output', OUT / 'iphonesimulator/libmeshchat_core.a')
        if args.security_probe:
            # Exercise the production provider with test-only native storage/wrapping.
            # Real Secure Enclave use remains the physical-device gate.
            run(sys.executable, '-B', ROOT / 'tests/integration/identity/run_apple.py')


if __name__ == '__main__':
    main()
