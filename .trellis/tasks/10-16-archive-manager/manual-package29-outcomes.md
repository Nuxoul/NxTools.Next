# User-requested current-source package — sequence29

2026-10-26. User explicitly requested packaging. Existing trusted Release packaging entry completed
with exit0, sequence29, display0.7.0. No business-source repairs or warning suppression, installation,
product launch or installed/protected-state observation. This package is not overall UI/A–F/first-release
acceptance; [current archive checkpoint](phase-a-checkpoint.md) remains the acceptance-state owner.

## Artifact

- `artifacts/installer/NxTools.Next-setup-0.7.0.exe`
- 25,917,756bytes /24.72MiB; SHA256 `15B78331B2E778C72AF4CFD07C3EC02C0FD26C8865AA32C0EA5FFF7F3A3DAA93`.
- ReleaseID `3df1a530-7ca9-4c94-a80c-b1e2c74ed1ce`.
- ManifestSHA256 `c50aae175c190858089e38c435a508732332bf39cfb7480a0e787cf2a8785fe4`.
- Authenticode NotSigned; internal RSA3072-PSS/SHA256 signature independently verified against
  existing public anchor03e979d5c6babda29aa217b7ce72268aab12870bda46a9072397fe52ba083b69.

## Execution / verification

Original package.ps1 Release -SkipVerification with existing publisher key and sequence29. Ledger28
and package28 were checked before reservation; no key creation/rotation, counter reuse or reset.
Original committed-memory preflight13.66GiB, wrapper pre-Cargo11.39GiB (default minimum6GiB).
Single Cargo lane, CARGO_BUILD_JOBS1/CARGO_INCREMENTAL0/RUSTFLAGS codegen-units256 inherited.
Old package/installer28 backed up before replacement; no unrelated process/service termination.
Only original package wrapper's exact-path process/service handling is authorized.

Independent offline verification:77manifest files, exact package member set,10build outputs,
5archive approvals, public anchor in packaged host and helper/Desktop binding all match.
Original installer runtime9scenarios and BrowserHost replacement5scenarios pass, including expected
negative exits. Same Inno resource set for migration/normal install, no duplicate migration ZIP.
1938recorded source inputs unchanged across build. Two Desktop warnings remain: SPACING_12 unused
import in main_window.rs and animated unused field in floating_assistant/media.rs. No full test/
strict-lint/codec/CLI/resource/performance rerun; existing targeted verification reused.

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\package-user29-20261026-195214`.
package-status.json records start19:53:09/completion20:04:26+08:00/exit0. package-verification.json,
raw stdout/stderr, source baseline/drift and old artifacts retained. INSTALL.md/evidence.json/setup
SHA256 updated for29, not historical28 reports. No install, runtime or visual acceptance claim.
