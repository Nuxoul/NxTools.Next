# Protocol backend

Owner: typed PlatformHost/indexer wire model, not transport/state/persistence. Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#Envelope`.

## Pre-Development Checklist
Read [engineering](../../shared/engineering.md), [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md). Schema changes trace Rust/C#/caller/test, preserve version/defaults and bounds.

## Quality Check
`cargo test -p nxtools-protocol --lib`; affected IPC/C# roundtrips and lifecycle tests; scoped lint/check/fmt per [verification](../../shared/verification.md). No actual native command required for DTO tests. [Standards entry](../../index.md).

Archive7z additive contracts: `ArchiveJobModifyAction::ReplaceContent` is SevenZ-only with2paths/
1ordinal/valueNone; optional source_prefix binds implicit-directory rename to its old descendant.
Optional ArchiveBrowserContext.sevenz_edit is advisory actual-open eligibility; missing legacy
facts do not grant edit authority. Rust/C# strict shape checks and original source revision/attempt
remain. See [owning format contract](../../nxtools-archive/backend/sevenz-edit.md).

HZ3 explicit ArchiveSpace version2 adds optional bounded private source_set facts while version1
shape remains unchanged; exact Rust/C#/caller binding and limits are in
[archive batch contract](../../nxtools-desktop/backend/archive-batches.md). These read-only native
observations grant no execution/write authority; missing facts fail closed for dedup/retry.

HZ3-AC01 adds optional `ArchiveJobTerminal.output_published: Option<bool>` / C# `bool?`:
absent/null is unknown, false is affirmative native zero-output evidence, true includes retained
directory output. Default omission/strictboolean remain symmetric; facts do not grant execution,
commit or cleanup authority and do not change native terminal precedence. See
[native publication contract](../../platformhost/archive-creation.md) and original batch contract.

HZ5 adds optional `ArchiveJobExtract.reclamation {mode:start|continue,recovery_id:UUID}` with symmetric Rust/C# full/Here/Ask/nonnil/no-readmission validation. Absence preserves ordinary extraction; Continue is a new Start, not live Resume. See [reclamation contract](../../nxtools-archive/backend/reclamation.md).

ArchiveEntry v1 optional preview_options follows [the frozen preview budget contract](../../nxtools-desktop/backend/archive-preview.md):
missing/null keeps legacy behavior, strict non-null options are preview-only and omitted from private
worker materialize payloads. Ordinary materialize/drag/authorize authority is unchanged.

Ordinary selected drag extraction adds optional `ArchiveJobSelection.strip_prefix` / C# `StripPrefix`;
missing/null preserves original paths. A present prefix is a safe forward-slash directory ending `/`,
with native255-UTF-16 component and32767-byte transport bounds, and requires strategy `Here`.
Selection uniqueness and real frame/resource limits remain; no fixed4096-ordinal Extract cap.
See [selected output contract](../../nxtools-archive/backend/reusable-plans.md#scenario-selected-extraction-output-prefix).
