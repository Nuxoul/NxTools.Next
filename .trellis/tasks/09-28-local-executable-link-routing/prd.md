# Route local executable links to Explorer

## Goal

Make the link-detector panel distinguish local filesystem targets from web links so clicking an existing executable opens Windows Explorer and selects it.

## Requirements

- Preserve the current conservative detector: only existing drive/UNC paths are local targets; nonexistent paths remain ignored.
- Carry a bounded target kind through detection and panel rendering: web, local file, or local directory.
- For a local file, copy the original value, close the panel, and send the existing `OpenShellTarget` command with `ShellOpenAction::SelectInExplorer`.
- For a local directory, copy the original value, close the panel, and send the existing shell `Open` action.
- For HTTP(S) URLs and recognized bare domains, retain the configured browser auto-open and copy-only behavior.
- Keep the link panel's generation/echo suppression and do not log raw target values or URL payloads.

## Acceptance Criteria

 - [x] A fixture path named `Throne.exe` is detected as a local file even when it is not a registered application.
 - [x] The panel action for that fixture emits `OpenShellTargetCommand { action: SelectInExplorer }` and never constructs an `https://` URL.
 - [x] Existing local directories route to `ShellOpenAction::Open`; HTTP(S) and bare domains retain browser behavior; copy-only mode never launches either route.
 - [ ] The panel closes before copying, and the panel's own clipboard echo does not reopen it.
 - [x] Unit/interaction tests cover file, directory, web, nonexistent, and sensitive-value diagnostic cases.
 - [x] The existing PlatformHost `ShellTarget` validation and Explorer `/select` launch path remain unchanged and its focused tests pass.

## Notes

- The current bug is in `rust/apps/nxtools-desktop/src/tools/link_panel.rs`: every clicked value is treated as a browser URL after detection.
- No new IPC command or C# Shell implementation is expected.
