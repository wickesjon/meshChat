"""Build pinned AndroidX graphics-path JNI with 16 KB LOAD and RELRO alignment.

The upstream 1.0.1 AAR supplies unchanged Java classes, resources and licenses.
Only its two packaged native ABIs are rebuilt from the corresponding release
source. No downloaded source or build output leaves the repository.
"""
import hashlib
import os
from pathlib import Path
import platform
import runpy
import subprocess
import tarfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[3]
WORK = ROOT / '.work/ui-graphics'
REVISION = '8a05a22af450d589ef911d772a001a49dcb05b71'
SOURCE_SHA = '2cca0866850f2bf1ac4b42b63a1f719b1230d9985b913b977d7c4a2175cf2ec1'
AAR_SHA = '8ca4032b6d79b351f0b59ad4b580eddbb9423e1652f7c958830687f1eee2ec03'


def download(name, url, digest):
    path = WORK / name
    if not path.exists():
        urllib.request.urlretrieve(url, path)
    if digest is not None and hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise ValueError(f'{name}: checksum mismatch')
    return path


def main():
    WORK.mkdir(parents=True, exist_ok=True)
    tmp = ROOT / '.work/tmp'
    tmp.mkdir(exist_ok=True)
    for variable in ['TMPDIR', 'TEMP', 'TMP']:
        os.environ[variable] = str(tmp)
    archive = download('source.tar.gz',
        f'https://android.googlesource.com/platform/frameworks/support/+archive/{REVISION}/graphics/graphics-path/src/main/cpp.tar.gz', None)
    aar = download('upstream.aar',
        'https://dl.google.com/dl/android/maven2/androidx/graphics/graphics-path/1.0.1/graphics-path-1.0.1.aar', AAR_SHA)
    source = WORK / 'source'
    source.mkdir(exist_ok=True)
    with tarfile.open(archive) as contents:
        # Gitiles archive headers vary across downloads. Pin the complete named
        # file contents, excluding only tar/gzip metadata, before extraction.
        digest = hashlib.sha256()
        for member in sorted(contents.getmembers(), key=lambda item: item.name):
            if member.isfile():
                digest.update(member.name.encode() + b'\0' + hashlib.sha256(contents.extractfile(member).read()).digest())
            elif not member.isdir():
                raise ValueError('Unexpected source archive entry')
        if digest.hexdigest() != SOURCE_SHA:
            raise ValueError('Graphics source content checksum mismatch')
        contents.extractall(source, filter='data')
    ndk = Path(os.environ['ANDROID_HOME']) / 'ndk/27.3.13750724'
    host = {'Windows': 'windows-x86_64', 'Linux': 'linux-x86_64'}[platform.system()]
    compiler = ndk / 'toolchains/llvm/prebuilt' / host / 'bin' / ('clang++.exe' if platform.system() == 'Windows' else 'clang++')
    check = runpy.run_path(str(ROOT / 'tests/integration/ffi/check_android_apk.py'))['check_elf']
    for abi, target in [('arm64-v8a', 'aarch64-linux-android29'), ('x86_64', 'x86_64-linux-android29')]:
        output = WORK / abi / 'libandroidx.graphics.path.so'
        output.parent.mkdir(exist_ok=True)
        subprocess.run([str(compiler), f'--target={target}', '-std=c++17', '-O2', '-fPIC',
            '-shared', '-static-libstdc++', '-fvisibility=hidden', '-fno-exceptions', '-fno-rtti',
            '-ffunction-sections', '-fdata-sections', '-Wl,--gc-sections',
            '-Wl,-z,relro,-z,now,-z,max-page-size=16384,-z,common-page-size=16384',
            '-Wl,-soname,libandroidx.graphics.path.so',
            f'-Wl,--version-script={source / "libandroidx.graphics.path.map"}',
            *[str(source / name) for name in ['Conic.cpp', 'PathIterator.cpp', 'pathway.cpp']],
            '-o', str(output)], check=True, cwd=source)
        check(output.read_bytes(), f'lib/{abi}/libandroidx.graphics.path.so')
    with zipfile.ZipFile(aar) as original, zipfile.ZipFile(WORK / 'graphics-path-1.0.1-aligned.aar', 'w') as result:
        for item in original.infolist():
            if not item.filename.startswith('jni/'):
                result.writestr(item, original.read(item.filename))
        for abi in ['arm64-v8a', 'x86_64']:
            result.write(WORK / abi / 'libandroidx.graphics.path.so', f'jni/{abi}/libandroidx.graphics.path.so')
    print(f'AndroidX graphics-path 1.0.1 ({REVISION}): both ABIs pass 16 KB LOAD/RELRO checks')


if __name__ == '__main__':
    main()
