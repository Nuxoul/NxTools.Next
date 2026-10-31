# Phase F — Explorer integration and genuine native drag: P206/P207 formal source outcomes

Date: 2026-10-22. Authority: [plan amendment](implement.md#approved-phase-f-p206p207-formal-source-amendment--2026-10-22),
recorded together with the [single checkpoint](phase-a-checkpoint.md) BEFORE code. Phase names unchanged.
**Not P206 live, not P207 live, not P208, not Phase F, not first-release acceptance.**
P208 controlled spike and earlier archive matrices were not rerun (not touched by this round).

## Delivered source

| Requirement | Source | Boundary kept |
| --- | --- | --- |
| P206 service, explicit Apply/Revert/Status | `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveIntegrationService.cs` | Reuses `ArchiveIntegrationPlan` + `ArchiveIntegrationTransaction` unchanged; one gate `ProductApplyAuthorized=false` returns `disabled/apply_not_authorized` before any capture/write. |
| Observed applied facts | same file, `FileArchiveIntegrationLedger` | Atomic bounded JSON in local app data; persisted BEFORE `applied`; ledger failure compensates writes; existing facts ⇒ `revert_required`; partial revert keeps ledger. |
| Real Registry adapter | `Windows/Archive/WindowsArchiveIntegrationStore.cs` | HKCU `Software\Classes\` only, target inside owned root, no `UserChoice`/HKLM; exact value restore/delete; only captured-missing owned keys removed when empty; no `DeleteSubKeyTree`; SHChangeNotify advisory. Never constructed by tests. |
| Wire | `rust/crates/nxtools-protocol/src/archive_shell_integration.rs`; C# parser/pipe/adapter | `archive_shell_integration` command carries action/revision/flags/extensions only (no path authority, `deny_unknown_fields`); result `validate()` refuses `applied` without revision. |
| Desired intent | `rust/crates/nxtools-core/src/archive.rs` `ArchiveShellIntegrationSettings` | Default off; serde default only for absent legacy field; extension allowlist/uniqueness; monotonic `desired_revision`. |
| Rust call chain | `rust/apps/nxtools-desktop/src/archive/shell_integration.rs` | Existing `ArchiveApplication` owner, `send_command_with_id`, bounded 4 pending, reply deferred to a later turn, Disconnected ⇒ not delivered; invalid intent never sent. |
| 集成 settings page | `rust/apps/nxtools-desktop/src/main_window/archive_page.rs` | Official Settings compound; Switch (cascade/OpenWith), Checkbox (extensions), outlined Button (应用/撤销/刷新状态, loading/disabled while pending). Saving bumps desired revision only; observed text comes only from native replies. No page redesign. |
| P207 helper | new `csharp/NxTools.ShellSelectionHelper` (WinExe, x64, net8.0-windows) | `ArchiveSelectionCommand : IExecuteCommand, IObjectWithSelection` (SDK vtable order); once-only Execute; whole-selection reject; bounds = Desktop intent (256 paths/32KiB/256KiB); Open/Test exactly one. |
| P207 forwarding | `ShellSelectionHost.cs` `DesktopIntentForwarder` | Exact sibling `NxTools.exe` + release SHA-256 (empty ⇒ refuse) → one launch with existing `--archive-*` argv → existing parser/single-instance/peer-authenticated pipe/32 queue/request ID. No timer/coalescing/new IPC; never kills a launched desktop. |
| P207 lifetime | `LocalServer`, `ServerLifetime`, `ArchiveSelectionClassFactory` | `-Embedding` only; STA, suspended registration + resume, pump until objects+locks zero, 30s idle/2min hard bound, ≤8 live objects, shutdown cancels forwarding. Diagnostics path-free. |
| Future registration plan | `Windows/Archive/ArchiveSelectionHandlerPlan.cs` | Pure HKCU `CLSID\{..}\LocalServer32` + cascade `DelegateExecute` (no `%1`); CLSIDs cross-checked with helper; unreachable from product apply. |
| Source gate | `tests/architecture/archive-phase-f-shell.py` | Gate off, no tree delete, tests never use real store/COM registration/process, helper absent from package/build/release manifest. |

Helper is NOT referenced by `scripts/package.ps1`/`build.ps1`, not signed, not registered, and not in
`ReleaseBinaryManifest`/protected redirect/atomic tuple. `ExpectedDesktopSha256` is empty, so even a
manual activation refuses delivery. Next authorization needed: helper packaging/signing + manifest hash,
release-tuple decision, real HKCU apply/revert, CLSID/DelegateExecute registration, live Explorer tests.

## Verification (evidence root, per-command raw logs + `commands.log`)

`C:\Users\Administrator\.pi-desktop\scratch\46a534b7-c8c9-4656-a9de-76e0c8baac60\phase-f-p206-p207-20261023-025118`.
Every Cargo command used jobs1/incremental0/codegen-units256 after checking no cargo/rustc was running.

| Check | Result |
| --- | --- |
| C# build (Tests + PlatformHost + helper) | 0 warnings / 0 errors (final step 38) |
| Exact C# selection `archive Phase F integration` + `P206` + `P207` | **LIST13 = RUN13 = PASS13**, skip0/fail0/not_run0, names equal (39/40) |
| Composition | 5 retained pure-plan + 4 P206 service/ledger/wire/target + 4 P207 COM/selection/identity-lifetime/future-plan |
| Red capability (mutations, then hash-equal restore) | once-guard removed ⇒ P207 order FAIL; silent drop ⇒ partial-failure FAIL; success-before-ledger ⇒ P206 failures FAIL (30–32) |
| Rust Protocol lib | 69 PASS (incl. 2 new) |
| Rust Core lib | 76 PASS (incl. new desired-intent test; invalid-section test extended) |
| Desktop targeted | shell_integration 1, archive_page_tests 6 (incl. new integration page), archive_launch 8 — all PASS |
| Strict Clippy `-p nxtools-desktop -p nxtools-core -p nxtools-protocol --tests -D warnings` | PASS |
| `cargo check --workspace --tests` | PASS (compile only) |
| `cargo fmt --all -- --check` | PASS (only touched files were rustfmt'ed) |
| **ONE full Desktop** `--test-threads=1` | **1022 PASS / 0 FAIL**, 0 ignored/filtered, 123.15s test time |
| Source drift during full run | 62 dirty rust/csharp files hashed before; 0 changed/new after |
| Gates | archive-phase-f-shell, archive-phase-f-spike, gpui-release, markdown-editor, component-catalog, platformhost-test-selection, spec-contracts PASS |

Honest process notes: first C# RUN (04) failed on a test-harness misuse (`GetObjectForIUnknown` on a
managed CCW), fixed in the test to use exact vtable slots. RUN 37 used a stale mutant binary because
`Copy-Item` restored files with old timestamps; sources were touched/rebuilt (38) and the final 13/13 is
from 40. Clippy 19 found one test-only `field_reassign_with_default`; fixed and rerun. Core run 09 failed
because the pre-existing "every present field required" test needed the new additive field exemption plus
explicit invalid cases. A first background full-run launcher never started (no process/log); the single
full Desktop run was then executed in the foreground. The historical 1005/7 independent observation is
not rewritten; the current concurrent workspace snapshot simply passed this run.

## Not executed / remaining

- Real HKCU/HKLM writes or deletes, product Apply/Revert: **NOT RUN** (gated off; tests use fakes).
- CLSID/DelegateExecute/LocalServer32 registration, `CoRegisterClassObject` in a real server, Explorer
  invocation, Windows 10/11 menus/caching/restart, packaging/signing/install/deploy: **NOT RUN**.
- Live GUI screenshots/pointer/keyboard/DPI/UIA/user acceptance of the 集成 page: **NOT RUN** (self-app
  launch denial retained; compiled production-view test only).
- P208 real pressed-HWND/physical gesture/foreign target remains unaccepted; production archive-row drag
  remains DISCONNECTED. P209, source recycling, JOB07/SAFE08 and earlier A–E native/user debts unchanged.
- No optional review was started. No commit or push.
