"""Independent encoded bodies + Python envelope, cryptography AES; private stdin oracle.
Small synthetic fixture generation only, never an NxTools writer or user archive.
"""
import binascii
import bz2
import hashlib
import lzma
import hmac
import json
import os
from pathlib import Path
import runpy
import struct
import subprocess
import zipfile
import zlib
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

ROOT = Path(__file__).parent
PASSWORD = b'synthetic-private-ZIP-secret'
PAYLOAD = '独立 ZIP method fixture\n'.encode() + bytes(range(256)) * 4
classic_encrypt = runpy.run_path(str(ROOT / 'generate-zip-crypto.py'))['encrypt']


def records(method):
    if method in (9, 14, 95, 93):
        name = {9: 'deflate64', 14: 'lzma', 95: 'xz', 93: 'zstd'}[method]
        path = ROOT / f'independent-{name}.zip'
        raw = path.read_bytes()
        result = []
        with zipfile.ZipFile(path) as archive:
            for item in archive.infolist():
                position = item.header_offset
                offset = position + 30 + int.from_bytes(raw[position+26:position+28], 'little') + int.from_bytes(raw[position+28:position+30], 'little')
                result.append((item.filename, PAYLOAD if item.file_size else b'', raw[offset:offset+item.compress_size], item.flag_bits & 2))
        return result
    result = []
    for name, data in [('资料/payload.bin', PAYLOAD), ('empty.bin', b''), ('empty-dir/', b'')]:
        compressor = zlib.compressobj(6, zlib.DEFLATED, -15)
        encoded = data if method == 0 else bz2.compress(data) if method == 12 else compressor.compress(data) + compressor.flush()
        result.append((name, data, encoded, 0))
    return result


