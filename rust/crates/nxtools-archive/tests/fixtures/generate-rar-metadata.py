"""Generate small, deterministic RAR metadata fixtures using only the Python stdlib.

The caller must provide an existing, caller-owned output directory. By default no
existing output is replaced; pass --force to replace only the generator's known
regular-file outputs.
"""

from __future__ import annotations

import argparse
import binascii
import datetime as dt
import hashlib
import json
import pathlib
import struct
from typing import Any

RAR4_SIGNATURE = b"Rar!\x1a\x07\x00"
RAR5_SIGNATURE = b"Rar!\x1a\x07\x01\x00"
PAYLOAD = b"synthetic rar metadata\n"
RAR4_LOCAL_TIME = (2020, 2, 3, 4, 5, 6)
RAR5_UNIX_SECONDS = 1_646_370_367
RAR5_NANOSECONDS = 123_456_789
RAR5_ENCODED_UTC = "2022-03-04T05:06:07.123456789Z"
RAR5_DLL_FILETIME_UTC = "2022-03-04T05:06:07.1234567Z"

WINDOWS_DIRECTORY_ATTRS = 0x13  # READONLY | HIDDEN | DIRECTORY
WINDOWS_FILE_ATTRS = 0x03  # READONLY | HIDDEN
UNIX_DIRECTORY_MODE = 0o040777
UNIX_FILE_MODE = 0o100777

OUTPUT_NAMES = (
    "synthetic-rar4-windows.rar",
    "synthetic-rar5-windows.rar",
    "synthetic-rar5-unix.rar",
)


def crc32(data: bytes) -> int:
    return binascii.crc32(data) & 0xFFFFFFFF


def encode_vint(value: int) -> bytes:
    if value < 0:
        raise ValueError("RAR variable integers cannot be negative")
    encoded = bytearray()
    while value >= 0x80:
        encoded.append((value & 0x7F) | 0x80)
        value >>= 7
    encoded.append(value)
    return bytes(encoded)


