"""Synthetic ZIP crypto fixtures: Python stdlib oracle; passwords never on argv."""
import binascii
import hashlib
from pathlib import Path
import struct
import zipfile
import zlib

ROOT = Path(__file__).parent
PASSWORD = b'synthetic-private-ZIP-secret'
PAYLOAD = '独立 ZIP crypto fixture\n'.encode() + bytes(range(256)) * 4


def crc_step(value, byte):
    value ^= byte
    for _ in range(8):
        value = (value >> 1) ^ (0xedb88320 if value & 1 else 0)
    return value


def encrypt(data):
    keys = [0x12345678, 0x23456789, 0x34567890]
    def update(byte):
        keys[0] = crc_step(keys[0], byte)
        keys[1] = ((keys[1] + (keys[0] & 255)) * 134775813 + 1) & 0xffffffff
        keys[2] = crc_step(keys[2], keys[1] >> 24)
    for byte in PASSWORD:
        update(byte)
    output = bytearray()
    for byte in data:
        value = keys[2] | 2
        output.append(byte ^ (((value * (value ^ 1)) >> 8) & 255))
        update(byte)
    return bytes(output)


def make(descriptor):
    local, central = bytearray(), bytearray()
    for name, data in [('资料/payload.bin', PAYLOAD), ('empty.bin', b''), ('empty-dir/', b'')]:
        raw = name.encode()
        crc, offset, time = binascii.crc32(data), len(local), 0x4321
        compressor = zlib.compressobj(6, zlib.DEFLATED, -15)
        compressed = compressor.compress(data) + compressor.flush()
        flags = 0x801 | (8 if descriptor else 0)
        check = (time >> 8) if descriptor else crc >> 24
        encoded = encrypt(bytes(range(11)) + bytes([check]) + compressed)
        local += struct.pack('<4s5H3I2H', b'PK\x03\x04', 20, flags, 8, time, 33,
                             0 if descriptor else crc, 0 if descriptor else len(encoded),
                             0 if descriptor else len(data), len(raw), 0) + raw + encoded
        if descriptor:
            local += struct.pack('<4s3I', b'PK\x07\x08', crc, len(encoded), len(data))
        central += struct.pack('<4s6H3I5H2I', b'PK\x01\x02', 20, 20, flags, 8, time, 33,
                               crc, len(encoded), len(data), len(raw), 0, 0, 0, 0,
                               0x10 if name.endswith('/') else 0, offset) + raw
    return local + central + struct.pack('<4s4H2IH', b'PK\x05\x06', 0, 0, 3, 3, len(central), len(local), 0)


if __name__ == '__main__':
    for descriptor in (False, True):
        path = ROOT / ('independent-zipcrypto-dd.zip' if descriptor else 'independent-zipcrypto.zip')
        path.write_bytes(make(descriptor))
        with zipfile.ZipFile(path) as archive:
            assert len(archive.infolist()) == 3
            assert archive.read('资料/payload.bin', pwd=PASSWORD) == PAYLOAD
            assert archive.read('empty.bin', pwd=PASSWORD) == b''
            assert archive.read('empty-dir/', pwd=PASSWORD) == b''
        print(path.name, hashlib.sha256(path.read_bytes()).hexdigest(), len(PAYLOAD), hashlib.sha256(PAYLOAD).hexdigest())
