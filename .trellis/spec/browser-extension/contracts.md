# Extension capture, handoff and privacy

Evidence: `repo:browser_extension/nxtools-interceptor/background.js`, `repo:browser_extension/nxtools-interceptor/handoff-policy.js`, `repo:browser_extension/nxtools-interceptor/batch-handoff-policy.js`, `repo:browser_extension/nxtools-interceptor/blob-transfer-policy.js`, `repo:browser_extension/nxtools-interceptor/capture-policy.js`, `repo:browser_extension/nxtools-interceptor/tests/safe-handoff.test.js`.

## Context and dependencies
Manifest MV3 background service_worker, document_start/all_frames content scripts and web-accessible page-hook. Permissions/CSP are product/security surface; no widening for convenience. Policy modules expose browser global/CommonJS seams; background owns browser APIs, pending Native Messaging and privileged request context; popup gets sanitized projection; page hook is untrusted page-side input. No SQL/ORM or general engine-state database: chrome storage owns approved local prefs/projection, engine owns task persistence.

## Scenario: Identity-bound download handoff
### 1. Scope / Trigger
Interception/context menu/batch/Blob intake and response/error handling.
### 2. Signatures
`acceptedHandoffResponse(response, expectedHandoffId)`; `completeAcceptedHandoff({response,expectedHandoffId,cancelBrowserDownload,eraseBrowserDownload,rollbackDesktopTask})`; host protocolVersion2/requestId and accepted handoffId; batch maximum32.
### 3. Contracts
Bound Native Messaging256KiB; pending request ID separate from log correlation, timeout/disconnect rejects once. Browser item remains authoritative until ok+accepted+matching handoffId and successful cancel; cancel failure never erase item and may rollback accepted desktop task. Batch checks each expected identity, partial result stays per item. Blob chunk/capability/meta bounded and source/token/listener lifetime checked; no privileged header/cookie/referrer context in popup/storage projection. Explicit unsafe preempted paths must keep their limitation visible rather than assert universal safe interception.
### 4. Validation & Error Matrix
Wrong identity/rejection/timeout/disconnect -> browser fallback, no erase. Cancel failure -> keep browser ownership, bounded rollback, no second desktop task via same idempotency. Invalid capability/source/size -> reject; stale page reply -> discarded/listener cleaned. Do not swallow error into accepted=true.
### 5. Good/Base/Bad Cases
Good: matching accepted ID then cancel then erase. Base: rejected host leaves browser item untouched. Bad: cancel before host acceptance, log request payload, or expose cookies to popup.
### 6. Tests Required
safe-handoff/batch/capture policy tests, direct Blob assert test; exact identity/cancel-failure/no-erase, isolated log request IDs, sanitized projection and bounded source bridge tests. Background/content/popup integration must be live tested separately; source assertions only prove code shape.
### 7. Wrong vs Correct
```text
Wrong: cancelBrowserDownload -> send native request -> hope response accepted.
Correct: identity-matching accepted response -> successful cancel -> erase; failure retains fallback.
```

## Diagnostics and coding
Whitelist fixed event names and bounded correlation at Rust host; extension logger is not arbitrary remote log ingestion. No raw URL/query/path/title/request/headers/cookies/capability/chunk bytes/error object dumps. Browser runtime errors may carry payload, sanitize before logs. Helper module owns normalized policy fields rather than repeating ad hoc property casts. No new npm dependency/build/runtime just for UI consistency; preserve CSP and tests. [Shared diagnostics](../shared/engineering.md).

## Demand-only native transport lifetime
Native Messaging is browser-owned, not a desktop child. Browser startup and best-effort diagnostics do not connectNative. A real request takes a transport owner before connecting, registers pending before postMessage, rejects once on timeout/disconnect/send failure and releases in finally. Full Blob prepare→upload→commit/retry retains an outer lease; idle release cannot terminate its capability listener. With no connecting/pending/owner, disconnect the current port after1s, without autonomous reconnect; future explicit requests may reconnect. Old-port callbacks cannot mutate a replacement. Popup snapshot is passive existing-engine-only and never resurrects an exited desktop. Source/extracted-function tests do not prove real browser stdio EOF or installer file-lock release.

## Resource cache identity and alias lifecycle
resourcesByTab/resourcesById are lookup indexes, not independent resource owners. Count distinct resource objects for the120per-tab limit; normalized URL aliases never consume capacity or overwrite another real canonical ID. removeCachedResource owns capacity/TTL/unusable/tab-clear/canonical-replacement cleanup and removes keys only while they point to that resource, protecting aliases reassigned to survivors. Same-ID recapture updates the authoritative object in place; same-URL source-ranked metadata enrichment cannot transfer ID/tab/URL/timestamp/pending/acceptance state. Restore uses identical index/capacity rules and still rewrites through the privacy projection. Required production-entry isolated regressions: `repo:browser_extension/nxtools-interceptor/tests/resource-cache.test.js`; these do not certify installed MV3 suspension or live multi-tab Native Messaging.

