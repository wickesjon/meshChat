"""Android host/toolchain selection; all temporary inputs stay inside the repo."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('build_bindings', ROOT / 'src/core/build_bindings.py')
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class AndroidLinkerTests(unittest.TestCase):
    def setUp(self):
        temporary = ROOT / '.work/tmp'
        temporary.mkdir(parents=True, exist_ok=True)
        self.directory = tempfile.TemporaryDirectory(dir=temporary, prefix='ndk test ')
        self.addCleanup(self.directory.cleanup)
        self.sdk = Path(self.directory.name) / 'SDK with spaces'

    def compiler(self, host, name):
        path = self.sdk / 'ndk/27.3.13750724/toolchains/llvm/prebuilt' / host / 'bin' / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.touch()
        return path

    def test_windows_both_abis_use_executable_and_android_api(self):
        linker = self.compiler('windows-x86_64', 'clang.exe')
        for target in ['aarch64-linux-android', 'x86_64-linux-android']:
            with self.subTest(target=target), patch.dict(os.environ, {'MC046_SENTINEL': 'kept'}):
                before = dict(os.environ)
                env = build.android_linker_env(str(self.sdk), 'Windows', target, target + '29-clang')
                prefix = 'CARGO_TARGET_' + target.upper().replace('-', '_')
                self.assertEqual(env[prefix + '_LINKER'], str(linker))
                self.assertIn('SDK with spaces', env[prefix + '_LINKER'])
                self.assertIn('link-arg=--target=' + target + '29', env[prefix + '_RUSTFLAGS'])
                self.assertIn('max-page-size=16384', env[prefix + '_RUSTFLAGS'])
                self.assertIn('common-page-size=16384', env[prefix + '_RUSTFLAGS'])
                self.assertEqual(env['MC046_SENTINEL'], 'kept')
                self.assertEqual(dict(os.environ), before)

    def test_linux_and_macos_keep_existing_wrapper_and_flags(self):
        for system, host in [('Linux', 'linux-x86_64'), ('Darwin', 'darwin-x86_64')]:
            for target in ['aarch64-linux-android', 'x86_64-linux-android']:
                with self.subTest(system=system, target=target):
                    name = target + '29-clang'
                    linker = self.compiler(host, name)
                    env = build.android_linker_env(str(self.sdk), system, target, name)
                    prefix = 'CARGO_TARGET_' + target.upper().replace('-', '_')
                    self.assertEqual(env[prefix + '_LINKER'], str(linker))
                    self.assertEqual(env[prefix + '_RUSTFLAGS'],
                                     '-C link-arg=-Wl,-z,max-page-size=16384 '
                                     '-C link-arg=-Wl,-z,common-page-size=16384')

    def test_missing_sdk_ndk_and_unsupported_host_fail_clearly(self):
        args = ('aarch64-linux-android', 'aarch64-linux-android29-clang')
        with self.assertRaisesRegex(ValueError, 'Set ANDROID_HOME'):
            build.android_linker_env(None, 'Windows', *args)
        with self.assertRaisesRegex(ValueError, 'Missing pinned Android NDK compiler'):
            build.android_linker_env(str(self.sdk), 'Windows', *args)
        with self.assertRaisesRegex(ValueError, 'Unsupported Android build host'):
            build.android_linker_env(str(self.sdk), 'unsupported', *args)


if __name__ == '__main__':
    unittest.main()
