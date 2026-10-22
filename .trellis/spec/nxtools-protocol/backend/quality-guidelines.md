# Protocol quality

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#tests`, `repo:csharp/NxTools.PlatformHost/Protocol/ProtocolEnvelope.cs`.

Roundtrip real enum/DTO payloads, version rejection, exact frame-size boundary and serde default/unknown-field compatibility. For a changed native command verify C# parser and actual owning callback/result, not only encode equality. Bounds/size units (logical/device pixels, bytes/ms, HWND/PID) are explicit in field contract; no name-only authorization. Keep optional source HWND/native result metadata coherent. `cargo test -p nxtools-protocol --lib` and affected IPC/native tests follow [verification](../../shared/verification.md); no new warning suppression.

IndexSearchContext search extensions retain_kinds_apart/allow_homophone_fallback/recent_modified default false when absent and omit false serialization. Test legacy context and opt-in roundtrip. Caller explicitly permits approximation only after local ordinary sources are empty; recent mode is launcher-only opt-in, not inferred for file-dialog callers. SearchFailed carries generation/fixed reason and preserves request-envelope correlation at the runtime owner. Indexer/desktop own semantics; no C# native DTO changed. Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#search_retention_tests`; executable cross-layer contracts in `.trellis/tasks/10-14-winshun-w2-w5/code-spec.md`.

ArchiveRead command/event definitions live in archive_read.rs and preserve outer protocol1.
Require explicit archive version1 or opt-in2, bounded read-only fields and symmetric defaults/rejects;
unknown capability grants no authority. Test real owner->pipe->C#session->Job->worker, not just
JSON equality; exact lifecycle/budget/error cases are in
[product read](../../nxtools-archive/backend/product-read.md).
Readv2 requires explicit limits; v1defaults4096/4MiB and omitted default transport field stay
compatible. Version precedes budget rejection. Batch bytes/count are independent, v2terminal
stats are optional/omitted for oldv1, required for v2success. Exact budget/negotiation matrix:
`repo:.trellis/tasks/10-16-archive-manager/product-100k-contract.md`.
