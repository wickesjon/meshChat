"""Run only on the named isolated CI emulator; never select an attached phone."""
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser()
parser.add_argument('--serial', required=True)
args = parser.parse_args()
if not args.serial.startswith('emulator-'):
    raise ValueError('This automated fixture runner requires an explicit emulator serial')
adb = Path(os.environ['ANDROID_HOME']) / 'platform-tools/adb'


def run(*command):
    return subprocess.check_output([str(adb), '-s', args.serial, *command], text=True, timeout=180)


if run('shell', 'getprop', 'ro.kernel.qemu').strip() != '1':
    raise ValueError('Selected device is not an emulator')
print('Emulator API=' + run('shell', 'getprop', 'ro.build.version.sdk').strip())
print('Emulator page size=' + run('shell', 'getconf', 'PAGE_SIZE').strip())
package = 'org.meshchat.securityprobe'
for directory in ['debug', 'androidTest/debug']:
    apks = list((ROOT / 'src/android/security/build/outputs/apk' / directory).glob('*.apk'))
    if len(apks) != 1:
        raise ValueError(f'Expected one APK in {directory}')
    apk = apks[0]
    print(run('install', '-r', '-t', str(apk)).strip())
for phase in ['create', 'reopen', 'key-loss']:
    run('shell', 'am', 'force-stop', package)
    output = run('shell', 'am', 'instrument', '-w', '-e', 'phase', phase,
                 package + '.test/' + package + '.SecurityInstrumentation')
    print(output.strip())
    if f'INSTRUMENTATION_RESULT: mc005=PASS {phase} synthetic_functional_only' not in output or 'INSTRUMENTATION_CODE: -1' not in output:
        raise RuntimeError(f'Emulator phase {phase} did not pass')
print('MC005 emulator persistence/wrong-key/key-loss/reset passed; physical acceptance remains pending')
