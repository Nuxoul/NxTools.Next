"""Independent codec + PKWARE ZIP framing; never invokes an NxTools writer."""
import binascii
import hashlib
import lzma
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).parent
TAR = Path(os.environ['WINDIR']) / 'System32' / 'tar.exe'
PAYLOAD = '独立 ZIP method fixture\n'.encode() + bytes(range(256)) * 4


def zip_bytes(records, method):
    local, central = bytearray(), bytearray()
    for name, data, encoded in records:
        raw = name.encode('utf-8')
        crc, offset = binascii.crc32(data), len(local)
        local += struct.pack('<4s5H3I2H', b'PK\x03\x04', 63, 0x800, method, 0, 33,
                             crc, len(encoded), len(data), len(raw), 0) + raw + encoded
        central += struct.pack('<4s6H3I5H2I', b'PK\x01\x02', 63, 63, 0x800, method, 0, 33,
                               crc, len(encoded), len(data), len(raw), 0, 0, 0, 0,
                               0x10 if name.endswith('/') else 0, offset) + raw
    return local + central + struct.pack('<4s4H2IH', b'PK\x05\x06', 0, 0, len(records),
                                        len(records), len(central), len(local), 0)


if __name__ == '__main__':
    codec = sys.argv[1] if len(sys.argv) > 1 else 'zstd'
    assert codec in ('zstd', 'xz', 'lzma', 'deflate64')
    print(subprocess.check_output([str(TAR), '--version'], text=True).strip())
    with tempfile.TemporaryDirectory(prefix='archive-zip-method-oracle-') as temporary:
        owned = Path(temporary)
        def compress(data):
            if codec == 'xz':
                return lzma.compress(data, format=lzma.FORMAT_XZ, preset=1)
            source, target = owned / 'source.bin', owned / 'encoded.zst'
            source.write_bytes(data)
            subprocess.run([str(TAR), '--zstd', '--format=raw', '-cf', str(target),
                            '-C', str(owned), source.name], check=True)
            return target.read_bytes()
        path = ROOT / f'independent-{codec}.zip'
        sources = [('资料/payload.bin', PAYLOAD), ('empty.bin', b''), ('empty-dir/', b'')]
        if codec == 'lzma':
            with zipfile.ZipFile(path, 'w', compression=zipfile.ZIP_LZMA) as archive:
                for name, data in sources:
                    info = zipfile.ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
                    info.compress_type = zipfile.ZIP_LZMA
                    archive.writestr(info, data)
            with zipfile.ZipFile(path) as archive:
                assert archive.read(sources[0][0]) == PAYLOAD
        elif codec == 'deflate64':
            oracle = Path(os.environ['NXTOOLS_ARCHIVE_7ZIP_ORACLE'])
            for name, data in sources:
                item = owned / name
                if name.endswith('/'):
                    item.mkdir(parents=True, exist_ok=True)
                else:
                    item.parent.mkdir(parents=True, exist_ok=True)
                    item.write_bytes(data)
            generated = owned / 'generated.zip'
            subprocess.run([str(oracle), 'a', '-tzip', '-mm=Deflate64', '-mx=9', '-mmt=1',
                            str(generated), *[name for name, _ in sources]], cwd=owned,
                           check=True, stdout=subprocess.PIPE)
            # Keep independent Deflate64 bodies; normalize only ZIP framing/timestamps/order.
            original = generated.read_bytes()
            records = []
            with zipfile.ZipFile(generated) as archive:
                for name, data in sources:
                    item = archive.getinfo(name)
                    assert item.compress_type == (0 if not data else 9)
                    position = item.header_offset
                    start = position + 30 + int.from_bytes(original[position+26:position+28], 'little') + int.from_bytes(original[position+28:position+30], 'little')
                    encoded = original[start:start+item.compress_size]
                    # Empty Deflate64 stream is the standard fixed-Huffman final EOB.
                    records.append((name, data, encoded if data else bytes.fromhex('0300')))
            path.write_bytes(zip_bytes(records, 9))
        else:
            records = [(name, data, compress(data)) for name, data in sources]
            path.write_bytes(zip_bytes(records, 95 if codec == 'xz' else 93))
        if codec == 'deflate64':
            subprocess.run([str(oracle), 't', str(path)], check=True, stdout=subprocess.PIPE)
            observed = subprocess.check_output([str(oracle), 'x', '-so', str(path), '资料/payload.bin'])
        else:
            observed = subprocess.check_output([str(TAR), '-xOf', str(path), '资料/payload.bin'])
        assert observed == PAYLOAD
        print(path.name, hashlib.sha256(path.read_bytes()).hexdigest(), 'payload', len(observed),
              hashlib.sha256(observed).hexdigest())
