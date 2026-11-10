# R7 native picker outcomes

## Ownership and lifetime

Every supported human file/folder/save chooser carries its initiating Desktop HWND and PID through the existing Rust command, IPC parser and C# adapter. The pipe binds the PID to the authenticated peer. C# validates the HWND/process before presentation and again before returning a result; it does not infer an owner from the foreground window on a worker thread.

Human selection has no elapsed router, native or feature deadline. Ordinary machine operations retain bounded deadlines. The existing detached dispatcher and bounded STA retain admission until the actual modal call returns and cleanup completes. Cancellation closes only recognized dialogs on that STA. The modal-thread owner monitor retires selection when its original HWND/process disappears. Same-Desktop visible Topmost windows use scoped suspension leases and restoration; unrelated applications are not restyled or covered by a new global Topmost dialog.

The affected callers include clipboard import/export/storage, Quick Launch directory/program selection, extension manifest/runtime/settings paths, archive selection and temporary storage, download directory/FFmpeg selection, and browser extension export destination selection. Existing operation/session/generation checks remain in their owning features.

## Result isolation and export

Directory and FFmpeg selections belong to the original window, draft and edit generation. Results are checked before scheduling and again at deferred application. Closing/reopening settings or editing a field cannot apply an old manual choice to the replacement draft. Managed FFmpeg installation remains independent of manual picker origin.

Browser extension export has two stages: `PickBrowserExtensionExportPath` waits for a destination, then a successful nonempty choice starts bounded `ExportBrowserExtension`. Pending state is installed before each send. Command IDs are distinct and share one logical operation/correlation. Duplicate, retired or stale selections cannot start export. Cancel quietly dismisses the waiting notice without writing settings or producing an error. Closing settings retires destination selection; an already-started machine export may finish normally.

Cross-entity work is deferred to the next application turn rather than synchronously re-entering the leased draft. Diagnostics carry operation correlation and bounded state transitions without selected paths, filenames or clipboard payloads.

## Verification

| Boundary | Final evidence |
| --- | --- |
| Rust protocol | `cargo test -p nxtools-protocol --lib`: 112 passed |
| Rust IPC/router | `cargo test -p nxtools-ipc --lib`: 61 passed |
| Protocol/IPC quality | Strict scoped Clippy passed |
| C# archive picker ownership/lifetime | Seven OPT07 fixtures passed; rerun in the 16-fixture safe Host group |
| Extension pickers | Five manifest fixtures and one runtime fixture passed |
| Directory/browser ownership | Production Desktop tests passed for owner close/reopen, duplicate/stale results, deferred application and quiet cancellation |
| Human/machine policy | Controlled-clock Desktop tests passed for unlimited browser/FFmpeg human wait while machine operations expire |
| FFmpeg draft origin | Two focused tests passed for edited drafts and close/reopen isolation |
| Two-stage export | Terminal routing/duplicate suppression passed in the Desktop run; final real-button completion/notification test passed focused verification |
| C# compilation | Final Host/tests build: 0 warnings, 0 errors |

The one full Desktop run recorded 1765 passed, 15 failed and 1 ignored. Seven in-scope integration failures subsequently passed focused verification. Seven archive-wire fixture prerequisites and one unrelated context-menu failure remain in that recorded run; it is not a green workspace-suite result.

## Acceptance limits

The C# fixtures use self-owned HWNDs and injected presentation/fake effects. They do not establish real file-dialog foreground/focus, physical keyboard/Escape, supported DPI or behavior alongside unrelated applications. Those live scenarios, Explorer integration and installed acceptance are NOT RUN. No product launch, UAC, real picker, registry/service mutation, installation, packaging or current-user-data deletion was performed.
