# Archive diagnostics and privacy

Evidence: `repo:rust/crates/nxtools-archive/src/control.rs#OperationControl`, `repo:rust/crates/nxtools-archive/src/control.rs#ArchiveProgress`, `repo:rust/crates/nxtools-archive/src/error.rs`.

OperationControl carries the caller operation UUID and fixed-count/size progress only; no paths/names/comments/passwords in its values. Intermediate progress uses one snapshot at <=10Hz; exact terminal snapshot is forced independently. Worker private frames carry metadata/secrets only through bounded inherited stdio and are not diagnostic events. Never log chunk streams or raw payloads.

Full accepted/start/state/terminal/elapsed/native correlation events are still Phase A/application integration work, not implemented merely because this UUID exists. Future desktop and native owners must carry one operation correlation through child command IDs and test synthetic log sentinels. No current library logging is a claim of complete DIAG01. Source paths, raw names/comments and snapshots intentionally have no derived Debug; local UI metadata is not log-safe payload.

The bounded headless product-read slice now emits Rust archive.read.lifecycle accepted/
negotiated/terminal and C# archive.read accepted/started/negotiated/terminal under the original
operation correlation. Only fixed category/count/elapsed/native-code/cleanup fields are safe;
ArchiveRead DTO Debug is NOT log-safe. No batch/frame/tick logs. C# throwing-sink fixtures and
real product-chain source-native error mapping protect result/diagnostic consistency. This does
not complete application-wide DIAG01 or UI/jobs; see [read lifecycle](product-read.md).

Private worker Cancelled terminals may additionally retain one `cancellation` object: first
valid original-input `received_us`, first work-thread `observed_us`, fixed boundary enum
checkpoint/paused_checkpoint/native_reply/metadata_credit. Both elapsed values share the original
OperationControl monotonic origin; missing means unmeasured, never synthesized from send/ACK.
No chunk log, public pause command, cleanup claim or cross-process clock subtraction. Ordinary
success frames omit this object. Evidence: `repo:rust/crates/nxtools-archive/tests/worker_cancel_contract.rs`,
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveCancellationBoundaryTests.cs`; bounded contract:
`repo:.trellis/tasks/10-16-archive-manager/worker-cancel-contract.md`.
