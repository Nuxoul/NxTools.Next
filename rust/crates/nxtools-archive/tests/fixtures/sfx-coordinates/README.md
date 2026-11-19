# ZIP-SFX physical-coordinate fixtures

Project-owned, data-only format fixtures. The 512-byte PE-shaped prefix has no executable section or entry point. **Never execute the `.exe` files.**

`manifest.json` and the 42 physical files plus four `expected/*.bin` files are unchanged byte copies of the independently qualified REF-20 coordinate oracle. The oracle uses CPython `struct`, raw `zlib`, CRC32 and literal/deterministic source data; stock `zipfile` additionally verifies the ordinary forms. No product writer or recipient stub produced these inputs.

Six profiles cover ordinary/split Store, ordinary/split Deflate and ordinary/split forced ZIP64. There are 24 catalog entries and 49,380 decoded bytes across the profiles. The names are `资料/`, `资料/data.bin`, `empty.txt` and `hello.txt`. A payload crosses up to eleven physical boundaries. Split names are `.exe`, `.z02` onward, and final `.zip`; all coordinates include physical prefix bytes on their disk.

The descriptor is 48 bytes at raw PE offset 512; its 16-byte magic is `NXTOOLS-ZIP-SFX1`, version 1, prefix length 560. Forced ZIP64 is coordinate coverage, not a physical greater-than-4-GiB executable test.

Qualification provenance: `b7-ref20-coordinate-prerequisite.py`, successful qualification-r1 and `b7-ref20-coordinate-oracle/result.json` in the owning task's retained evidence. Stub unit tests copy these files to owned temporary directories before mutation and remove only their own copies. Source fixtures are immutable.
