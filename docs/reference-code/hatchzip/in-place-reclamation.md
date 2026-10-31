# HZ5 — Destructive in-place extraction and source-space reclamation

[Authority / source-only evidence](../hatchzip.md); [fixed snapshot manifest](source-manifest.json). The separately approved NxTools.Next bounded implementation and actual results are linked below. This reference note itself grants no native source-write or execution authority. Ordinary NxTools extraction remains source-preserving; post-success recycling remains its separately restricted feature.

## Meaning and user-visible contract

This is NOT extraction followed by deleting/recycling the original. It progressively removes source compressed bytes after durable output/checkpoint publication. Interruption may leave a shortened/sparse nonstandard source plus indispensable journal/tail/partial output. Completion consumes the original; no Recycle Bin promise. A 'restore' generally rebuilds only the **remaining unextracted entries**, not the complete original. Some mid-stream checkpoints cannot be converted back at all. Any future UX must explain that before starting and use a separately explicit/default-off mode, not reuse ordinary Extract or a saved generic delete-source preference.

## Verified source map and full call chain

| Source entry | Responsibility |
| --- | --- |
| `crates/core/src/session.rs::{locate,open,open_encrypted,plan,begin}` | fixed volume family/output/mode; fresh vs resumed/orphan; password proof; space admission; create original journal |
| `crates/core/src/engine.rs:184` Job::build | reverse physical-offset ordering and name/duplicate/layout checks |
| `engine.rs:133` Job::resume | old keep mode protects source; validate current length, finish pending truncation, reopen through saved tail/resume point |
| `engine.rs:777` Run::execute / `:827` commit | unit/batch execution, output flush -> journal committed_len -> source shrink |
| `engine.rs:1026` save_point / `:1230` compressed-stream point region | output prefix flush -> decoder/parser checkpoint -> source punch |
| `engine.rs:1319` next_batch / `:1295` ensure_room | live space reserve and bounded steps/batches |
| `engine.rs:1422` finish | finalize links; release decoder handles; remove residual source then sidecars (keep mode only removes sidecars) |
| `crates/core/src/archive/storage.rs::{open,from_parts,protect,may_change,set_len,punch,cut_to,write_at,remove,reader}` | joined split stream, saved tail view, mutation choke point and keep-mode guard |
| `crates/core/src/journal.rs::{Journal,Resume,create,load,save}` | old volume layout/tail CRC/output/committed frontier/links/decoder state/restore intent/keep mode |
| `crates/core/src/fsutil.rs::{write_atomic,can_punch,punch_probe}` | flushed temp+rename journal, real scratch allocation probe |
| `crates/core/src/platform/windows.rs::{open_exclusive,punch_hole,allocated_bytes,sync_dir}` | Win32sharing/sparse+zero-data calls/allocation observation; directory-sync limitation |
| `crates/core/src/archive/resumable.rs::{Decoder,Inflate,Gzip}` | Stored/Deflate bit-boundary resume,32KiB decoder history, multi-member gzip checkpoints |
| `crates/core/src/restore.rs::{restore,recover,discard}` | rebuild remaining archive, interrupted restore undo, kept-run progress discard |
| `crates/core/src/archive/mod.rs::rebuild_index` | ZIP central index/TARend/7zheader+signature/RARend reconstruction dispatch |

Fresh path: GUI work::extract / CLI extract -> locate/open/password+layout checks -> space plan -> optional verify-first -> session::begin (save tail then journal) -> Run::execute -> output publication/flush -> journal -> shrink/punch -> finalize/cleanup. Resume path can **mutate source while reopening** to finish a previously journaled truncation; 'open' is not automatically read-only when writable=true.

## Algorithm and data dependencies

### Whole units / batches

ZIP entry and plain TAR entry are independent units;7zsolidblock is one multi-file unit. Job sorts by descending offset, then extracts a bounded batch. Defaults: max512units / nominal2GiBbatch target, but next_batch permits a larger first unit when space fits (`BATCH_MAX_BYTES.max(first)`); these are reference tuning, not our memory/output budgets. Independent units use Rayon; RAR serial worker walks forward within selected batch and releases archive handle before mutation.

