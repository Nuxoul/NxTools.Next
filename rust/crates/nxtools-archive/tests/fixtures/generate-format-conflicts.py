"""Tiny independent inputs for original native format-conflict integration."""
import io
import json
import pathlib
import subprocess
import sys
import tarfile

root = pathlib.Path(sys.argv[1])
sevenzip = sys.argv[2]
root.mkdir(parents=True, exist_ok=True)
for name in ["a.txt", "b.txt", "c.txt"]:
    (root / name).write_bytes(("incoming-" + name).encode())
command = [sevenzip, "a", "-t7z", "-m0=LZMA2", "-ms=on", "-mhc=off", "solid.7z", "a.txt", "b.txt", "c.txt"]
subprocess.run(command, cwd=root, capture_output=True, check=True)
listing = subprocess.run([sevenzip, "l", "-slt", "solid.7z"], cwd=root, capture_output=True, text=True, check=True).stdout
assert "Solid = +" in listing and "Blocks = 1" in listing
for filename, mode, names in [("plain.tar", "w", ["a.txt", "b.txt"]), ("plain.tar.gz", "w:gz", ["a.txt", "b.txt"]),
                              ("single.tar", "w", ["a.txt"]), ("collision.tar", "w", ["a.txt", "A.TXT"]), ("unsafe.tar", "w", ["../escape"]),
                              ("prefix.tar", "w", ["a.txt", "a.txt/child"])]:
    with tarfile.open(root / filename, mode, format=tarfile.USTAR_FORMAT) as archive:
        for name in names:
            payload = ("incoming-" + name).encode()
            header = tarfile.TarInfo(name)
            header.size = len(payload)
            archive.addfile(header, io.BytesIO(payload))
(root / "solid-list.txt").write_text(listing, encoding="utf-8")
print(json.dumps({"producer": listing.strip().splitlines()[0], "solid": True, "blocks": 1, "entries": 3}))
