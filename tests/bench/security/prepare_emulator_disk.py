"""Free this disposable Linux CI job's build caches after APK verification."""

import os
from pathlib import Path
import shutil
import sys


ROOT = Path(__file__).resolve().parents[3]
CACHES = ('target', '.work/security-target', '.work/gradle/caches',
          '.work/security-sqlcipher/obj')


def main():
    if (sys.platform != 'linux' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or Path(os.environ.get('GITHUB_WORKSPACE', '')).resolve() != ROOT):
        raise SystemExit('Cache cleanup is restricted to this repository in Linux CI')

    # Validate the complete allowlist before deleting anything. Reject symlink
    # redirection even when it happens to point elsewhere inside this repository.
    targets = []
    for relative in CACHES:
        candidate = ROOT / relative
        resolved = candidate.resolve()
        if (resolved != candidate or ROOT not in resolved.parents
                or candidate.is_symlink()
                or (candidate.exists() and not candidate.is_dir())):
            raise SystemExit(f'Unexpected build-cache path: {relative}')
        targets.append(candidate)

    print(f'Free bytes before cache cleanup: {shutil.disk_usage(ROOT).free}', flush=True)
    for target in targets:
        if target.exists():
            shutil.rmtree(target)
            print(f'Removed disposable build cache: {target.relative_to(ROOT)}', flush=True)
    free = shutil.disk_usage(ROOT).free
    print(f'Free bytes after cache cleanup: {free}', flush=True)
    # The emulator requires 1.2 times its 6 GiB userdata size; retain extra room
    # for APK installation and the small synthetic database as well.
    if free < 8 * 1024 ** 3:
        raise SystemExit('Less than 8 GiB available for the security emulator')


if __name__ == '__main__':
    main()
