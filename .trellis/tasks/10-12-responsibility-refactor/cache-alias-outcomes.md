# Resource cache alias eviction — outcome (2026-10-13)

## Scope and authority
User approved this concrete behavior fix from whole-project-review.md. Preserve all accumulated dirty work. Only background.js cache-index maintenance, a new isolated Node regression file, reference/spec evidence and this outcome are changed. No permissions/CSP/manifest/UI/Native Messaging/handoff policy/dependency change; no live browser/URL/cookie/history/native host/service/install/package/commit operation.

Read extension/shared contracts and download reference first. ABDM Desktop NativeMessaging BrowserMessageRequest.kt was inspected first for identity but no equivalent extension-cache source claimed; legacy background.js cache/prune/restore/tab-cleanup was then inspected. New verified chain/invariants recorded in docs/reference-code/download.md before production fix. Cache has no native window/layout/focus/show-hide change.

## Reproduction and root cause
Baseline seven Node test files before adding cache tests:28/28 passed. Production cache functions executed unchanged in isolated node:vm, using actual capture/batch policy modules, controlled clock/timers and in-memory Chrome storage. No copied cache algorithm and no entire worker startup/native API execution.

Initial2tests:1pass/1fail; capacity2 and two canonical captures already deleted the first canonical key. Expanded4tests:1pass/3fail demonstrated:
1. sorted bucket.values counts canonical+normalized URL alias twice, prematurely consumes capacity;
2. eviction removes only stale.id while normalized alias remains in both indexes;
3. same-URL Object.assign of generic merge copies another capture's ID into the cached object.

Further real-entry regressions exposed URL-alias/canonical collisions and duplicate-ID restore leaving an old URL alias. These were fixed at the same index owner. Logs cache-alias-red.log/cache-alias-red-confirmed.log/cache-alias-restore-red.log in session scratch preserve actual failures; baseline source snapshot cache-alias-before.js is dirty pre-task authority, not Git HEAD.

## Implementation
- removeCachedResource(tabId, resource) is the single removal owner for capacity, TTL/unusable URLs, tab clear, recapture and canonical replacement. It removes per-tab entries pointing to exactly that object and deletes matching global entries only if they still point to it. Reassigned aliases/surviving captures and in-flight objects are not deleted/mutated.
- indexCachedResource installs canonical and normalized lookup keys, cleans a replaced real canonical object and gives actual canonical IDs priority over aliases. An alias hit is not treated as the same-ID capture.
- enforceResourceCapacity sorts distinct objects, retaining original120resource limit; aliases cannot consume capacity. No unbounded side index/framework added.
- Same-ID recapture updates the existing authoritative object in place and removes stale URL keys first; pending identity and later current-object resolution remain coherent. Existing logging compares pre-merge MIME/size, preserving low-frequency change diagnostics.
- Same-URL enrichment uses original mergeCapturedResource source priority but excludes ID/tab/URL/time/pending/acceptance fields; other capture objects retain their own identity/handoff.
- Restore rebuilds aliases with the same registration/capacity rules; serializeResources continues unique-ID storage projection, excluding Header/Referrer/page context and retaining pending stable handoff identity. Existing storage schema unchanged.
- sendResource/sendResourcesBatch, accepted-before-browser-cancel, Native Messaging port ownership, TTL duration, persistence debounce and policy modules unchanged.

## Regression coverage
New tests/resource-cache.test.js has16tests, executing production cache/prune/serialize/load/clear/sendResource and actual policy code:
- unique-capacity/premature eviction/orphan alias and surviving same-URL alias;
- stable ID/object/pending preservation during source-ranked response enrichment;
- exact TTL cutoff, expired signed media removal, tab cleanup isolation;
- reload alias reconstruction/capacity/privacy/pending persistence and repeated canonical-ID cleanup;
- recapture URL change and canonical-vs-alias collisions;
- real120capacity with121captures;
- controlled sendResource persists identity before send, recapture and delayed acceptance cannot clear newer handoff;
- acceptance after eviction settles observed object without resurrecting cache.

Fixture failures were kept distinct: replaced TTL loop initially had an extra brace (syntax gate caught/fixed); test section insertion briefly malformed fixture source; microtask-count assumption replaced by explicit outbound-send signal. No product timeout/retry policy was relaxed and no warnings/tests ignored.

## Actual final verification
- Full extension suite:44/44,0failed/cancelled/skipped/todo (28baseline+16new Node-reported tests).
- All top-level extension .js syntax and new test syntax:exit0.
- browser-native-messaging.ps1 source gate:passed.
- spec-contracts.py:87documents/12owners passed.
- git diff --check:exit0.
- Log:session scratch cache-alias-final-checks.log. Earlier focused15/15 plus full43/43 succeeded before final duplicate-restore regression, final44/44 supersedes those counts.

No blocker remains for this isolated cache fix. Installed MV3 service-worker suspension/reload, real multi-tab browsing/header capture and live Native Messaging remain unverified and unauthorized; source-extracted production-function tests do not certify that acceptance. No Cargo/C# build required for unchanged host/protocol.
