"""Run production channel UI with synthetic fixtures on an explicit isolated emulator."""
import argparse
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser()
parser.add_argument('--serial', required=True)
args = parser.parse_args()
if not args.serial.startswith('emulator-'):
    raise ValueError('An explicit emulator serial is required; never select a phone')
adb = Path(os.environ['ANDROID_HOME']) / 'platform-tools/adb'
def run(*command):
    return subprocess.check_output([str(adb), '-s', args.serial, *command], timeout=240)
if run('shell','getprop','ro.kernel.qemu').strip() != b'1':
    raise ValueError('Selected device is not an emulator')
for directory in ['debug', 'androidTest/debug']:
    apks = list((ROOT/'src/android/app/build/outputs/apk'/directory).glob('*.apk'))
    if len(apks) != 1:
        raise ValueError(f'Expected one APK in {directory}')
    print(run('install','-r','-t',str(apks[0])).decode().strip())
package = 'org.meshchat.app'
run('shell','pm','clear',package)  # Only the synthetic app on this isolated emulator.
for phase in ['create','reopen','composer']:
    run('shell','input','keyevent','224');run('shell','input','keyevent','82')
    run('shell','am','force-stop',package)
    output = run('shell','am','instrument','-w','-e','phase',phase,'-e','class','org.meshchat.ui.ChannelUiTest',
                 package+'.test/androidx.test.runner.AndroidJUnitRunner').decode()
    print(output)
    if 'OK (1 test)' not in output or 'FAILURES' in output or 'INSTRUMENTATION_CODE: -1' not in output:
        raise RuntimeError('MC-028 UI phase failed: '+phase)
out = ROOT/'.work/mc028/screenshots'
out.mkdir(parents=True,exist_ok=True)
for name in ['dark-chat','light-channels','reopened-chat','large-text-composer']:
    (out/(name+'.png')).write_bytes(run('exec-out','run-as',package,'cat','files/mc028-'+name+'.png'))
print('MC-028 production onboarding/navigation/byte-boundary/offline/protected-history/restart UI PASS; physical certification pending')
