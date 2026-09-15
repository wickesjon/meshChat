"""Generate host-only Python FFI and run SQLite-double policy tests in the repo."""
import os,platform,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];out=ROOT/'.work/storage-python';out.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env['CARGO_TARGET_DIR']=str(ROOT/'target')
name={'Windows':'meshchat_core.dll','Darwin':'libmeshchat_core.dylib','Linux':'libmeshchat_core.so'}[platform.system()]
def run(*args):subprocess.run(list(map(str,args)),cwd=ROOT,env=env,check=True)
run('cargo','build','--locked','-p','meshchat-core','--features','bindgen,security-probe')
run('cargo','run','--locked','-p','meshchat-core','--features','bindgen,security-probe','--bin','uniffi-bindgen','--','generate','--library',ROOT/'target/debug'/name,'--language','python','--out-dir',out,'--no-format')
shutil.copy2(ROOT/'target/debug'/name,out/name)
run(sys.executable,'-B',ROOT/'tests/integration/storage/test_policy.py','-v')
