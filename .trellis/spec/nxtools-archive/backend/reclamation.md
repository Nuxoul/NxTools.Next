# Stored ZIP source-space reclamation

## 1. Scope / Trigger
Changes to HZ5 destructive extraction, journal replay, native source/output authority or fresh-process continuation.
Ordinary extraction remains source-preserving. This is original step7 within Phase E, not source deletion or a new scheduler.
Evidence: `repo:rust/crates/nxtools-archive/src/reclamation.rs`, `repo:rust/crates/nxtools-archive/src/formats/zip.rs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveReclamationSession.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.Reclamation.cs`,
`repo:rust/apps/nxtools-desktop/src/archive/forms.rs`.
Reference: [HatchZip source notes](../../../../docs/reference-code/hatchzip/in-place-reclamation.md).
Exact framing/actions: [schema1 wire contract](../../../tasks/10-16-archive-manager/hz5-wire-contract.md).
Results: [bounded acceptance](../../../tasks/10-16-archive-manager/hz5-acceptance.md).

## 2. Signatures
Public `ArchiveJobExtract.reclamation` and private `ExtractOptions.reclamation` are optional
`{mode:"start"|"continue",recovery_id:UUID}`. Absent means ordinary extraction; UUID must be nonnil.
Continue uses a new original Job Start/operation/session with the same recovery ID, never stale live Resume.
The existing acknowledged native rendezvous carries `hz5_info/read/plan_entry/seal/select/observe/pending/apply/complete`.
Rust `reclamation_layout` returns `ReclamationLayout {extents,structure}` without reading consumed payload.
C# `ArchiveReclamationSession.Execute` holds source, parent, marker and companion objects;
`ArchiveOutputSession.CommitCore` remains the only publication owner.

## 3. Contracts
- Default-off, one-operation intent with separate irreversible consent. No settings persistence, DeleteSource,
  low-space auto-enable, selected extraction, overwrite, skip, rename, password or ordinary readmission/retry.
  Require full extraction, Here, Ask, `single_file_folder=false` and an explicit fixed destination.
- Only ordinary single-volume unencrypted Stored ZIP regular files/directories, same local NTFS volume.
  Reject ZIP64/SFX/application containers, other methods/formats, special members, layout gaps/trailers/overlaps,
  initially sparse/compressed/encrypted/reparse/multiply-linked sources and conflicting source activity.
- Rust owns qualification/plan/replay/next-unit decisions. C# owns Windows handles, durable records and sparse mutation.
  GPUI presents consent/state. Original readonly `ArchiveSourceLease` sharing is not relaxed. A separate RW source
  capability and identity-bound activity reservation remain held through actual worker/output cleanup, including
  deferred virtual-content use and Modify's intentional FILE handoff. Browser invalidation is deferred cross-entity work.
- Release only inward-aligned payload units of a completely verified file. Native quantum is
  `max(NTFS cluster,65536)`; `(0,0)` authorizes no mutation. A plan with no useful unit refuses before source marker creation.
  Keep all ZIP structure and logical source length. This consumes the original; it does not recycle or restore it.
- Bounds:4096 regular files,32MiB journal,64KiB record/private body,1024byte marker,44KiB retained ZIP structure.
  Charge plan/journal allowance to existing metadata/working limits. One complete output file must fit before its release.
- Start info admits geometry/root only. First qualified plan entry creates one stable append companion. Seal uses
  `ArchiveOutputSession.PreflightReclamationTarget` for the whole target plan: existing files and source-parent identity/name
  aliases refuse before output or source consumption. Only a valid sealed plan binds source ADS `:NxTools.HZ5` and
  flushes/readbacks it before acknowledgment. An unsealed intact original has no mode marker. Continue validates
  source/parent/root/companion identities, full plan and retained structure; old full-source digest/mtime is not immutable after punching.
- Order: original partial identity persisted -> complete CRC/size/readback/metadata/MOTW/flush -> durable ready receipt ->
  original single native publication/final verification with protected final handle -> durable pending -> exact-handle
  punch/source flush/allocation observation -> durable applied. No generic abort/path cleanup of recovery-bearing data.
- Records use bounded `u32LE(length)+UTF8_JSON+u32LE(length)`, schema1/contiguous sequence/typed transitions.
  Append, flush and exact readback precede acknowledgment. Complete corruption/unknown kind refuses. A torn final append
  permits only reconciliation of an earlier complete pending record; never discard pending to retry.
- Before any new release, verify every consumed-prefix output's native identity, fingerprint, metadata and MOTW.
  A ready partial can finish original publication. An incomplete partial without ready is preserved and explicitly refused.
  Pending allocation is observed rather than decoded/recounted; repeated applied calls return the recorded result.
