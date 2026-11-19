# Browser integration workflow reference

## Sources

ABDM is inspected before legacy compatibility. Its main checkout `D:\Code\ab-download-manager-master\README.md:125` identifies the independent official repository `amir1376/ab-download-manager-browser-integration`. Reviewed extension1.5.2 at commit `bf6811dd8a3ee2b47b94d70e13c2c7e70be7be05`.

- `src/linkgrabber/DownloadLinkInterceptor.ts:146,361,438,492,546`: GET-only admission, MV2/MV3 response ownership, cancellation before request and incomplete MV3 failure restoration. Adopt explicit non-GET retention, not cancellation-first behavior.
- `src/contentscripts/ContentScript.ts:20,64`, `src/contentscripts/SelectionPopup.ts:38`, `src/contextmenus/ContextMenus.ts`: explicit selected-link actions and in-page selection affordance. Batch URLs are an explicit manual action, not automatic capture.
- `src/backend/Backend.ts:37`, `src/backend/nativemessaging/NativeMessagingTransport.ts:70`: HTTP/native fallback and per-message request IDs; neither replaces NxTools durable handoff identity.

Legacy compatibility: `D:\Code\NxTools\browser_extension\nxtools-interceptor\background.js:2630,3666,3690` owns URL/header submission, exact single-link/page menu actions and connection status; `content-script.js:1069` owns DOM/Performance/CSS scan; `popup.js:831` owns explicit media merge feedback. Keep available single-link/drop/media entry semantics, not the old WebSocket or unconditional scans. No legacy selected-link review owner exists.

Mister behavioral reference only (GPL-3.0-or-later); no implementation, comments, tests or assets incorporated. Local root `D:\Code\misterdownloadmanager-main\misterdownloadmanager-main`, extension0.1.0:

- `extension/background.js:351,392,436,1211,1334,1354`: limited POST context, redirects, application offer and browser cancellation/fallback. Do not replay POST or swallow cancellation failure.
- `extension/background.js:599,737,778,798`: demand-only4096byte sniff, bounded manifest fetch, HLS key method/keyformat and DASH ContentProtection inspection. AES-128 identity is distinct from DRM; manifest discovery is not proof every child is downloadable.
- `extension/background.js:1594,1605`: selected/page anchors and URL deduplication. Preserve NxTools HTTP(S)-only manual/batch boundary and require review before queue intake.
- `extension/popup.js:30,84`: fixed connection/refusal categories and help. Retain NxTools original demand-owned transport instead of one-shot native calls.

## NxTools invariants

Browser item ownership survives until identity-matched durable acceptance plus successful browser cancellation. Confirmation pending, channel enqueue and terminal download success are different states. Keep cancellation compensation, Cookie-store identity, source-authoritative metadata, bounded cache, per-item stable batch IDs and partial retry.

Popup uses its existing browser action lifecycle; page injection uses isolated one-shot browser scripting or the existing content namespace. No GPUI/C# window placement/focus/native show boundary changes. Existing page/frame cache owner retires on navigation/removal; async extraction and inspection must verify the original owner before publishing. Failures keep browser download or resource review available without secrets in diagnostics. No permission/transport/dependency expansion or implicit POST/manifest/key replay.

## Firefox controlled download state

Mozilla Firefox140 and current `toolkit/components/extensions/parent/ext-downloads.js::{DownloadItem.state,DownloadItem.paused,downloads.pause,downloads.resume,downloads.cancel}` define distinct pause/cancel outcomes. Pause calls `Download.cancel()` and retains partial data: `state:interrupted`, `paused:true`, `error:USER_CANCELED`. Resume starts the same Download; cancel calls `finalize(true)` to remove partial data, leaving `paused:false`. Source: https://github.com/mozilla-firefox/firefox/blob/FIREFOX_140_0_RELEASE/toolkit/components/extensions/parent/ext-downloads.js.

ABDM's pinned `DownloadLinkInterceptor.ts::{shouldHandleRequestForDirectDownload,registerListeners}` retains GET-only interception and separate browser ownership; legacy `D:\Code\NxTools\browser_extension\nxtools-interceptor\background.js::interceptBrowserDownload` cancels before asynchronous submission and does not provide a safe pause-state contract. Keep the Next controlled lifecycle instead of cancellation-first behavior.

Next's DownloadItem fallback `background.js` Firefox `downloads.onCreated` → `interceptBrowserDownload` → `runControlledBrowserHandoff` → `advanceControlledHandoff` owns downloads not decided at the response boundary below. The handoff predicate admits Chrome `in_progress` and Firefox `interrupted` only with `paused:true`. Both the direct cancel and `cancelling` recovery paths use `browserDownloadIsCancelled`: `interrupted/USER_CANCELED` with `paused:true` is not cancellation evidence. Recovery retries cancellation while the item is paused and aborts the hold before resuming if cancellation has no effect. An observed cancellation can commit the same held task. Identity, session journal, Pending/Held, correlation and browser cancellation before activation retain their existing owners. No popup, focus, placement or native window show/hide owner changes.

