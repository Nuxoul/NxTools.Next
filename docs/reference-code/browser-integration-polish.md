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
