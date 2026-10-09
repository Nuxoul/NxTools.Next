# Download Dialog / Explorer Reveal Reference Scope

## Required inspection order before implementation

1. Read `docs/reference-code/download.md` for the behavior inventory and recorded call chains; it is 190+ KB, so do not rely on Trellis context injection to include its tail.
2. Read the AB Download Manager Desktop references first (visual/interaction reference only):
   - `D:\Code\ab-download-manager-master\desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\pages\addDownload\single\AddDownloadPage.kt`
   - `...\singleDownloadPage\ProgressDownloadPage.kt`
   - `...\singleDownloadPage\CompletedDownloadPage.kt`
   - `...\singleDownloadPage\ShowDownloadDialogs.kt`
   - `D:\Code\ab-download-manager-master\shared\app\src\commonMain\kotlin\com\abdownloadmanager\shared\singledownloadpage\BaseSingleDownloadComponent.kt`
3. Then re-read old NxTools product-behavior sources:
   - `D:\Code\NxTools\Views\AddDownloadWindow.axaml`
   - `D:\Code\NxTools\Views\DownloadProgressWindow.axaml` and `.axaml.cs`
   - `D:\Code\NxTools\Views\DownloadCompletedWindow.axaml` and `.axaml.cs`
   - `D:\Code\NxTools\ViewModels\MainWindowViewModel.cs::OpenTaskFolder/OpenPathInExplorer`
   - `D:\Code\NxTools\Services\WindowActivationService.cs`
4. Only then inspect current NxTools.Next and refresh this scope with any newly verified chain/invariant before coding.

## Verified compatibility facts

- AB Download Manager keeps one task-keyed dialog, aligns the add-form URL/configuration/metadata/actions, separates progress details from its action/progress region, and gives completion a compact file-identity body plus divided open/folder/drag/close footer. Do not copy Compose code or transplant its tabbed page wholesale.
- Legacy NxTools preserves one progress window per task, transforms it in-place to completion, supports open/reveal/drag/close, and treats closing a progress window as presentation-only. Its completed-folder behavior selects an existing final file, opens an existing directory, and opens the task save directory if the final file is missing.
- Current GPUI schedule production regression: `production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle`; form footer regression: `confirmed_new_download_schedule_and_actions_stay_inside_the_footer`; keyed task-window regression: `production_task_window_reuses_one_skeleton_and_generation_across_phases`. The existing schedule test does not compare the actual Clear/Confirm hit-target baseline, inset, gap or overlap; the form footer regression is not evidence for schedule-popup alignment.
- Current completed Reveal chain is `DownloadTaskWindowView -> DownloadApplication::open_task_artifact_from_task_window -> OpenShellTarget(SelectInExplorer) -> PlatformHost OpenShellTargetAsync -> ShellTarget.Launch -> ExplorerShellActivation.LaunchAndActivate -> correlated result/identity-guarded close`. Add red tests before altering it.
- Verified current C# gaps: a matching existing Explorer folder HWND may return before `/select,<file>`, leaving a file unselected; when no matching directory HWND exists, the launch path uses `/select,<directory>` rather than directly opening that directory; Rust currently errors if the completed artifact path is missing instead of preserving the legacy save-directory fallback. Implement each target-kind path distinctly and only report reveal success after foreground-root readback.
- Screenshot file paths were provided but the browser guest could not be used to display them; actual screenshot and DPI visual comparison remains a manual acceptance gate.

## Implementation verification — 2026-10-03

- Re-read the recorded AB Download Manager references first, then the legacy NxTools files listed above, and then traced the current GPUI -> Rust -> typed `open_shell_target` -> PlatformHost STA -> Shell HWND activation result path before editing.
- The production Popover now keeps time selectors and Clear/Confirm in separate rows. The test measures the actual hit targets for shared action height/baseline, insets, non-overlap and shell containment; the existing schedule draft/commit/clear/dismiss cases remain green.
- The C# activation seam is injectable and has focused noninteractive coverage for existing-folder file selection, directory open/reuse, delayed Shell HWND reuse, minimized restore, a foreground child owned by the requested root, destroyed target, denied activation and unrelated foreground. It reports success only on a still-valid intended Explorer root foreground readback.
- Rust Reveal preserves file-vs-directory semantics and falls back only from a missing final file to its saved directory. Pending/repeated actions, timeout/disconnect, late success and generation-safe close are covered by the production task-window regression; failure remains retryable.
 - Automated verification passed: desktop suite 428/428, formatter, download lifecycle gate, five PlatformHost Explorer activation cases plus the Shell-responsiveness case, and PlatformHost Debug build.
- Native Windows visual/DPI review and interactive Explorer foreground/file-selection smoke remain pending; the screenshots were not previewed here.
