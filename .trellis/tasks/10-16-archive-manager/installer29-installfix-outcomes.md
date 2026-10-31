# Installer29 protected-preparation compatibility revision

2026-10-26 user requested a usable installer after code1 preparation failure. Original failure is
NOT root-caused: normal authorized diagnostic exited5, elevated identical preparation exited0;
12old installed files were subsequently restored through original Undo, hashes equal, rolled_back
journal retained. No new installation executed by this revision.

Installer-only change: protected indexer service presence selects original helper runas before first
preparation attempt. Presence grants no mutation authority: original exact path/hash/old-journal
checks and full compensation unchanged. Entire installer remains lowest. New installs without a
service retain normal path and existing5fallback. SetupLogging enabled. No generic code1 retry or
success override, no key/sequence change. Original close-owned-processes helper byte-identical.

Red protected-success fixture returned helpercode1/installerexit7 before branch change. After change
12actual Inno callback scenarios pass including protected-success/cancel/failure; UAC/discovery
injected, no real service or UAC in fixtures. Original BrowserHost5 pass. Preparation100assertions,
owned-set632assertions, exact-file test and source gate pass:21commands exit0. This synthetic
compatibility test does not identify the original code1 cause or certify complete live installation.

Reused signed complete sequence29payload, public anchor/RSA-PSS signature and77hashes verified
before/after Inno compilation. NoCargo/keyuse/sequence reservation; releaseID unchanged.
New explicit installer artifacts/installer/NxTools.Next-setup-0.7.0-installfix.exe; default name
updated to same bytes.25917817bytes, SHA256
B8E2C6C83909D6EA66EC30DD5BF22FF96256CBE44924A7689FE74FDAD0018172.
AuthenticodeNotSigned. Old setup15B78331... preserved in persistent evidence:
.diagnostics/packaging/installer-repair29-20261026-224002/verification.json and previous-setup-sequence29.exe.
Raw test evidence: current session scratch installer-repair-package29. No UI/fullphase acceptance.
