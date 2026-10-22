"""Tiny independent raw-name ZIPs. No product encoder; outputs only to explicit directory."""
import pathlib
import struct
import sys
import zlib

PAYLOAD = b"independent encoding payload\n"


def write_zip(path, entries):
    local, central = bytearray(), bytearray()
    for raw, flags, extra in entries:
        offset = len(local)
        crc = zlib.crc32(PAYLOAD)
        local += struct.pack("<4s5H3I2H", b"PK\x03\x04", 20, flags, 0, 0, 0, crc, len(PAYLOAD), len(PAYLOAD), len(raw), len(extra)) + raw + extra + PAYLOAD
        central += struct.pack("<4s6H3I5H2I", b"PK\x01\x02", 20, 20, flags, 0, 0, 0, crc, len(PAYLOAD), len(PAYLOAD), len(raw), len(extra), 0, 0, 0, 0, offset) + raw + extra
    path.write_bytes(local + central + struct.pack("<4s4H2IH", b"PK\x05\x06", 0, 0, len(entries), len(entries), len(central), len(local), 0))


def generate(root):
    root.mkdir(parents=True, exist_ok=True)
    for label, text, codec in [
        ("auto-gbk", "中文文件资料.txt", "gbk"),
        ("auto-sjis", "日本語のファイルｱ.txt", "shift_jis"),
        ("ambiguous-long", "日本語のファイル.txt", "shift_jis"),
        ("auto-utf8", "文件名称测试.txt", "utf-8"),
        ("ambiguous", "中文.txt", "gbk"),
        ("big5", "繁體中文.txt", "big5"),
        ("cp949", "한글파일.txt", "cp949"),
        ("cp1250", "Příliš.txt", "cp1250"),
        ("cp1251", "Привет.txt", "cp1251"),
        ("cp1252", "café.txt", "cp1252"),
    ]:
        write_zip(root / (label + ".zip"), [(text.encode(codec), 0, b"")])
    raw = "旧名.txt".encode("gbk")
    value = b"\x01" + struct.pack("<I", zlib.crc32(raw)) + "权威名称.txt".encode()
    write_zip(root / "authority.zip", [("权威UTF8.txt".encode(), 0x800, b""), (raw, 0, struct.pack("<HH", 0x7075, len(value)) + value)])
    # CP437 safe -> CP1252 superscript reserved device. CP437 box drawing differs
    # from CP1252 micro sign, whose Unicode uppercase collides with Greek MU.
    for label, names in [
        ("switch-unsafe", [b"COM\xb9.txt"]),
        ("switch-case", [b"\xb5.txt", "Μ.txt".encode()]),
        ("switch-prefix", [b"\xb5", "Μ/child.txt".encode()]),
    ]:
        write_zip(root / (label + ".zip"), [(raw, 0x800 if index else 0, b"") for index, raw in enumerate(names)])


if __name__ == "__main__":
    generate(pathlib.Path(sys.argv[1]))
