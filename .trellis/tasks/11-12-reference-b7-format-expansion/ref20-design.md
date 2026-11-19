# REF-20 — ZIP-SFX creation design

Status: **accepted for the declared profile at the coordinator's recorded levels**. [Coordinator acceptance](coordinator-acceptance.md) records independent original-owner creation/native/readback/TestPlatform verification and audited recipient receipts. Parent stays **Phase E — Format expansion and ZIP editing**, open; physical/installed/Release layers remain separate.

## Scope and intent

Add explicit ordinary/split ZIP-SFX creation to the existing Create → immutable Job → worker → native publication chain. The first supported recipient profile is Windows x64, ZIP Store/Deflate, clear UTF-8 names, files/directories/empty files and ZIP64 coordinates. SFX passwords and other methods are rejected explicitly, not silently removed or converted. Ordinary ZIP/7z and all existing password behavior remain unchanged.

Public `ArchiveJobCreate` and private ZIP creation options carry an additive default-false self-extracting choice. Validation restricts it to ZIP, a nonempty `.exe` output name, no password/unsupported options and new-family/no-replace publication. Old requests omit the field. A specific `zip_sfx_v1` capability prevents an old worker from silently emitting a plain ZIP for SFX intent. All relevant immutable snapshot/serialization owners retain the same captured value.

The original Create form/draft presents the explicit choice using released official components and existing InputState/Form/menu owners. Exact API/variant/parent adaptations and production interaction coverage must be recorded before Desktop UI edits. No recipient GUI toolkit or second product controller is introduced.

### Create component and lifecycle selection

Keep the existing two-column `Form::vertical().with_size(Size::Small).columns(2)` format/level group. Add a ZIP-only full-width `Field::new().col_span(2).visible(...).description(...)` containing released `Checkbox::new("archive-self-extracting").label(...).with_size(Size::Small).checked(...).disabled(...).on_change(...)`. `Form` propagates field size/grid properties; `Field` owns label, control and description slots. The official Checkbox/Base owns pointer and keyboard activation; the parent must not add a competing click handler or repaint the control.

The choice is one-off draft state, default false, not a new archive format or persisted settings field. Preserve entered passwords and show an explicit validation error for incompatible SFX intent; never silently discard them. Switching away from ZIP clears only SFX intent through the existing format-change owner. Generate `.exe` names for SFX, including separately compressed inputs, and restore the selected format extension when switching back.

The existing `OperationForm` owns one scrolling body, fixed Medium32 primary/outline footer, titlebar close, initial focus, default Enter, Escape and retirement. Busy state disables the choice. Any programmatic name InputState update is deferred to the next application turn; the current checkbox/entity callback updates only local draft state. Keep the original InputState/Select entities, Root and window. Real pointer, first Enter/Space, repeated-key suppression, Escape/close and focus tests must exercise these original owners; TestWindow results remain distinct from native physical UI acceptance.

Released source reviewed: `vendor/gpui-component-080/src/form/form.rs::{child,columns,footer,with_size,render}`, `form/field.rs::{col_span,visible,description,ParentElement,render}` and `checkbox.rs::{checked,disabled,on_change,render}`. Parent adaptations are limited to the full-width field, original busy/validation state and deferred name synchronization. No component catalog, vendor API or theme override is required.

## Recipient source and executable

Use the project-owned separate binary target in the existing archive package with narrow ZIP parsing/coordinate/extraction source. Locked flate2/zlib-rs and crc32fast own payload primitives; existing tempfile3.27.0 provides safe Windows no-replace candidate publication. The835,584-byte static Windows x64 PE imports only system DLLs and needs no installed NxTools, PlatformHost, managed runtime or external unzip. Exact source/build inputs, toolchain and embedded project/dependency/Rust notices are retained. See [recipient native execution](ref20-native-execution.md).

The recipient supports a read-only `--test`, explicit `--output <absent-directory>` and `--licenses`. With no arguments it prompts for a destination with a deterministic sibling default. It never launches members, subprocesses, network, elevation, Registry or association operations. Extraction requires a new root, validates all catalog paths before effects, uses bounded streaming and checks complete packed consumption/decoded size/CRC. Handled failure removes only its own unverified file; completed files/directories remain factual output. Interrupted extraction cannot report success or imply set-wide rollback.

Recipient metadata is bounded at64MiB,65536entries and4096UTF-8 bytes per name, with1024disks and fixed64KiB streaming buffers plus the Deflate window. It accepts only the supported regular/directory kinds and clear Stored/Deflate profile; other encryption/method/type requests return an explicit error. Source bytes are never executed.

## Physical prefix and descriptor

The bare project PE must have no unaccounted overlay or certificate. Locate its raw-image end using the DOS/PE headers, SizeOfHeaders and maximum section raw end, then read exactly48descriptor bytes:

| Offset | Field |
| ---: | --- |
| 0 | 16ASCII bytes `NXTOOLS-ZIP-SFX1` |
| 16 | UInt32 version1 |
| 20 | UInt32 descriptor bytes48 |
| 24 | UInt32 total physical disks1…1024 |
| 28 | UInt32 flags0 |
| 32 | UInt64 physical prefix length: bare PE bytes+48 |
| 40 | UInt64 reserved0 |

