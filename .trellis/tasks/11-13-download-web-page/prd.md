# Reject unexpected download web pages

## Goal
A login, waiting or captcha page must not be published as a successfully downloaded binary file.

## Requirements
- Detect HTML responses for clearly expected binary file kinds using response metadata and a bounded byte prefix, including misleading or absent Content-Type.
- Cover single-stream and segmented transfer boundaries; resumed work retains existing bytes and checkpoints on rejection.
- Explicit web-page downloads and legacy Office HTML exports remain downloadable; unknown generic binary data is not rejected merely because it contains '<'.
- Surface one typed nonretryable, actionable fault and an explicit browser-open action through the original task window and native command owner.
- No extra HTTP request, full-response buffer, payload log, dependency or separate error-window owner.

## Acceptance
Real localhost engine/HTTP tests fail before the fix when HTML is published as ZIP, then pass for truthful/misleading/absent MIME, single/segmented and resume paths. Normal HTML, old Office HTML, short binary and fragmented response prefixes retain exact bytes. Production task-window fixture exercises the actual browser button and deferred command without changing window registration, focus or task state.
