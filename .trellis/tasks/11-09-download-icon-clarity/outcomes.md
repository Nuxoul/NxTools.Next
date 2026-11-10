# Download icons, reference features and archive capabilities

Date:2026-11-09.

## Icon delivery

`ShellIconExtractor` obtains high-resolution system image-list artwork for ordinary Shell requests above32px and preserves native pixels through the existing PNG/cache result. Its image-list cache discriminator prevents reuse of legacy small/large rasters. Small requests, explicit resource-index selectors, reserved type parsing, bounded STA, cancellation, HICON/COM release and existing128-memory/2048-disk cache limits remain. The download list retains its40DIP transparent `img(...).object_fit(Contain)` slot and all original task/action/state owners. No Ghost source/assets or new dependency were copied.

C# build passes with0warnings/0errors. Exact LIST13/RUN13/PASS13/SKIP0/FAIL0/NOT_RUN0 covers11icon/STA/cache fixtures and2pure association-plan/whole-group fixtures. The native PNG fixture queries a folder type, ZIP type and a GUID-owned completed ZIP at48/96px; all six return256×256pixels, reuse their cache entry, avoid the seeded legacy key and clean their owned root. These are actual native image/cache results, not installed or mixed-DPI visual acceptance.

## Ghost Downloader comparison

[Reference](../../../docs/reference-code/ghost-downloader.md): its task cards use48px QFileIconProvider system icons; Fluent operation/status glyphs are separate. Its torrent/m3u8 association icon assets do not supply the list. Source inspection only; the GPLv3 project was not launched, built or copied.

Priorities worth evaluating:

1. Batch URL templates such as `[001-100]`, stepped ranges and alternatives, with a bounded preview/expansion before submission.
2. HTTP range-coverage progress in the list, rendering existing segment facts instead of adding a second progress sampler or download engine. Unknown-size and non-Range tasks keep ordinary progress.
3. A paused-task relink workflow for expired signed URLs, retaining current resource-identity/resume validation rather than accepting equal length alone.

Queue promotion/concurrency, segmented resume and serialized collision decisions already have NxTools owners; compare their presentation and behavior rather than reimplementing them. No unrelated feature was added.

## Current archive format boundaries

| Operation | Implemented families |
| --- | --- |
| Browse/Test/extract | ZIP;7z;RAR4/RAR5;TAR;TAR+Gzip/Bzip2/XZ/Zstd;standalone Gzip/Bzip2/XZ/Zstd/LZMA/LZ4 frame or size-prefixed block/Brotli;CAB;ISO9660/Joliet/Rock Ridge;UDF;WIM;AR;CPIO |
| Create | ZIP,7z,TAR,TGZ (`tar.gz`),TXZ (`tar.xz`),GZ,XZ. GZ/XZ are single-file streams, not directory containers. |
| Modify | ZIP; qualified ordinary7z; qualified TAR/TGZ/TXZ. RAR, additional readers and TAR+Bzip2/Zstd are not general write/edit formats. |

Sources: `rust/crates/nxtools-archive/src/model.rs`, `worker_formats.rs`, `tar_edit_outer.rs`; `rust/crates/nxtools-core/src/archive.rs::ArchiveCreateFormat`; protocol `ArchiveJobFormat`. Existing ZIP/7z/RAR encrypted reading/password interaction and split-source paths retain their supported-method/source-lease limits. Full volume sets are required. Split WIM is refused. Supplied WIM native evidence covers an uncompressed sample, not every WIM compression variant. Unsafe Windows names and link/special entries are not silently rewritten/materialized. Format support is separate from extension association/menu registration.

## Current-machine context-menu diagnosis

Read-only observation:

- Desired configuration and applied ledger both have `desired_revision=4`.
- Saved commands are `open,extract_smart,extract_here,extract_name,test`; `create` is absent.
- Applied ledger contains182values and the same five command COM identities.
- Merged HKCR has `SystemFileAssociations\.zip\shell\NxTools.ArchiveSelection`, but no NxTools menu beneath `*`, `Directory` or `AllFilesystemObjects`.

The saved old command set therefore does not request ordinary-file/folder registration. This is not merely a Windows11 menu-folding issue. Existing `ArchiveSelectionHandlerPlan`35–56 emits `*`/`Directory` roots only when Create is selected. The existing helper forwards one complete ordered file/folder/mixed selection through `--archive-compress`; the two selected pure fixtures pass.

Supported repair through the existing UI:

1. Open 压缩包设置 → 集成.
2. Revert the already-applied integration with 撤销.
3. Keep the classic-menu option enabled and select 创建压缩包…（普通文件或文件夹）. The existing section-owned save persists that choice.
4. Click 应用 and wait for the observed applied state. Windows11 exposes this classic integration under 显示更多选项.

Existing custom choices are not automatically overwritten. No real configuration/registry Apply/Revert, COM activation, Explorer invocation, package/install, deployed-file replacement, service action or commit/push was performed. Current-machine menu activation and installed icon appearance remain operational follow-up, not completed native/UI acceptance.

## Source/document verification

`spec-contracts.py` passes126documents/14owners; `gpui-release-contracts.py` confirms the locked0.8.0/pre0.3.9 graph and two narrow vendor deltas; `markdown-editor-contracts.py` passes the pinned57-file contract. Scoped `git diff --check` passes. No Rust product code changed and no Cargo, Release packaging or installed runtime verification was run for this slice.