def encode_dos_datetime(parts: tuple[int, int, int, int, int, int]) -> int:
    year, month, day, hour, minute, second = parts
    if year < 1980 or year > 2107 or second % 2:
        raise ValueError("RAR4 fixture time must fit DOS date/time at two-second precision")
    return (
        ((year - 1980) << 25)
        | (month << 21)
        | (day << 16)
        | (hour << 11)
        | (minute << 5)
        | (second // 2)
    )


def rar4_header(header_type: int, flags: int, body: bytes = b"") -> bytes:
    header_size = 7 + len(body)
    protected = struct.pack("<BHH", header_type, flags, header_size) + body
    return struct.pack("<H", crc32(protected) & 0xFFFF) + protected


def rar4_file_record(
    name: str,
    data: bytes,
    *,
    directory: bool,
    host_os: int,
    attributes: int,
) -> bytes:
    name_bytes = name.encode("ascii")
    flags = 0x8000 | (0x00E0 if directory else 0)
    body = struct.pack(
        "<IIBIIBBHI",
        len(data),
        len(data),
        host_os,
        crc32(data),
        encode_dos_datetime(RAR4_LOCAL_TIME),
        20,
        0x30,
        len(name_bytes),
        attributes,
    ) + name_bytes
    return rar4_header(0x74, flags, body) + data


def build_rar4_windows() -> bytes:
    main = rar4_header(0x73, 0, struct.pack("<HI", 0, 0))
    directory = rar4_file_record(
        "metadata",
        b"",
        directory=True,
        host_os=2,
        attributes=WINDOWS_DIRECTORY_ATTRS,
    )
    file_record = rar4_file_record(
        r"metadata\payload.txt",
        PAYLOAD,
        directory=False,
        host_os=2,
        attributes=WINDOWS_FILE_ATTRS,
    )
    end = rar4_header(0x7B, 0)
    return RAR4_SIGNATURE + main + directory + file_record + end


def rar5_block(body: bytes) -> bytes:
    size = encode_vint(len(body))
    protected = size + body
    return struct.pack("<I", crc32(protected)) + protected


def rar5_high_time_extra() -> bytes:
    flags = 0x01 | 0x02 | 0x10  # Unix time, mtime present, nanoseconds present.
    field = encode_vint(3) + encode_vint(flags) + struct.pack(
        "<II", RAR5_UNIX_SECONDS, RAR5_NANOSECONDS
    )
    return encode_vint(len(field)) + field


def rar5_file_record(
    name: str,
    data: bytes,
    *,
    directory: bool,
    host_os: int,
    attributes: int,
) -> bytes:
    name_bytes = name.encode("utf-8")
    extra = rar5_high_time_extra()
    common_flags = 0x01 | (0x02 if data else 0)  # Extra area, and data area for a file.
    file_flags = 0x02 | 0x04 | (0x01 if directory else 0)  # Unix time, CRC32, directory.

    body = bytearray()
    body += encode_vint(2)  # HEAD_FILE
    body += encode_vint(common_flags)
    body += encode_vint(len(extra))
    if data:
        body += encode_vint(len(data))
    body += encode_vint(file_flags)
    body += encode_vint(len(data))
    body += encode_vint(attributes)
    body += struct.pack("<I", RAR5_UNIX_SECONDS)
    body += struct.pack("<I", crc32(data))
    body += encode_vint(0)  # RAR5 algorithm, stored method, non-solid.
    body += encode_vint(host_os)
    body += encode_vint(len(name_bytes))
    body += name_bytes
    body += extra
    return rar5_block(bytes(body)) + data


def build_rar5(*, windows_origin: bool) -> bytes:
    main = rar5_block(encode_vint(1) + encode_vint(0) + encode_vint(0))
    if windows_origin:
        directory_name = "metadata"
        file_name = "metadata/payload.txt"  # RAR5 uses '/' even for Windows-origin entries.
        host_os = 0
        directory_attributes = WINDOWS_DIRECTORY_ATTRS
        file_attributes = WINDOWS_FILE_ATTRS
    else:
        directory_name = "metadata"
        file_name = "metadata/payload.txt"
        host_os = 1
        directory_attributes = UNIX_DIRECTORY_MODE
        file_attributes = UNIX_FILE_MODE

    directory = rar5_file_record(
        directory_name,
        b"",
        directory=True,
        host_os=host_os,
        attributes=directory_attributes,
    )
    file_record = rar5_file_record(
        file_name,
        PAYLOAD,
        directory=False,
        host_os=host_os,
        attributes=file_attributes,
    )
    end = rar5_block(encode_vint(5) + encode_vint(0) + encode_vint(0))
    return RAR5_SIGNATURE + main + directory + file_record + end


def expected_manifest(generated: dict[str, bytes]) -> dict[str, Any]:
    payload_crc = f"{crc32(PAYLOAD):08x}"
    payload_sha256 = hashlib.sha256(PAYLOAD).hexdigest()
    rar5_time = {
        "archive_encoding": "Unix seconds plus nanoseconds (UTC)",
        "encoded_utc": RAR5_ENCODED_UTC,
        "encoded_precision": "1 nanosecond",
        "unrar_dll_utc_filetime": RAR5_DLL_FILETIME_UTC,
        "unrar_dll_precision": "100 nanoseconds",
        "precision_note": (
            "RARReadHeaderEx returns normalized FILETIME only; it does not expose "
            "the original RAR5 time encoding or declared precision."
        ),
    }
    manifest: dict[str, Any] = {
        "payload": {
            "utf8": PAYLOAD.decode("utf-8"),
            "size": len(PAYLOAD),
            "crc32": payload_crc,
            "sha256": payload_sha256,
        },
        "archives": [],
    }

    manifest["archives"].append(
        {
            "file": OUTPUT_NAMES[0],
            "format": "RAR4",
            "method": "stored",
            "sha256": hashlib.sha256(generated[OUTPUT_NAMES[0]]).hexdigest(),
            "time": {
                "archive_encoding": "DOS local wall clock",
                "local": "2020-02-03T04:05:06",
                "precision": "2 seconds",
                "unrar_dll_note": (
                    "UnRAR converts this local calendar to normalized UTC FILETIME "
                    "using the reader's Windows time-zone rules."
                ),
            },
            "members": [
                {
                    "name": "metadata",
                    "kind": "directory",
                    "host_os_raw": 2,
                    "unrar_host_os": 2,
                    "attributes_raw": "0x00000013",
                    "readonly": True,
                    "hidden": True,
                },
                {
                    "name": r"metadata\payload.txt",
                    "kind": "file",
                    "host_os_raw": 2,
                    "unrar_host_os": 2,
                    "attributes_raw": "0x00000003",
                    "readonly": True,
                    "hidden": True,
                    "content": "payload",
                },
            ],
        }
    )
    manifest["archives"].append(
        {
            "file": OUTPUT_NAMES[1],
            "format": "RAR5",
            "method": "stored",
            "sha256": hashlib.sha256(generated[OUTPUT_NAMES[1]]).hexdigest(),
            "time": rar5_time,
            "members": [
                {
                    "name": "metadata",
                    "kind": "directory",
                    "host_os_raw": 0,
                    "unrar_host_os": 2,
                    "attributes_raw": "0x00000013",
                    "readonly": True,
                    "hidden": True,
                },
                {
                    "name": "metadata/payload.txt",
                    "kind": "file",
                    "host_os_raw": 0,
                    "unrar_host_os": 2,
                    "attributes_raw": "0x00000003",
                    "readonly": True,
                    "hidden": True,
                    "content": "payload",
                },
            ],
        }
    )
    manifest["archives"].append(
        {
            "file": OUTPUT_NAMES[2],
            "format": "RAR5",
            "method": "stored",
            "sha256": hashlib.sha256(generated[OUTPUT_NAMES[2]]).hexdigest(),
            "time": rar5_time,
            "members": [
                {
                    "name": "metadata",
                    "kind": "directory",
                    "host_os_raw": 1,
                    "unrar_host_os": 3,
                    "attributes_raw": "0o040777",
                    "posix_mode": "0o040777",
                    "windows_readonly_hidden": False,
                },
                {
                    "name": "metadata/payload.txt",
                    "kind": "file",
                    "host_os_raw": 1,
                    "unrar_host_os": 3,
                    "attributes_raw": "0o100777",
                    "posix_mode": "0o100777",
                    "windows_readonly_hidden": False,
                    "content": "payload",
                },
            ],
            "attribute_note": (
                "The raw Unix modes have low bits 0x1 and 0x2 set; those bits "
                "must not be interpreted as Windows readonly and hidden attributes."
            ),
        }
    )
    return manifest


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "output_directory",
        type=pathlib.Path,
        help="existing caller-owned directory that will receive the three fixtures",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="replace existing regular files with the generator's known output names",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root: pathlib.Path = args.output_directory
    if not root.exists() or not root.is_dir():
        raise SystemExit(f"output directory must already exist and be a directory: {root}")
    if root.is_symlink():
        raise SystemExit(f"refusing a symlink output directory: {root}")

    generated = {
        OUTPUT_NAMES[0]: build_rar4_windows(),
        OUTPUT_NAMES[1]: build_rar5(windows_origin=True),
        OUTPUT_NAMES[2]: build_rar5(windows_origin=False),
    }
    paths = {name: root / name for name in OUTPUT_NAMES}
    for path in paths.values():
        if path.exists():
            if not args.force:
                raise SystemExit(f"refusing to overwrite existing output: {path}")
            if path.is_symlink() or not path.is_file():
                raise SystemExit(f"--force can replace only a regular file: {path}")

    mode = "wb" if args.force else "xb"
    for name, data in generated.items():
        with paths[name].open(mode) as output:
            output.write(data)

    result = expected_manifest(generated)
    result["output_directory"] = str(root.resolve())
    result["fixture_count"] = len(generated)
    result["generated_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
