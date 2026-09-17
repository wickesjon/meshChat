"""Run after android-ui/run_emulator.py on the explicit synthetic emulator."""
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--serial', required=True)
args = parser.parse_args()
if not args.serial.startswith('emulator-'):
    raise ValueError('Explicit emulator serial required')
adb = Path(os.environ['ANDROID_HOME'])/'platform-tools/adb'


def run(*command):
    return subprocess.check_output([str(adb), '-s', args.serial, *command], timeout=600)


if run('shell', 'getprop', 'ro.kernel.qemu').strip() != b'1':
    raise ValueError('Never run synthetic sharing tests on a physical phone')
out = ROOT/'.work/mc031/screenshots'
out.mkdir(parents=True, exist_ok=True)
for phase in ['share', 'reopen']:
    run('shell', 'input', 'keyevent', '224');run('shell', 'input', 'keyevent', '82')
    run('shell', 'am', 'force-stop', 'org.meshchat.app')
    result = run('shell', 'am', 'instrument', '-w', '-r', '-e', 'phase', phase,
                 '-e', 'class', 'org.meshchat.ui.SharingUiTest',
                 'org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner').decode()
    print(result, flush=True)
    for name in ['channel-confirmation', 'channel-qr', 'channel-actions', 'friend-qr']:
        data = run('exec-out', 'run-as', 'org.meshchat.app', 'cat', 'files/mc031-'+name+'.png')
        if data.startswith(b'\x89PNG\r\n\x1a\n'):
            (out/(name+'.png')).write_bytes(data)
    if 'OK (1 test)' not in result or 'FAILURES' in result or 'INSTRUMENTATION_CODE: -1' not in result:
        raise RuntimeError('Sharing phase failed: '+phase)
print('PASS: native sharing, confirmation/cancel/restart, exact offline QR, PNG export and sensitive clipboard flag; physical and domain verification pending')
