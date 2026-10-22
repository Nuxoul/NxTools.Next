"""Independent CPython ZIP boundary fixtures; no NxTools writer."""
from pathlib import Path
import zipfile

root = Path(__file__).parent
with zipfile.ZipFile(root / 'independent-directory.zip', 'w') as archive:
    for name, data in [('folder/', b''), ('keep.txt', b'independent selected file\n')]:
        info = zipfile.ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(info, data)
with zipfile.ZipFile(root / 'independent-directory.zip') as archive:
    assert archive.read('folder/') == b''
    assert archive.read('keep.txt') == b'independent selected file\n'
