# Phase E — approved implementation boundary

2026-10-22 user decision closes the [capability gate](research/phase-e-zip-capability.md).
[implement.md](implement.md) owns exact sequence; [checkpoint](phase-a-checkpoint.md) alone owns current
status. Earlier outcomes/acceptances are immutable history. Only P202/P203; no P204/P205/F or release.

## Approved edit-only lease transition amendment — 2026-10-22

Supersedes the prior continuously-held FILE requirement for SAME-PATH ZIP EDIT publication only.
Current GPUI authority is exact0.8.0/defaultpre0.3.9,not historical0.7.1. Preserve approved list owner.
Handle order,all within original worker/native output session:
1. Candidate complete and required structure/content verification succeeds while source protected.
2. Worker stops source reads and ACTUALLY closes every original source reader; only then sends the
   original acknowledged edit-read-complete boundary. No self-reported pending read may hand off.
3. C# uses still-held original source FILE to verify original full nativeID/version/hash and exact
   target consistency. Candidate and continuously held no-reparse parent handles stay live.
4. Final precommit cancellation check precedes original CommitInFlight. At that boundary close
   ONLY edit-source FILE; never source.Dispose/parent release. One existing atomic replace follows.
5. Actual native success immediately records Committed before any late cancel/postverification.
   Failure/unknown remains existing typed/journal fact,no automatic retry or source path deletion.

The user explicitly accepts final-check→FILE-close→path-publication TOCTOU,not strict nativeID-CAS
or elimination of external writes/replacements in that window. Ordinary extract/create/test source
sharing/lifetime unchanged; no early release,global write/delete share,backup move,new rights/API/
backend or second commit framework. Reader-finished/candidate-verified/expected source identity are
owned execution facts,not a browser boolean or arbitrary wire permission. Direct native fixtures
prove boundary independently; worker lifecycle evidence requires actual matching worker execution.

Required reds/greens: source protected during generation,readersclose-before-handoff,changed-source/
target precheck refuses,pre-handoffcancel retains old hash,postlinearizationnative-success wins,
failure/unknown no retry,parent/candidate ownership retained and ordinary operations unchanged.
Keep original3success/cancel/revision fixtures,adjust only invalid old-FILE-postcommit assumptions.
Uppercase.Z01 repair uses parsed family+ordinal and Windows path semantics,not unconditional path
ignore/broad discovery; explicitly test case-sensitive-platform behavior and invalid volume members.

## Container and identity invariants

Rust ZIP container adapter maps disk+offset to checked global encoded ranges; central/local/EOCD/
ZIP64 retain distinct raw records. Payload codecs/crypto/integrity stay original. A finite anonymous
encoded spool may normalize a protected volume set for original readers; it is not plaintext payload
extraction or a whole archive Vec. Each volume revision participates in snapshot identity and native
leases protect all exact siblings throughout actual worker operation. Old ordinals cannot bind to a
new/replaced set. Discovery is same-directory/exact stem/index bounded, never a broad recursive scan.

Writer regenerates genuine per-disk member locations, central-directory start/counts and EOCD/ZIP64
fields. Local/central structural headers and end structures are placed intact; data ranges may cross
volumes. Original local/header/encryption/descriptor bytes stay unchanged except explicit renamed
names/extras; absolute offsets and ZIP64 offsets are recalculated. Unsupported overlap/gaps/signature/
application structures are refused before edits, not silently dropped. Metadata budgets are charged
before allocation, encoded bytes finite, cancellation checked per record and <=64KiB copy chunk.

## Transactions and lifecycle

Split NEW target family only: bounded native family conflict check BEFORE output effects; no-replace
at every final publication, including races. All exclusive candidates written/flushed/readback-set-
validated before first publication. Per-volume identity/state/publication journal survives partial
failure/cancel; no global atomicity or inferred zero modification before last.zip. No existing set
backup/overwrite/recovery machinery. Cleanup only unchanged owned products by original native rules.
The existing output session retains at most4096 transaction records after exact-token cleanup;
terminal projection occurs after actual Dispose. Each output carries path-free full native fileID,
published true/false (absent for uncertain), and cleanup category. Original encoder spool is explicitly
excluded from volume journal/counts. Published members remain retained after a later-member failure.

Ordinary ZIP edit: immutable browser source/revision/entry IDs -> original bounded per-source FIFO ->
original worker/Job/source capability -> full exclusive candidate -> full original payload/structure
verification -> source/native identity check -> existing atomic replacement -> truthful once-only
terminal. Never delete/truncate/move original first. Same-source held-lease replacement must be an
actual native regression; neither share flags nor POSIX_SEMANTICS alone prove its result. No rights
change without precise native evidence and approval. Failure before commit retains source and edit
draft; commit-bearing/unknown facts never relabel as unchanged or blind retry.
P203 Add creates new regular entries with original ZIP writer Deflate/plain and Stored directory
records; the form explicitly discloses that additions are not encrypted. Existing encrypted records
remain byte-preserved, not decrypted/re-encrypted. Editing preserves source Zone.Identifier through
the existing bounded native MOTW owner and ordinary ReadOnly/Hidden/System/Archive bits only;
filesystem-managed flags/ACLs/arbitrary ADS and source last-write time are not claimed preserved.

UI uses existing official Form/Field/Input/Textarea/Dialog/outlined Medium Toolbar.content Buttons,
approved details-only ListItem fallback and one selection owner. Stable input entities, copied IDs,
per-draft generations/tickets, pending-before-dispatch and deferred cross-entity callbacks including
immediate/cache completions. Browser close retires presentation, not independent job. Success explicitly
refreshes to new snapshot; failed refresh retains last-good stale/read-only. No fake Apply/actions.

Diagnostics extend original correlation/start/state/terminal/elapsed/native-code owners, no new
runtime/sink, no paths/names/password/comments/content or byte ticks. Required reds target production
container/worker/native/UI seams; small independent oracle only, no large/performance matrix. Final
C# same-set LIST→RUN after isolated host build/final-feature matching worker; one final Desktop after
source convergence. No GUI/Shell/DPI/UIA/user claim from TestWindow/compilation; live NOT RUN remains.
