# REF-02/03 — Extension behavior

## REF-02

The existing request/resource owners retain transient native source context through download, request, redirect, tab, single-resource and batch paths. Cookie enrichment queries `cookies.getAll({url,storeId})` in the originating store; every fallback URL keeps that store. Header master and Cookie subordinate gates skip Cookie reads and store enumeration. No IPC store field, permission or credential persistence is added.

Observed Chromium normal/private identity survives source-tab closure and resolves only its existing enumerated store0/1 on the filename-interception-capable browser path. An unresolved tab ID alone cannot infer a store; Firefox privacy never selects a container/default store. Later tab lookup may enrich only the still-owned snapshot and must be compatible with observed privacy/store facts. Capture rejects contradictory live-tab page/referrer metadata before deriving resource context.

Same-URL observations cannot exchange Authorization, Cookie, referrer, page or store identity across tabs. Privacy filters candidates but does not identify a particular tab/container. Unique referrer-tab resolution is fed back into snapshot and Cookie selection; ambiguous matches leave the source unresolved. Privileged context is excluded from popup/storage projections. The original dedicated Cookie wire field remains.

## REF-03

Actual Chrome `onDeterminingFilename` and Firefox `onCreated` listeners publish one bounded transient outcome in the existing popup task workspace. Fixed states distinguish policy skip, submitting, enqueue-only, confirmation pending, persisted acceptance, browser transfer and failure; queued is not completed. Only an identity-matched durable receipt permits browser cancellation. Cancellation failure retains the browser item and invokes original task cancellation with the same correlation ID. History cleanup failure after successful cancellation does not restore ownership; connection failure retires the rejected Promise for immediate demand retry.

Manual confirmation admission requires original `ok:true, accepted:false`, matching nonempty handoff ID, no task ID, `awaiting_confirmation/confirmation_pending` and no queue admission. Shared URL and Blob wrappers preserve it without inventing task acceptance. Context menu, drop/direct link, popup page/resource/activity, selected/latest/direct media and Blob sends show **确认窗口已打开，尚未创建任务** as normal pending. Popup stays open; the media entry preserves the reply message. Context-menu pending reuses the existing whitelisted `handoff_state` event and transient task-workspace projection.

Resource pending replies retain their original handoff ID and do not set `sentToDesktopAt`; only matching durable acceptance clears pending identity. Batch keeps `openDialog:false` and original per-item receipts. Blob's existing upload/retry/fallback owner also obeys the shared classification, so a contradictory accepted+pending reply is a failure. Refusal, wrong/missing identity, malformed pending, timeout and disconnect remain failures. Automatic pending never cancels or erases a browser item.

Diagnostics retain fixed state/reason/elapsed/boundary/compensation fields without request payload or raw error text. One safe `lastHandoff` remains transient and is not persisted.

## Verification

Counts overlap and are not summed. Logs are in the session scratch directory.

| Check | Current acceptance-correction result |
| --- | --- |
| Recursive extension `*.test.js` suite | **170 passed / 0 failed**, `b1-repair-extension-final.log` |
| Cookie production-event regressions | **21 passed**, included above; standalone `b1-repair-cookie-final.log` |
| Manual confirmation/transport production events | **44 passed**, included above: eight manual entry paths per browser, context-menu status, invalid pending, durable receipt, Blob classification, stable resource retry, timeout/disconnect |
| Automatic reason/ownership listener regressions | **32 passed**, included above: valid pending/no cancellation, durable transfer, refusal/identity/timeout/disconnect, original compensation and stable retry |
| Browser fixture syntax | `node --check tests/handoff-popup-browser.mjs` passed |
| Current supplementary popup/content DOM run | **Incomplete; not counted as passed.** The isolated browser test space is no longer available for continuation. |

RED witnesses preceded the corrections: closed Chromium normal/private sources emitted no Cookie; contradictory live sources selected another store or lost source Authorization; valid manual pending returned failure; Blob commit pending was rejected; contradictory Blob acceptance bypassed outcome classification. Focused RED logs: `b1-repair-cookie-closed-red.log`, `b1-repair-cookie-conflict-red.log`, `b1-repair-cookie-source-red.log`, `b1-repair-manual-red.log`, `b1-repair-blob-red.log`, `b1-repair-blob-invalid-red.log`. The final full suite observes actual production events, API arguments, wire fields, status projection and cancellation rather than source tokens.

The prior B1 baseline had117Node tests and a12-scenario/76-check popup DOM pass. Those historical results do not certify the current supplemental DOM run. Current real host/framed-pipe/engine evidence is recorded in `host-outcomes.md`.

## Limitations

Browser APIs are deterministic fixtures. No installed extension, live Chrome/Edge/Firefox Native Messaging registration, physical user input, product launch or human visual acceptance was performed. Supplementary real-DOM feedback acceptance remains incomplete; source, production-event fixtures and DOM/native acceptance are separate evidence levels.
