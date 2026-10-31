# Step7: HZ5 — progressive source-space reclamation

## Scope and current boundary

Canonical work: **第7步：HZ5 — 边解压边释放原件空间**, within **Phase E — Format expansion and ZIP editing**. Original steps1–6 retain their scoped acceptance. Release42 remains the current package. The bounded implementation follows this design; actual process/allocation evidence and remaining acceptance boundaries are recorded in [HZ5 acceptance](hz5-acceptance.md).

HZ4 has actual process-interruption, source/output verification and fresh-process safe-refusal evidence. `ArchiveOutputSession.CommitJournal()` is an in-memory projection. `ArchiveJobSession` active/retired state, terminal events and fixture capabilities are not persistent recovery authority. HZ5 adds that authority rather than relabeling HZ4.

## Minimum supported operation

- A separate per-operation destructive intent, initially off. Never reuse `DeleteSource`, save this as an extraction default, or enable it automatically on low disk space.
- Ordinary, single-volume, unencrypted ZIP with Stored regular files and ordinary directories. Full extraction only, original safe path/collision planning and finite resource limits. No skip/overwrite/automatic rename of existing output files in the initial destructive operation.
- Local NTFS, source and destination on the same native volume; named-stream and sparse-file support must be observed. Reject initially sparse, compressed, encrypted, reparse or multiply-linked source objects for this first capability.
- Reclaim only inward-aligned sparse ranges wholly inside a **completed regular file's payload**, never local headers, descriptors, central-directory bytes, end records or another entry. Validate all extents for overlap before the first output. The native release quantum is `max(NTFS cluster,64KiB sparse deallocation unit)`, not an assumed logical ZIP boundary.
- No mid-file release or decoder checkpoint. A large file needs enough free space for that complete file plus bounded journal/output overhead. A source with no useful complete allocation unit is refused as having no reclaimable payload; logical zeros are not reported as reclaimed allocation.
- Keep the ZIP structure and native source object as a consumed sparse shell. Completion does not recycle/delete the archive, recreate its removed data or claim a fully usable original archive. This deliberately avoids saved-tail dependencies and tail truncation in the first implementation.

### Explicitly unavailable combinations

| Combination | Initial refusal reason |
| --- | --- |
| Deflate or any other ZIP method | No qualified decoder-state/release contract; ordinary extraction remains available |
| Encrypted entries or headers | No qualified encrypted resume/reclamation contract |
| ZIP64, SFX, signed/application containers | Outside initial ordinary ZIP layout admission |
| Split archives | No destructive all-original-volume capability |
| TAR / compressed TAR / standalone streams | No qualified payload-integrity and parser/decoder recovery contract |
| 7z, including solid blocks | No qualified dependency-block reclamation adapter |
| RAR | No qualified native decoder/restart adapter |
| Links, special members, ambiguous or overlapping extents | No independently releasable ordinary file unit |
| Other filesystems, different volumes, allocation support absent | No demonstrated usable allocation benefit for this capability |
| Active ordinary source/browser/preview/drag/edit lease | Original source owner is busy; do not relax its protection |

This matrix restricts reclamation only, not existing source-preserving format support.

## User-visible semantics and presentation

In the original extraction Form/Field, use one unchecked official `Checkbox` for `边解压边释放原件空间（消耗原压缩包）`, with a persistent Small nonclosable warning `Alert`. Before start, require a separate explicit acknowledgement in the original confirmation owner. Unsupported combinations remain disabled with the actual reason, not silently changed into another mode. Use the existing outlined action buttons and the original progress window; no new settings system, scheduler or progress controller.

Confirmation copy must say:

> 原压缩包中的文件数据会在解压核验并保存恢复记录后逐步释放，不能撤销，也不会进入回收站。中断后请保留原压缩包、恢复记录和已解压文件，使用“继续空间回收解压”。本版本只能继续剩余解压，不能重建剩余归档或恢复完整原压缩包。当前文件完整输出所需的空间仍必须足够。

Recovery is an explicit operation on persisted state, not the existing pause/resume action on a live Job. Restart does not require remembering the original checkbox. Cancellation stops further units; failure stops at the first unproven boundary. Published output corresponding to released payload is indispensable. Later external deletion or alteration of that output cannot be undone from the consumed source.

