# Design

## Existing owners
Browser request snapshots own observed methods and source identity; `interceptBrowserDownload` owns automatic hand-back before credentials/native demand. The existing Native Messaging transport owns classified failures and connection lifetime. `resourcesByTab/resourcesById` remain the sole bounded resource indexes; batch intake retains per-item durable handoff IDs. Content owns DOM selection and in-page fetches; Popup owns presentation and drafts. Rust owns task acceptance and only admits fixed diagnostic metadata.

## Changes
1. Preserve method on actual request/redirect snapshots. Resolve source-matched final-request evidence before automatic handoff; known non-GET yields skipped/unsupported_request_method. Missing evidence retains existing behavior. Explicit link sends still mean GET, never replay a form.
2. Normalize native runtime errors at transport failure boundaries into fixed connection codes. Popup state exposes code/message/guidance only; raw browser errors never reach diagnostics. Existing demand-only reconnect remains unchanged.
3. Context-menu and Popup explicit extraction use a one-shot isolated script in the originating frame. HTTP(S) URLs are deduplicated and capped at120; page extraction is review-only, and selection includes selected anchors plus literal URLs. Source bucket identity is checked after async work. Existing resource cache/batch projection and partial retry are reused. A transient `linkReview:{id,tabId,state,count,message}` selects Capture without creating tasks; popup opening failure leaves a badge and review available on manual open.
4. Explicit `popup_inspect_resource` asks the resource's original content frame to inspect its HTTP(S) URL. A content helper streams at most2MiB with a5second abort deadline, sniffs at most4096bytes for unknown types and cancels unused readers. Cookie preference gates credentials; content fetch stays in the original container/private context, never imports another store or captured credentials. HLS/DASH summaries distinguish manifest/master, audio/video separation, live, AES-128 and protected media. No recursive variant/key/segment requests. Resource identity/page retirement rejects stale publication. Summary is ephemeral and secret-free.
5. Popup retains original dimensions/tokens. Home pause reuses original session-pause message. View preference is local presentation storage, not download settings. Search is local presentation filtering and preserves selections. Other-page resources collapse natively. Inspection errors retain retry, protected summaries disable unsupported sends.

## Invariants
- Browser cancellation requires an identity-matched durable engine receipt; pending confirmation is not acceptance. Cancel failure keeps browser ownership and attempts desktop cancellation.
- No automatic task, probe, manifest/key request or credential read during link extraction. Automatic rules do not restrict explicit manual intake.
- Current tab/container/frame is captured before async operations; navigation/removal retires old work. Late results cannot replace newer resource state.
- Popup open/close, native lease release, original fixed width, keyboard tabs, Escape preview focus return, CSP-safe DOM rendering and header/cookie privacy remain.
- New operation diagnostics use one UUID, fixed start/terminal reason and nonnegative elapsed time; no URLs, manifest contents, keys, request fields or raw errors.

## References
[Verified browser chains](../../../docs/reference-code/browser-integration-polish.md). This is a browser extension task; no GPUI/native window show/hide behavior changes.
