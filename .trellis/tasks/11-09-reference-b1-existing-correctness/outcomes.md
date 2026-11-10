# B1 — Existing correctness

Status: complete at the source/fixture level. The three acceptance corrections pass their production-boundary regressions. Whole-workspace lint/protocol/source-gate baselines and supplementary DOM/native acceptance remain separate limits. No B2 work, package/install, product/reference launch, registry/service change, commit or push.

## Delivery

- **REF-01:** current clipboard settings govern ordinary ListItem activation after local close and deferred original-owner dispatch. Explicit Copy/Open, local targets, focus/reuse/timers/IME and window/generation protection remain. Explicit Copy now closes its diagnostic lifecycle even when the old window has been replaced: one `copy_cancelled`, fixed `panel_replaced`, original correlation and elapsed. Normal/ownerless Copy each retain one completion. Official Kit0.8.0 ListItem/Label/StatusBar and XSmall ghost Buttons keep420-DIP/28-DIP geometry. [Detail](ref01-outcomes.md).
- **REF-02:** observed Chromium normal/private source identity survives tab closure and resolves only the existing originating store. Incompatible live-tab metadata cannot overwrite privacy/store/Header/page/referrer. Bare unresolved tab IDs and Firefox privacy do not infer a default/container store. Exact-store queries, Header/Cookie gates, redirects and single/batch flows remain; no permission, credential store or unused IPC field. [Detail](extension-outcomes.md).
- **REF-03:** valid identity-bound confirmation admission remains `ok:true, accepted:false` with no task, and manual context/drop/popup/resource/media/Blob feedback says **确认窗口已打开，尚未创建任务** as normal pending. Invalid pending, refusal, timeout and disconnect remain failures. Pending resources retain handoff identity without a sent marker. Automatic downloads still require a durable matching receipt and successful browser cancellation; original compensation/stable replay remain. [Host boundary](host-outcomes.md).

Reference review used Mister first, ABDM `DefinedHandlers.kt` / `IntegrationHandlerImp.kt` next, then legacy NxTools compatibility. The independent ABDM extension source was unavailable locally. References supplied behavior only, not copied source/tests/assets. Existing input Link, server information, high-resolution icon and parallel ZIP work remains outside this slice.

## Acceptance-correction verification

