# Browser integration reliability and workflow polish

## Result

- Automatic downloads with source-matched non-GET or conflicting method evidence remain in the browser before credential reads, native demand or cancellation. Observed final GET after a single request's redirect retains the original durable handoff.
- Native connection failures expose fixed actionable diagnosis. An available native host with an unavailable desktop reports desktop_unavailable. Passive polling does not reconnect or emit connection-check lifecycle events.
- Selected-anchor/text and page-anchor collection is explicit, HTTP(S)-only, deduplicated and capped120. Capture provides review and progressive24-row disclosure; original32-item batch identities and partial retry remain authoritative.
- Media inspection is explicit, frame/document-scoped, coalesced and retryable. Content reads have a5second deadline,2MiB manifest limit and4096byte unknown-type sniff. HLS AES-128 remains distinct from DRM; DASH split/live/protected facts are summarized without child requests. Inspecting/protected resources cannot be submitted through single or batch runtime routes.
- Popup retains456px graphite styling and four tabs, with synchronized Home/Settings pause, fixed connection help, race-safe remembered view, local name/domain search and collapsed other-page resources. Delayed prior-view refresh cannot omit the newly requested view's scan.
- Link/media/explicit connection lifecycle logs use fixed event/reason, UUID correlation and elapsed time. Rust validates the fields and excludes payload secrets. Resource inspection summaries remain ephemeral and outside persistence.

## Verification

| Boundary | Result |
| --- | --- |
| Recursive extension `*.test.js` suites |299passed,0failed |
| `cargo test -p nxtools-browser-host --lib` |49passed,0failed |
| Production Popup DOM through supplied ego-browser Page |23assertions passed |
| Production content DOM |Selected anchors plus literal URLs; page anchors with deduplication; native DOMParser DASH split/live/protected; HLS AES-128;3inspection calls, exactly3resource requests |
| `python tests/architecture/spec-contracts.py` |129documents passed; navigation/evidence/contracts |
| Scoped `git diff --check` |Passed |

Node coverage includes original Cookie-store isolation, Blob transport, identity-bound acceptance, browser cancellation compensation, batch partial retry, resource ownership and sender boundaries. New public-event regressions cover conflicting method evidence, original iframe capture, navigation before/during inspection, selection-frame retirement, pending/protected send guards and bounded native diagnostic frames.

Popup DOM coverage includes keyboard pause/focus, POST retention explanation, delayed view refresh, local search/selection persistence, review-only extraction, inspection busy/failure/retry, protected selection/send state, Escape focus restoration, remembered keyboard tab, fixed connection guidance, four-tab containment, all120review items, reduced motion and no unhandled rejection. The isolated browser space is closed after the checks.

Rust verification is serial with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; no concurrent Cargo operation is started. Host tests use isolated pipes/temp storage and include the actual framed diagnostic route and fixed unavailable-snapshot response.

## References

Behavior sources are recorded in [browser integration reference](../../../docs/reference-code/browser-integration-polish.md): official ABDM extension1.5.2, legacy NxTools browser integration and local Mister0.1.0. No GPL implementation, comments, tests or assets are incorporated. Existing Native Messaging, download-engine acceptance and Cookie-store boundaries remain.

## Limitations

Installed Chrome/Edge/Firefox behavior, real authenticated websites, physical/high-DPI/assistive-reader behavior and human visual acceptance are separate acceptance. Content inspection obeys the original frame's browser origin/CORS policies; a blocked read returns failure with retry. No package, registry, browser-profile or installed-extension operation is part of this change.
