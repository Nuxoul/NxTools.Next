"""Independent WinZip-AES fixtures: cryptography/OpenSSL AES + stdlib PBKDF2/HMAC.
Synthetic passwords enter oracle stdin only; never argv, settings or logs.
"""
import binascii
import hashlib
import hmac
import os
from pathlib import Path
import struct
import subprocess
import zipfile
import zlib
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

ROOT = Path(__file__).parent
PASSWORD = b'synthetic-private-ZIP-secret'
PAYLOAD = '独立 ZIP crypto fixture\n'.encode() + bytes(range(256)) * 4


def make(version, strength, method=8):
    local, central = bytearray(), bytearray()
    for ordinal, (name, data) in enumerate([('资料/payload.bin', PAYLOAD), ('empty.bin', b''), ('empty-dir/', b'')]):
        raw = name.encode()
        crc, offset = binascii.crc32(data) if version == 1 else 0, len(local)
        compressor = zlib.compressobj(6, zlib.DEFLATED, -15)
        compressed = data if method == 0 else compressor.compress(data) + compressor.flush()
        key_length = (strength + 1) * 8
        salt = bytes((ordinal * 31 + i) & 255 for i in range(key_length // 2))
        derived = hashlib.pbkdf2_hmac('sha1', PASSWORD, salt, 1000, 2 * key_length + 2)
        aes = Cipher(algorithms.AES(derived[:key_length]), modes.ECB()).encryptor()
        counters = b''.join(i.to_bytes(16, 'little') for i in range(1, (len(compressed) + 15) // 16 + 1))
        key_stream = aes.update(counters) + aes.finalize()
        ciphertext = bytes(a ^ b for a, b in zip(compressed, key_stream))
        encoded = salt + derived[-2:] + ciphertext + hmac.digest(derived[key_length:2*key_length], ciphertext, 'sha1')[:10]
        extra = struct.pack('<HHH2sBH', 0x9901, 7, version, b'AE', strength, method)
        local += struct.pack('<4s5H3I2H', b'PK\x03\x04', 51, 0x801, 99, 0, 33,
                             crc, len(encoded), len(data), len(raw), len(extra)) + raw + extra + encoded
        central += struct.pack('<4s6H3I5H2I', b'PK\x01\x02', 51, 51, 0x801, 99, 0, 33,
                               crc, len(encoded), len(data), len(raw), len(extra), 0, 0, 0,
                               0x10 if name.endswith('/') else 0, offset) + raw + extra
    return local + central + struct.pack('<4s4H2IH', b'PK\x05\x06', 0, 0, 3, 3, len(central), len(local), 0)


if __name__ == '__main__':
    oracle = os.environ['NXTOOLS_ARCHIVE_7ZIP_ORACLE']
    for version in (1, 2):
        for strength in (1, 2, 3):
            path = ROOT / f'independent-ae{version}-{(strength + 1) * 64}.zip'
            path.write_bytes(make(version, strength))
            # Omit -p: this tool treats bare -p as empty password; prompt reads private stdin.
            result = subprocess.run([oracle, 't', str(path)], input=(PASSWORD + b'\n') * 4,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
            assert result.returncode == 0, 'independent AES integrity oracle failed'
            result = subprocess.run([oracle, 'x', '-so', str(path), '资料/payload.bin'],
                                    input=(PASSWORD + b'\n') * 4, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
            assert result.returncode == 0 and result.stdout == PAYLOAD, 'independent AES byte oracle failed'
            print(path.name, hashlib.sha256(path.read_bytes()).hexdigest())
    # Empty stored ciphertext proves authentication cannot be skipped on immediate decoder EOF.
    path = ROOT / 'independent-ae2-store.zip'
    path.write_bytes(make(2, 3, 0))
    result = subprocess.run([oracle, 't', str(path)], input=(PASSWORD + b'\n') * 4,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
    assert result.returncode == 0, 'empty AES authentication oracle failed'
    print(path.name, hashlib.sha256(path.read_bytes()).hexdigest())
