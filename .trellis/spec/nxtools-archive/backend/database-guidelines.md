# Archive persistence and file ownership

Evidence: `repo:rust/crates/nxtools-archive/src/model.rs#SourceRevision`, `repo:rust/crates/nxtools-archive/src/engine.rs#revision`, `repo:rust/crates/nxtools-archive/tests/engine_contract.rs`.

No SQL or settings persistence lives here. Current snapshot retains path/size/mtime/streamed SHA256; execution reopens and checks the revision before using ordinals, then checks again before success. Metadata is immutable, entries use session+ordinal rather than names or sorted indices. Current hashing reads full input with a fixed buffer; large-input latency/100k-entry performance is unverified, not a target achievement.

Ordinary extraction uses held-relative final-name `CREATE_NEW`/direct decided overwrite,required codec/source integrity,metadata+drain+close and incomplete-output-only deletion; earlier closed files are not globally rolled back and no ordinary output hash/file-ID fingerprint is retained. Isolated RAR/7z keep required solid/block integrity before close. Archive Create/Edit separately retain verified candidates,target recheck and one atomic path replacement; their committed/unverified/unknown outcomes never use blind retry. C# still owns held no-reparse namespace handles. Source recycle/MOTW/attributes/full journal keep their owning contracts; no source deletion without full eligible success/test/consent/native identity results.

Isolated-worker remediation: schema2 source_acquire is acknowledged only after C# owns
ArchiveSourceLease's no-reparse parent chain/non-share-delete handles and read-only/read-share
source handle. Every fingerprint/7z reader/RAR path reopen therefore resolves the same protected
object until actual worker/pump cleanup. Existing writers/delete handles (including the owned
writable-mapping fixture) fail acquisition; do not replace this with before/after path hashes.
No source snapshot or secret persistence is used. Schema1 ZIP remains compatible; old isolated
codec requests fail UnsupportedOperation explicitly. Evidence:
`repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveRemediationTests.cs#SourceRaceBarriers`.
