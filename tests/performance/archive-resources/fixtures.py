"""Independent CPython fixtures; only a fresh GUID-owned session scratch root is accepted."""
import argparse
import hashlib
import json
import pathlib
import shutil
import sys
import uuid
import zipfile

GIB = 1 << 30


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def generate(root, small=False, catalog_only=False):
    root = root.resolve()
    scratch = pathlib.Path(__import__('os').environ['PI_SCRATCH_DIR']).resolve()
    assert root.parent == scratch and root.name.startswith('archive-resources-')
    uuid.UUID(root.name.removeprefix('archive-resources-'))
    assert not root.exists(), 'refuse overwrite'
    assert shutil.disk_usage(scratch).free >= 4 * GIB
    root.mkdir()
    (root / 'owned-resource-fixtures').write_text('v1\n', encoding='utf-8')
    if small:
        data = bytes(range(256)) * 1024
        with zipfile.ZipFile(root / 'small.zip', 'x', compression=zipfile.ZIP_STORED) as archive:
            archive.writestr('synthetic.bin', data)
        row = dict(ordinal=0, name='synthetic.bin', kind='File', size=len(data), packed_size=len(data))
        (root / 'entries.jsonl').write_text(json.dumps(row) + '\n', encoding='utf-8')
        with zipfile.ZipFile(root / 'credit.zip', 'x', compression=zipfile.ZIP_STORED) as archive:
            for ordinal in range(5):
                archive.writestr(f'credit-{ordinal}.txt', b'abc')
        summary = dict(mode='small-tool-regression', entries=1, synthetic_payload_bytes=len(data),
                       files={name: dict(bytes=(root / name).stat().st_size, sha256=digest(root / name))
                              for name in ('small.zip', 'entries.jsonl', 'credit.zip')})
        (root / 'fixture-summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
        print(json.dumps(summary))
        return
    total = dirs = empty = unicode = 0
    with (root / 'entries.jsonl').open('x', encoding='utf-8', newline='\n') as manifest:
        with zipfile.ZipFile(root / '100k.zip', 'x', compression=zipfile.ZIP_STORED, allowZip64=True) as archive:
            for ordinal in range(100000):
                directory = ordinal % 1000 == 0
                name = f'd{ordinal // 1000:03}/' if directory else f'd{ordinal // 1000:03}/f{ordinal:06}.txt'
                if not directory and ordinal % 997 == 0:
                    name = f'd{ordinal // 1000:03}/目录-é-{ordinal:06}.txt'
                    unicode += 1
                data = b'' if directory or ordinal % 11 == 0 else f'payload-{ordinal:06}\n'.encode('ascii')
                info = zipfile.ZipInfo(name, (2024, 1, 2, 3, 4, 6))
                info.compress_type = zipfile.ZIP_STORED
                info.external_attr = ((0o40755 if directory else 0o100644) << 16) | (0x10 if directory else 0)
                archive.writestr(info, data)
                row = dict(ordinal=ordinal, name=name, kind='Directory' if directory else 'File', size=len(data), packed_size=len(data))
                manifest.write(json.dumps(row, ensure_ascii=False, separators=(',', ':')) + '\n')
                total += len(data)
                dirs += int(directory)
                empty += int(not data and not directory)
    if not catalog_only:
        block = bytes(range(256)) * 4096  # exactly1MiB, no raw1GiB duplicate
        payload_hash = hashlib.sha256()
        with zipfile.ZipFile(root / '1gib.zip', 'x', compression=zipfile.ZIP_STORED) as archive:
            info = zipfile.ZipInfo('synthetic.bin', (2024, 1, 2, 3, 4, 6))
            info.file_size = GIB
            with archive.open(info, 'w') as target:
                for _ in range(1024):
                    target.write(block)
                    payload_hash.update(block)
    with zipfile.ZipFile(root / 'credit.zip', 'x', compression=zipfile.ZIP_STORED) as archive:
        for ordinal in range(5):
            archive.writestr(f'credit-{ordinal}.txt', b'abc')
    summary = dict(python=sys.version, generator_sha256=digest(pathlib.Path(__file__)), entries=100000,
                   directories=dirs, empty_files=empty, unicode_files=unicode, small_payload_bytes=total,
                   synthetic_payload_bytes=0 if catalog_only else GIB,
                   synthetic_payload_sha256=None if catalog_only else payload_hash.hexdigest(),
                   files={name: dict(bytes=(root / name).stat().st_size, sha256=digest(root / name))
                          for name in (('100k.zip', 'entries.jsonl', 'credit.zip') if catalog_only else ('100k.zip', 'entries.jsonl', '1gib.zip', 'credit.zip'))})
    (root / 'fixture-summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
    print(json.dumps(summary))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=pathlib.Path)
    parser.add_argument('--small', action='store_true', help='Only 256KiB tool fixture plus small credit ZIP; no performance repeat')
    parser.add_argument('--catalog-only', action='store_true', help='100k ZIP/streaming manifest plus credit ZIP only; never generate 1GiB')
    args = parser.parse_args()
    generate(args.root, args.small, args.catalog_only)
