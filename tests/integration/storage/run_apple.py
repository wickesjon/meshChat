"""Run actual Swift/SQLCipher/Rust storage with test-only wrapping on Mac."""
from pathlib import Path
import os,platform,subprocess,uuid
ROOT=Path(__file__).resolve().parents[3]
if platform.system()!='Darwin':raise RuntimeError('Mac/Xcode host required')
work=ROOT/'.work/storage-swift';work.mkdir(exist_ok=True,parents=True)
frameworks=list((ROOT/'.work/security-swift-packages/artifacts').glob('**/macos-*/SQLCipher.framework'))
if len(frameworks)!=1:raise RuntimeError('Expected pinned SQLCipher macOS framework from existing Xcode resolution')
framework=frameworks[0].parent
ffi=ROOT/'.work/security-ffi/swift';library=ROOT/'.work/security-target/debug'
cache=work/'module-cache';cache.mkdir(exist_ok=True)
args=['xcrun','swiftc','-swift-version','6','-warnings-as-errors','-parse-as-library','-module-cache-path',str(cache),'-I',str(ffi),'-Xcc','-fmodule-map-file='+str(ffi/'meshchat_coreFFI.modulemap'),'-F',str(framework),'-framework','SQLCipher','-Xlinker','-rpath','-Xlinker',str(framework),str(ffi/'meshchat_core.swift'),str(ROOT/'src/ios/Security/IdentityProvider.swift'),str(ROOT/'src/ios/Security/EncryptedStorage.swift'),str(ROOT/'tests/integration/storage/StorageMain.swift'),'-L',str(library),'-lmeshchat_core','-Xlinker','-rpath','-Xlinker',str(library),'-o',str(work/'storage-checks')]
env=os.environ.copy();env['TMPDIR']=str(ROOT/'.work/tmp')
subprocess.run(args,cwd=ROOT,env=env,check=True)
fixtures=work/('fixtures-'+uuid.uuid4().hex);fixtures.mkdir()
for phase in ['create','reopen','checks','key-loss','reset']:subprocess.run([str(work/'storage-checks'),str(fixtures),phase],cwd=ROOT,env=env,check=True)
