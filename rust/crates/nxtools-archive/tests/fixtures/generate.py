"""Independent, deterministic Python stdlib fixtures; never reads user archives."""
import pathlib
import zipfile
import warnings

# Deliberate duplicate records are fixtures, not accidental generation warnings.
warnings.filterwarnings("ignore", message="Duplicate name:.*", category=UserWarning)

def fixed_info(path):
    info = zipfile.ZipInfo(path, (2024, 2, 3, 4, 5, 6))
    info.create_system = 3
    info.external_attr = (0o40755 if path.endswith("/") else 0o100644) << 16
    return info

# Requested large metadata fixtures only write to the caller-owned temporary path.
import sys
if len(sys.argv) == 3:
    with zipfile.ZipFile(pathlib.Path(sys.argv[1]), "w", allowZip64=True) as archive:
        for index in range(int(sys.argv[2])):
            archive.writestr(fixed_info(f"f{index:06}"), b"")
    raise SystemExit(0)

root = pathlib.Path(__file__).parent
for compression, name in [(zipfile.ZIP_STORED, "stored.zip"), (zipfile.ZIP_DEFLATED, "deflated.zip")]:
    with zipfile.ZipFile(root / name, "w", compression=compression) as archive:
        archive.comment = b"independent-python-fixture"
        for path, payload in [("资料/", b""), ("资料/说明.txt", "你好，压缩包\n".encode()), ("empty", b""), ("bytes.bin", bytes(range(256)))]:
            info = fixed_info(path)
            info.compress_type = compression
            info.external_attr = (0o40755 if path.endswith("/") else 0o100644) << 16
            archive.writestr(info, payload)

unsafe_names = ["../escape", "..\\escape", "/root", "C:relative", "C:/drive", "\\\\server\\share", "data:stream", "CON.txt", "NUL", "COM¹.log", "trailing.", "trailing ", "a//b", "a/./b", "bad?.txt", "control\x01.txt"]
with zipfile.ZipFile(root / "unsafe.zip", "w") as archive:
    for path in unsafe_names:
        archive.writestr(fixed_info(path), b"unsafe")

import warnings
with warnings.catch_warnings():
    warnings.simplefilter("ignore", UserWarning)
    with zipfile.ZipFile(root / "collisions.zip", "w") as archive:
        for path in ["same", "same", "CASE", "case", "parent", "parent/child", "Folder/one", "folder/two"]:
            archive.writestr(fixed_info(path), b"collision")

# Change a stored payload without changing either header's expected CRC.
damaged = bytearray((root / "stored.zip").read_bytes())
payload = damaged.index(bytes(range(32)))
damaged[payload] ^= 1
(root / "bad-crc.zip").write_bytes(damaged)

# Minimal independent ZIP records preserve non-UTF8 bytes exactly, unlike ZipInfo.
import struct
import zlib
def raw_zip(name, raw_name, extra=b"", flags=0):
    data = b"independent-encoding"
    checksum = zlib.crc32(data)
    local = struct.pack("<4s5H3I2H", b"PK\x03\x04", 20, flags, 0, 0, 0, checksum, len(data), len(data), len(raw_name), len(extra)) + raw_name + extra + data
    central = struct.pack("<4s6H3I5H2I", b"PK\x01\x02", 20, 20, flags, 0, 0, 0, checksum, len(data), len(data), len(raw_name), len(extra), 0, 0, 0, 0, 0) + raw_name + extra
    end = struct.pack("<4s4H2IH", b"PK\x05\x06", 0, 0, 1, 1, len(central), len(local), 0)
    (root / name).write_bytes(local + central + end)

for codec, text in [("gbk", "中文.txt"), ("shift_jis", "日本語.txt"), ("cp437", "café.txt")]:
    raw_zip(codec + ".zip", text.encode(codec))
raw = "旧名.txt".encode("gbk")
unicode_value = b"\x01" + struct.pack("<I", zlib.crc32(raw)) + "权威名称.txt".encode()
raw_zip("unicode-extra.zip", raw, struct.pack("<HH", 0x7075, len(unicode_value)) + unicode_value)
raw_zip("unicode-invalid-crc.zip", raw, struct.pack("<HH", 0x7075, len(unicode_value)) + b"\x01\0\0\0\0" + unicode_value[5:])

local_mismatch = bytearray((root / "stored.zip").read_bytes())
local_mismatch[30] = ord("x")
(root / "local-mismatch.zip").write_bytes(local_mismatch)

payload = b"a" * 4096
codec = zlib.compressobj(wbits=-15)
packed = (codec.compress(payload) + codec.flush())[:-1]
raw_name = b"truncated-deflate"
checksum = zlib.crc32(payload)
local = struct.pack("<4s5H3I2H", b"PK\x03\x04", 20, 0, 8, 0, 0, checksum, len(packed), len(payload), len(raw_name), 0) + raw_name + packed
central = struct.pack("<4s6H3I5H2I", b"PK\x01\x02", 20, 20, 0, 8, 0, 0, checksum, len(packed), len(payload), len(raw_name), 0, 0, 0, 0, 0, 0) + raw_name
end = struct.pack("<4s4H2IH", b"PK\x05\x06", 0, 0, 1, 1, len(central), len(local), 0)
(root / "truncated-deflate.zip").write_bytes(local + central + end)

raw_zip("shift-jis-trailing-5c.zip", "表".encode("shift_jis"))
fake_end = struct.pack("<4s4H2IH", b"PK\x05\x06", 0, 0, 0, 0, 0, 0, 0)
with zipfile.ZipFile(root / "comment-eocd.zip", "w") as archive:
    archive.writestr(fixed_info("valid.txt"), b"comment-safe")
    archive.comment = b"arbitrary-comment" + fake_end

with zipfile.ZipFile(root / "empty.zip", "w"):
    pass
