# Fix DOpus dialog auto-switch race

## Goal

When a known Windows file dialog is reopened after the user briefly switches to Directory Opus, NxTools must perform the pending Auto Quick Switch exactly once and navigate the dialog to the DOpus directory. A short DOpus UI-settling interval must not make the source disappear merely because the file dialog became foreground first.

## Confirmed Evidence

- `WindowsPlatformAdapter.EventWindow.ProcessForeground` recognizes a directory source before `AutoQuickSwitchTracker.ObserveFileDialog` consumes it.
- `RetryDirectorySourceRecognition` currently gates every retry on the current foreground HWND matching the originally pending HWND.
- The retry runs on the PlatformHost Windows event thread. If the user returns to the file dialog before DOpus recognition succeeds, retries observe the file dialog instead of the pending DOpus lister and no source is published.
- The existing PlatformHost suite covers successful DOpus capture, stale-source clearing, and pane selection, but not the pending-source-to-file-dialog foreground transition.
- The existing reference contract requires one source-to-dialog synchronization after a supported source becomes active, preserves the captured source HWND/pane context, and keeps failed recognition from reusing stale source state.
- Real DOpus verification exposed a second boundary: the lister can expose localized `C:\用户` display segments that are not parsing paths, and a post-navigation activation using the captured pane focus can restore the pre-command directory.
- A second real-flow check found that `lib://Pictures` is rejected as non-filesystem input before DOpus receives it, while nested filesystem targets must remain intact through the Quick Launch action and navigation command.

## Requirements

1. Keep a pending DOpus source recognition attempt associated with its captured lister HWND while the bounded retry budget is active.
2. Allow the bounded retry to inspect that pending lister when the foreground has moved to the file dialog, without activating either window and without changing the navigation protocol.
3. Publish at most one fresh DOpus source for the pending recognition generation and let the existing file-dialog observation consume it once.
4. Preserve current behavior when the source window is destroyed, the pane cannot be captured, the retry budget expires, or a newer source/window supersedes the pending attempt.
5. Add low-noise diagnostics at the recognition boundary for accepted/published/exhausted source recognition, excluding paths, titles, credentials, and other payload content.

## Acceptance Criteria

- [x] Regression test reproduces: known file dialog -> DOpus foreground capture is temporarily unavailable -> file dialog foreground -> source recognition completes against the pending DOpus HWND -> exactly one auto-switch candidate is produced.
- [x] Regression coverage proves supersession and bounded exhaustion do not publish the retired source; the architecture gate also guards the destroy-target branch.
- [x] Existing auto-switch, DOpus pane, file-dialog, and stale-source tests remain green.
- [x] The implementation never calls native activation, `WM_COPYDATA`, or Rust IPC while performing background recognition; navigation still occurs only through the existing candidate/command path.
- [x] Diagnostic events carry the existing bounded PlatformHost logging shape and do not log directory paths or window titles.
- [x] The architecture gate, full PlatformHost suite, localized Users-root regression, and real DOpus C-drive navigation/readback pass; the complete DOpus-to-real-file-dialog soak remains environment-dependent and is not represented as automated coverage.
- [x] Library URI navigation resolves known folders, nested target metadata is preserved, and the target diagnostic fields distinguish a full `D:\Qingfeng` input from a drive-root-only input without logging raw paths.

## Scope

In scope: PlatformHost directory-source recognition retry state, its regression seam, and the associated bounded lifecycle diagnostics.

Out of scope: Rust Quick Launch UI, file-dialog search layout, DOpus `WM_COPYDATA` navigation semantics, other file-manager adapters, and unrelated foreground/focus policy.

## Open Questions

None. The observable behavior and compatibility boundary are defined by the existing reference note and the user's report.
