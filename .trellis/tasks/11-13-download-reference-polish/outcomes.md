# Outcomes

## Delivered

All five ordered slices are complete.

| Slice | Result | Evidence |
| --- | --- | --- |
| Reject unexpected download web pages | Single-stream and segmented downloads reject HTML when binary data is expected, retain durable resume data and offer the original failed window's browser action. | [Web-page outcomes](../11-13-download-web-page/outcomes.md) |
| Download disk backpressure | Actual writer and limiter waits do not consume network stall/retirement budgets. Bounded writes, durable checkpoints and genuine network-stall detection remain intact. | [Disk-backpressure outcomes](../11-13-download-disk-backpressure/outcomes.md) |
| Download finishing progress | Concatenation reports actual output bytes; muxing reports child media time. Publishing and unknown totals remain indeterminate. HLS/DASH finishing retains pause, resume and cancellation. | [Finishing outcomes](../11-13-download-finishing-progress/outcomes.md) |
| Grouped download completion | Original batch members settle independently into one action-free summary. First dispatch, retry, removal, cancellation and historical tasks retain distinct authority. Configured sound, directory actions and B5 power intentions remain unchanged. | [Grouped-completion outcomes](../11-13-download-grouped-completion/outcomes.md) |
| Durable download page provenance | Captured page URL/title survives browser framing, the owned pipe, engine storage and SQLite reopen independently of download URL, redirects and Referer. Legacy absent fields, edited names and controlled handoff identities remain compatible. | [Page-provenance outcomes](../11-13-download-page-provenance/outcomes.md) |

AB Download Manager and legacy NxTools supplied the reference chains recorded in [the reference note](../../../docs/reference-code/mister-download-manager.md). Mister supplied behavioral comparisons only; no code, tests, comments or assets were copied.

## Verification

The child outcomes retain the exact boundary checks and scratch log names. Counts below describe separate scoped runs, not a deduplicated full-suite total.

- Web-page admission: six real HTTP/artifact tests; production browser-action, conflict and keyed-window regressions; engine/HTTP/message/length/storage contracts and IPC passed.
- Disk backpressure: seven planned-transfer tests, seven real HTTP fault-recovery tests and 137 engine/HTTP/web-page integration tests passed.
- Finishing: 106 engine contracts, ten media/process/parser tests, real HLS finishing pause/resume/cancel, two GPUI/taskbar tests and 29 task-window regressions passed.
- Grouped completion: 107 engine contracts, 18 completion/B5 coordinator tests, two production GPUI/HTTP/notification tests and original platform-feedback/action tests passed.
- Provenance: 55 BrowserHost tests, 182 affected Download contracts, nine IPC tests, ten selected Desktop tests and 320 extension tests passed. Extended selected-detail and paused replacement/reopen assertions passed.
- Strict all-target Clippy passed for the final affected BrowserHost, Download, Download IPC, Desktop and CLI packages. Scoped formatting/whitespace and Browser Native Messaging, download UI lifecycle, specification, GPUI release and pinned-editor gates passed.
- The final Desktop test binary passed five shared-entry checks: the web-page browser action, finishing window/taskbar presentation and grouped HTTP/notification interaction. Log: `download-polish-final-ui.log` in the session scratch directory.
- Action-free completion presentation passed two production notification tests, 18 completion/B5 tests, bounded platform feedback and the existing notification-action regression. Desktop all-target Clippy and related source gates passed. Logs: `download-notice-*.log` in the session scratch directory.

## Known limitations

The independent B6 `ref13_successful_width_prevents_repeated_extra_connection_refusals` experiment retains its historical native-refusal-only assertion. Its recorded full-library run has 159 passed, one failed and four ignored; the failure is a four-versus-three error-count expectation. The current fault-recovery contract accepts other typed extra-connection refusals. This work does not change connection learning.

The extended task-window selection for action-free notices has27passed and2failed: an inline notification assertion precedes the existing deferred publication, and the Open With pending-state assertion fails after601seconds. These paths were not modified by the notification-action work; exact tests and evidence are in the grouped-completion outcomes.

Local HTTP, SQLite, synthetic extension APIs, TestPlatform and source gates establish the tested boundaries, not installed-browser behavior, real Windows activation/focus, physical pointer/DPI, visual or user acceptance. No new process-memory benchmark was performed. No package, installation, registry change, product launch or commit was performed.
