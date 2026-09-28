# Technical Design

## Behavior Gap

The source-to-dialog state machine currently treats the current foreground HWND as both the retry target and the observation context. That is valid for initial foreground recognition, but not after a DOpus lister has been captured as the pending target and the user returns to the file dialog before the lister's child controls settle.

## Boundaries

- PlatformHost/C# owns Win32 window inspection, bounded recognition retries, diagnostics, and the source HWND/pane context.
- `AutoQuickSwitchTracker` remains the owner of source-dirty and known-dialog consumption semantics.
- Rust continues to receive the existing `QuickLaunchAutoSwitchRequested` event and emits the existing `NavigateDirectory` command. No protocol field changes are needed.
- DOpus pane selection and path canonicalization remain in `DirectoryOpusFileManagerOperations`.

## Data Flow

```text
DOpus foreground event
  -> capture attempt
  -> pending source HWND + bounded retry state
  -> retry captures the pending HWND even if file dialog is foreground
  -> ObserveDirectorySource
  -> known file-dialog foreground
  -> ObserveFileDialog
  -> existing auto-switch event and navigation command
```

The retry must inspect a specific pending HWND, not replace that target with the current foreground HWND. The operation is passive: it reads the lister and pane state only. It must not foreground, focus, activate, navigate, or send IPC during recognition.

## Retry Contract

1. A pending attempt has one target HWND and one bounded attempt budget.
2. Foreground changes do not cancel the pending source attempt solely because the foreground is now the file dialog.
3. A retry captures a context for the pending target HWND through the existing foreground-context provider, then invokes the existing local directory adapter.
4. Success publishes one source and completes the retry. Fatal capture failure, destroyed target, supersession, or budget exhaustion completes without publishing.
5. The file dialog remains the only consumer that turns a dirty source into an auto-switch candidate.

If the existing retry helper is reused, add an explicit background-target operation rather than weakening the semantics of the foreground-gated operation used by file-dialog recognition. Keep the distinction visible in names and tests.

## DOpus Path and Activation Contract

- `ThirdPartyFileManagerDisplayPath` must canonicalize DOpus friendly path segments through both `SHGetFileInfo(SHGFI_DISPLAYNAME)` and the parent of the current user profile; a known-folder identifier that is unavailable on a machine must not make `C:\Users` aliases disappear.
- `DirectoryOpusFileManagerOperations.TryNavigate` owns activation around its `WM_COPYDATA` `Go` command. `ThirdPartyFileManagerSessionRegistry` must not reapply the saved pane/focus after a successful DOpus navigation, because that stale focus can restore the previous directory.
- The real boundary test must assert one valid localized C-drive capture, one nested C-drive navigation, native readback, and restoration.
- `DirectoryNavigationTarget` is the Windows normalization boundary for filesystem paths and `lib://` known-folder identifiers. Rust must not rewrite the identifier before this boundary.
- `PlatformPipeClient` records only bounded target metadata on `navigate_directory` and `query_shell_actions` received/completed events so a future failure can distinguish an upstream truncation from a Shell resolver truncation.

## Diagnostics

Use the existing PlatformHost logger with bounded lifecycle events:

- source recognition accepted: target HWND and attempt budget only;
- source recognition published: target HWND, attempt count, elapsed time;
- source recognition exhausted/failed: target HWND, terminal reason, attempt count, elapsed time, and native error code when available.

Do not log paths, window titles, pane display names, or payload data. Do not log every timer tick.

## Compatibility and Rollback

The change is additive to PlatformHost recognition state and does not alter the wire contract. If the new passive background capture is not valid for a particular DOpus build, the bounded failure path leaves existing behavior intact and the change can be reverted by restoring the retry caller/helper pair without changing Rust state.

## Verification Strategy

- First add a red regression at the retry/foreground coordination seam.
- Run the filtered PlatformHost test to observe the failure before implementation.
- Implement the smallest retry-target fix and run the same test.
- Run all related PlatformHost tests, then the full C# test executable.
- Run repository static checks and, when available, the interactive DOpus/file-dialog soak described in `design-qa.md` and `docs/reference-code/listary-file-dialog-quick-switch.md`.