Outputs are written under temporary part names, decoder CRC/declared-size checks applied, finished parts published. commit flushes output, writes new committed_len/links and clears spent resume point atomically, then truncates source. Saved ZIP/7z tail preserves index bytes at original offsets through a virtual Reader after real EOF; removed regions may read zeros. Source units cannot be reclaimed merely because a UI row finished.

### Large-file streaming / compressed tar

Where capability probe proves allocation release and source/output truly share a filesystem, selected large units can journal checkpoints then punch consumed data. Defaults stream threshold512MiB and step256MiB, plus decoder boundary/buffers/alignment; these are not hard upper bounds on distance to the next Deflate boundary.

Stored data resumes at a byte. Deflate resumes only at a valid block boundary, preserving bit position and32KiB output history, running CRC and durable output prefix. The byte containing the restart bit is retained. Some encoders produce one huge block, so no useful mid-file release is promised. Gzip also tracks member CRC/length and TARparser state; its history can span headers/files and is serialized in StreamPoint rather than read from one file. Partial state is no longer disposable after source punch.

If punch later reports unsupported, reference stops relying on mid-file reclaim and rechecks capacity; this is not proof a formerly feasible plan stays feasible. Only measured allocated bytes and target-volume capacity show physical benefit; logical file length alone cannot prove space freed. Copy-on-write snapshots/clones may retain allocation.

## Format capability matrix (reference only, not NxTools support)

| Format | Release unit | Mid-unit reclaim / important refusal |
| --- | --- | --- |
| ZIP/ZIP64 incl compatible SFX | completed file extent | unencrypted Stored/Deflate only for streaming; zip.rs::stream explicitly excludes encryption/other methods |
|7znon-solid/solid | completed block | no mid-solid-block resume/punch implementation; one huge block can eliminate savings; anti-items rejected |
| Plain TAR | completed entry | regular files stream Stored with `crc:None`; TARpayload has no checksum, no invented verification |
| `.tar.gz`/`.tgz`, multi-member/split | whole front-to-back stream with checkpoints | requires full planning scan; can punch behind decoder; without support source kept whole until completion; partly reclaimed stream cannot rebuild remaining standard tar.gz |
| RAR4/5non-solid single/set | file starting at first header, possibly spans volumes | only unencrypted stored file wholly in one volume with CRC and size agreement streams; compressed files require whole-file completion |
| Solid RAR | no destructive mode | refuses_in_place; kept-source extraction only |
| RARencrypted names/links/unsupported dictionary | unsupported cases | encrypted names/links rejected; dictionary>4GiB rejected by reference; our budgets must remain smaller where configured |
| Raw `.001` split family | joined logical byte range | fully consumed trailing volumes deleted; boundary volume shortened; any volume name resolves family |
| Native ZIP `.z01`; `.tar.xz`/`.tar.zst` | no claimed support | README marks planned/refused; our existing read support does not imply resumable reclamation |

## Journal, recovery and format-version rules

Journal v3destructive / v4keep (reads2,3,4) prevents old versions from consuming a source explicitly kept. Mode remains fixed on resume even if CLIomits flag; changing in-place run to keep is refused before writable reopen. Contains original volume paths/lengths, format, output, tail offset+CRC, committed_len, links, Resume and optional restoring undo bytes. Passwords are not serialized.

Source length below committed frontier refuses; longer prefix may be pending shrink. `restore` writes remaining index, journals original header patch bytes before modifying7zsignature, reopens and compares remaining entry list, removes journal before tail. Mid-stream Resume causes explicit refusal because some member payload is gone. `sevenz.rs::rebuild_header` reads saved headers with Password::empty and rejects unusual packed layouts; do not infer encrypted-header7z reconstruction merely from encrypted extraction support. Keep mode only discards progress; it does not need reconstruction. `Journal::Paths::remove` ordinary cleanup iterates tail then journal, whereas restore deletes journal then tail: do not unify cleanup order from prose without inspecting orphan/failure states and proving interrupted cleanup behavior. Missing/corrupt sidecars must never invite deleting them blindly.

