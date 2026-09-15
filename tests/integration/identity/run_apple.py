"""Run synthetic Swift/provider/Rust integration on the Mac build host."""
from pathlib import Path
import os,platform,subprocess
ROOT=Path(__file__).resolve().parents[3]
if platform.system()!='Darwin':raise RuntimeError('Identity Apple checks require the Mac/Xcode host')
out=ROOT/'.work/identity-swift';out.mkdir(parents=True,exist_ok=True)
cache=out/'module-cache';cache.mkdir(exist_ok=True)
ffi=ROOT/'.work/security-ffi/swift';library=ROOT/'.work/security-target/debug'
args=['xcrun','swiftc','-swift-version','6','-warnings-as-errors','-parse-as-library','-module-cache-path',str(cache),'-I',str(ffi),'-Xcc','-fmodule-map-file='+str(ffi/'meshchat_coreFFI.modulemap'),str(ffi/'meshchat_core.swift'),str(ROOT/'src/ios/Security/IdentityProvider.swift'),str(ROOT/'tests/integration/identity/IdentityMain.swift'),'-L',str(library),'-lmeshchat_core','-Xlinker','-rpath','-Xlinker',str(library),'-o',str(out/'identity-checks')]
env=os.environ.copy();env['TMPDIR']=str(ROOT/'.work/tmp')
subprocess.run(args,cwd=ROOT,env=env,check=True)
subprocess.run([str(out/'identity-checks')],cwd=ROOT,env=env,check=True)
