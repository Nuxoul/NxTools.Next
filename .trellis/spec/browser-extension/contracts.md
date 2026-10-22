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
