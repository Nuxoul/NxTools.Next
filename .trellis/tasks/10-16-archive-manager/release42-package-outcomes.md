# Formal Release42

2026-10-30. Repackaged current source through original `package-local.ps1 -SkipVerification`, reusing the documented focused validation in [runtime feedback](runtime-feedback-2026-10-30.md). No full Rust/C# suite rerun, installation or product launch. Actual Release compilation, original signing identity and normal Inno installer completed.

## Artifact and verification

- Version0.7.0 / sequence42 / releaseID`3142864b-37e5-4d0d-961f-05af0ca8f66b`.
- Normal installer:`artifacts/installer/NxTools.Next-setup-0.7.0.exe`,26,789,268bytes /25.55MiB.
- SHA256:`E83AB46D6764A07BD5BAF076F6C64F0220DF71EC151F2C4C8582C69A32C9A263`; adjacent checksum matches.
- Original RSA3072-PSS/SHA256 independently verified with public trust anchor;77manifest members and complete79-file set match.10actual build outputs,5embedded archive/helper approvals and helper Desktop binding match.
- Manifest SHA256:`6d9abd779f9b849bbfd31d7e274ba0668ff6a50bed6e7ae6846fd41f599e4002`.
- Installer runtime11 and BrowserHost replacement5scenarios passed. Final packaging log contains no full Rust/C# test summaries. Source baseline unchanged throughout build.
- Installer helper remains`4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`, identical to the prior [running-upgrade repair](installer41-running-upgrade-outcomes.md).
- Windows Authenticode remainsNotSigned; distinct from the verified internal release signature.

Persistent evidence:`.diagnostics/packaging/20261030-225937-058011cf/`, stateCompleted/exit0. Independent evidence:`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\repackage-20261030-225935/package-verification.json` retains current source identities, exact signature/file/binding checks and verification-reuse disclosure.

## Acceptance boundary

Includes the current Quick Launch/Clipboard/archive runtime feedback source, not merely the earlier41source. Existing documented targeted behavior/lint results were reused, not independently rerun or relabeled as a new acceptance round. Original package41 backups/logs remain retained. No new installer acceptance on the real installed environment was performed; earlier41running-upgrade success is historical evidence only. Physical Explorer/GUI/DPI/user acceptance and service-query latency remain as described by the owning runtime-feedback record. No HZ5, key replacement, ledger rollback, unrelated process termination or commit/push.