Every Rust command used one serial Cargo lane and same-execution `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, scratch TEMP/TMP. Counts overlap and are not summed. Evidence logs are in the session scratch directory.

| Executed check | Current result | Evidence |
| --- | --- | --- |
| Recursive extension `*.test.js` suite | **170 passed / 0 failed** | `b1-repair-extension-final.log` |
| Cookie / manual / automatic reason groups | **21 / 44 / 32 passed**, included above | Production background events/API arguments/wire/status/ownership assertions |
| BrowserHost full library | **47 passed / 0 failed**, includes nine B1 cases and extended cross-language fixture | `b1-repair-host-full-final.log` |
| BrowserHost strict all-targets Clippy | **Passed** | `b1-repair-host-clippy.log` |
| REF-01 production GPUI | **4 passed / 0 failed**, including real explicit Copy tracing lifecycle | `b1-repair-ref01-green.log` |
| Adjacent polish / link-panel GPUI | **11 / 8 passed**, counts overlap REF-01 | `b1-repair-polish.log`, `b1-repair-link-panel.log` |
| Scoped Rust formatting | **Passed** for clipboard owner/test, link panel and host wire tests | `b1-repair-rustfmt.log` |
| Browser fixture JavaScript syntax | **Passed** | `node --check tests/handoff-popup-browser.mjs` |
| Spec / component catalog / Kit release / pinned Markdown gates | **Passed**:128documents/14owners; coherent0.8.0/pre0.3.9; approved vendor/editor boundaries | Architecture gate output |
| Download message contracts | **14 passed / 1 failed**, actual9/expected8 | `b1-repair-message-contract.log` |
| Desktop strict all-targets Clippy | **Failed** on two unrelated dirty-baseline diagnostics, listed below | `b1-repair-desktop-clippy.log` |
| Clipboard lifecycle source gate | **Failed** on the historical export-call token | `b1-repair-clipboard-lifecycle.log` |
| Supplementary popup/content DOM | **Incomplete; no current all-DOM pass claimed** | `b1-repair-popup-dom.log` |

The real extension/host fixture exchanges **seven framed business requests in five scenarios**. Firefox container/private automatic events and a manual drop reach the real engine confirmation consumer with exact originating Cookies. Pending means no task and no browser cancellation. A real QueueOnly receipt then drives original cancellation-failure compensation over the actual isolated current-user pipe/engine, same-ID replay and conflicting refusal; exactly one Cancelled task remains. Existing temporary SQLite/new-engine reopen coverage verifies durable replay. These are deterministic fixtures, not installed process/native-window acceptance.

### RED → GREEN evidence

| Correction | Observed RED before production change | Final GREEN |
| --- | --- | --- |
| Closed Chromium Cookie source | Normal/private Cookie absent; conflicting live tab selected another store or lost source Authorization |21Cookie event tests and170-test full suite |
| Manual pending | Valid host pending returned failure; Blob pending rejected; contradictory Blob acceptance bypassed classification |44manual event tests, automatic pending/no-cancel and real host manual roundtrip |
| Explicit Copy stale terminal | Real explicit click/replacement test failed: stale case had0same-ID terminal, expected1; clipboard/window/command guards already held |4REF-01 GPUI tests, including normal/stale/ownerless exact-once lifecycle |

RED logs: `b1-repair-cookie-closed-red.log`, `b1-repair-cookie-conflict-red.log`, `b1-repair-cookie-source-red.log`, `b1-repair-manual-red.log`, `b1-repair-blob-red.log`, `b1-repair-blob-invalid-red.log`, `b1-repair-copy-red.log`. Assertions observe production event effects and captured tracing, not source-token substitutes.

## Known issues and acceptance limits

1. **Message protocol baseline:** `message_contract.rs:158` expects8 while the product constant is9. Neither assertion nor protocol was changed.
2. **Desktop strict lint baseline:** `downloads/browser_handoff_impl.rs:436` reports `nonminimal_bool`; `downloads/settings_task_rem_tests.rs:603` reports `unwrap_used`. Both files were already dirty before this correction; neither was edited here. No suppression was added. The current run reports no new Copy-test/link-panel lint.
3. **Clipboard source-gate drift:** the gate expects `application.request_clipboard_export(cx)` while the baseline API carries `source_window_handle,cx`. Its early failure does not validate later link-panel source checks; the gate was not weakened.
4. **Full Desktop baseline:** the prior own full run recorded1788passed/8failed/1ignored, with missing native fixture exports and a download-context-menu expectation. It was not rerun for this scoped correction and is not current whole-workspace green evidence.
5. **Supplementary DOM:** the current popup/content run did not complete, and its isolated test space is no longer available for continuation. The earlier B1 baseline12-scenario/76-check DOM pass does not certify current feedback changes.
6. BrowserHost payloads that cannot deserialize retain the original generic error path; identity guarantees cover decoded submissions and subsequent failures.

No installed browser/Native Messaging registration, actual Shell/browser/confirmation HWND, real Windows clipboard, physical pointer/keyboard, IME/mixed-DPI/accessibility-provider, human visual, packaged or release acceptance was performed. Source gates, synthetic browser APIs, compiled GPUI TestPlatform, real temporary host/pipe/SQLite fixtures and native/installed acceptance remain distinct evidence levels.

The original B1 baseline also recorded Download IPC7/7, engine browser11/11, Clipboard48/48 and Core clipboard/link settings10/10. Those historical checks were not rerun or relabelled as current acceptance-correction results. B2 remains unstarted.
