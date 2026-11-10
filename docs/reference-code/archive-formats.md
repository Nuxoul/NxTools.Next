# Additional archive formats: verified reader boundaries

This approved compatibility expansion has no legacy NxTools general archive browser. The Bandizip family dispatch and WIM native-handler evidence remain in [archive.md](archive.md) and the linked format-family/WIM research. The sources below are the published crate source for the selected implementation, not claims of runtime acceptance.

| Reader | Source and entry chain | Constraint |
| --- | --- | --- |
| CAB MSZIP/LZX | `caddy-archive-core` 0.3.4 `src/lib.rs::Archive::open_as(..., Format::Cab, Limits)` → `src/cab_backend.rs`; `entries` → `extract(EntryId, Write)` | Explicit CAB signature and one held native source; no shell decoder or replacement of existing ZIP/TAR/7z/RAR. |
| UDF | `caddy-archive-core` 0.3.4 `src/udf.rs::UdfArchive::open_reader` → `entries`/`extract`; calls `libmkiso::udf::UdfReader` | Choose UDF after explicit image discovery, not by extension alone. Symbolic links remain non-materializing. |
| WIM LZMS | `caddy-archive-core` 0.3.4 `src/wim.rs::images_reader` → `FileWimArchive::open_reader(reader, image_index, limits)` → `entries`/`extract`; payload codec through `wim-format` 0.1.1 | One-based image/global-member mapping; multiple images use numeric path roots, single-image names stay unchanged. Split WIM is refused. `wim-format` is LGPL; distribution must satisfy its license notices and source/relinking obligations. No vendored DLL. |
| ISO9660 / Joliet / Rock Ridge | `libmkiso` 0.1.5 `src/iso9660.rs::IsoReader::open_with_options(ReadOptions { namespace: Namespace::PreferRockRidge, limits })` → `entries`/`extract` | Prefer Rock Ridge, then Joliet, then primary. The generic core ISO default is primary-only and loses names. Preserve reader-identified links as non-materializing. |
| AR | `ar` 0.9.0 `src/lib.rs::Archive::new` → `next_entry` → `header().identifier()/size()`, `Entry: Read` | Streaming archive permits replay of held source for selected entry, no filesystem output from reader. |
| CPIO | `kpea` 0.4.2 `src/archive.rs::Archive::new` → `read_entry` → `entry.path`, `entry.metadata`, `entry.reader`; accepts newc/crc/odc/binary | Real bsdtar sample uses odc `070707`; do not substitute a newc-only parser. Never call its filesystem `unpack` path. |
| Standalone streams | existing `lzma-rust2` 0.21 `src/lzma_reader.rs::LzmaReader::new_mem_limit`, `brotli` 8.0.2 `Decompressor::new`, `lz4_flex` 0.12.2 frame decoder and block size-prepended decoder | The supported Python `lz4.block` sample carries a four-byte LE decoded-size prefix; raw anonymous LZ4 blocks have no auto-detection claim. Encoded+decoded block buffers are working-memory charged before allocation. LZMA/Brotli/LZ4 frame output streams through the original capability. |

All readers remain inside `isolated-codecs` child, with original native source acquisition, BrowserContext/source binding, operation/correlation, bounded metadata and output, typed first failure, native partial/commit/abort and original terminal/cleanup. List does not imply test/payload verified. Cancellable output is fixed-size and publication requires a complete decoded/verified member; special and link entries are never published as regular files. Source revision and native source identity are checked before extraction. No payload is logged.

## Supplied-corpus boundaries

The file named `wim-lzms.wim` has header flags0x00000080, chunk_size0 and one image: it is an
uncompressed WIM, not runtime LZMS evidence. The compressed-handler capability above is source-
verified; the supplied native payload comparison does not certify every compression variant.
The Windows bsdtar odc CPIO includes ANSI names and irreversible `?` substitutions. Preserve its
raw names and explicit encoding choice; Test/safe-member preview remain available, while whole
Windows extraction returns UnsafePath rather than silently rewriting those names.