`tests/controlled-handoff.behavior.test.js::Firefox recovery retries cancellation instead of committing a still-paused original` covers worker recovery and failed cancellation. Installed-browser acceptance is separate.

## Firefox response interception and export prerequisite

Verified local browser sources:

- ABDM `D:\Code\ab-download-manager-browser-integration\src\linkgrabber\DownloadLinkInterceptor.ts:146-189,361-402` admits GET frame responses and waits for the application decision in `onHeadersReceived`; `Manifest2DownloadLinkInterceptor.ts:5-19` supplies the blocking cancellation result. Its later `onCreated` cancellation-first path is not NxTools ownership authority.
- Legacy `D:\Code\NxTools\browser_extension\nxtools-interceptor\background.js:3613-3663,4016-4019` submits only after browser cancellation and recreates a URL-only download on failure. Preserve Next's stronger controlled ownership and exact source context instead.
- Mister `D:\Code\misterdownloadmanager-main\misterdownloadmanager-main\extension\background.js:1727-1734,1782-1805,1828-1868` classifies successful full frame responses, awaits the offer inside a Promise-returning `onHeadersReceived` listener registered with `blocking,responseHeaders`, then returns `cancel:true` only when taken; refusal returns the original response. `manifest.firefox.json:19-27` grants `webRequestBlocking`, absent from its Chromium manifest. `background.js:1604-1627,1808-1810` leaves already-decided response fallbacks alone and uses pause/resume only for downloads not classified at the response boundary. This avoids creating the normal Firefox download dialog/history item for intercepted frame responses; it does not establish durable task acceptance.
- Ghost `D:\Code\Ghost-Downloader-3-main\Ghost-Downloader-3-main\browser_extension\app\src\background.ts:381-444` cancels/erases in `takeBrowserDownload` before `resourceBridge.routeBrowserDownload`, using Firefox `onCreated`. `app/public/manifest.json:7-18` has no blocking permission. This cleanup is after creation and cannot prevent Firefox download animation; do not adopt cancellation-first or indiscriminate history deletion.

Next `background.js::registerFirefoxResponseHandoff` registers a Firefox-only Promise-returning blocking response listener before the existing `onCreated` fallback. `bg/browser-handoff.js::{firefoxResponseDownload,runFirefoxResponseHandoff}` classifies successful full HTTP(S) GET frame downloads from Content-Disposition/MIME, retains the original response through Pending and durable Held, persists cancellation intent, then returns cancellation. Inline documents/media, unknown types, partial/empty/error responses and non-GET requests remain native. Exact request/source context, response filename/size/range facts, captured Referrer and redirect source feed original capture gates and credential projection.

`finishFirefoxResponseCancellation` waits up to12seconds for the matching `onErrorOccurred` before persisting verified cancellation and reusing the original Commit retry path. A matching response-start/DownloadItem observation aborts Held; an unknown cancellation retains Held without guessing. The existing32-operation journal/admission bound and ten-minute quiet confirmation loop remain. Response records persist only IDs/state/timestamps, never URL/Header/Cookie. Recovery aborts lost pre-cancellation response owners, retains unverified cancellation and retries Commit only from verified `browser_cancelled`.

`firefoxResponseWasDecided` suppresses a second `onCreated` offer for the released response using a bounded source-matched in-memory decision, including after long confirmation/request-cache expiry. Its short TTL begins again at release; a newer observed request or another named Cookie store is not that fallback. This does not erase browser history or delete files. Unclassified/scripted downloads retain the existing controlled DownloadItem lifecycle.

`csharp/NxTools.PlatformHost/Windows/BrowserExtensionPackageService.cs::CreateFirefoxManifest` adds `webRequestBlocking` only to its parsed Firefox export after removing Chromium DNR. The shared manifest, local CRX and key-free Chrome-store ZIP retain their original permission lists. `BrowserExtensionSigningTests::{MissingCorruptAndFirefoxKeyBehaviorRemainCompatible,IsolatedCrxSignatureAndKeyReuseMatchRegistrationIdentity}` checks actual isolated export contents and unchanged source bytes. No signing key/registration/window/transport owner changes.

Diagnostics reuse `handoff_state`, the same handoff UUID/native request correlation, fixed state/reason and elapsed time; no URL, Header, Cookie, body, raw browser errors or per-poll logging. `tests/firefox-response-handoff.behavior.test.js` drives the real production listeners and cancellation/restore boundaries. See [Firefox interception results](../../.trellis/tasks/11-13-browser-handoff-lifecycle/firefox-interception.md). Installed Firefox animation/Save As/history and authenticated-site acceptance remain separate from API fixtures.