Component ledger: current Kit/Base/Component/Assets0.8.0, default gpui-pre0.3.9; `component::form::{Form, Field}`, `component::checkbox::Checkbox`, `component::alert::Alert`, and original confirmation/footer `Button`s. Controlled requested boolean, one callback, deferred application updates, unchanged focus/close/cancel owner. Checkbox and inline Alert are not Toolbar children; outlined footer actions must not pass through `Toolbar.child`'s ghost/compact adaptation. Exact released builders and actual minimum-window layout are checked before UI edits. No visual or physical acceptance follows from this ledger.

## Ownership and permission

Rust owns qualification, immutable ZIP extent/output planning, the versioned recovery model, state transitions, next-unit decisions and recovery acceptance/refusal. Extend the original Job, worker private rendezvous, OperationControl and diagnostics. C# owns source namespace/object protection, exact-handle sparse operations, output and journal file handles, filesystem observations and native errors. GPUI only presents state and explicit choices.

`ArchiveSourceLease.Open` remains read-only/read-share with protected parent handles. A separate reclamation lease opens the exact source object for read/write while retaining its no-reparse parent chain and allowing only read sharing. Ordinary leases then conflict in both acquisition orders. Existing worker read-only opens may resolve that held namespace; arbitrary worker write/delete/truncate opens cannot acquire access. Only the native lease may modify an already-approved interval of its own object. No source FILE handoff, delete right, edit replacement or ordinary recycle authorization is reused.

Before effects: reject lexical source/output overlap and native same-object aliases through original source/output owners. Require one source link for the first version. Browser/cache generations must not continue to advertise a consumed snapshot. A live conflicting lease is a refusal, not permission to close another owner's handles. File sharing is not sufficient for deferred drag authorization or Modify's intentional source-FILE handoff: connect operation-bound source-activity reservations at the original read/entry/drag/job admission owners, keyed by the actual native source identity. Hold the reclamation reservation through actual cleanup; preserve existing per-archive serialization and queue capacity.

## Persistent authority

Use a bounded versioned companion record in the source's already-held parent and an attached source-mode marker containing the recovery ID and exact companion identity. The native store only opens its derived companion name relative to that protected parent; it never accepts arbitrary cleanup paths from JSON. The marker distinguishes an unfinished/consumed source from a fresh archive even when the main record is missing. Initially sparse inputs without a valid matching HZ5 record are not admitted as fresh reclamation jobs.

Schema1 carries:

- Fixed mode `stored_zip_whole_file_sparse`, recovery/logical Job ID, original source volume/full128-bit file ID, original logical length, protected parent identity and original archive metadata.
- Original destination-root identity and fixed validated relative output plan; at most4096 entries,32MiB total journal and64KiB per acknowledged private transport chunk. Charge retained metadata to the existing budget and reserve the finite journal allowance before the first output. Source-mode marker is limited to1024bytes. Larger plans refuse before any source mutation.
- Per-entry ordinal, validated header/payload/descriptor extents, size and ZIP CRC, required output metadata/MOTW facts and exact approved inward-aligned release interval.
- Native identities for the current partial **before publication**, final outputs, existing output fingerprint/version facts used by `ArchiveOutputSession`, and a monotone completed frontier.
- One pending source release, its pre-release allocation observation, last acknowledged applied release, terminal state and any indispensable owned partial. Passwords and file contents are absent.

Only existing ZIP CRC and output identity/fingerprint contracts participate in decisions; do not add an unread hash inventory. On restart, rederive/compare the still-retained ZIP structure, validate original source/root object identities, and verify every already-consumed unit's real published output. A legitimate source ChangeTime/allocation change from a recorded punch must not fail the old whole-file digest rule. The immutable object identity and retained format structure remain mandatory.

Use an append-only native journal on one continuously held companion object, not a replaceable JSON snapshot. Each schema1 record has a bounded length, monotone sequence, typed payload and matching end framing; append -> FlushFileBuffers -> exact record readback precedes acknowledgement. Immutable plan is written once; partial identity, pending release, applied observation and terminal are bounded subsequent records. Before the first punch, flush the attached source marker binding the stable companion file ID. A torn final append preserves the last complete record and permits only reconciliation of its existing pending operation; a malformed complete record, gap, invalid transition or mismatched identity refuses. Never discard a complete pending-release record to continue. There is no journal-rename/directory-sync claim. Missing, corrupt, foreign-version or mismatched state is a typed refusal preserving all files, not permission to clean sidecars.