## Critical gaps / why not directly import

1. **NxTools source capability conflicts.** ArchiveSourceLease holds parents without delete sharing and source with read-only sharing. Widening/releasing it to let a worker reopen and truncate is forbidden. Design a separate original-object mutation capability at C# Windows owner, bound to operation/session/generation/volume/fileID and exact approved intervals/checkpoints. Rust plans/decodes, never acquires arbitrary Windows write/delete rights.
2. **Recovery authority.** Reference sidecar paths/lengths/CRC and clear_uncommitted path deletion are not sufficient protection against tampering, user replacement, reparse/hardlink races or all volumes being replaced. Persist bounded original source/output identities, generation, versions and verified mutation frontier; authenticate/ACL-bind journal as appropriate. Do not accept arbitrary paths or grant deletion from a hash supplied by the same untrusted record. Define how legitimate source size/hash changes advance identity; old whole-source hash cannot remain constant during reclaim.
3. **All-volume ownership.** Reference locks first source and reopens later volumes by path; our adaptation must protect every original volume through read/commit/mutation and prohibit foreign/late/alias volumes. Source and output must not overlap/hardlink each other.
4. **Durability.** Windows reference sync_dir/sync_batch are no-ops; share-delete is permitted in its open_exclusive; Storage::set_len is path/handle mixed and not a complete proven durable Windows transaction. Flush ordering, rename/publish, metadata/MOTW, journal replacement, source truncation/deletion and disk write caching need explicit Windows proof. Process abort tests alone are insufficient.
5. **Output survives reclaim.** Antivirus may quarantine/remove a newly published file; README/TODO proposes readback before source cut but it remains TODO. Require our real output owner to verify identity/content/metadata/actual publication before irrevocable source release. Decide semantics for subsequent external deletion; cannot promise indefinite data preservation.
6. **Budgets and failure.** Journal/tail/parser/history/entry count allocation must be bounded before allocating (reference loads JSON/tail into memory). CRC is not authenticity; TAR has no payload checksum. Wrong password/corruption/unknown outcome/partial metadata failure must never grant next reclaim. No readmission retry may restart from0 over already punched data.
7. **Coexistence.** Browser/preview/nested/read/drag/modify leases must be excluded or coherently suspended; cached snapshots become invalid as source changes. Ordinary source-delete eligibility/recycle and ZIPedit atomic replace remain distinct; no permanent-delete fallback or implicit behavior changes.

## Future development gates and acceptance

HZ5-0: approved explicit UX/default-off/recoverability wording and minimum format scope; no automatic fallback into destructive mode because disk is low. HZ5-1: native source/output/journal capability design, bounded schema/compatibility and inspect-only recovery preview; ordinary extraction protection unchanged. HZ5-2: smallest disposable unencrypted StoredZIP fixture, full file units, original output readback -> journal -> exact-handle truncate, fail-safe at every boundary. HZ5-3: [HZ4test matrix](crash-recovery-tests.md), actual allocation/space benefit, disk-full/flush/native error, replaced source/output/sidecars, all-volume race/refusal and repeated crash. HZ5-4: only after proof expand Deflate/TAR/7z/volumes; compressed streams/RAR need separate exact codec/state validation. No format is accepted from README coverage alone. HZ5-5: separately authorized disposable Windows/NTFSandotherfilesystem/VMpowercut/native/user acceptance before release/default changes.

Open choices for future owner: smallest first-release format/method/volume/encryption matrix; verify-first default; reserves/low-space policy; explicit handling when source cannot be fully reconstructed; journal location/ACL/recovery export; whether reusable batch checkpoints are enough or decoder checkpoints required. User asked for this capability, but these decisions are not resolved by source inspection. Implementing it safely is a multi-layer feature, not a checkbox on existing DeleteSource.

## Step7 implementation re-entry

