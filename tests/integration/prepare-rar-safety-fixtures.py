"""Copy attributed RAR data and decode with an independent oracle; never run a RAR compressor.

--compressed names an existing directory containing compressed-manifest.json, original .uu files
and independently decoded .payloads from the fixed libarchive revision. Negative copies change
only documented unknown-size header fields/CRC; they are NOT independent production fixtures.
"""
import argparse
import binascii
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, required=True)
p.add_argument('--tool', type=Path, required=True)
p.add_argument('--volumes', type=Path, required=True)
p.add_argument('--compressed', type=Path)
a = p.parse_args()
root = a.root.resolve(strict=True)
tool = a.tool.resolve(strict=True)
volumes = a.volumes.resolve(strict=True)
fixtures = root / 'fixtures'
fixtures.mkdir(exist_ok=False)
repo = Path(__file__).resolve().parents[2]
upstream = repo / 'rust/crates/nxtools-archive/tests/fixtures/upstream'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


files = []
for row in json.loads((upstream / 'manifest.json').read_text(encoding='utf-8-sig')):
    name = row['fixture']
    if not name.endswith('.rar'):
        continue
    source = upstream / name
    assert digest(source) == row['sha256'], name
    shutil.copyfile(source, fixtures / name)
    files.append({'name': name, 'source': row['source'], 'bytes': source.stat().st_size, 'sha256': row['sha256']})
for name in ['README.md', 'UNRAR-LICENSE.txt', 'manifest.json']:
    shutil.copyfile(upstream / name, fixtures / ('upstream-' + name))
volume_manifest = json.loads((volumes / 'manifest.json').read_text(encoding='utf-8-sig'))
for row in volume_manifest['files']:
    source = volumes / row['name']
    assert digest(source) == row['sha256'], row['name']
    shutil.copyfile(source, fixtures / row['name'])
    files.append({**row, 'source': volume_manifest['repository'] + '@' + volume_manifest['revision'] + '/libarchive/test'})
for name in ['manifest.json', 'expected.json', 'COPYING', 'revision.txt']:
    shutil.copyfile(volumes / name, fixtures / ('volumes-' + name))

commands = []
expected = [('rar4.rar', 'VERSION', b'unrar-0.4.0'), ('rar5.rar', 'unicodefilename\u2764\ufe0f.txt', b'foobar\n')]
for archive, member, payload in expected:
    for label, args in [('list', ['l', '-slt', str(fixtures / archive)]),
                        ('test', ['t', str(fixtures / archive)]),
                        ('decode', ['x', '-so', str(fixtures / archive), member])]:
        result = subprocess.run([str(tool), *args], capture_output=True, timeout=20)
        (root / (archive + '-' + label + '.stdout')).write_bytes(result.stdout)
        (root / (archive + '-' + label + '.stderr')).write_bytes(result.stderr)
        commands.append({'archive': archive, 'kind': label, 'args': args, 'exit': result.returncode})
        assert result.returncode == 0, (archive, label, result.returncode)
        if label == 'decode':
            assert result.stdout == payload, (archive, member)

if a.compressed:
    compressed = a.compressed.resolve(strict=True)
    manifest = json.loads((compressed / 'compressed-manifest.json').read_text(encoding='utf-8-sig'))
    assert manifest['source_revision'] == '79c571f845dbdd250d69bff9d93ae034f674135e'
    for row in manifest['files']:
        name = row['name']
        assert Path(name).name == name and digest(compressed / name) == row['sha256']
        assert digest(compressed / (name + '.uu')) == row['uu_sha256']
        shutil.copyfile(compressed / name, fixtures / name)
        shutil.copyfile(compressed / (name + '.uu'), fixtures / (name + '.uu'))
        for member in row['payloads']:
            relative = Path(member['name'])
            assert not relative.is_absolute() and '..' not in relative.parts
            source = compressed / (name + '.payloads') / relative
            assert source.stat().st_size == member['bytes'] and digest(source) == member['sha256']
            target = fixtures / (name + '.payloads') / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        files.append(row)
    shutil.copyfile(compressed / 'compressed-manifest.json', fixtures / 'compressed-manifest.json')

# A controlled negative copy, NOT a production/independent unknown-size compatibility oracle.
raw = bytearray((fixtures / 'rar4.rar').read_bytes())
pos = 7
while pos < len(raw):
    kind = raw[pos + 2]
    flags, size = struct.unpack_from('<HH', raw, pos + 3)
    if kind == 0x74:
        assert not flags & 0x100 and raw[pos + 25] == 0x33
        assert struct.unpack_from('<I', raw, pos + 11)[0] == 11
        struct.pack_into('<I', raw, pos + 11, 0xffffffff)
        struct.pack_into('<H', raw, pos, binascii.crc32(raw[pos + 2:pos + size]) & 0xffff)
        break
    pos += size
else:
    raise AssertionError('Expected RAR4 header missing')
(fixtures / 'rar4-unknown-size.rar').write_bytes(raw)
negatives = [{'name': 'rar4-unknown-size.rar', 'source': 'rar4.rar', 'bytes': len(raw),
              'sha256': hashlib.sha256(raw).hexdigest(), 'change': 'LowUnpSize=ffffffff; repaired header CRC; unchanged payload'}]

if a.compressed:
    raw = bytearray((fixtures / 'test_read_format_rar5_multiple_files_solid.rar').read_bytes())

    def vint(at):
        value = shift = 0
        for _ in range(10):
            byte = raw[at]
            at += 1
            value |= (byte & 127) << shift
            if not byte & 128:
                return value, at
            shift += 7
        raise AssertionError('Invalid bounded vint')

    pos = 8
    while pos < len(raw):
        size, start = vint(pos + 4)
        end = start + size
        kind, at = vint(start)
        flags, at = vint(at)
        if flags & 1:
            _, at = vint(at)
        data = 0
        if flags & 2:
            data, at = vint(at)
        if kind == 2:
            field = at
            value, at = vint(at)
            assert value < 128
            raw[field] |= 8
            struct.pack_into('<I', raw, pos, binascii.crc32(raw[pos + 4:end]) & 0xffffffff)
            break
        pos = end + data
    else:
        raise AssertionError('Expected RAR5 file header missing')
    (fixtures / 'rar5-solid-first-unknown.rar').write_bytes(raw)
    negatives.append({'name': 'rar5-solid-first-unknown.rar', 'source': 'test_read_format_rar5_multiple_files_solid.rar',
                      'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(),
                      'change': 'First member FHFL_UNPUNKNOWN; repaired header CRC; unchanged packed data'})

manifest = {'producer': 'Existing upstream/licensed RAR data only; negative copies are structural tests, not independent production',
            'oracle': {'path': str(tool), 'sha256': digest(tool)}, 'files': files, 'commands': commands,
            'payloads': [{'archive': archive, 'name': member, 'bytes': len(payload), 'sha256': hashlib.sha256(payload).hexdigest()}
                         for archive, member, payload in expected], 'negative_copies': negatives,
            'limitations': 'Fresh ordinary4/5 list/test/decode here; encrypted/volume/compressed decoded payloads retain separately recorded oracles. No all-feature matrix.'}
(fixtures / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
print(json.dumps(manifest, indent=2))