## Per-file release protocol

1. Native lease and source/destination capability admission; Rust validates the complete supported layout and resource plan. Persist initial mode/plan/identities; bind the marker before any punch.
2. Original output owner creates an exclusive sibling partial. Persist its native identity and relative final binding before the original publication can happen. A crash before this identity is recorded grants no path-based cleanup authority.
3. Rust streams the complete Stored payload through existing bounded writes and format CRC/size checks. Apply required metadata/MOTW using the original output owner. Flush the actual partial and verify readback.
4. Publish through the original single native no-replace transaction. Verify actual final identity, content fingerprint, required metadata/MOTW and flush. Retain the verified final handle without write/delete sharing through durable pending logging and source release. Unknown/postcommit verification states do not approve release. Current CommitCore only briefly reopens the final file and does not compare final attributes/MOTW; these receipt facts must be added in that original owner.
5. Append a pending-release record naming this verified output and exact source interval. Flush and read back the complete real record before acknowledging it. This is the only source-release authority.
6. C# rechecks the held source object and output, observes cancellation, queries actual allocation and punches only that approved interval on its held handle. Flush the source, query allocated ranges/bytes and same-volume free space. Record observed bytes, not nominal interval size.
7. Persist the applied result, then advance to the next complete file. If this update fails, retain the **already durable pending intent**. A new process validates outputs and inspects actual allocated ranges to distinguish an unapplied operation from an already-applied hole; it never discards that intent, counts the same released bytes twice or decodes that payload again.
8. Finalization persists `completed_consumed`, verifies output set and releases only owned transient resources. Keep the consumed-source marker and recovery receipt. Final cleanup does not delete published outputs, original source, indispensable records or arbitrary paths.

No source release precedes steps3–5. No next release follows a failed/unknown content, publication, metadata, flush or journal result. Original ordinary output cleanup remains unchanged; this mode excludes consumed output from generic cleanup/retry authority.

## Fresh-process entry and scheduling

Add an explicit `continue_reclamation` intent routed through the existing ArchiveApplication queue and native ArchiveJobSession/ArchiveWorkerRunner. It loads native persisted state before any ordinary archive execution. A new native attempt retains the recovery/logical correlation; stale live-session controls remain stale. Ordinary `resume` retains its existing live-session meaning.

Both original finite output readmission and batch retry must reject a consumed or recovery-bearing operation. A recovery continuation can complete a recorded pending release or continue **remaining extraction** only after the durable validation above. Rebuilding a remaining standard archive and restoring the full original are unavailable operations with distinct wording.

## Verified owner chain and expected change map

| Owner / files | Necessary HZ5 change |
| --- | --- |
| `rust/crates/nxtools-archive/src/formats/zip.rs` | Reuse `preflight_created`, `validate_local`, `validate_descriptor`, `stream_entry` and the original path/output plan; expose only a qualified full-file payload plan. Ordinary extraction still uses `ensure_revision`. |
| `rust/crates/nxtools-archive/src/worker.rs`, `worker_job.rs`, `worker_output.rs`, new `reclamation.rs` | Typed bounded journal parser/state and a scoped Stored path over the original acknowledged native rendezvous; source checks use the new capability rather than old whole-file digest after a punch. No new codec or Windows FFI. |
| `rust/crates/nxtools-protocol/src/archive_job.rs`, C# `Protocol/ArchiveJobContract.cs` | Explicit default-off mode, distinct fresh-process continuation and bounded recovery progress/terminal facts with symmetric validation. |
| `rust/crates/nxtools-ipc/src/archive_job.rs`, Archive `product_job.rs` | Preserve original dispatch/correlation; exclude reclamation from ordinary finite readmission. |
| Desktop `archive/draft.rs`, `forms.rs`, `jobs.rs`, `readmission.rs`, `batch_jobs.rs`, original browser context | Explicit choice/consent/recovery entry, original FIFO/Job/progress, source snapshot retirement and no generic retry. Exact released control builders are inspected before edits. |
| C# `Protocol/ArchiveJobSession.cs`, `ArchiveEntrySession.cs`, `ArchiveReadSession.cs`; `Windows/Archive/ArchiveVirtualContent.cs`, `ArchiveWorkerRunner.cs` | Original admission and shared source-activity exclusion, fresh-process recovery dispatch, same suspended worker/Job/process-pump/actual-cleanup ownership. Current child inherits only stdio; do not assume a source handle is inherited. |
| C# `Windows/Archive/ArchiveOutputSession.cs`, `ArchiveWorkerOutputBridge.cs`, new `ArchiveReclamationSession.cs` | Existing output transaction gains durable prepublication identity, full verified output receipt and retained final handle. New native reclamation owner holds the separate writable source, companion/marker and exact approved sparse interval; performs native allocation observations. |
| `csharp/NxTools.PlatformHost.Tests/Hz4ProcessTests.cs`, `tests/integration/archive-hz4-process.py`, focused source/output and Rust worker contract tests | Add Stored reclaim checkpoints and fresh disk-state recovery while preserving HZ4 ordinary safe-refusal assertions; exact owned child and independent output oracle. |

