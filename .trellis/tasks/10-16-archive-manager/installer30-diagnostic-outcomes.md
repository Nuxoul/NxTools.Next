# Installer30 recurring code1 — diagnostic delivery, root cause OPEN

2026-10-27 user reported the same Preparing to Install code1 from sequence30. This supersedes any implication that installer29 UAC compatibility resolved the actual failure. No original root-cause fix is claimed.

## Actual observations and authorized recovery

- Current Setup log08:21:37–46 confirms new default package, `protected_elevation_requested`, then helpercode1; extracted helper SHA8533951E...2CD1 matches original source. No helper error details were persisted.
- Prior actual29 installfix log22:44:19–32 shows preparation and trusted update success. This is newly observed user-side evidence, not assistant installation.
- Copy-only replay of actual12old members + new30sidecars passes in x64/x86 PowerShell; readonly target discovery11/native initialization passes. These are not the actual failing operation.
- User explicitly authorized one elevated logged original preparation and immediate original rollback on success. Fixed input hashes/baseline, no new program replacement. Preparation returned0/count0/12suspended: original code1 NOT reproduced.
- Diagnostic wrapper then failed restore-preflight because dot-sourced helper parameter `StatePath` cleared caller's case-insensitive same-name variable. Error and suspended journal retained. Corrected restore-only script assigns distinct caller names AFTER library load, guards exact operation/journal/helper/12backup hashes/no destination overwrite, runs original Undo.
- Restore exit0:12old hashes match, phase rolled_back/failure_count0/pendingfalse, no backups; journal retained. No new installation/product/service start.

Persistent actual evidence: `.diagnostics/installer/preparation30-20261027-085504/` (original Setup log, input hashes, preparation-result/stdout, original wrapper failure, journal snapshots and restore-result). Scratch copy-only fixtures retained separately. No repeated actual preparation.

## Explicitly approved scope

User then approved diagnostics-only installer reseal, no program rebuild or installation. `installer/close-owned-processes.ps1` accepts optional DiagnosticPath; bounded JSONL carries original correlation/stage/event/elapsed/bitness and failure exception type/line/HRESULT/native code. No exception messages, paths/member contents, child output or secrets. Sink failure cannot change original exit or recovery. Inno retains normal/runas/security/transaction conditions and copies launch summary/availableJSONL/Setup snapshot to per-correlation user logs, displaying directory rather than blind retry advice. Original validation/compensation code unchanged.

Red-capable test `tests/architecture/installer-preparation-diagnostics.ps1`: original actual helper exit1/missing persistent diagnostics -> red; new diagnostics ->5cases pass (pending/private-error/native5/unwritable-sink/success). Only process/service discovery is isolated; actual original preparation/exit/catch owner runs. Inno runtime12 scenarios assert persistent summary+JSONL when helper starts and summary when elevation cancelled; paths isolated to scratch. Original BrowserHost replacement5 pass; preparation recovery/owned-set632/exact-file/source gate pass. Total22verificationcommands exit0. Initial Inno GetLogFileName compile failure corrected to `{log}`; malformed Edit insertion removed before tests/packaging, no delivered malformed source. Failures retained, not original-code1 reproductions.

## Delivery

- `artifacts/installer/NxTools.Next-setup-0.7.0-diagnostic.exe`,25,929,548bytes.
- SHA256 `92878E30A9C915FD4A08C26FED4EAFC78FDC8CEDBAD91F421924380EB4E9A540`.
- Same sequence30/releaseID6f9c2aea-f033-444e-99b7-ec912e5ff0a4; existing signed payload77files exactly unchanged, RSA3072-PSS verified before/after. No private key/new sequence/Cargo. AuthenticodeNotSigned.
- Original default package preserved (still rejected by actual user); oldinstallfix remains historical29. Current diagnostic guide [DIAGNOSTIC.md](../../../artifacts/installer/DIAGNOSTIC.md).
- Reseal evidence `.diagnostics/packaging/installer30-diagnostic-20261027-091127/verification.json`, source hashes/compile logs/previous-installer.

Actual recurring code1 root cause and complete real diagnostic-installer run remain OPEN. No visual acceptance, safety bypass, journal deletion, native state reset or unrelated product changes.
