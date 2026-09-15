"""Compile actual Android adapter and run its provider with test-only stores/wrapping.
Requires pinned Gradle artifacts already provisioned by the Android build. No downloads.
This is JVM/Rust integration, not an emulator/Keystore or hardware result.
"""
from pathlib import Path
import os,subprocess
ROOT=Path(__file__).resolve().parents[3]
cache=ROOT/'.work/gradle/caches/modules-2/files-2.1'
out=ROOT/'.work/identity-jvm';out.mkdir(parents=True,exist_ok=True)
def jar(group,name,version):
    paths=list((cache/group/name/version).glob('*/'+name+'-'+version+'.jar'))
    if len(paths)!=1:raise RuntimeError('Missing/ambiguous pinned artifact: '+name+' '+version)
    return paths[0]
compiler=[jar('org.jetbrains.kotlin',n,'2.2.0') for n in ['kotlin-compiler-embeddable','kotlin-stdlib','kotlin-script-runtime','kotlin-daemon-embeddable']]
compiler += [jar('org.jetbrains.kotlin','kotlin-reflect','1.6.10'),jar('org.jetbrains.intellij.deps','trove4j','1.0.20200330'),jar('org.jetbrains','annotations','13.0'),jar('org.jetbrains.kotlinx','kotlinx-coroutines-core-jvm','1.8.0')]
std=jar('org.jetbrains.kotlin','kotlin-stdlib','2.2.0');jna=jar('net.java.dev.jna','jna','5.17.0')
android=ROOT/'.work/android-sdk/platforms/android-36/android.jar'
if not android.exists():android=Path(os.environ['ANDROID_HOME'])/'platforms/android-36/android.jar'
java=ROOT/'.work/tools/jdk-17.0.15+6/bin'/('java.exe' if os.name=='nt' else 'java')
if not java.exists():java=Path(os.environ['JAVA_HOME'])/'bin/java'
env=os.environ.copy()
for name in ['TEMP','TMP','TMPDIR']:env[name]=str(ROOT/'.work/tmp')
base=[str(java),'-Djava.io.tmpdir='+str(ROOT/'.work/tmp'),'-Duser.home='+str(ROOT/'.work/java-home')]
cp=lambda paths:os.pathsep.join(map(str,paths))
sources=list((ROOT/'src/android/security/src/main/java/org/meshchat/identity').glob('*.kt'))+[ROOT/'.work/security-ffi/kotlin/uniffi/meshchat_core/meshchat_core.kt',ROOT/'tests/integration/identity/IdentityMain.kt']
subprocess.run(base+['-cp',cp(compiler),'org.jetbrains.kotlin.cli.jvm.K2JVMCompiler','-Werror','-no-stdlib','-no-reflect','-jvm-target','17','-classpath',cp([std,jna,android,jar('androidx.annotation','annotation-jvm','1.9.1')]),'-d',str(out/'identity.jar')]+list(map(str,sources)),cwd=ROOT,env=env,check=True)
subprocess.run(base+['-Djna.library.path='+str(ROOT/'.work/security-target/debug'),'-Djna.tmpdir='+str(ROOT/'.work/tmp'),'-cp',cp([out/'identity.jar',std,jna,android]),'org.meshchat.identity.IdentityMainKt'],cwd=ROOT,env=env,check=True)