def make(method, kind, version=2, strength=3, mode='normal'):
    local, central = bytearray(), bytearray()
    for ordinal, (name, data, compressed, method_flags) in enumerate(records(method)):
        raw, offset, time = name.encode(), len(local), 0x4321
        aes_mode = kind == 'aes'
        crc = 0 if aes_mode and version == 2 else binascii.crc32(data)
        descriptor = mode in ('descriptor', 'unsigned-descriptor', 'zip64', 'bad-descriptor')
        flags = 0x801 | method_flags | (8 if descriptor else 0)
        if mode == 'trailing' and ordinal == 0:
            compressed += b'\x00'
        if aes_mode:
            key_length = (strength + 1) * 8
            salt = bytes((ordinal * 31 + i) & 255 for i in range(key_length // 2))
            derived = hashlib.pbkdf2_hmac('sha1', PASSWORD, salt, 1000, 2 * key_length + 2)
            cipher = Cipher(algorithms.AES(derived[:key_length]), modes.ECB()).encryptor()
            stream = cipher.update(b''.join(i.to_bytes(16, 'little') for i in range(1, (len(compressed)+15)//16+1))) + cipher.finalize()
            encrypted = bytes(a ^ b for a, b in zip(compressed, stream))
            encoded = salt + derived[-2:] + encrypted + hmac.digest(derived[key_length:2*key_length], encrypted, 'sha1')[:10]
            aes_extra = struct.pack('<HHH2sBH', 0x9901, 7, version, b'AE', strength, method)
        else:
            encoded = classic_encrypt(bytes(range(11)) + bytes([time >> 8 if descriptor else crc >> 24]) + compressed)
            aes_extra = b''
        zip64 = mode == 'zip64'
        size_extra = struct.pack('<HHQQ', 1, 16, len(data), len(encoded)) if zip64 else b''
        local_extra = central_extra = size_extra + aes_extra
        if ordinal == 0:
            if mode == 'duplicate-local': local_extra += aes_extra
            if mode == 'duplicate-central': central_extra += aes_extra
            if mode == 'truncated-local': local_extra = aes_extra[:-1]
            if mode == 'truncated-central': central_extra = aes_extra[:-1]
            if mode == 'missing-extra': local_extra = central_extra = b''
        local += struct.pack('<4s5H3I2H', b'PK\x03\x04', 63, flags, 99 if aes_mode else method, time, 33,
                             0 if descriptor else crc,
                             0xffffffff if zip64 else 0 if descriptor else len(encoded),
                             0xffffffff if zip64 else 0 if descriptor else len(data), len(raw), len(local_extra)) + raw + local_extra + encoded
        if descriptor:
            local += (b'' if mode == 'unsigned-descriptor' else b'PK\x07\x08')
            local += struct.pack('<IQQ' if zip64 else '<III', crc, len(encoded), len(data) + (1 if mode == 'bad-descriptor' and ordinal == 0 else 0))
        central += struct.pack('<4s6H3I5H2I', b'PK\x01\x02', 63, 63, flags, 99 if aes_mode else method, time, 33,
                               crc, 0xffffffff if zip64 else len(encoded), 0xffffffff if zip64 else len(data),
                               len(raw), len(central_extra), 0, 0, 0, 0x10 if name.endswith('/') else 0, offset) + raw + central_extra
    return local + central + struct.pack('<4s4H2IH', b'PK\x05\x06', 0, 0, 3, 3, len(central), len(local), 0)


if __name__ == '__main__':
    oracle = os.environ['NXTOOLS_ARCHIVE_7ZIP_ORACLE']
    manifest = []
    jobs = []
    for method in (0, 8, 9, 12, 14, 93, 95):
        for kind in ('classic', 'aes'):
            jobs.append((f'combo-{kind}-{method}.zip', method, kind, 2, 3, 'normal', None))
    for version in (1, 2):
        for strength in (1, 2, 3):
            jobs.append((f'combo-ae{version}-store-{(strength+1)*64}.zip', 0, 'aes', version, strength, 'normal', None))
    for version in (1, 2):
        for mode in ('descriptor', 'unsigned-descriptor', 'zip64'):
            jobs.append((f'combo-ae{version}-{mode}.zip', 8, 'aes', version, 3, mode, None))
    for mode in ('duplicate-local', 'duplicate-central', 'truncated-local', 'truncated-central', 'missing-extra', 'bad-descriptor', 'trailing'):
        jobs.append((f'bad-aes-{mode}.zip', 8, 'aes', 2, 3, mode, 'AuthenticationOrDataError' if mode == 'trailing' else 'CorruptArchive'))
    for name, method, kind, version, strength, mode, failure in jobs:
        path = ROOT / name
        path.write_bytes(make(method, kind, version, strength, mode))
        if failure is None:
            if method != 95:
                checked = subprocess.run([oracle, 't', str(path)], input=(PASSWORD+b'\n')*4, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
                assert checked.returncode == 0, ('independent integrity oracle', name)
            else:
                # 7-Zip26 reports trailing data for encrypted XZ empty entries. Validate all
                # bounded decrypted XZ streams independently with CPython/liblzma instead.
                source = path.read_bytes()
                with zipfile.ZipFile(path) as archive:
                    for item in archive.infolist():
                        position = item.header_offset
                        offset = position + 30 + int.from_bytes(source[position+26:position+28], 'little') + int.from_bytes(source[position+28:position+30], 'little')
                        encrypted = source[offset:offset+item.compress_size]
                        if kind == 'classic':
                            decoded = zipfile._ZipDecrypter(PASSWORD)(encrypted)[12:]
                        else:
                            key_length = (strength+1)*8
                            salt_length = key_length//2
                            derived = hashlib.pbkdf2_hmac('sha1', PASSWORD, encrypted[:salt_length], 1000, 2*key_length+2)
                            body = encrypted[salt_length+2:-10]
                            assert hmac.compare_digest(hmac.digest(derived[key_length:2*key_length], body, 'sha1')[:10], encrypted[-10:])
                            cipher = Cipher(algorithms.AES(derived[:key_length]), modes.ECB()).encryptor()
                            stream = cipher.update(b''.join(i.to_bytes(16,'little') for i in range(1,(len(body)+15)//16+1)))+cipher.finalize()
                            decoded = bytes(a^b for a,b in zip(body,stream))
                        reader = lzma.LZMADecompressor()
                        assert reader.decompress(decoded) == (PAYLOAD if item.file_size else b'') and reader.eof and not reader.unused_data
            observed = subprocess.run([oracle, 'x', '-so', str(path), '资料/payload.bin'], input=(PASSWORD+b'\n')*4, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20)
            assert observed.returncode == 0 and observed.stdout == PAYLOAD, ('independent bytes oracle', name)
        manifest.append(dict(name=name, method=method, kind=kind, version=version if kind == 'aes' else None, strength=strength if kind == 'aes' else None,
                             error=failure, sha256=hashlib.sha256(path.read_bytes()).hexdigest(), bytes=1050, files=2))
    (ROOT / 'zip-combinations.json').write_text(json.dumps(manifest, indent=2)+'\n', encoding='utf-8')
    print('independent positive oracles', sum(item['error'] is None for item in manifest), 'malformed cases', sum(item['error'] is not None for item in manifest))
