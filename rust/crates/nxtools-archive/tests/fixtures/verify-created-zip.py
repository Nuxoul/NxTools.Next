"""Independent bounded ZIP creation oracle. Request JSON/password are private stdin, never argv.
Expectations are supplied from synthetic source literals before reading the NxTools output.
Plain ZIP: CPython zipfile; AES: stock 7-Zip, exact extraction SHA256 and sizes, not exit0 alone.
"""
import hashlib
import json
import os
import subprocess
import sys
import zipfile

request = json.load(sys.stdin)
expected = request['entries']
assert len(expected) <= 1000
with zipfile.ZipFile(request['path']) as archive:
    entries = archive.infolist()
    assert len(entries) == len(expected)
    assert len({item.filename for item in entries}) == len(entries)
    assert {item.filename for item in entries} == set(expected)
    for item in entries:
        assert item.file_size == expected[item.filename]['size']
        assert item.is_dir() == item.filename.endswith('/')
        if not item.filename.isascii():
            assert item.flag_bits & 0x800, 'UTF8 flag missing'
        if not request.get('password') or item.is_dir():
            assert item.compress_type == (0 if item.is_dir() else request['method'])
            digest, size = hashlib.sha256(), 0
            with archive.open(item) as source:
                while chunk := source.read(65536):
                    size += len(chunk)
                    assert size <= item.file_size
                    digest.update(chunk)
            assert size == item.file_size
            assert digest.hexdigest() == expected[item.filename]['sha256']
            if item.file_size > 0xffffffff:
                assert item.extract_version >= 45 and b'\x01\x00' in item.extra
        else:
            assert item.compress_type == 99 and item.flag_bits & 1
            # AES metadata must be actual AE2/strength3 and the selected inner method.
            extra, position, aes = item.extra, 0, []
            while position < len(extra):
                tag = int.from_bytes(extra[position:position+2], 'little')
                length = int.from_bytes(extra[position+2:position+4], 'little')
                value = extra[position+4:position+4+length]
                assert len(value) == length
                if tag == 0x9901:
                    aes.append(value)
                position += 4 + length
            assert len(aes) == 1 and aes[0][:5] == b'\x02\x00AE\x03'
            assert int.from_bytes(aes[0][5:7], 'little') == request['method']
            oracle = os.environ['NXTOOLS_ARCHIVE_7ZIP_ORACLE']
            decoded = subprocess.run([oracle, 'x', '-so', request['path'], item.filename],
                                     input=(request['password']+'\n').encode()*4,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
            assert decoded.returncode == 0, 'AES independent authentication failed'
            assert len(decoded.stdout) == item.file_size
            assert hashlib.sha256(decoded.stdout).hexdigest() == expected[item.filename]['sha256']
print(json.dumps({'verified_entries': len(expected), 'oracle': 'CPython zipfile + stock7Zip' if request.get('password') else 'CPython zipfile'}))
