"""Prepare the pinned public solid RAR4 fixture and independent 7-Zip expectations."""
import binascii
import json
import os
from pathlib import Path
import subprocess
import urllib.request
import uuid

scratch = Path(os.environ['PI_SCRATCH_DIR']).resolve()
oracle = Path(os.environ['NXTOOLS_ARCHIVE_7ZIP_ORACLE']).resolve(strict=True)
root = scratch / ('archive-product-opt04-rar4-' + str(uuid.uuid4()))
root.mkdir()
(root / 'owned-opt04-rar4').write_text('OPT04 owned solid RAR4 fixture\n', encoding='utf-8')
commit = '79c571f845dbdd250d69bff9d93ae034f674135e'
base = f'https://raw.githubusercontent.com/libarchive/libarchive/{commit}/'
relative = 'libarchive/test/test_read_format_rar4_solid_encrypted.rar.uu'
raw = urllib.request.urlopen(base + relative, timeout=45).read()
(root / 'source.rar.uu').write_bytes(raw)
lines = raw.splitlines()
start = next(i for i, line in enumerate(lines) if line.startswith(b'begin '))
payload = bytearray()
for line in lines[start + 1:]:
    if line == b'end':
        break
    if line:
        payload.extend(binascii.a2b_uu(line))
source = root / 'source.rar'
source.write_bytes(payload)
assert payload[:7] == b'Rar!\x1a\x07\x00', 'genuine RAR4 signature'
listing = subprocess.run([str(oracle), 'l', '-slt', '-ppassword', str(source)], check=True, capture_output=True)
text = listing.stdout.decode('utf-8', errors='replace')
assert 'Solid = +' in text and 'Type = Rar' in text, 'independent solid RAR4 declaration'
(root / 'independent-list.txt').write_bytes(listing.stdout)
expected = root / 'expected'
expected.mkdir()
subprocess.run([str(oracle), 'x', '-y', '-ppassword', '-o' + str(expected), str(source)], check=True, capture_output=True)
files = sorted(p for p in expected.rglob('*') if p.is_file())
assert [p.name for p in files] == ['a.txt', 'b.txt', 'c.txt', 'd.txt']
assert all(p.read_bytes() == ('This is from ' + p.name).encode('ascii') for p in files)
for remote, local in [('COPYING', 'libarchive-COPYING'), ('libarchive/test/test_read_format_rar_encryption.c', 'upstream-test.c')]:
    (root / local).write_bytes(urllib.request.urlopen(base + remote, timeout=45).read())
(root / 'fixture-provenance.json').write_text(json.dumps({
    'name': 'rar4-solid-encrypted', 'archive': source.name, 'source': base + relative,
    'archive_bytes': len(payload), 'independent_decoder': str(oracle),
    'files': [{'name': p.name, 'size': p.stat().st_size} for p in files],
}, indent=2), encoding='utf-8')
print(root)
