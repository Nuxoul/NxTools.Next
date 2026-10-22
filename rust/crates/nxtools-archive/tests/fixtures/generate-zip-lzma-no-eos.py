"""Stock7-Zip26.00 genuine eos=off ZIP; never edits flags or compressed bytes."""
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

root = Path(__file__).parent
oracle = Path(os.environ['NXTOOLS_ARCHIVE_7ZIP_ORACLE'])
payload = '独立 ZIP method fixture\n'.encode() + bytes(range(256)) * 4
expected = '25f64ae048f3c7276adefa0f585e6d54dd5309a163b6a9a4521b55ece831083a'
with tempfile.TemporaryDirectory(prefix='archive-no-eos-oracle-') as temporary:
    owned = Path(temporary)
    (owned / '资料').mkdir()
    (owned / '资料/payload.bin').write_bytes(payload)
    streams = []
    for mode in ('off', 'on'):
        target = owned / f'{mode}.zip'
        subprocess.run([str(oracle), 'a', '-tzip', f'-mm=LZMA:eos={mode}', '-mx=9', '-mmt=1', '-mcu=on', '-mtc=off', '-mta=off', '-mtm=off', '-y', str(target), '资料/payload.bin'], cwd=owned, check=True, stdout=subprocess.PIPE)
        raw = target.read_bytes()
        with zipfile.ZipFile(target) as archive:
            item = archive.infolist()[0]
            assert item.compress_type == 14 and bool(item.flag_bits & 2) == (mode == 'on')
            assert item.file_size == 1050 and len(archive.infolist()) == 1
            offset = item.header_offset + 30 + int.from_bytes(raw[26:28], 'little') + int.from_bytes(raw[28:30], 'little')
            streams.append(raw[offset:offset + item.compress_size])
        subprocess.run([str(oracle), 't', str(target)], check=True, stdout=subprocess.PIPE)
        assert subprocess.check_output([str(oracle), 'x', '-so', str(target), '资料/payload.bin']) == payload
        if mode == 'off':
            assert hashlib.sha256(raw).hexdigest() == expected
            (root / 'independent-lzma-no-eos.zip').write_bytes(raw)
    assert not streams[1].startswith(streams[0]) and not streams[0].startswith(streams[1])
print('genuine noEOS ZIP', expected, 'payload', hashlib.sha256(payload).hexdigest())
