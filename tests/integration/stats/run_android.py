"""Synthetic local stats acceptance, after the channel baseline, explicit emulator only."""
import argparse, os, subprocess
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--serial',required=True);args=parser.parse_args()
if not args.serial.startswith('emulator-'):raise ValueError('Explicit emulator required')
adb=Path(os.environ['ANDROID_HOME'])/'platform-tools/adb'
def run(*cmd):return subprocess.check_output([str(adb),'-s',args.serial,*cmd],timeout=600)
if run('shell','getprop','ro.kernel.qemu').strip()!=b'1':raise ValueError('Not an emulator')
package='org.meshchat.app'
run('shell','input','keyevent','224');run('shell','input','keyevent','82');run('shell','am','force-stop',package)
output=run('shell','am','instrument','-w','-r','-e','class','org.meshchat.ui.StatsUiTest',package+'.test/androidx.test.runner.AndroidJUnitRunner').decode()
print(output,flush=True)
if 'OK (1 test)' not in output or 'FAILURES' in output or 'INSTRUMENTATION_CODE: -1' not in output:raise RuntimeError('Stats acceptance failed')
out=Path(__file__).resolve().parents[3]/'.work/mc042/screenshots';out.mkdir(parents=True,exist_ok=True)
for name in ['panel','preview','card']:
    data=run('exec-out','run-as',package,'cat','files/mc042-'+name+'.png')
    if not data.startswith(b'\x89PNG\r\n\x1a\n'):raise RuntimeError('Missing screenshot')
    (out/(name+'.png')).write_bytes(data)
print('MC-042 local panel, explicit selection, image export bounds and reset PASS; synthetic emulator, no delivery or battery certification')
