"""Exercise the actual private-pipe executable; never print its secret responses."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time
import unittest

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('console', ROOT / 'src/organizer-tools/console.py')
console = importlib.util.module_from_spec(spec)
spec.loader.exec_module(console)
BIN = ROOT / 'target/release' / ('meshchat-organizer.exe' if sys.platform == 'win32' else 'meshchat-organizer')


class OfflineConsoleChecks(unittest.TestCase):
    def test_pipe_issuance_dry_run_and_private_error_redaction(self):
        now = int(time.time())
        request = {'operation': 'create', 'name': 'Synthetic local event', 'expiry': now + 86400}
        response = console.backend(request)
        self.assertEqual(len(response['unlock']), 64)
        request = {'operation': 'check-staff', 'vault': response['vault'], 'unlock': response['unlock'],
                   'label': 'Test staff', 'before': now, 'after': now + 3600}
        preview = console.backend(request)
        self.assertEqual(set(preview), {'ok', 'name', 'expiry'})
        result = console.backend({**request, 'operation': 'issue'})
        self.assertIn('matrix', result)
        self.assertNotIn('unlock', result)
        self.assertNotIn('staff', result)
        self.assertNotIn(response['unlock'], json.dumps(result))
        malformed = subprocess.run([str(BIN), '--private-pipe'], input=b'{"unlock":"PRIVATE-MARKER","unknown":true}',
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
        self.assertNotEqual(malformed.returncode, 0)
        self.assertEqual(malformed.stdout, b'')
        self.assertNotIn(b'PRIVATE-MARKER', malformed.stderr)
        response.clear(); request.clear(); result.clear()

    def test_strict_dates_and_public_export_never_overwrite(self):
        self.assertEqual(console.timestamp('1970-01-02 00:00'), 86400)
        for value in ['', '2026-02-30 12:00', '2026-09-17T12:00Z']:
            with self.assertRaises(ValueError):
                console.timestamp(value)
        folder = ROOT / '.work/mc041/console-check'
        folder.mkdir(parents=True, exist_ok=True)
        path = folder / ('public-' + str(time.time_ns()) + '.txt')
        console.write_new(path, 'public synthetic value')
        with self.assertRaises(FileExistsError):
            console.write_new(path, 'replacement')
        self.assertEqual(path.read_text().strip(), 'public synthetic value')


if __name__ == '__main__':
    unittest.main(verbosity=2)
