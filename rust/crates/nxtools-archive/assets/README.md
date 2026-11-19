# Windows x64 ZIP-SFX stub

`zip-sfx-x64.bin` is the project's bare recipient PE, built from `../src/sfx_main.rs` and private `../src/sfx/*.rs`. It is not a ZIP member or a third-party stub. The writer appends the 48-byte descriptor followed by the coordinate-correct ordinary/split ZIP data; it must not modify or trim the PE.

## Bundled build facts

- Built 2026-11-12 from the checked-in standalone source with the workspace locked offline dependency graph.
- Rust 1.96.0 (`ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96`, 2026-05-25), host `x86_64-pc-windows-msvc`, LLVM 22.1.2.
- Exact command: `cargo rustc --locked --offline -p nxtools-archive --bin nxtools-zip-sfx --release -- -C target-feature=+crt-static`.
- Environment: `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; TEMP/TMP point to the build's scratch directory. Available commit before Cargo was 25.19 GiB, above the 8 GiB gate.
- Bare file size and maximum raw PE end: **835,584 bytes**. Machine **0x8664**, PE32+, six sections, zero overlay bytes, certificate directory `(0, 0)`.
- Imports: `kernel32.dll` / `KERNEL32.dll`, `api-ms-win-core-synch-l1-2-0.dll`, `ntdll.dll`. No delay imports. No dynamic MSVC/UCRT, managed runtime, NxTools/PlatformHost or external codec DLL dependency.
- Asset bytes are a direct unchanged copy of the verified `target/release/nxtools-zip-sfx.exe`. No truncation, signing or post-link patch was applied.
- Recipient notices include project MIT/Apache, flate2 1.1.9, crc32fast 1.5.0, cfg-if 1.0.4, zlib-rs 0.6.6, tempfile 3.27.0 and its locked Windows dependencies, plus the installed Rust 1.96.0 standard-library distribution notices. See `../notices/zip-sfx/README.md`.

Rebuild with `../scripts/build-zip-sfx.ps1` using a fresh evidence directory; instructions are in `../src/sfx/README.md`. The helper retains exact source input copies, toolchain/environment/command, build exit/log and static PE/import/copy receipts. It copies only the exact owned output and refuses to replace a differing existing asset.

This asset has not been executed for native recipient acceptance. Unit coverage uses non-executable owned coordinate fixtures. Windows-loader physical executable size limits and real recipient commands remain separate verification boundaries.
