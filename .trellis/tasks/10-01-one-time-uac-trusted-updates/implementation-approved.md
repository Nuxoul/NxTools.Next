# Approved implementation: trusted protected updates

## Approval and boundary

The user approved implementation after the Listary/SwiftList/WinShun comparison: "那你就开始，根据你得到的结论来". The requested outcome is one initial trust-enrollment authorization, then no repeated UAC for correctly signed newer releases. Moving UAC to installer time alone is not completion.

Smallest gap: `IndexerServiceInstaller.EnsureCoreAsync` routes every healthy mismatching tuple to the elevated installer. There is no authenticated resident writer, signed package, durable anti-replay state or rollback journal.

## Ownership

- C# `TrustedReleaseManifest`: bounded signed complete-release validation, stable source handles, verified staging.
- C# `TrustedUpdateBroker`: independent LocalSystem service role hosted by a fixed protected bootstrap copy of PlatformHost, separate from the replaceable Indexer/Desktop/PlatformHost tuple. It serializes bounded named-pipe requests, verifies pinned release trust, stages, backs up and invokes the existing native deployment entry without `runas`.
- C# `TrustedUpdateTransaction`: durable prepared/committed recovery around the existing deployment entry; last-good signed package and sequence retained until success. Correlation/start/state/terminal/elapsed/native events contain no package paths or payloads.
- Existing `IndexerServiceInstaller`: remains the single SCM/indexer deployment owner, plus explicit bootstrap enrollment and current-package metadata copy.
- Publishing scripts: explicit release signing private key outside repository/artifacts, public SPKI embedded before publish, detached signature after final package assembly, strictly increasing signed release sequence independent of `0.7.0` display version. No automatic production key creation. A separate explicit key-creation tool is permitted, not executed on a production key in this task.
- Installer: includes signed metadata and performs update preparation before ordinary original-user launch, retains owned cleanup/rollback tests.

Rust retains domain state and typed existing EnsureIndexerService/CommandResult flow; no MFT/search changes, weaker pipe peer approval, UI redesign or global privilege change.

## Execution / safety

Start with red real EnsureCoreAsync regression for enrolled upgrade without elevated InstallAsync, and fail-closed signed update errors. Add real signed temporary-file verifier/transaction failure-recovery tests. Existing same-release, cancellation and upgrade lifecycle tests remain.

No live service install/start/stop/reconfiguration, registry/ACL/shortcut mutation, formal installer execution, production signing key access or package overwrite is authorized by this source implementation task. Temporary keys/files and owned pipe fixtures are allowed. No commit/push/reset. Full privileged deployment and broker self-recovery acceptance requires a disposable VM or separate native authorization.

Stable bootstrap is not replaced by ordinary release updates; a future bootstrap security upgrade/key rotation requires explicit maintenance authorization. Do not imply that the replaceable indexer can stop itself and continue the update. Unconfigured builds retain legacy installation only while no broker exists; an enrolled broker never accepts unsigned updates or silently falls back to UAC after authentication rejection.
