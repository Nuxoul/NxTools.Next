"""Independent CPython standard-library stream fixtures; no NxTools writer oracle."""
import gzip
from pathlib import Path

root = Path(__file__).parent
payload = "独立 gzip fixture\n".encode("utf-8") + bytes(range(256))
(root / "independent.gz").write_bytes(gzip.compress(payload, mtime=0))
(root / "multi.gz").write_bytes(gzip.compress(b"first\n", mtime=0) + gzip.compress(b"second\n", mtime=0))

import zipfile
with zipfile.ZipFile(root / "independent-bzip2.zip", "w", compression=zipfile.ZIP_BZIP2) as archive:
    entry = zipfile.ZipInfo("资料/payload.bin", (2020, 1, 2, 3, 4, 6))
    entry.compress_type = zipfile.ZIP_BZIP2
    archive.writestr(entry, payload)

import io
import tarfile
with tarfile.open(root / "independent.tar", "w", format=tarfile.PAX_FORMAT) as archive:
    folder = tarfile.TarInfo("资料/empty/")
    folder.type = tarfile.DIRTYPE
    folder.mtime = 1577934246
    archive.addfile(folder)
    item = tarfile.TarInfo("资料/payload.bin")
    item.size = len(payload)
    item.mtime = 1577934246
    archive.addfile(item, io.BytesIO(payload))
    link = tarfile.TarInfo("link")
    link.type = tarfile.SYMTYPE
    link.linkname = "../outside"
    archive.addfile(link)

for mode in ("unsafe", "collision", "gnu", "sparse", "huge-pax", "orphan-pax"):
    with tarfile.open(root / (mode + ".tar"), "w", format=tarfile.GNU_FORMAT if mode == "gnu" else tarfile.PAX_FORMAT) as archive:
        names = ["same", "same"] if mode == "collision" else ["../escape" if mode == "unsafe" else "long/" + "a" * 150 if mode == "gnu" else "file"]
        for name in names:
            item = tarfile.TarInfo(name)
            item.size = 3
            if mode == "sparse":
                item.pax_headers = {"GNU.sparse.size": "3"}
            if mode == "huge-pax":
                item.pax_headers = {"comment": "x" * 70000}
            if mode == "orphan-pax":
                item.type = tarfile.XHDTYPE
                item.size = len(b"15 comment=abc\n")
                archive.addfile(item, io.BytesIO(b"15 comment=abc\n"))
            else:
                archive.addfile(item, io.BytesIO(b"abc"))

(root / "independent.tar.gz").write_bytes(gzip.compress((root / "independent.tar").read_bytes(), mtime=0))

import bz2
(root / "independent.bz2").write_bytes(bz2.compress(payload))
(root / "multi.bz2").write_bytes(bz2.compress(b"first\n") + bz2.compress(b"second\n"))
(root / "independent.tar.bz2").write_bytes(bz2.compress((root / "independent.tar").read_bytes()))

import lzma
(root / "independent.xz").write_bytes(lzma.compress(payload))
(root / "multi.xz").write_bytes(lzma.compress(b"first\n") + lzma.compress(b"second\n"))
(root / "independent.tar.xz").write_bytes(lzma.compress((root / "independent.tar").read_bytes()))
