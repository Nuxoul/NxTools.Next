"""Independent Windows bsdtar/libzstd fixtures, no NxTools writer or Python package install."""
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).parent
bsdtar = Path(os.environ["WINDIR"]) / "System32" / "tar.exe"
print(subprocess.check_output([str(bsdtar), "--version"], text=True).strip())
MASK = (1 << 64) - 1
P1, P2, P3, P4, P5 = 11400714785074694791, 14029467366897019727, 1609587929392839161, 9650029242287828579, 2870177450012600261
def rotate(value, bits):
    return ((value << bits) | (value >> (64 - bits))) & MASK
def step(accumulator, value):
    return (rotate((accumulator + value * P2) & MASK, 31) * P1) & MASK
def xxh64(data):
    position = 0
    if len(data) >= 32:
        values = [(P1 + P2) & MASK, P2, 0, (-P1) & MASK]
        while position <= len(data) - 32:
            values = [step(value, int.from_bytes(data[position + i * 8:position + i * 8 + 8], "little")) for i, value in enumerate(values)]
            position += 32
        result = sum(rotate(value, bits) for value, bits in zip(values, (1, 7, 12, 18))) & MASK
        for value in values:
            result = ((result ^ step(0, value)) * P1 + P4) & MASK
    else:
        result = P5
    result = (result + len(data)) & MASK
    while position + 8 <= len(data):
        result = (rotate(result ^ step(0, int.from_bytes(data[position:position + 8], "little")), 27) * P1 + P4) & MASK
        position += 8
    if position + 4 <= len(data):
        result = (rotate(result ^ (int.from_bytes(data[position:position + 4], "little") * P1 & MASK), 23) * P2 + P3) & MASK
        position += 4
    for byte in data[position:]:
        result = (rotate(result ^ (byte * P5 & MASK), 11) * P1) & MASK
    result ^= result >> 33
    result = result * P2 & MASK
    result ^= result >> 29
    result = result * P3 & MASK
    return result ^ (result >> 32)
assert xxh64(b"") == 0xef46db3751d8e999
assert xxh64(b"a") == 0xd24ec4f1a98c6e5b
def checked(frame, data):
    assert frame[:4] == bytes.fromhex("28b52ffd") and not frame[4] & 4
    result = bytearray(frame)
    result[4] |= 4
    return bytes(result) + (xxh64(data) & 0xffffffff).to_bytes(4, "little")
payload = "独立 gzip fixture\n".encode("utf-8") + bytes(range(256))
with tempfile.TemporaryDirectory(prefix="nxtools-zstd-oracle-") as temporary:
    owned = Path(temporary)
    def compress(data, name):
        source = owned / "source.bin"
        source.write_bytes(data)
        target = owned / name
        subprocess.run([str(bsdtar), "--zstd", "--format=raw", "-cf", str(target), "-C", str(owned), source.name], check=True)
        return target.read_bytes()
    (root / "independent.zst").write_bytes(compress(payload, "single.zst"))
    (root / "multi.zst").write_bytes(compress(b"first\n", "first.zst") + compress(b"second\n", "second.zst"))
    (root / "independent.tar.zst").write_bytes(compress((root / "independent.tar").read_bytes(), "compound.zst"))
    (root / "checked.zst").write_bytes(checked(compress(payload, "checked.zst"), payload))
    tar_bytes = (root / "independent.tar").read_bytes()
    (root / "checked.tar.zst").write_bytes(checked(compress(tar_bytes, "checked-tar.zst"), tar_bytes))
    # The independent libarchive/libzstd reader validates the Python-added XXH64 too.
    subprocess.run([str(bsdtar), "-tf", str(root / "checked.tar.zst")], check=True, stdout=subprocess.PIPE)
