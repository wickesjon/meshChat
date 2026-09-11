"""Regression checks for cleanup boundaries; no real file is deleted."""

from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import prepare_emulator_disk as cleanup


class CleanupBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.enterContext(patch.object(cleanup.sys, 'platform', 'linux'))
        self.enterContext(patch.dict(cleanup.os.environ, {
            'GITHUB_ACTIONS': 'true', 'GITHUB_WORKSPACE': str(cleanup.ROOT)}))
        self.remove = self.enterContext(patch.object(cleanup.shutil, 'rmtree'))
        self.enterContext(patch.object(cleanup.shutil, 'disk_usage',
                                       return_value=SimpleNamespace(free=10 * 1024 ** 3)))
        self.enterContext(patch('builtins.print'))

    def test_refuses_non_ci_and_other_workspace_without_deletion(self):
        for changes in ({'GITHUB_ACTIONS': 'false'},
                        {'GITHUB_WORKSPACE': str(cleanup.ROOT.parent)}):
            with self.subTest(changes=changes), patch.dict(cleanup.os.environ, changes):
                with self.assertRaises(SystemExit):
                    cleanup.main()
        self.remove.assert_not_called()

    def test_only_allowlisted_caches_are_deleted(self):
        with patch.object(Path, 'exists', return_value=True), \
                patch.object(Path, 'is_dir', return_value=True), \
                patch.object(Path, 'is_symlink', return_value=False):
            cleanup.main()
        self.assertEqual([call.args[0].relative_to(cleanup.ROOT).as_posix()
                          for call in self.remove.call_args_list],
                         ['target', '.work/security-target', '.work/gradle/caches',
                          '.work/security-sqlcipher/obj'])

    def test_symlink_aborts_entire_allowlist_before_deletion(self):
        with patch.object(Path, 'is_symlink', lambda path: path.name == 'obj'):
            with self.assertRaises(SystemExit):
                cleanup.main()
        self.remove.assert_not_called()

    def test_parent_redirection_outside_repository_aborts_before_deletion(self):
        resolve = Path.resolve

        def redirected(path, *args, **kwargs):
            if path == cleanup.ROOT / '.work/gradle/caches':
                return cleanup.ROOT.parent / 'other-cache'
            return resolve(path, *args, **kwargs)

        with patch.object(Path, 'resolve', redirected):
            with self.assertRaises(SystemExit):
                cleanup.main()
        self.remove.assert_not_called()

    def test_insufficient_remaining_space_refuses_emulator(self):
        with patch.object(cleanup.shutil, 'disk_usage',
                          return_value=SimpleNamespace(free=4 * 1024 ** 3)):
            with self.assertRaisesRegex(SystemExit, 'Less than 8 GiB'):
                cleanup.main()


if __name__ == '__main__':
    unittest.main()
