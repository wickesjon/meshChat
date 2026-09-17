"""Run after the channel baseline on an explicitly named synthetic emulator."""
import argparse
import os
from pathlib import Path
import subprocess
import re
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--serial', required=True)
parser.add_argument('--phase', choices=['enable', 'reopen-exit', 'touch'])
args = parser.parse_args()
if not args.serial.startswith('emulator-'):
    raise ValueError('Explicit emulator serial required')
adb = Path(os.environ['ANDROID_HOME'])/'platform-tools/adb'


def run(*command):
    return subprocess.check_output([str(adb), '-s', args.serial, *command], timeout=600)


def window():
    run('shell', 'uiautomator', 'dump', '/data/local/tmp/mc033-window.xml')
    return ET.fromstring(run('exec-out', 'cat', '/data/local/tmp/mc033-window.xml'))


if run('shell', 'getprop', 'ro.kernel.qemu').strip() != b'1':
    raise ValueError('Never inject synthetic Beacon results on a physical phone')
out = ROOT/'.work/mc033/screenshots'
out.mkdir(parents=True, exist_ok=True)
phases = [args.phase] if args.phase else ['enable', 'reopen-exit', 'enable', 'touch']
for phase in phases:
    run('shell', 'input', 'keyevent', '224');run('shell', 'input', 'keyevent', '82')
    run('shell', 'am', 'force-stop', 'org.meshchat.app')
    if phase == 'touch':
        # Real Android input outside Compose's virtual test clock. Read the
        # target's actual screen bounds rather than assuming a device layout.
        run('shell', 'am', 'start', '-n', 'org.meshchat.app/.MainActivity')
        bounds = None
        for _ in range(10):
            nodes = window().iter('node')
            target = next((n for n in nodes if n.get('text') == 'Hold 2 seconds to exit'), None)
            if target is not None:
                bounds = [int(n) for n in re.findall(r'\d+', target.get('bounds', ''))]
                break
            time.sleep(0.5)
        if bounds is None or len(bounds) != 4 or bounds[2] <= bounds[0] or bounds[3] <= bounds[1]:
            raise RuntimeError('The real Beacon exit target was not visible')
        x, y = (bounds[0]+bounds[2])//2, (bounds[1]+bounds[3])//2
        run('shell', 'input', 'swipe', str(x), str(y), str(x), str(y), '2300')
        # The model publishes normal UI only after its protected writes finish.
        # Do not kill the process one second into that asynchronous save.
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            labels = {n.get('text') for n in window().iter('node')}
            if 'Channels' in labels and 'Hold 2 seconds to exit' not in labels:
                break
            time.sleep(0.5)
        else:
            raise RuntimeError('The real touch hold did not exit Beacon Mode')
        run('shell', 'am', 'force-stop', 'org.meshchat.app')
    instrument_phase = 'verify-touch' if phase == 'touch' else phase
    result = run('shell', 'am', 'instrument', '-w', '-r', '-e', 'phase', instrument_phase,
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
