# Independent WAC01–03 acceptance before Release32

2026-10-28. **PASS at source / automated GPUI behavior level.** User requested acceptance of wac-password-focus-fix.md then packaging if no blocker. Original window-tabs-icons-drag-acceptance.md remains unchanged as negative historical evidence. No product implementation changed by this acceptance.

## Evidence and source review

Read actual password.rs per-host PasswordRecovery, browser_host.rs activation, entry.rs ordinary terminal retirement and wac_password_tests.rs. Original passwords remains sole ticket owner; deferred recovery validates host/session/current operation, retires only matching focused scopes, waits under newer modal via public focus subscription, clears original masked Input independently and restores current active content after old-dialog cleanup. No timer/retry-budget/vendor/protocol/native authority change. Ordinary entry terminal now calls hide_password_dialog; early host content-focus defer removed.

Original raw RED logs inspected: WAC01 missing prompt after competing Dialog close; WAC02 alive token retained at ordinary terminal; corrected WAC03 exact original B focus assertion. Initial loading-state CtrlF failure is not used as exact WAC03RED. Original green12 and archive253 logs match reported counts. Final tests assert actual original Root/Dialog/input/transport/focus/CtrlF, newer overlay preservation, late zero-dispatch, current generation/cancel/background/disconnect and host replacement. Test clock advances original close animation, not production polling.

## Independently executed this round

Persistent evidence: `.diagnostics/packaging/acceptance32-controller-20261028-082614/verification-commands.json` plus per-command stdout/stderr.

- WAC exact subset:12PASS/0FAIL/0ignored,3.38s.
- Archive + titlebar union:261PASS/0FAIL/0ignored,80.52s (union includes original253archive and8caption tests).
- Desktop strict Clippy --tests -Dwarnings PASS; workspace check --tests PASS.
- Scoped rustfmt for application/browser_host/entry/password/wac_password_tests PASS.
- Reproducible approved-icon resources, spec/catalog/release/editor/Tree and DesktopShell/DownloadUI/UI-state lifecycle source gates PASS.14commands actual exit0.
-1975build-source input hashes unchanged through acceptance; no production code edits. Previous independent fullDesktop1398unit+10integration/Core93/IPC43/Protocol83/C#6 remain prior baseline, not newly rerun after WAC changes.

## Verdict and limits

WAC01/02/03 CLOSED for requested source/automated defects, with preserved red→green evidence and independent affected regression. No new confirmed in-scope defect found; packaging authorized by user's conditional request proceeds through original package-local.ps1.

Physical/nativeGUI/OLE/Explorer/DPI/IME/UIA/user visual acceptance and fresh-process preference restore remain NOT RUN. Prior production-only SPACING_12/animated warnings and globalfmt3network-file differences are unchanged independent limits; tests-strict/scopedfmt results do not claim those gates pass. Historical installationerror1 remains OPEN; current installer retains separately documented code5repair and diagnostics, not a claim actual installation succeeds. No installer execution, product startup or system association/registry/service mutation by acceptance. Release package success/signature evidence belongs to manual-package32-outcomes.md after actual completion, not this acceptance.
