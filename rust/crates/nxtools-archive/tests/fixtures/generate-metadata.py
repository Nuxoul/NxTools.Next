"""Independent CPython synthetic extraction metadata fixtures; output directory is caller-owned."""
import bz2
import gzip
import io
import lzma
import pathlib
import sys
import tarfile
import os
import subprocess

root = pathlib.Path(sys.argv[1])
with tarfile.open(root / 'synthetic.tar', 'w', format=tarfile.PAX_FORMAT) as archive:
    for name, directory in [('nested/', True), ('nested/payload.txt', False)]:
        entry = tarfile.TarInfo(name)
        entry.mtime = 1646370367
        entry.pax_headers = {'mtime': '1646370367.1234567'}
        entry.mode = 0o755 if directory else 0o644
        entry.type = tarfile.DIRTYPE if directory else tarfile.REGTYPE
        entry.size = 0 if directory else len(b'tar metadata')
        archive.addfile(entry, None if directory else io.BytesIO(b'tar metadata'))
raw = (root / 'synthetic.tar').read_bytes()
(root / 'synthetic.tar.gz').write_bytes(gzip.compress(raw, mtime=123))
(root / 'synthetic.tar.bz2').write_bytes(bz2.compress(raw))
(root / 'synthetic.tar.xz').write_bytes(lzma.compress(raw))
subprocess.run([str(pathlib.Path(os.environ['WINDIR']) / 'System32' / 'tar.exe'), '--zstd', '--format=raw', '-cf', str(root / 'synthetic.tar.zst'), '-C', str(root), 'synthetic.tar'], check=True)
for index, value in enumerate(['-0.0000000001', '-1.1234567899', '0', '1646370367.1234567899', '253402300800', 'not-a-time']):
    with tarfile.open(root / f'edge-{index}.tar', 'w', format=tarfile.PAX_FORMAT) as archive:
        entry = tarfile.TarInfo('payload.txt'); entry.size = 1; entry.mtime = 123
        entry.pax_headers = {'mtime': value}; archive.addfile(entry, io.BytesIO(b'x'))
print('synthetic_metadata_fixtures=11')