The nine indexed HZ5 source files (session, engine, journal, storage, fsutil, Windows,
restore, ZIP and resumable decoders) were rechecked against the existing source manifest;
all nine match. Actual source windows establish session::open/begin, Job::resume/build,
Run::commit/save_point/finish, Journal::create/load/save, Storage::may_change/set_len/punch,
the allocation probe, Windows sparse/flush limitations and restore/recover distinctions.
No reference runtime or source mutation was executed.

Current NxTools source confirms ArchiveSourceLease::OpenCore keeps protected parents and
read-only/read-share source access; ArchiveOutputSession::CommitCore applies MOTW/metadata,
flushes, captures identity/fingerprint, publishes once, then verifies actual final identity.
Its CommitJournal remains process-local. ZIP preflight_created validates original local and
descriptor extents for overlap; stream_entry and extract retain CRC/size/budget checks and
ordinary full-source revision validation. These are reuse seams, not HZ5 recovery authority.

The [Step7 design](../../../.trellis/tasks/10-16-archive-manager/hz5-design.md) limits the first
capability to full-file Stored ZIP payload punching on a demonstrated same-volume NTFS
capability. It preserves headers/index and a consumed-source marker, separates persistent
pending-release authority from applied observation, and leaves ordinary source protection
unchanged. Native destructive fixture execution requires the user's separate confirmation.

## NxTools.Next bounded implementation evidence

[HZ5 acceptance](../../../.trellis/tasks/10-16-archive-manager/hz5-acceptance.md) records the separately approved ordinary Stored ZIP/same-volume NTFS implementation. The reference remains behavioral research, not a source of Windows durability or execution authority.

Verified implementation entry chain: `rust/crates/nxtools-archive/src/reclamation.rs` reuses `formats/zip.rs::reclamation_layout/stream_entry` and the original acknowledged worker output calls; `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveReclamationSession.cs` holds the separate source/companion/marker authority; `ArchiveOutputSession.Reclamation.cs` and original `CommitCore` own ready receipts, one native publication and protected verified outputs. `ArchiveReclamationNative.cs` owns NTFS capability, allocated ranges and exact-handle sparse APIs. `Hz4ProcessTests.cs` and `tests/integration/archive_hz5.py` exercise the original Job/worker/cleanup boundary with fresh processes. Ordinary source sharing remains read-only.

The native release quantum is at least64KiB even on a smaller-cluster NTFS volume. Actual net allocation reduction includes sparse conversion's retained-tail allocation; nominal punched length is not space reclaimed. The original native output owner preflights the complete target plan, including source-parent identity/name aliases and existing files, before sealing and binding the source marker. New-process continuation rechecks retained ZIP structure and every consumed output before another release. The tested contract is remaining extraction only, with explicit preservation/refusal for incomplete unready output or missing/changed authority; no original-archive reconstruction or power-loss acceptance follows.

## Read-only recovery record discovery

The separate additive `archive_recovery_inspect` command observes the original HZ5 marker and companion without invoking `ArchiveReclamationSession` or the worker. Native entry: `PlatformPipeClient` → detached `ArchiveRecoveryInspection.DispatchAsync` → bounded `Run` → `Observe`. `ArchiveSourceLease.OpenForRecoveryInspection` uses the existing held read-only source and parent chain with the explicit recovery-marker allowance; ordinary `Open` still refuses marked sources. `ArchiveReclamationNative.OpenMarker(..., false)` opens the ADS read-only and compares native file identity. The derived `.nxtools-hz5-{recovery_id:N}.journal` is opened read-only relative to the held source parent. Original journal authority/framing is `ArchiveReclamationSession.CreateAuthority/BindMarker/OpenContinuation/Load/Accept` and the schema1 wire contract. Header `root_identity` is a native identity, not a path.

The inspector bounds the marker to1024 bytes and the journal/frame to32MiB/65536 bytes, checks schema, sequence, typed record order and marker/header/source/parent/companion IDs (including source length). A torn final append cannot certify completed state. A matching original target is returned only after a held no-reparse directory open of the caller-provided candidate, or when the source's held parent equals the recorded root identity and no candidate was supplied. The result is a discovery descriptor, not output verification, source/write authority or permission to resume. Original output publication, replay validation and source mutation remain exclusively with the original execution path.
