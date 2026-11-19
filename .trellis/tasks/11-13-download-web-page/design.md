# Design

## Behavior and owner
The ordinary HTTP writer currently accepts successful HTML as a binary artifact. Add a private bounded response-content guard at the original single-stream and planned-segment body boundaries, before a byte is written. It uses the requested/probed filename, known binary MIME and actual response Content-Type; generic unknown data remains unrestricted. Inspect only byte-zero bodies (including a full200 returned to a resumed Range); declared HTML can reject at any offset. Keep existing Range/length/validator checks authoritative for other responses.

`HttpDownloadError::UnexpectedWebPage` is fatal/nonretryable with a payload-free diagnostic kind. Project it to a distinct DownloadFaultCode and expose it through existing command version negotiation, not message text matching. Existing task/error persistence retains it on reopen. A failed task offers an explicit original OpenShellTarget browser action, deferred out of the view entity callback; no automatic browser launch, retry or task creation.

## Reference evidence
Read `docs/reference-code/download.md` HTTP and lifecycle chains and `mister-download-manager.md` before local sources. ABDM `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/part/HttpPartDownloader.kt::connectAndVerify` checks success/length/Range; `downloaditem/http/HttpDownloadJob.kt` preserves validation-before-writers and checkpoint/finalization. Legacy `Services/DownloadManagerService.cs::BuildProbeResult`1139, `DownloadSegmentAsync`2124 preserve server filename priority and response validation. Mister `crates/engine/src/download.rs::expects_other_file`3974, `never_a_page`4031, `not_a_page`4085 and request3703 reject HTML with metadata and byte-prefix evidence, leaving intentional HTML/legacy Office exports alone. Behavioral reference only.

## Layout/lifecycle invariants
Keep one keyed task window, same phase sizes and existing native Show/hide/close/placement/activation chain. Failure content remains scrollable. Closing presentation does not cancel. New browser action uses an official Small outlined Button directly in original DialogFooter (no Toolbar adaptation); existing failure title/detail slots disclose the reason. No custom ordinary control, native window or new notification owner. Source review of exact current Button and DialogFooter precedes UI editing.

## Files
- `http.rs`, private `http/web_page.rs`, `http/segmented.rs`: existing request/body validation and bounded prefix ownership.
- `engine.rs`, `message.rs`: typed failure and protocol negotiation.
- `tests/web_page_contract.rs`: real HTTP/engine/artifact/reopen regression.
- Existing Desktop task-window/application/test modules: failed-state action and actual deferred native command assertion.
- Reference note and owning quality contract: verified source chain and required regressions.

## Diagnostics
Use original transfer/probe/terminal correlation/task IDs and fixed `unexpected_web_page` error kind. No URL, headers, response text, file paths or extra byte events. Browser-open operation retains the existing native command ID/result diagnostics.
