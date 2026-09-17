"""Run after the channel baseline on an explicitly named synthetic emulator."""
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--serial', required=True)
parser.add_argument('--phase', choices=['enable', 'reopen-exit'])
args = parser.parse_args()
if not args.serial.startswith('emulator-'):
    raise ValueError('Explicit emulator serial required')
adb = Path(os.environ['ANDROID_HOME'])/'platform-tools/adb'


def run(*command):
    return subprocess.check_output([str(adb), '-s', args.serial, *command], timeout=600)


if run('shell', 'getprop', 'ro.kernel.qemu').strip() != b'1':
    raise ValueError('Never inject synthetic Beacon results on a physical phone')
out = ROOT/'.work/mc033/screenshots'
out.mkdir(parents=True, exist_ok=True)
phases = [args.phase] if args.phase else ['enable', 'reopen-exit']
for phase in phases:
    run('shell', 'input', 'keyevent', '224');run('shell', 'input', 'keyevent', '82')
    run('shell', 'am', 'force-stop', 'org.meshchat.app')
    result = run('shell', 'am', 'instrument', '-w', '-r', '-e', 'phase', phase,
                 '-e', 'class', 'org.meshchat.ui.BeaconUiTest',
                 'org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner').decode()
    print(result, flush=True)
    for name in ['beacon']:
        data = run('exec-out', 'run-as', 'org.meshchat.app', 'cat', 'files/mc033-'+name+'.png')
        if data.startswith(b'\x89PNG\r\n\x1a\n'):
            (out/(name+'.png')).write_bytes(data)
    if 'OK (1 test)' not in result or 'FAILURES' in result or 'INSTRUMENTATION_CODE: -1' not in result:
        raise RuntimeError('Beacon phase failed: '+phase)
print('PASS: synthetic Beacon phases '+', '.join(phases)+'; physical endurance and battery certification pending')
