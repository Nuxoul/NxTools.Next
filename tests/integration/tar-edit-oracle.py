"""Independent CPython TAR fixtures/oracle. Test-only; never a product codec backend."""
import hashlib
import io
import json
import sys
import tarfile
import os
import subprocess


def seed(path, fmt):
    with tarfile.open(path, {"Tar": "w", "TarGz": "w:gz", "TarXz": "w:xz"}[fmt], format=tarfile.PAX_FORMAT) as archive:
        for name, content in [("keep.txt", b"kept-data"), ("folder/", None), ("folder/child.txt", b"child-data"), ("implicit/child", b"implicit-data"), ("empty/", None), ("zero", b"")]:
            header = tarfile.TarInfo(name)
            header.uid, header.gid, header.mode, header.mtime = 123, 456, 0o750 if content is None else 0o640, 1700000000
            header.uname, header.gname = "owner", "group"
            header.pax_headers = {"mtime": "1700000000.123456789", "atime": "1700000001.987654321"}
            header.type = tarfile.DIRTYPE if content is None else tarfile.REGTYPE
            header.size = 0 if content is None else len(content)
            archive.addfile(header, None if content is None else io.BytesIO(content))


def inspect(path):
    result = {}
    with tarfile.open(path, "r:*") as archive:
        for header in archive:
            name = header.name.rstrip("/") + ("/" if header.isdir() else "")
            assert name not in result, "duplicate member"
            data = None if header.isdir() else archive.extractfile(header).read()
            result[name] = {"content": None if data is None else data.decode("utf-8"), "sha256": None if data is None else hashlib.sha256(data).hexdigest(), "uid": header.uid, "gid": header.gid, "mode": header.mode, "uname": header.uname, "gname": header.gname, "pax": header.pax_headers}
    bsdtar = os.environ.get("NXTOOLS_TAR_EDIT_BSDTAR")
    if bsdtar:
        listed = subprocess.run([bsdtar, "-tf", path], capture_output=True, check=True)
        names = listed.stdout.decode("utf-8").splitlines()
        assert sorted(names) == sorted(result), "independent bsdtar exact member list"
    return result


if __name__ == "__main__":
    if sys.argv[1] == "seed":
        seed(sys.argv[2], sys.argv[3])
    else:
        assert sys.argv[1] == "inspect"
        print(json.dumps(inspect(sys.argv[2]), ensure_ascii=True, sort_keys=True))
