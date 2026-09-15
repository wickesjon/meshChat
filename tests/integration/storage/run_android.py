"""Exercise MC-018 only on the explicitly named isolated Android emulator."""
import argparse,os,subprocess
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--serial',required=True);args=parser.parse_args()
if not args.serial.startswith('emulator-'):raise ValueError('Storage fixtures require an explicit emulator serial')
adb=Path(os.environ['ANDROID_HOME'])/'platform-tools/adb'
def run(*cmd):return subprocess.check_output([str(adb),'-s',args.serial,*cmd],text=True,timeout=240)
if run('shell','getprop','ro.kernel.qemu').strip()!='1':raise ValueError('Selected device is not an emulator')
component='org.meshchat.securityprobe.test/org.meshchat.storage.StorageInstrumentation'
if f'instrumentation:{component} (target=org.meshchat.securityprobe)' not in run('shell','pm','list','instrumentation').splitlines():raise RuntimeError('Storage instrumentation is missing from the installed test APK')
for phase in ['create','reopen','checks','key-loss','reset']:
 run('shell','input','keyevent','224');run('shell','input','keyevent','82');run('shell','am','force-stop','org.meshchat.securityprobe')
 output=run('shell','am','instrument','-w','-e','phase',phase,component)
 print(output.strip())
 if f'INSTRUMENTATION_RESULT: mc018=PASS {phase} synthetic_functional_only' not in output or 'INSTRUMENTATION_CODE: -1' not in output:raise RuntimeError('Storage fixture phase failed: '+phase)
print('MC-018 Android SQLCipher/provider lifecycle PASS; physical certification pending')
