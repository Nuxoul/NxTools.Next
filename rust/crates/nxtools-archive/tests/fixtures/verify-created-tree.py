"""Independent small owned-fixture oracle. Expectations/password use stdin, never argv.
7z: stock 7-Zip exact listing plus independently materialized kinds/sizes/SHA256.
TAR compounds: CPython tarfile streaming hashes. Never used on user archives.
"""
import hashlib
import json
import os
import subprocess
import sys
import tarfile
import tempfile

request = json.load(sys.stdin)
expected = request['entries']
assert len(expected) <= 1000
assert sum(item['size'] for item in expected.values()) <= 16 << 20
if request['format'] == '7z':
    oracle = os.environ['NXTOOLS_ARCHIVE_7ZIP_ORACLE']
    secret = ((request.get('password') or '') + '\n').encode() * 8
    def run(*args, supplied=secret):
        return subprocess.run([oracle, *args, '-sccUTF-8'], input=supplied,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
    listing = run('l', '-slt', request['path'])
    assert listing.returncode == 0, '7z independent listing failed'
    head, body_bytes = listing.stdout.replace(b'\r\n', b'\n').split(b'----------\n', 1)
    body = body_bytes.decode('utf-8')  # Strict metadata decoding; localized password prompt is outside this area.
    if request.get('solid') is not None:
        assert ('Solid = ' + ('+' if request['solid'] else '-')).encode() in head
    records = []
    for paragraph in body.strip().split('\n\n'):
        record = dict(line.split(' = ', 1) for line in paragraph.splitlines() if ' = ' in line)
        if 'Path' in record:
            records.append(record)
    assert len(records) == len(expected)
    expected_bare = {name.rstrip('/'): item for name, item in expected.items()}
    assert len(expected_bare) == len(expected)
    listed = set()
    for item in records:
        name = item['Path'].replace('\\', '/').rstrip('/')
        assert name not in listed and name in expected_bare, ascii(item)
        listed.add(name)
        assert int(item['Size']) == expected_bare[name]['size']
        if expected_bare[name]['size']:
            assert item['Encrypted'] == ('+' if request.get('password') else '-')
            assert 'LZMA2' in item['Method']
    assert listed == set(expected_bare)
    if request.get('password'):
        missing = run('l', '-slt', request['path'], supplied=b'\n' * 8)
        assert (missing.returncode != 0) == request['encrypt_header'], 'header encryption mismatch'
    with tempfile.TemporaryDirectory(prefix='tree-oracle-', dir=os.path.dirname(request['path'])) as destination:
        decoded = run('x', '-y', '-o' + destination, request['path'])
        assert decoded.returncode == 0, '7z independent decoding failed'
        actual = set()
        for parent, directories, files in os.walk(destination):
            for name in directories + files:
                path = os.path.join(parent, name)
                directory = os.path.isdir(path)
                relative = os.path.relpath(path, destination).replace('\\', '/') + ('/' if directory else '')
                assert not os.path.islink(path) and relative in expected
                actual.add(relative)
                if not directory:
                    with open(path, 'rb') as source:
                        digest = hashlib.file_digest(source, 'sha256').hexdigest()
                    assert os.path.getsize(path) == expected[relative]['size']
                    assert digest == expected[relative]['sha256']
        assert actual == set(expected)
else:
    with tarfile.open(request['path'], mode='r:*', encoding='utf-8', errors='strict') as archive:
        names = set()
        for item in archive:
            assert item.isfile() or item.isdir()
            name = item.name.rstrip('/') + ('/' if item.isdir() else '')
            assert name not in names and name in expected
            names.add(name)
            assert item.size == expected[name]['size']
            if item.isfile():
                digest, count = hashlib.sha256(), 0
                with archive.extractfile(item) as source:
                    while chunk := source.read(65536):
                        count += len(chunk)
                        assert count <= expected[name]['size']
                        digest.update(chunk)
                assert count == expected[name]['size'] and digest.hexdigest() == expected[name]['sha256']
        assert names == set(expected)
print(json.dumps({'verified_entries': len(expected), 'oracle': 'stock7Zip' if request['format'] == '7z' else 'CPython tarfile'}))
