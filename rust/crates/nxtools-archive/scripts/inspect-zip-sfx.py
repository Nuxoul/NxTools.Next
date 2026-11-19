"""Inspect and copy the owned bare recipient PE without loading or executing it."""
import argparse
import json
import struct
from pathlib import Path


def inspect(data):
    def need(condition, message):
        if not condition:
            raise ValueError(message)

    def unpack(fmt, offset):
        size = struct.calcsize('<' + fmt)
        need(0 <= offset <= len(data) - size, 'PE field outside file')
        return struct.unpack_from('<' + fmt, data, offset)

    need(data[:2] == b'MZ', 'missing DOS header')
    pe, = unpack('I', 60)
    need(data[pe:pe + 4] == b'PE\0\0', 'missing PE header')
    machine, count = unpack('HH', pe + 4)
    optional_size, = unpack('H', pe + 20)
    optional = pe + 24
    need(machine == 0x8664 and 0 < count <= 96, 'recipient must be Windows x64 PE')
    magic, = unpack('H', optional)
    need(magic == 0x20b and optional_size >= 240, 'recipient must be PE32+')
    headers, = unpack('I', optional + 60)
    directories, = unpack('I', optional + 108)
    need(14 <= directories <= 16, 'unexpected PE data-directory layout')
    table = optional + optional_size
    need(table + count * 40 <= headers <= len(data), 'invalid PE headers extent')
    sections = []
    raw_end = headers
    for index in range(count):
        offset = table + index * 40
        virtual_size, rva, raw_size, raw = unpack('IIII', offset + 8)
        need(raw_size == 0 or (raw >= headers and raw + raw_size <= len(data)), 'invalid section raw extent')
        raw_end = max(raw_end, raw + raw_size if raw_size else 0)
        sections.append(dict(name=data[offset:offset + 8].split(b'\0')[0].decode('ascii'), rva=rva,
                             virtual_size=virtual_size, raw=raw, raw_size=raw_size))
    certificate = unpack('II', optional + 112 + 4 * 8)
    need(certificate == (0, 0), 'certificate directory must be absent')
    need(raw_end == len(data), 'bare PE has an unaccounted overlay; no bytes will be trimmed')

    def physical(rva, size=1):
        for section in sections:
            delta = rva - section['rva']
            if 0 <= delta and delta + size <= section['raw_size']:
                return section['raw'] + delta
        if 0 <= rva and rva + size <= headers:
            return rva
        raise ValueError('PE RVA has no file-backed extent')

    def text(rva):
        start = physical(rva)
        end = data.find(b'\0', start, min(len(data), start + 512))
        need(end >= start, 'unbounded PE import name')
        physical(rva, end - start + 1)
        return data[start:end].decode('ascii')

    imports = []
    for directory, width in [(1, 20), (13, 32)]:
        rva, length = unpack('II', optional + 112 + directory * 8)
        if rva == 0:
            need(length == 0, 'invalid absent import directory')
            continue
        need(width <= length <= 1024 * 1024, 'invalid import directory size')
        terminated = False
        for index in range(min(length // width, 4096)):
            record = unpack('I' * (width // 4), physical(rva + index * width, width))
            if not any(record):
                terminated = True
                break
            if directory == 1:
                name_rva, thunk_rva = record[3], record[0] or record[4]
            else:
                need(record[0] == 1, 'delay imports must use RVAs')
                name_rva, thunk_rva = record[1], record[4] or record[3]
            library = text(name_rva)
            lower = library.lower()
            system = {'kernel32.dll', 'advapi32.dll', 'ntdll.dll', 'bcrypt.dll', 'bcryptprimitives.dll',
                      'userenv.dll', 'ws2_32.dll', 'user32.dll', 'shell32.dll', 'ole32.dll'}
            need(lower in system or (lower.startswith('api-ms-win-') and not lower.startswith('api-ms-win-crt-')),
                 'non-system or dynamic CRT import: ' + library)
            symbols = []
            for symbol_index in range(65536):
                thunk, = unpack('Q', physical(thunk_rva + symbol_index * 8, 8))
                if thunk == 0:
                    break
                symbols.append('#' + str(thunk & 0xffff) if thunk >> 63 else text(thunk + 2))
            else:
                raise ValueError('unterminated import thunk table')
            imports.append(dict(library=library, delay=directory == 13, symbols=symbols))
        need(terminated, 'unterminated PE import directory')
    return dict(machine='0x8664', pe32_plus=True, file_bytes=len(data), raw_pe_end=raw_end,
                certificate_directory=list(certificate), overlay_bytes=0, sections=sections,
                imports=imports, runtime_dependency_policy='Windows system DLLs only; no dynamic CRT or managed/external codec runtime')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--evidence', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    crate = root / 'rust/crates/nxtools-archive'
    binary = root / 'target/release/nxtools-zip-sfx.exe'
    asset = crate / 'assets/zip-sfx-x64.bin'
    size = binary.stat().st_size
    if not 0 < size <= 64 * 1024 * 1024:
        raise ValueError('owned bare stub is outside bounded inspection size')
    data = binary.read_bytes()
    facts = inspect(data)
    facts.update(source_binary=str(binary), bundled_asset=str(asset), executable_launched=False)
    with (args.evidence / 'pe-inspection.json').open('x', encoding='utf-8') as output:
        json.dump(facts, output, indent=2)
        output.write('\n')
    asset.parent.mkdir(exist_ok=True)
    if asset.exists():
        if asset.read_bytes() != data:
            raise ValueError('existing bundled asset differs; explicit owner replacement review required')
        action = 'existing identical asset retained'
    else:
        with asset.open('xb') as output:
            output.write(data)
        action = 'new asset copied'
    if asset.read_bytes() != data:
        raise ValueError('bundled bytes differ from owned build output')
    receipt = dict(bytes=len(data), action=action, source=str(binary), asset=str(asset), direct_bytes_equal=True)
    with (args.evidence / 'asset-copy.json').open('x', encoding='utf-8') as output:
        json.dump(receipt, output, indent=2)
        output.write('\n')
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
