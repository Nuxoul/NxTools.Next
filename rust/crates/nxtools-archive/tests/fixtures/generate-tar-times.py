"""Small caller-owned TAR time fixtures. CPython GNU writer is independent of NxTools."""
import bz2
import gzip
import lzma
import os
import subprocess
import io
import pathlib
import sys
import tarfile

root = pathlib.Path(sys.argv[1])
path = root / 'gnu-negative.tar'
with tarfile.open(path, 'w', format=tarfile.GNU_FORMAT) as archive:
    entry = tarfile.TarInfo('payload.txt')
    entry.size = 1
    entry.mtime = -1
    archive.addfile(entry, io.BytesIO(b'x'))
with tarfile.open(path) as archive:
    entry = archive.getmembers()[0]
    assert entry.mtime == -1
    assert archive.extractfile(entry).read() == b'x'
assert path.read_bytes()[136:148] == b'\xff' * 12
print('independent_gnu_negative_verified=1')

def pax_time(value):
    record = b'mtime=' + value.encode('ascii') + b'\n'
    length = len(record) + 2
    while len(str(length)) + 1 + len(record) != length:
        length = len(str(length)) + 1 + len(record)
    return str(length).encode('ascii') + b' ' + record


def member(name, kind, mtime, payload):
    entry = tarfile.TarInfo(name)
    entry.type = kind
    entry.mtime = mtime
    entry.size = len(payload)
    return entry.tobuf(format=tarfile.USTAR_FORMAT) + payload + b'\0' * (-len(payload) % 512)


# POSIX raw record oracle, not CPython's merge behavior (which differs for empty times).
raw = member('global', tarfile.XGLTYPE, 0, pax_time('1500000000'))
raw += member('first.txt', tarfile.REGTYPE, 1600000000, b'x')
raw += member('clear', tarfile.XGLTYPE, 0, pax_time(''))
raw += member('second.txt', tarfile.REGTYPE, 1600000001, b'y')
raw += b'\0' * 1024
assert pax_time('') == b'9 mtime=\n'
(root / 'pax-global-clear.tar').write_bytes(raw)
print('raw_pax_global_clear_verified=1')

raw = member('global', tarfile.XGLTYPE, 0, pax_time('1500000000'))
raw += member('local', tarfile.XHDTYPE, 0, pax_time('1400000000'))
raw += member('override.txt', tarfile.REGTYPE, 1600000000, b'a')
raw += member('global-again.txt', tarfile.REGTYPE, 1600000001, b'b')
raw += member('delete', tarfile.XHDTYPE, 0, pax_time(''))
raw += member('deleted.txt', tarfile.REGTYPE, 1600000002, b'c')
raw += member('global-still.txt', tarfile.REGTYPE, 1600000003, b'd')
raw += member('global-clear', tarfile.XGLTYPE, 0, pax_time(''))
raw += member('header.txt', tarfile.REGTYPE, 1600000004, b'e')
(root / 'pax-local.tar').write_bytes(raw + b'\0' * 1024)

for value, label in [('-86401', 'negative-day'), ('-2208988800', 'year1900'), ('0', 'zero'),
                     ('1600000001', 'positive'), ('-11644473599', 'lower-native'),
                     ('253402300799', 'upper-native'), ('-11644473601', 'below-native'),
                     ('253402300800', 'above-native')]:
    with tarfile.open(root / ('gnu-' + label + '.tar'), 'w', format=tarfile.GNU_FORMAT) as archive:
        entry = tarfile.TarInfo('payload.txt'); entry.size = 1; entry.mtime = int(value)
        archive.addfile(entry, io.BytesIO(b'x'))

# Nonempty malformed values stay invalid, including after a committed valid member.
for scope in [tarfile.XHDTYPE, tarfile.XGLTYPE]:
    raw = member('first.txt', tarfile.REGTYPE, 1600000000, b'x')
    raw += member('bad', scope, 0, pax_time('not-a-time'))
    raw += member('second.txt', tarfile.REGTYPE, 1600000001, b'y')
    (root / ('pax-invalid-' + scope.decode() + '.tar')).write_bytes(raw + b'\0' * 1024)

# Directory metadata validation is after file publication, independently of token ordering.
for scope in [tarfile.XHDTYPE, tarfile.XGLTYPE]:
    raw = member('bad', scope, 0, pax_time('not-a-time'))
    raw += member('nested/', tarfile.DIRTYPE, 1600000000, b'')
    if scope == tarfile.XGLTYPE:
        raw += member('clear', tarfile.XGLTYPE, 0, pax_time(''))
    raw += member('nested/payload.txt', tarfile.REGTYPE, 1600000001, b'x')
    (root / ('pax-invalid-directory-' + scope.decode() + '.tar')).write_bytes(raw + b'\0' * 1024)

raw = bytearray(member('payload.txt', tarfile.REGTYPE, 0, b'x'))
for label, value in [('absent', b'\0' * 12), ('invalid-marker', b'\x81' + b'\xff' * 11),
                     ('overflow', b'\x80\0\0\x01' + b'\0' * 8),
                     ('invalid-octal', b'00000000008\0')]:
    current = raw.copy(); current[136:148] = value; current[148:156] = b' ' * 8
    current[148:156] = ('%06o\0 ' % sum(current[:512])).encode('ascii')
    (root / ('header-' + label + '.tar')).write_bytes(current + b'\0' * 1024)

for name in ['gnu-negative', 'pax-global-clear']:
    path = root / (name + '.tar'); raw = path.read_bytes()
    (root / (name + '.tar.gz')).write_bytes(gzip.compress(raw, mtime=0))
    (root / (name + '.tar.bz2')).write_bytes(bz2.compress(raw))
    (root / (name + '.tar.xz')).write_bytes(lzma.compress(raw))
    subprocess.run([str(pathlib.Path(os.environ['WINDIR']) / 'System32' / 'tar.exe'),
                    '--zstd', '--format=raw', '-cf', str(root / (name + '.tar.zst')),
                    '-C', str(root), path.name], check=True)
print('tar_time_fixtures=' + str(len(list(root.glob('*.tar*')))))