The worker owns this descriptor and supplies the exact final disk count. The stub reads it at the PE boundary, not by scanning arbitrary ZIP members for a marker. It verifies that declared family completeness matches the final end records and exact expected sibling names. Prefix bytes participate in original finite output admission and all disk-local ZIP coordinates.

Ordinary form is one `a.exe`, with no split marker and one-disk local/central/end offsets including the prefix. Split form starts with `a.exe`, continues with `a.z02…` and ends with `a.zip`; disk0 contains the whole prefix followed by the original span marker. All nonfinal disks retain exact requested byte size where the original intact-record policy permits; the final disk may be smaller. The existing writer still refuses records larger than the volume size.

The selected835,584-byte PE plus48-byte descriptor and four-byte span marker requires at least835,636bytes per split volume, also respecting the existing65536-byte minimum. Actual bounded record admission remains authoritative. Small split sets retain at least first EXE and final ZIP. There is no SFX split-set editing grant.

The own Windows loader accepts a4,294,967,295-byte physical PE and refuses4,294,967,296/4,294,967,297bytes with Windows193 under reviewed read-only commands. Limit every first EXE, including its complete prefix/ZIP bytes, to4,294,967,295bytes; split volume requests use that same upper bound. Total split output remains under the original finite output/count budgets. ZIP64 local extras, central/end records and locator retain actual64-bit semantics; their independently verified small fixtures do not replace large-payload correctness evidence.

## Creation and publication

1. Capture the explicit intent in the existing immutable request; negotiate SFX capability and validate method/password/name/limits.
2. Prepare the original held output namespace and fresh-family preflight before effects. Include `.exe`, final `.zip` and numbered `.zNN` siblings in the same-family check.
3. Run the unchanged source plan/encoding/source-digest and full original ZIP readback. The original encoder token is unpublished.
4. Extend `zip_container`'s existing coordinate-aware layout with the known prefix. Rebuild local disk offsets and central/end/ZIP64 records, then patch the owned descriptor's final disk count.
5. Reuse original candidate staging, same-held-token readback and full set normalization/decode verification, including exact known prefix/descriptor. Ordinary SFX likewise verifies its final prefixed candidate, not just the intermediate ZIP.
6. Recheck sources/cancellation; reuse native no-replace publication and journal. All candidates verify before the first publish. A later failure preserves already published outputs and actual cleanup/unknown facts; no family-atomicity claim.

## Diagnostics

Retain the original correlation ID across Create/Job/IPC/worker/native owners. Accepted/start, creation_planning/encoding/verifying/committing, volumes_verified and terminal events carry only fixed form/outcome/count/elapsed/native-code facts. Do not log paths, names, passwords, member contents or byte ticks. Stub console status likewise uses fixed stages/counts/errors; no persistent diagnostic sink or member execution.

## Verification sequence

- Six independently qualified data-only coordinate fixtures precede reader/layout tests. Nonzero LIST → same-filter RUN; compiler/fixture/lane failures are prerequisites.
- Stub reader tests cover ordinary/split Store/Deflate, UTF-8/empty/directory/ZIP64, missing/reordered/truncated/damaged volumes, descriptors/CRC/exact consumption, path refusal, limits and actual file closure. The same tests must be red-capable before implementation.
- Build the own static stub, inspect PE architecture/imports/notices, review exact read-only and fixed-output commands, then execute only owned samples. Independently compare every output byte. Do not execute archive members.
- Product Create/Job tests cover ordinary/split output, small forced boundaries, immutable intent, final-family conflicts, cancellation/source change/native write and later-publication failure with exact journal/cleanup facts. Independent Python decoding of final native outputs is separate from stub execution.
- Preserve original ZIP/7z creation, B3 sizes/quick-create/raw join, password/extract/Job/UI behavior and B1–B6 accepted decisions. Run relevant checks/lint/format/lifecycle gates on converged source; no frozen performance rerun.

Intent/layout/source results are recorded in [boundary evidence](ref20-intent-layout-sources.md); creation/publication and actual-product qualification are in [creation evidence](ref20-creation-publication.md). Worker advertises zip_sfx_v1 for explicit implemented ZIP-SFX requests. Seven unique [Create UI/completion selections](ref20-create-ui.md) pass. The [REF-20 handoff](ref20-outcomes.md) indexes frozen evidence and [converged verification](verification.md).

## Necessary paths

New: `rust/crates/nxtools-archive/src/sfx_main.rs`, private `src/sfx/` recipient modules/tests, owned stub notices/build instructions, reproducible stub build helper and bundled own stub asset; independent coordinate/product fixtures and REF-20 records/evidence.

Existing: archive Cargo binary declaration, `worker.rs` ZIP options/capability/validation, `worker_zip_create.rs`, `zip_container.rs`, `worker_volume_output.rs`; Rust public ArchiveJob/Core creation options and Desktop Create draft/forms/Job mappings; C# ArchiveJob contract/session, Worker request/admission/runner, OutputSession family preflight and PhaseE output-name verification. Before each existing-file edit preserve the original B7 idempotent before-copy and record the scoped delta. No replacement runtime, publishing manager or unrelated cleanup.
