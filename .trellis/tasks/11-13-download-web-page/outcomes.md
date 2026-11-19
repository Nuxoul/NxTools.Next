# Outcomes

## Implementation
The existing single-stream and planned-segment response/write boundaries reject a declared or bounded-prefix HTML page when a binary file is expected. A full200 waiting page on resumed Range work leaves the saved artifact unchanged. Intentional HTML, legacy Office HTML exports and unknown generic data retain original behavior. The typed nonretryable UnexpectedWebPage fault persists through SQLite and command negotiation.

The original failed task window offers an official Small outlined browser action in DialogFooter. It defers into the existing application Shell owner, coalesces pending clicks, retains correlated result/timeout/disconnect handling and never closes or restarts the failed task. Original geometry, registration, focus, placement and cancellation owners are unchanged.

## Verification
- Real HTTP/engine artifact regression: original HTML-as-ZIP cases failed before the change; 6 tests pass after, including exact normal segmented bytes, fragmented/missing/misleading MIME, intentional HTML/Office, durable reopen and saved-prefix preservation.
- Production GPUI browser-action regression: failed before the button existed; passes after, including native failure, timeout, disconnect, pending coalescing, retained window generation and unchanged engine task state.
- Existing conflict-window tests: 6 passed; artifact feedback: 1 passed; task-window selection: 27 passed.
- Scoped rustfmt check passed for changed HTTP guard/transfer and desktop action/window/test files.
- spec-contracts, download-ui-lifecycle, gpui-release-contracts and markdown-editor-contracts passed.
- Existing engine/HTTP/message/length/storage integration suites: 160 passed. IPC library: 9 passed. Strict Clippy passed for download, IPC and desktop across all targets.
- Full download library: 155 passed, 1 failed, 4 ignored. The independent B6 connection experiment asserts native-refusal-only classification, superseded by the current connection policy; its 4-versus-3 error-count assertion remains unresolved.
- Deterministic fixture maintenance keeps the IPC pause test on an explicitly held partial response and excludes no-payload preemption from the slow-connection reconnect assertion.

## Remaining acceptance
No package, installation or product launch was requested. Real HWND/browser activation, physical pointer/DPI and user visual acceptance remain separate from TestPlatform and source gates.
