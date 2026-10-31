#!/usr/bin/env python3
"""Replay retained HZ4 reader witnesses and fail-closed classifier controls; owned scratch only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import uuid
import zlib
from archive_hz4_reader import SEVEN_SHA, TAR_SHA, qualify_bytes


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('witness-json', 'encoded-output', 'sevenzip', 'bsdtar', 'evidence'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    scratch = Path(os.environ['PI_SCRATCH_DIR']).resolve()
    assert a.evidence.resolve().is_relative_to(scratch)
    a.evidence.mkdir(exist_ok=False)
    seven_sha = hashlib.sha256(a.sevenzip.read_bytes()).hexdigest().upper()
    tar_sha = hashlib.sha256(a.bsdtar.read_bytes()).hexdigest().upper()
    assert seven_sha == SEVEN_SHA and tar_sha == TAR_SHA
    version_run = subprocess.run([str(a.bsdtar), '--version'], capture_output=True, timeout=30)
    assert version_run.returncode == 0
    version = version_run.stdout.decode().strip()
    witness = json.loads(a.witness_json.read_text(encoding='utf-8'))
    owned = a.evidence / ('archive-product-' + str(uuid.uuid4()))
    owned.mkdir()
    rows = []
    for index, (variant, expected) in enumerate(zip(witness['variants'], ((0, 1), (2, 0), (0, 0)))):
        source = Path(witness['root']) / (variant['variant'] + '.7z')
        data = source.read_bytes()
        assert hashlib.sha256(data).hexdigest().upper() == variant['sha256']
        copy = owned / source.name
        copy.write_bytes(data)
        readers = []
        for reader_index, argv in enumerate(([str(a.sevenzip), 't', '-bd', str(copy)], [str(a.bsdtar), '-tf', str(copy)])):
            run = subprocess.run(argv, capture_output=True, timeout=30)
            stem = str(index) + '-' + str(reader_index)
            (a.evidence / (stem + '.stdout')).write_bytes(run.stdout)
            (a.evidence / (stem + '.stderr')).write_bytes(run.stderr)
            readers.append({'argv': argv, 'exit': run.returncode})
        assert tuple(r['exit'] for r in readers) == expected
        try:
            result = qualify_bytes(data, seven_sha, tar_sha, version)
        except AssertionError as e:
            result = {'status': 'FAIL', 'rejected': str(e)}
        assert result['status'] == ('reader-unsupported' if index == 0 else 'FAIL')
        rows.append({'variant': variant['variant'], 'readers': readers, 'classifier': result})
    encoded = a.encoded_output.read_bytes()
    assert qualify_bytes(encoded, seven_sha, tar_sha, version)['status'] == 'reader-unsupported'
    canonical = (owned / (witness['variants'][0]['variant'] + '.7z')).read_bytes()
    def reframe(data):
        value = bytearray(data)
        offset, length = struct.unpack_from('<QQ', value, 12)
        struct.pack_into('<I', value, 28, zlib.crc32(value[32 + offset:32 + offset + length]))
        struct.pack_into('<I', value, 8, zlib.crc32(value[12:32]))
        return bytes(value)
    bad_start = bytearray(canonical); bad_start[8] ^= 1
    bad_next = bytearray(canonical); bad_next[-1] ^= 1
    bad_main = bytearray(canonical); bad_main[32] ^= 1
    other_structure = bytearray(canonical); other_structure[-8] ^= 1
    bad_encoded_pack = bytearray(encoded); bad_encoded_pack[81] ^= 1
    tests = [
        ('start-crc', bad_start, seven_sha, tar_sha, version),
        ('next-crc', bad_next, seven_sha, tar_sha, version),
        ('main-pack-crc', bad_main, seven_sha, tar_sha, version),
        ('different-valid-crc-metadata', reframe(other_structure), seven_sha, tar_sha, version),
        ('encoded-pack-crc', bad_encoded_pack, seven_sha, tar_sha, version),
        ('trailing-bytes', canonical + b'\0', seven_sha, tar_sha, version),
        ('unknown-primary-hash', canonical, '0' * 64, tar_sha, version),
        ('unknown-secondary-hash', canonical, seven_sha, '0' * 64, version),
        ('unknown-version', canonical, seven_sha, tar_sha, 'bsdtar 3.8.0 - libarchive 3.8.0'),
    ]
    for name, data, s, t, v in tests:
        try:
            qualify_bytes(bytes(data), s, t, v)
        except (AssertionError, ValueError) as e:
            rows.append({'negative': name, 'status': 'PASS', 'rejected': str(e)})
        else:
            raise AssertionError('fail-closed control accepted: ' + name)
    value = {'version': version, 'seven_sha256': seven_sha, 'bsdtar_sha256': tar_sha,
             'encoded_sha256': hashlib.sha256(encoded).hexdigest().upper(), 'rows': rows,
             'summary': {'reader_witnesses': 3, 'encoded_positive': 1, 'negative_rejections': len(tests)}}
    (a.evidence / 'results.json').write_text(json.dumps(value, indent=2), encoding='utf-8')
    print(json.dumps(value['summary']))


if __name__ == '__main__':
    main()
