# Download Open With icon and recurrent Explorer Reveal repair

## Outcome and authorization
User asks for Open With as an icon-only button immediately to the right of Open File, and reports both completed-window and main-list Reveal errors despite Explorer opening. Source changes are implemented. No package/install/service/formal product launch, live Explorer/clipboard/history/config operation, delegation/contact, commit/push/archive. Current installed executable was not replaced. Preserve concurrent Archive/editor/browser/release work.

## Reference and boundary
ABDM first: CompletedDownloadPage.kt Actions and WindowsFileUtils.kt open/reveal/native/fallback. Legacy second: DownloadCompletedWindow constructor/Open/Folder actions, MainWindowViewModel OpenTaskFolder/OpenPathInExplorer, WindowAnimationService Attach/Closing and MainWindow ShowIndependentSecondaryWindow -> Show. Avalonia native Window implementation is external/unavailable; no invented lower show/close evidence.

Both Next entry points feed original artifact_actions OpenShellTarget SelectInExplorer -> detached C# ShellTarget -> ExplorerShellActivation -> active visible view/Folder2/selected paths -> correlated result. Rust already closes only matching current task-window HWND+generation on success; main-list actions carry no window identity. Native failure therefore keeps the completion window available, explaining the logical relation between error and non-close without proving the exact installed root cause.

## Actual red evidence
1. Existing audited `Explorer reveal feedback fixture native COM exact vtable contracts` extended with owned successful observation. Real HWND/QueryService/QueryActiveShellView/GetItemObject/Folder2 chain plus injected synthetic automation payload succeeds for file selection, but directory request returns empty selected paths. LIST1/RUN1/PASS0/FAIL1. Production matcher requires selected directory in its parent, while reader returned early for every directory.
2. `Explorer reveal feedback fixture active selection rejects tab root ambiguity` extended with equivalent normal DOS versus extended DOS path. LIST1/RUN1/PASS0/FAIL1: same selected file is rejected when requested as an extended path.
3. GPUI first attempt failed at missing debug selector, not actual layout proof. Added observation-only selector, then transient peer IPC compile failure prevented execution (serde_json unresolved in archive_virtual_drag.rs; no peer edit). After its owner changed that source, actual completed-window geometry red: **60x24DIP versus required24x24**. Current chooser was a text button before OpenFile.

Logs are in session scratch with download-reveal-red-* and download-openwith-red-layout* prefixes. These reproduce causal source defects, not the user's exact installed existing-file action. Ordinary-path, tab/provider readiness and live deployment remain separate evidence.

## Minimal changes
- task_window.rs: existing official Small outlined icon_action with MaterialIcon::ExpandMore after OpenFile;24x24DIP/no visible text. Keep Tooltip and accessibility name `选择应用打开下载文件`, one official pointer/keyboard click owner, original deferred command/task-window identity. DialogFooter has no Toolbar.child ghost adaptation;440x184 client unchanged.
- ExplorerShellActivation.cs: native reader always reads selected items for files and directories; remove obsolete isDirectory constructor switch. Normalize only explicit extended DOS-drive and extended UNC aliases for requested path, observed folder and selected paths. Device/volume namespaces are not arbitrarily stripped; no basename/folder-only/foreground-only acceptance.
- Existing tests.rs regression observes actual size/order/accessibility and clicks new chooser button, then verifies cancellation/no error and late-result isolation. Existing ExplorerObservationTests/ExplorerRevealFeedbackTests extended, no new catalog/TestEffects symbols. Reviewed call graph remains owned hidden noactivate HWNDs/in-memory COM/synthetic snapshots and GUID-owned temporary targets only. No real Shell enumeration, launch, activation or network-share access.
- Original8attempts/75ms/onefallback, exact selected target/activeview/hidden-wrongroot guards, detachedSTA/COM cleanup, foreground-independent target success and payload-free diagnostics remain. No timeout inflation or unconditional completion-window close.

## Verification actually executed
All Rust commands jobs1/incremental0/codegen-units256; busy preflight75 launched no build. No process killed/contacted.

| Check | Result |
| --- | --- |
| C# final test-project build --no-restore -m:1 |PASS,0warnings/0errors |
| Exact audited Explorer feedback6 + compatibility3 + OpenWith1 |LIST10/RUN10/PASS10/SKIP0/FAIL0 |
| Native reader successful file+directory controls |PASS inside exact COM fixture, original HRESULT/refcount/hidden/wrongroot checks retained |
| DOS/UNC alias and wrong filename/server controls |PASS inside exact matcher fixture; pure strings for UNC, no network IO |
| Actual completed-window skeleton/geometry/buttonclick/result test |1/1PASS |
| Desktop downloads:: module |153/153PASS,0ignored |
| Completed main-list terminal context menu |1/1PASS |
| Strict Desktop Clippy --tests -D warnings incl dependencies |PASS |
| cargo check --workspace --tests |PASS |
| One final full Desktop |**1042/1042PASS**,0failed/0ignored,125.69s |
| Relevant source/provenance/lifecycle gates |**9/10PASS**; existing QuickLaunch gate remainsFAIL |
| Scoped rustfmt2files/git diff --check |PASS |

Final native10 includes exact selected-target success with denied foreground, missing/wrong/destroyed targets, delayed selection/retry, COM success/failure, native fallback/readback/PIDL lifetime and chooser cancellation/failure. Actual GPUI skeleton test asserts confirmed Reveal removes both real window and keyed registration; failure/timeout/disconnect/unrelated/late/stale results retain appropriate window; main-list success cannot close replacement; new chooser actual click sends OpenWith and cancel keeps popup/no failure notice. Main-list menu regression confirms original terminal entries and dismissed hit-target isolation. This is not full native live Explorer acceptance.

Static failure is the pre-existing quick-launch-lifecycle.ps1 broad elevated-navigation condition at1765, including the known1762 literal requiring write-release-binary-manifest.ps1 in scripts/build.ps1 (current script delegates publish-release-hosts.ps1). No release scripts/gate edits or full-condition root-cause enumeration. Other9gates actually ran, not skipped after failure. Earlier fullDesktop1040PASS/1ArchiveFAIL remains historical; this fresh1042PASS run is separate evidence after concurrent source changes, not an Archive fix by this slice.

Scratch final logs: download-openwith-reveal-{skeleton,downloads,clippy,workspace,context-menu,desktop}-final.log; csharp-final-build.log/csharp-final.log under same prefix; static-results.json and per-gate logs. Baseline download-openwith-reveal-baseline/ retained. Baseline SHA256 proves artifact_actions.rs and window_support.rs unchanged; direct production diff review shows only the two stated owners changed.

## Remaining acceptance
No visual screenshot/user acceptance, physical keyboard/IME/mixedDPI/reader or installed Explorer action was performed. File-path alias and directory-selection bugs are repaired and regression-protected, but the exact installed recurrence is not blanket-proven resolved without deployment and real action evidence. Do not call launch acceptance success or hide failures to make the completion window disappear. Existing completion close policy is retained and compiled verified. Separate authorization is required for packaging/deployment/live interaction.
