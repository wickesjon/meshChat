"""Check shipped native libraries, not just that Gradle produced an APK."""
import struct
import sys
import zipfile


def check_elf(data, name):
    if data[:6] != b'\x7fELF\x02\x01':
        raise ValueError(f'{name}: expected little-endian ELF64')
    offset = struct.unpack_from('<Q', data, 32)[0]
    entry_size, count = struct.unpack_from('<HH', data, 54)
    loads = 0
    relro = 0
    for index in range(count):
        kind, _, _, address, _, _, memory_size, alignment = struct.unpack_from(
            '<IIQQQQQQ', data, offset + index * entry_size)
        if kind == 1:
            loads += 1
            if alignment < 16384:
                raise ValueError(f'{name}: LOAD alignment {alignment} below 16 KB')
        if kind == 0x6474E552:
            relro += 1
            if (address + memory_size) % 16384:
                raise ValueError(f'{name}: RELRO end is not 16 KB aligned')
    if not loads or not relro:
        raise ValueError(f'{name}: missing LOAD or RELRO segments')


def main():
    for path in sys.argv[1:]:
        with zipfile.ZipFile(path) as apk:
            names = {name for name in apk.namelist() if name.startswith('lib/') and name.endswith('.so')}
            expected = {f'lib/{abi}/{library}.so'
                        for abi in ['arm64-v8a', 'x86_64']
                        for library in ['libmeshchat_core', 'libjnidispatch']}
            if names != expected:
                raise ValueError(f'{path}: unexpected native library set: {sorted(names)}')
            for name in sorted(names):
                check_elf(apk.read(name), name)
        print(f'{path}: both ABIs include Rust/JNA with 16 KB LOAD and RELRO alignment')


if __name__ == '__main__':
    main()
