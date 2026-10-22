# Extension responsibility refactor outcomes

## Scope and reference evidence (2026-10-12)

Only `browser_extension/nxtools-interceptor/background.js`, `browser_extension/nxtools-interceptor/tests/safe-handoff.test.js` and this task evidence are owned by this slice. Preserve the pre-existing dirty capture, transport, popup and UI work. No permission/CSP/manifest, content-script, page-hook, schema or native-host change; no browser/profile/network/Native Messaging operation.

Read extension contracts, shared architecture/engineering/verification and `docs/reference-code/download.md:416-427` before changes. ABDM was checked first: `D:\Code\ab-download-manager-master` has no checked-in JS/TS extension in the inspected tree; its `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/BrowserMessageRequest.kt:7-25,49-50` provides message identity/error envelopes, and `integration/server/src/main/kotlin/com/abdownloadmanager/integration/model/DownloadCredentialsFromIntegration.kt:18-23,41-58` retains link/headers/page/suggested-name intake. These are boundary references, not evidence of ABDM browser-cancellation behavior.

Actual legacy reference: `D:\Code\NxTools\browser_extension\nxtools-interceptor\background.js:2692-2726` registers preempted identity, dispatches asynchronous handoff and returns `{cancel:true}` before acceptance; `:3815-3884` registers the blocking response hook; `:3613-3664` cancels the browser item before send and restarts a URL-only download on failure. These recorded legacy limitations are intentionally NOT restored: the approved Next contract retains identity-matching acceptance before cancel/erase.

## Reachability proof before removal

Exact word-boundary scan of workspace executable/source gate files (`*.js,*.cjs,*.json,*.ps1,*.py,*.rs`) returned 20 references, all in the dirty pre-refactor worker. No external caller, export or listener reference was found. Positions below refer to the saved dirty pre-refactor source, not Git HEAD:

| Symbol | Original lines / references | Reachability conclusion |
| --- | --- | --- |
| `preemptedBrowserDownloads` | 169,1373,1375,1560,1572 | Only writer is uncalled `rememberPreemptedBrowserDownload`; never populated. |
| `PREEMPTED_DOWNLOAD_TTL_MS` | 41,1374 | Only used for cleanup of the permanently empty map. |
| `rememberPreemptedBrowserDownload` | 1553 | Declaration only, no callers. |
| `isLikelyBrowserDownloadResponse` | 1528 | Declaration only, no callers. |
| `sendPreemptedBrowserDownload` | 2910 | Declaration only, no callers. |
| `handlePreemptiveBrowserDownload` | 2955 | Declaration only, no listener; body returns `{}` without effects. |
| `wasPreemptedBrowserDownload` | 1570,3971 | Only reader guards an always-false branch. Its cache cleanup is redundant: both retained header resolution and `browserDownloadHandoffId` already clean the cache. |
| `dispositionRequestsDownload` | 1524,1534 | Only called inside the uncalled classifier. Not the active filename parser/cache. |
| `BINARY_DOWNLOAD_EXTENSIONS` | 195,1541 | Exclusive to the uncalled classifier. |
| `FORCED_DOWNLOAD_MIME_TYPES` | 199,1546 | Exclusive to the uncalled classifier. |
| `readBlobResourceViaContentScript` | 2654 | Declaration only; obsolete base64-reader wrapper, not active Blob upload owner. |

Real startup invokes `registerHeaderCapture` (original :4254): request headers -> `rememberRequestHeaders`/`captureRequestResource`, response-start -> `captureNetworkResource`, redirects -> `rememberRedirectHint`. There is no blocking response-header takeover hook. Mutually exclusive download registrations (original :4354-4363) call `interceptBrowserDownload` through `onDeterminingFilename` or `onCreated`. Blob `sendResource` (original :2994-3008) resolves a network source or calls the retained `sendBlobResourceViaContentScript` (original :2682-2791).

## Invariants retained

- Browser owns its item until positive identity-bound desktop acceptance and successful cancel. Rejection/wrong identity/timeout retain fallback; cancel failure never erases and can roll back the accepted desktop task.
- Save-dialog suggestion remains pending during handoff, resolves on the four reachable disabled/site-disabled/accepted/rejected paths; the fifth preempted path is unreachable and removed.
- Header/cookie/referrer capture, passive response metadata, Content-Disposition filename parsing/cache/redirect propagation, resource provenance/sanitized projection and request cache limits remain intact.
- Native Messaging identities, pending-before-send, demand-only port lifetime, diagnostics, timeout/disconnect and bounded Blob prepare/upload/commit/retry/outer lease remain intact.
- Content-script `content_read_blob` -> `readBlobInPage` and its base64 fallback remain untouched, as do content/page Blob fallback/download messages. Removing the uncalled worker wrapper does not authorize removing page/content compatibility.
- No layout, placement or focus changes. Existing worker/listener lifecycle and error/badge behavior remain unchanged.

## Regression and verification

Baseline PowerShell-expanded 7 `*.test.js` files: Node v22.23.3, exit 0; 26 tests passed, 0 failed/cancelled/skipped/todo. Log: `$env:PI_SCRATCH_DIR\extension-refactor-baseline-tests.log`.

Before production deletion, safe-handoff gained explicit absent-obsolete-owner/passive-capture/download-listener assertions and retained Blob upload/content fallback assertions; existing acceptance/rejection/duplicate/cancel-failure/identity tests remain. Updated suggestion count 5 -> 4. Red run: 10 tests, 8 passed, 2 failed (old fifth yield and obsolete owners), 0 skipped; Blob retention regression passed. Log: `$env:PI_SCRATCH_DIR\extension-refactor-red-safe-handoff.log`.

Final whole-project closeout run: `node --test` with PowerShell-expanded seven `*.test.js` files passed28/28,0failed/cancelled/skipped/todo; `node --check background.js` exit0. Log `$env:PI_SCRATCH_DIR\whole-refactor-extension.log`. Obsolete preemptive ownership and the unused worker Blob-reader wrapper were removed; passive header capture, accepted-before-cancel and content/page Blob fallback remain. No installed-browser acceptance claimed.
