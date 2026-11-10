"""Prepare owned OPT-00 format fixtures and independent payload expectations."""
import binascii
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request
import uuid

scratch = Path(os.environ['PI_SCRATCH_DIR']).resolve(strict=True)
workspace = Path(__file__).resolve().parents[3]
root = scratch / ('archive-product-format-' + str(uuid.uuid4()))
root.mkdir()
(root / 'owned-opt-fixtures').write_text('OPT-00 owned format baseline\n', encoding='utf-8')
upstream = workspace / 'rust/crates/nxtools-archive/tests/fixtures/upstream'
shutil.copyfile(upstream / 'README.md', root / 'upstream-fixture-README.md')
decoder = subprocess.run(['tar.exe', '--version'], check=True, capture_output=True, text=True)
(root / 'independent-decoder.txt').write_text(decoder.stdout, encoding='utf-8')
fixtures = []


def prepare(name, archive):
    folder = root / name
    folder.mkdir()
    return folder, folder / archive


folder, archive = prepare('sevenz-solid', 'source.7z')
shutil.copyfile(upstream / 'solid.7z', archive)
fixtures.append((folder, archive, 'sevenz-rust2 0.23.0 tests/resources/solid.7z; checked-in attributed fixture'))
commit = '79c571f845dbdd250d69bff9d93ae034f674135e'
base = f'https://raw.githubusercontent.com/libarchive/libarchive/{commit}/'
folder, archive = prepare('rar5-solid', 'source.rar')
relative = 'libarchive/test/test_read_format_rar5_multiple_files_solid.rar.uu'
raw = urllib.request.urlopen(base + relative, timeout=45).read()
(folder / 'source.rar.uu').write_bytes(raw)
lines = raw.splitlines()
start = next(i for i, line in enumerate(lines) if line.startswith(b'begin '))
payload = bytearray()
for line in lines[start + 1:]:
    if line == b'end':
        break
    if line:
        payload.extend(binascii.a2b_uu(line))
archive.write_bytes(payload)
assert len(payload) == 677, 'pinned independent RAR fixture length'
(root / 'libarchive-COPYING').write_bytes(urllib.request.urlopen(base + 'COPYING', timeout=45).read())
fixtures.append((folder, archive, base + relative))
for name, suffix, mode in [('targz', 'tar.gz', 'w:gz'), ('tarxz', 'tar.xz', 'w:xz')]:
    folder, archive = prepare(name, 'source.' + suffix)
    with tarfile.open(archive, mode, format=tarfile.USTAR_FORMAT) as output:
        for index in range(2):
            data = bytes((i * 31 + index * 17) % 251 for i in range(65536))
            entry = tarfile.TarInfo(f'member-{index}.bin')
            entry.size = len(data)
            entry.mtime = 1767225600
            entry.mode = 0o644
            output.addfile(entry, io.BytesIO(data))
    fixtures.append((folder, archive, 'Python standard-library USTAR + gzip/xz; two deterministic 65536-byte members'))
manifest = []
for folder, archive, provenance in fixtures:
    expected = folder / 'expected'
    expected.mkdir()
    subprocess.run(['tar.exe', '-xf', str(archive), '-C', str(expected)], check=True, capture_output=True)
    files = sorted(p.relative_to(expected).as_posix() for p in expected.rglob('*') if p.is_file())
    assert files and not any(p.is_symlink() for p in expected.rglob('*')), 'ordinary independent payloads only'
    manifest.append({'name': folder.name, 'archive': archive.name, 'source': provenance,
                     'archive_bytes': archive.stat().st_size,
                     'files': [{'name': n, 'size': (expected / n).stat().st_size} for n in files]})
(root / 'fixture-provenance.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
(scratch / 'opt-format-root.txt').write_text(str(root), encoding='utf-8')
print(root)
print(json.dumps(manifest, indent=2))