The native call chain is `ArchiveJobSession.Dispatch -> ArchiveWorkerRunner.Start/Run -> source_acquire -> ArchiveVolumeSources/ArchiveSourceLease -> ArchiveWorkerOutputBridge -> ArchiveOutputSession.CommitCore -> actual worker/pump drainage -> original Job terminal`. HZ5 branches at the operation/source capability, not ordinary source sharing or a second runner. `PlatformPipeClient` composes the sibling sessions; deferred drag authorization and source-FILE edit handoff are real activity-reservation cases.

## Diagnostic contract

Use original correlation/logical Job and native attempt IDs. Add bounded mode/stage/checkpoint sequence, accepted/start, journal durable, release pending/applied, measured allocated-byte delta, recovery accepted/refused, completed/cancelled/failed, elapsed and owning native error code. No paths/member names, contents, passwords, tokens, raw journal data or per-chunk/progress-tick logging. Sink failure never changes permission or results.

## Relevant verification

| Boundary / failure | Required actual result |
| --- | --- |
| Before/after output flush and publication | Fresh process reconciles only recorded exact output identities; unreleased source payload remains intact |
| Before/after native journal append/flush/readback | Last complete real record leads to reconciliation, safe continuation or explicit refusal; no unproven punch and no discarded pending intent |
| Before/after each punch | Exact child is terminated; new process loads disk state and continues or refuses without redecoding consumed units |
| Punch succeeded, applied-record write failed | Durable pending intent plus native range observation identifies the already-applied release without double counting |
| Before/after finalization/cleanup; repeated interruptions | Completed receipt remains recognizable; published data and recovery authority survive |
| Disk-full112, output/log flush failure, native sparse failure | Exact owning failure; no next release; retained output/journal state is truthful |
| Replaced source/output/root; missing/corrupt marker/record | Native identity/structure/content refusal; unrelated replacements remain unchanged |
| Ordinary extraction at the same seams | Source bytes unchanged, original cleanup/cancel behavior and format admission retained |
| Capability unsupported / no allocation benefit | Explicit refusal, never success from logical size or nominal interval alone |
| Allocation/space measurement | Actual source allocated bytes, affected-volume free-space observations and peak additional source+outputs+record allocation retained per checkpoint |
| Release fault-hook gate | Process-kill/failure knobs are compiled out/refused in real Release |

Extend original HZ4 owned-process driver/fixture and source/output tests rather than adding a scheduler or copying production decision logic into test models. Exact spawned PID/creation/image/handle only; no process-name termination. Keep independent Python ZIP input/output oracle and actual persistent files. Injected112/flush/native faults are classified as injections, not actual physical disk exhaustion. Process interruption is not VM power loss; filesystem persistence, physical/native UI and user acceptance remain separately unverified until performed.

## One-time execution confirmation

Before the first real source truncate/punch, obtain explicit confirmation of this design and permission to create, write, punch and retain only disposable `hz5-<GUID>` fixtures under the current session scratch directory. This includes sparse/native allocation probes, exact owned child termination and fresh-process continuation. No user original, reference checkout, Release42 package, service/registry/system setting, signing material, installation, commit or push is included.

### Confirmed authorization

The user explicitly confirmed this bounded design and authorized destructive native tests only on newly created disposable `hz5-<GUID>` fixtures under the current session scratch directory. The authorization includes sparse allocation probes, actual payload punching, exact owned child interruption and fresh-process continuation. It does not include user originals, reference sources, packaging, installation, services/system changes, commits or pushes. Actual outcomes are in [HZ5 acceptance](hz5-acceptance.md).
