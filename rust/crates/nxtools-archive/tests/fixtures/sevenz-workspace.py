"""Produce tiny independent 7z workspace fixtures with CPython/liblzma.

Usage: python sevenz-workspace.py OUTPUT_DIRECTORY
Payload is encoded with a 4 KiB dictionary, then declared as 192 MiB: a larger
LZMA2 dictionary accepts the same stream. Headers use their own 4 KiB dictionary.
No NxTools or sevenz-rust2 writer is used. The two negative variants change only
the payload coder declaration, inside an otherwise valid encoded header.
"""
import lzma
import pathlib
import struct
import sys
import zlib

MEMBERS = (("a.txt", b"alpha\n"), ("b.txt", b"beta\n"))


def uint(value):
    for count in range(8):
        if value < (1 << (7 * (count + 1))):
            return bytes([((0xFF << (8 - count)) & 0xFF) | (value >> (8 * count))]) + value.to_bytes(8, "little")[:count]
    return b"\xff" + struct.pack("<Q", value)


def crc(data):
    return struct.pack("<I", zlib.crc32(data))


def compress(data):
    return lzma.compress(data, format=lzma.FORMAT_RAW,
                         filters=[{"id": lzma.FILTER_LZMA2, "dict_size": 4096}])


def streams(position, packed, unpacked, method, prop, members=None):
    data = b"\x06" + uint(position) + b"\x01\x09" + uint(packed) + b"\x00"
    data += b"\x07\x0b\x01\x00\x01\x21" + bytes([method, 1, prop])
    data += b"\x0c" + uint(unpacked) + b"\x00"
    if members:
        data += b"\x08\x0d" + uint(len(members)) + b"\x09"
        data += b"".join(uint(len(payload)) for _, payload in members[:-1])
        data += b"\x0a\x01" + b"".join(crc(payload) for _, payload in members) + b"\x00"
    return data + b"\x00"


def archive(method=0x21, prop=31):
    payload = b"".join(data for _, data in MEMBERS)
    packed = compress(payload)
    names = b"\x00" + b"".join((name + "\0").encode("utf-16le") for name, _ in MEMBERS)
    header = b"\x01\x04" + streams(0, len(packed), len(payload), method, prop, MEMBERS)
    header += b"\x05" + uint(len(MEMBERS)) + b"\x11" + uint(len(names)) + names + b"\x00\x00"
    encoded = compress(header)
    descriptor = b"\x17" + streams(len(packed), len(encoded), len(header), 0x21, 0)
    start = struct.pack("<QQ", len(packed) + len(encoded), len(descriptor)) + crc(descriptor)
    return b"7z\xbc\xaf\x27\x1c\x00\x04" + crc(start) + start + packed + encoded + descriptor


if __name__ == "__main__":
    destination = pathlib.Path(sys.argv[1])
    destination.mkdir(parents=True, exist_ok=True)
    for name, method, prop in (("sevenz-workspace.7z", 0x21, 31),
                               ("sevenz-invalid-properties.7z", 0x21, 41),
                               ("sevenz-unsupported-coder.7z", 0x7F, 31)):
        data = archive(method, prop)
        (destination / name).write_bytes(data)
        print(f"{name}: {len(data)} bytes")