## Sender trust, browser download hand-back and worker suspension (2026-10-22)
- Runtime router (`bg/router.js`) classifies every sender: another extension → ignored without response; tab content script → only the content message types (`content_*`, `page_download_media`, `drop_target_download`); extension pages → popup messages. `popup_save_settings` passes `sanitizeSettings` (known keys, boolean types, drop-target position enum, width 96..360, height 48..220) and reports storage failure instead of a stale state.
- `downloads.onDeterminingFilename` listener returns `true`; `interceptBrowserDownload` resolves `suggest` exactly once in `finally` on every path. Without `return true` Chrome auto-suggests immediately and opens its own Save As during the handoff.
- Every handoff entry (download, context menu, drop target, popup, Blob) reports success only for `ok && accepted` with the matching handoff ID.
- Browser download resource lookup uses only the download's own context (`item.tabId`, the observed request tab `tabIdObserved`, or the referrer tab); never `lastActiveTabId`.
- Content-Disposition names survive MV3 suspension via `chrome.storage.session` under a hashed normalized-URL key (bounded 200); no query text, headers, cookies or referrer are persisted. Missing context degrades to the browser-provided name.
- Content Blob bridge: `content_blob_info` / `content_upload_blob` are handled by the runtime message listener (they were previously unreachable inside the storage callback, so every Blob handoff fell back to the browser).
- Firefox XPI: `CreateFirefoxManifest` keeps the source ordered `background.scripts` and drops `service_worker`; its JSON options set an explicit `DefaultJsonTypeInfoResolver` (.NET 8 otherwise throws, which broke Firefox export).
- Required tests: `tests/defects.behavior.test.js`, `tests/native-lifecycle.test.js`, `tests/background-modules.test.js`, `tests/content-modules.test.js`, `tests/content-browser.cjs`, C# `browser signing fixture missing corrupt Firefox cancellation`.

## Scenario: Key-free Chrome store upload and optional exact identity
### 1. Scope / Trigger
First upload rejects manifest key before a store ID exists. User authorizes only upload ZIP and store-ID matching, not other audit findings. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/BrowserExtensionPackageService.cs`, `repo:rust/apps/nxtools-desktop/src/downloads/settings_view.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs`.
### 2. Signatures
Existing ExportBrowserExtension.format gains `chrome_store`; C# NormalizeFormat, store SaveFileDialog ZIP and ExportWithSigningKey reuse original owner. Core `DownloadSettings.chrome_web_store_extension_id: String` serde-default empty; `is_valid_chrome_web_store_extension_id` accepts empty or exactly32a–p. Existing EnsureBrowserNativeMessaging.chromium_extension_id carries configured store ID or original local ID; protocol unchanged.
### 3. Contracts
Store ZIP puts manifest at root, removes only key from a parsed copy, preserves permissions/scripts/other manifest values, never changes source or accesses PEM. CRX/XPI paths stay. Empty ID never blocks first ZIP. Existing Settings draft/Cancel/Save and durable section merge precede exact registration; UI rejects invalid ID, loaded malformed value normalizes empty. Command/pinned/exported origins remain an exact union, not wildcard; BrowserHost validates registrar origins at its original boundary. Local identity remains compatible. One pending command installed before caller-ID dispatch; matching result/disconnect/15s logical deadline clears ownership, old ACK/deadline cannot end replacement. Registration failure keeps saved intent with fixed retry guidance; queued/saved is not native applied. Existing correlated export/registration diagnostics only, no ID/path/key/payload logs.
### 4. Validation & Error Matrix
Missing old config field -> empty/local. Invalid UI ID -> no save/command. Commit failure -> prior live config/no new registration. Store ZIP -> no key and no PEM/source mutation. Cancel export -> original terminal. Old result/timeout -> no replacement mutation. Native fail/disconnect -> retry guidance, no false success.
### 5. Good/Base/Bad Cases
Good: first ZIP uploads without asking for an unavailable ID, then exact ID saved/registers beside local identity. Base: local CRX and Firefox unchanged. Bad: delete source key, trust any chrome-extension origin, or demand store ID before first upload.
### 6. Tests Required
Actual ZIP/temp-key test red before fix, locked/missing PEM and source bytes/permissions/root verification; existing7signing fixtures. Corelegacy/validation/normalize/roundtrip; actual settings Cancel/Save/failedsave/reconnect/stale/deadline and third official export Button. BrowserHost registrar-origin acceptance tests and browser NativeMessaging/source gates; real store upload/registry/browser acceptance separate.
### 7. Wrong vs Correct
Wrong: `format != firefox -> signed CRX`; fixed local ID ignores saved store ID. Correct: additive key-free ZIP path and optional validated ID through original exact registration chain.

## Published Chrome identity (2026-10-28 amendment)
The user-approved published entry is now known. This supersedes the pre-publication manual-ID UI above: ordinary settings expose a Chrome store Button, not an ID Input. Rust uses the published identity when the legacy field is empty; C# always unions the exact published origin with command/pinned/exported origins, deduplicated. Existing valid legacy fields remain non-editable draft data and survive Save/Cancel; Core serialization/validation and protocol stay unchanged. Missing/local-key IO fallback must not drop the published origin, and registration never creates/replaces PEM. Store navigation reuses the Firefox pending/result/disconnect/deadline and correlated diagnostic owner; both callbacks defer application updates. No wildcard, browser-profile scan, auto-trust, signing change or new permission. Evidence and reference chain: [download amendment](../../../docs/reference-code/download.md#published-chrome-store-identity-amendment-2026-10-28). Regression: actual reconnect default/legacy config, rendered no-ID Settings and Save/Cancel, shared Shell navigation and isolated C# allowed-origin union. Source/fixture checks do not certify live browser or registry acceptance.
