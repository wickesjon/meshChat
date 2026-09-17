"""Run synthetic friend/DM acceptance after the channel runner creates its profile."""
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser()
parser.add_argument('--serial', required=True)
args = parser.parse_args()
if not args.serial.startswith('emulator-'):
    raise ValueError('Use an explicit isolated emulator, never a phone')
adb = Path(os.environ['ANDROID_HOME']) / 'platform-tools/adb'


def run(*command):
    return subprocess.check_output([str(adb), '-s', args.serial, *command], timeout=600)


if run('shell', 'getprop', 'ro.kernel.qemu').strip() != b'1':
    raise ValueError('Selected device is not an emulator')
package = 'org.meshchat.app'
out = ROOT / '.work/mc029/screenshots'
out.mkdir(parents=True, exist_ok=True)
names = ['confirmation', 'friends', 'dm', 'reopened-dm', 'old-identity',
         'failed-pair', 'failed-reopen', 'failed-replace']
for name in names:
    (out / (name + '.png')).unlink(missing_ok=True)
for phase in ['pair', 'reopen', 'replace']:
    print('MC-029 UI phase: ' + phase, flush=True)
    run('shell', 'input', 'keyevent', '224')
    run('shell', 'input', 'keyevent', '82')
    run('shell', 'am', 'force-stop', package)
    output = run('shell', 'am', 'instrument', '-w', '-r', '-e', 'phase', phase,
                 '-e', 'class', 'org.meshchat.ui.FriendUiTest',
                 package + '.test/androidx.test.runner.AndroidJUnitRunner').decode()
    print(output, flush=True)
    for name in names:
        try:
            data = run('exec-out', 'run-as', package, 'cat', 'files/mc029-' + name + '.png')
            if data.startswith(b'\x89PNG\r\n\x1a\n'):
                (out / (name + '.png')).write_bytes(data)
        except subprocess.CalledProcessError:
            pass
    if 'OK (1 test)' not in output or 'FAILURES' in output or 'INSTRUMENTATION_CODE: -1' not in output:
        raise RuntimeError('MC-029 friend acceptance failed: ' + phase)
for name in names[:5]:
    if not (out / (name + '.png')).is_file():
        raise RuntimeError('Missing screenshot: ' + name)
print('MC-029 synthetic protected friend/DM/QR/link/denial/restart/replacement PASS; physical certification pending')
