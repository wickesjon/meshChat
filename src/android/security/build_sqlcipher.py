"""Rebuild pinned SQLCipher JNI with 16 KB RELRO; keep all output in .work.

Linux CI requires git, make, Tcl and the already pinned Android NDK. Java/classes
come from the checksummed 4.17.0 AAR; only its two supported native ABIs change.
"""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[3]
WORK = ROOT / '.work/security-sqlcipher'
SOURCE = WORK / 'upstream'
REVISION = '0725b962ffb60b00460b0e315bc632a543b399e7'
AAR_SHA256 = '44fc40c33d1de597c8339072a71fa0ff20e12d01ab352d6abe4ad5df668ead94'


def run(*args, cwd=SOURCE):
    subprocess.run([str(arg) for arg in args], cwd=cwd, check=True)


def main():
    WORK.mkdir(parents=True, exist_ok=True)
    tmp = ROOT / '.work/tmp'
    tmp.mkdir(exist_ok=True)
    for variable in ['TMPDIR', 'TEMP', 'TMP']:
        os.environ[variable] = str(tmp)
    if not SOURCE.exists():
        run('git', 'clone', '--no-checkout', '--depth=1', '--branch', 'v4.17.0',
            'https://github.com/sqlcipher/sqlcipher-android.git', SOURCE, cwd=WORK)
        run('git', 'checkout', '--detach', REVISION)
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=SOURCE, text=True).strip()
    if head != REVISION:
        raise ValueError('SQLCipher source revision mismatch; refusing build')
    # The top-level revision pins both gitlinks; no floating branch updates.
    run('git', 'submodule', 'update', '--init', '--depth=1')
    run('git', 'submodule', 'status')
    jni = SOURCE / 'sqlcipher/src/main/jni'
    core = jni / 'sqlcipher/src'
    run('./configure', '--with-tempstore=yes', '--disable-tcl', cwd=core)
    run('make', 'sqlite3.c', cwd=core)
    for name in ['sqlite3.c', 'sqlite3.h']:
        shutil.copy2(core / name, jni / 'sqlcipher' / name)
    makefile = jni / 'sqlcipher/Android.mk'
    original = subprocess.check_output(['git', 'show', f'{REVISION}:sqlcipher/src/main/jni/sqlcipher/Android.mk'],
                                       cwd=SOURCE, text=True)
    old = 'LOCAL_LDFLAGS += -Wl,-z,max-page-size=16384'
    if original.count(old) != 1:
        raise ValueError('SQLCipher linker patch anchor changed')
    makefile.write_text(original.replace(old, old + ' -Wl,-z,common-page-size=16384') +
                        '\n', encoding='utf-8')
    ndk = Path(os.environ['ANDROID_HOME']) / 'ndk/27.3.13750724'
    run(ndk / 'ndk-build', f'NDK_PROJECT_PATH={SOURCE / "sqlcipher"}',
        f'APP_BUILD_SCRIPT={jni / "Android.mk"}', f'NDK_APPLICATION_MK={jni / "Application.mk"}',
        'APP_ABI=arm64-v8a x86_64', 'APP_PLATFORM=android-29', 'APP_OPTIM=release',
        f'NDK_OUT={WORK / "obj"}', f'NDK_LIBS_OUT={WORK / "libs"}', '-j2')
    original_aar = WORK / 'upstream.aar'
    if not original_aar.exists():
        urllib.request.urlretrieve('https://repo.maven.apache.org/maven2/net/zetetic/sqlcipher-android/'
                                   '4.17.0/sqlcipher-android-4.17.0.aar', original_aar)
    if hashlib.sha256(original_aar.read_bytes()).hexdigest() != AAR_SHA256:
        raise ValueError('SQLCipher AAR checksum mismatch')
    with zipfile.ZipFile(original_aar) as source, zipfile.ZipFile(WORK / 'sqlcipher-4.17.0-aligned.aar', 'w') as result:
        for item in source.infolist():
            if item.filename.startswith('jni/'):
                continue
            result.writestr(item, source.read(item.filename))
        for abi in ['arm64-v8a', 'x86_64']:
            result.write(WORK / 'libs' / abi / 'libsqlcipher.so', f'jni/{abi}/libsqlcipher.so')
    print(f'SQLCipher {REVISION}: rebuilt both ABIs; packaging check still required')


if __name__ == '__main__':
    main()
