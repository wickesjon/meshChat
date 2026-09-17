"""Synthetic organizer UI acceptance on an explicitly selected isolated emulator."""
import argparse
import os
from pathlib import Path
import subprocess
ROOT=Path(__file__).resolve().parents[3]
parser=argparse.ArgumentParser();parser.add_argument('--serial',required=True);args=parser.parse_args()
if not args.serial.startswith('emulator-'):raise ValueError('Select an isolated emulator, never a phone')
adb=Path(os.environ['ANDROID_HOME'])/'platform-tools/adb'
def run(*command):return subprocess.check_output([str(adb),'-s',args.serial,*command],timeout=600)
if run('shell','getprop','ro.kernel.qemu').strip()!=b'1':raise ValueError('Not an emulator')
package='org.meshchat.app'
out=ROOT/'.work/mc030/screenshots';out.mkdir(parents=True,exist_ok=True)
for phase in ['import','reopen','forget']:
    run('shell','input','keyevent','224');run('shell','input','keyevent','82');run('shell','am','force-stop',package)
    output=run('shell','am','instrument','-w','-r','-e','phase',phase,'-e','class','org.meshchat.ui.OrganizerUiTest',package+'.test/androidx.test.runner.AndroidJUnitRunner').decode()
    print(output,flush=True)
    if 'OK (1 test)' not in output or 'FAILURES' in output or 'INSTRUMENTATION_CODE: -1' not in output:raise RuntimeError('Organizer phase failed: '+phase)
for name in ['verified','reopened','unverified']:
    data=run('exec-out','run-as',package,'cat','files/mc030-'+name+'.png')
    if not data.startswith(b'\x89PNG\r\n\x1a\n'):raise RuntimeError('Missing screenshot')
    (out/(name+'.png')).write_bytes(data)
print('MC-030 Android confirmed adoption/import, private cancellation/lifecycle, protected persistence, signed post and forget PASS; synthetic emulator, no physical certification')