- `released_bytes = pending.source_allocated_before - applied.source_allocated_after`, bounded by approved interval allocation.
  First sparse conversion can expand retained tail allocation; nominal hole length is not net reclaimed space.
  Keep interval/source allocation and volume-free-space observations. Log only bounded lifecycle facts/recovery correlation,
  elapsed time and owning native errors, never payloads/paths/raw records. Ordinary `write_file/flush_file/flush_or_commit`
  failures retain their original native cause rather than becoming generic worker `OutputIo`.

## 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Unsupported method/layout/filesystem or no useful unit | Typed unsupported/no-allocation refusal; no punch, no silent destructive fallback |
| Missing/replaced/corrupt source, output, marker, journal or root | Preserving refusal; no next release or path cleanup |
| Complete pending but interrupted punch/applied append | Verify output and native allocation; finish exactly that pending unit, no duplicate applied count |
| Incomplete output without durable ready | `reclamation_incomplete_partial_preserved`; retain source/output/authority |
| Output/log flush, disk-full112 or native mutation failure | Fail at owning boundary, preserve required state and native code; no next release |
| Completed receipt | Reverify consumed outputs, recognize completion without duplicate append or ordinary decode |
| Ordinary extraction at matching failure seams | Source bytes/native identity unchanged; stale live restart still refuses |

## 5. Good/Base/Bad Cases
Good: fresh process observes an already punched pending interval, verifies its published output and continues the next untouched file.
Base: ordinary extract omits reclamation and retains the whole source. Bad: retry a consumed source through readmission, remove
its output as cache, use a payload CRC failure as release authority, or report an interval's nominal length as actual space saved.

## 6. Tests Required
Rust journal/layout/worker/admission tests assert ready->pending->apply and rejection before output; use strict Clippy.
`repo:csharp/NxTools.PlatformHost.Tests/Hz5ReclamationTests.cs` checks real native allocation, protected output, metadata/MOTW,
ready reconciliation, framing and activity handoff. Extend original `repo:tests/integration/archive-hz4-process.py` with
`repo:tests/integration/archive_hz5.py`: owned PID/creation/image/retained handle, new process/session, each file's release boundary,
repeated interruption, corruption/missing/replaced authority and ordinary source-preserving controls. Destructive fixtures are
new `hz5-<GUID>` children of authorized `PI_SCRATCH_DIR` only. Measure allocated streams at lifecycle hooks plus independent5ms
sampling and observe the same volume's free bytes; exclude fixed binaries/evidence from feature peak accounting.
Desktop Archive GUI tests run serially (`--test-threads=1`) because dialogs share native test context; verify unchecked consent,
new Start without readmission/retry, deferred snapshot retirement and truthful terminal copy. Release fault seams must refuse
before capability/path effects. Process kill is not power loss; physical/user/release acceptance remains separate.

## 7. Wrong vs Correct
Wrong: `flush(output); punch(payload); replace_json_snapshot(); released += payload_length`.
Correct: `verify_publish_protect(output); append_flush_readback(pending); punch_held_source(); flush_source();`
`append_flush_readback(applied_with_native_net_allocation);` then advance only after acknowledged authority.

## Object-scoped Desktop continuation
The archive home has no global continuation entry or manual recovery UUID. A started reclamation task
offers a check/continue action only after failed/cancelled terminal cleanup. Reopening a top-level source
may inspect its recovery record once after failed/rejected read cleanup; cancelled, disconnected and
nested reads do not probe. Missing records leave the original read failure unchanged.

`ArchiveRecoveryInspectCommand` is a bounded read-only command with source, optional original-target
candidate, operation/session/generation and correlated reply. Native held source/parent, ADS marker,
derived companion and framed journal identities must agree. No writable open, repair, truncation,
payload scan, output creation or persistent schema change belongs to inspection. Native slots remain
owned until actual return. A completed receipt reports recorded completion, not current output validity.

The existing OperationForm fixes the original source and restores recovery ID and copy_motw internally.
A matching original target is readonly; otherwise the user selects the original folder and it is checked
again by native identity. No source picker, UUID input, ordinary capacity query, range/conflict/strategy
editing or remembered defaults appears in Continue. Pending, failure, completed, unidentified target
and disconnected states disable both Start and Enter. Discovery never submits. Explicit irreversible
consent and a fresh original Job Start still perform full execution validation.

ArchiveApplication owns pending discovery by current object, generation, epoch and connection; record
before dispatch, retire on close/refresh/cancel/disconnect/deadline and defer all view delivery. Late
receipts cannot re-enable a retired object. Production-view/fake-IPC regressions and synthetic read-only
native record fixtures are separate from destructive execution, native gesture and release acceptance.
