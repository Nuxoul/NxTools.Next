# About update download

## Reference and scope

`docs/reference-code/system-shell.md::About entry and first-run setup` records no legacy About/update window. The existing Next constructor opens one centered 420 × 480 DIP dialog with official Kit 0.8.0 TitleBar, DescriptionList and direct 32-DIP Buttons. Retain its geometry, copy/license/GitHub actions and window-scoped bounded query. User explicitly changes a newer-release result from GitHub advice to downloader submission; protected installation/update-broker behavior is unchanged.

## Release and download owner

Parse the bounded latest-release response's tag and assets. Only the current Windows x64 target and `NxTools.Next-setup-<numeric version>.exe` supported installer qualify; reject missing/ambiguous assets or invalid download URLs. A current/older release never creates a task. Keep a fixed missing-installer/admission message rather than falsely claiming current or opening a browser.

About registers a request ID before asynchronous work and retires it on close. Result receipt and deferred submission both verify that ID and the live owner. Cross-entity DownloadApplication updates occur only in a later application turn, outside the About lease; status publication occurs after the download owner lease is released. Existing submit_with_idempotency(StartNow) owns durable first admission and scheduling. A stable version-scoped UUID prevents duplicate task creation across checks/dialog reopen. The helper reuses an existing identical installer task before constructing a request from current directory, connection/adaptive defaults, so preference changes preserve its original options. Existing engine HTTP/network/organization policies remain authoritative. Never execute the installer.

## Lifecycle and diagnostics

Closing the dialog cancels its query task and pending dispatch. A successful durable task remains independently owned by the downloader after About closes. Log one correlation across update query/admission stages, elapsed and fixed outcomes/native errors only; no release URLs, names, paths or transport payloads. Original cancellation, global queue limits, progress and notification owners remain unchanged.

## Verification

Production parser tests cover numeric version ordering, supported asset choice, missing/invalid/ambiguous resources and unsupported target. GPUI boundary tests exercise deferred real downloader submission using an owned SQLite engine with its default queue paused before startup, repeat checks, stale/closed and unavailable owners. The downloader regression changes defaults after first admission and asserts the original task/options remain. No external network, installation, Shell activation or package replacement is performed. Existing About layout/copy/license tests remain applicable.
