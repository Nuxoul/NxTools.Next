"""Owned tiny CPython/libarchive samples, independent literal decode; no NxTools codec."""
import argparse
import bz2
import gzip
import hashlib
import io
import json
import lzma
import os
from pathlib import Path
import struct
import subprocess
import tarfile

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--root', type=Path, required=True)
p.add_argument('--oracle', type=Path, required=True)
a = p.parse_args()
root = a.root.resolve(strict=True)
oracle = a.oracle.resolve(strict=True)
fixtures = root / 'fixtures'
fixtures.mkdir(exist_ok=False)
payload = bytes(range(256)) * 16
files = {'alpha.bin': payload, 'beta.bin': b'B' * 2048}
archive = io.BytesIO()
with tarfile.open(fileobj=archive, mode='w', format=tarfile.PAX_FORMAT) as writer:
    for name, body in files.items():
        info = tarfile.TarInfo(name)
        info.size = len(body)
        info.mtime = 0
        writer.addfile(info, io.BytesIO(body))
tar = archive.getvalue()
(fixtures / 'plain.tar').write_bytes(tar)
for suffix, encoder, decoder in [('gz', lambda b: gzip.compress(b, mtime=0), gzip.decompress),
                                 ('bz2', bz2.compress, bz2.decompress),
                                 ('xz', lambda b: lzma.compress(b, preset=0), lzma.decompress)]:
    encoded = encoder(payload)
    assert decoder(encoded) == payload
    (fixtures / ('single.' + suffix)).write_bytes(encoded)
    encoded = encoder(tar)
    assert decoder(encoded) == tar
    (fixtures / ('plain.' + {'gz': 'tgz', 'bz2': 'tbz2', 'xz': 'txz'}[suffix])).write_bytes(encoded)
(fixtures / 'known.xz').write_bytes(lzma.compress(b'\0' * 4096, preset=0))
assert lzma.decompress((fixtures / 'known.xz').read_bytes()) == b'\0' * 4096
bsdtar = Path(os.environ['WINDIR']) / 'System32/tar.exe'
commands = []

def decode(target, body):
    args = [str(oracle), 'x', '-so', '-tzstd', str(target)]
    result = subprocess.run(args, capture_output=True, timeout=20)
    commands.append({'args': args, 'exit': result.returncode})
    assert result.returncode == 0 and result.stdout == body, (target.name, result.stderr)

def zstd(body, name):
    source = fixtures / (name + '.input')
    source.write_bytes(body)
    target = fixtures / name
    args = [str(bsdtar), '--zstd', '--format=raw', '-cf', str(target), '-C', str(fixtures), source.name]
    result = subprocess.run(args, capture_output=True, timeout=20)
    commands.append({'args': args, 'exit': result.returncode})
    assert result.returncode == 0, result.stderr
    decode(target, body)
    return target.read_bytes()

unknown = zstd(payload, 'single.zst')
zstd(tar, 'plain.tzst')
assert unknown[:4] == bytes.fromhex('28b52ffd') and unknown[4] >> 6 == 0 and not unknown[4] & 32
(fixtures / 'unknown.zst').write_bytes(unknown)
compressed = zstd(b'B' * 4096, 'known-base.zst')
# Structural correct-FCS variant of independent libarchive data, re-decoded independently.
assert compressed[4] >> 6 == 0 and not compressed[4] & 32
position = 6 + [0, 1, 2, 4][compressed[4] & 3]
known = bytearray(compressed)
known[4] |= 0x40
known[position:position] = struct.pack('<H', 4096 - 256)
(fixtures / 'known.zst').write_bytes(known)
decode(fixtures / 'known.zst', b'B' * 4096)
(fixtures / 'mixed.zst').write_bytes(bytes(known) + unknown)
decode(fixtures / 'mixed.zst', b'B' * 4096 + payload)
(fixtures / 'two-known.zst').write_bytes(bytes(known) + bytes(known))
decode(fixtures / 'two-known.zst', b'B' * 8192)
# Bounded structural false-declaration negative: actual65537, wrong FCS4096. Never a valid oracle.
false_base = zstd((bytes(range(256)) * 257)[:65537], 'false-size-base.zst')
assert false_base[4] >> 6 == 0 and not false_base[4] & 32
false_frame = bytearray(false_base)
false_frame[4] |= 0x40
false_position = 6 + [0, 1, 2, 4][false_frame[4] & 3]
false_frame[false_position:false_position] = struct.pack('<H', 4096 - 256)
(fixtures / 'false-size.zst').write_bytes(false_frame)
bad_xz = bytearray((fixtures / 'known.xz').read_bytes())
bad_xz[34] ^= 0x40
(fixtures / 'bad-known.xz').write_bytes(bad_xz)
for name in ['single.gz', 'plain.tgz']:
    bad = bytearray((fixtures / name).read_bytes())
    bad[-8] ^= 1
    (fixtures / ('bad.gz' if name == 'single.gz' else 'bad.tgz')).write_bytes(bad)
manifest = {'producer': 'CPython tarfile/gzip/bz2/lzma and Windows libarchive raw+zstd',
            'tool': {'path': str(bsdtar), 'sha256': hashlib.sha256(bsdtar.read_bytes()).hexdigest()},
            'oracle': {'path': str(oracle), 'sha256': hashlib.sha256(oracle.read_bytes()).hexdigest()},
            'files': [{'name': path.name, 'bytes': path.stat().st_size, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
                      for path in fixtures.iterdir() if path.is_file()],
            'payloads': [{'name': name, 'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()} for name, body in files.items()],
            'commands': commands,
            'structural_variants': ['known.zst correct FCS independently decoded', 'bad.gz/bad.tgz CRC mutation', 'bad-known.xz payload byte34 corruption', 'false-size.zst false FCS4096 actual65537, invalid negative only']}
(fixtures / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
print(json.dumps(manifest, indent=2))
