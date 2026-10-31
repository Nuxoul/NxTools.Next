# Scoped Chrome store upload and exact identity repair

## Authority and outcome
User explicitly requests ONLY (1) remove the store-upload manifest key problem and (2) store ID / desktop NativeMessaging allowlist matching. All other audit findings are out of scope. Initial upload failure means the store ID does not exist yet; the first ZIP must not require it. No further question/approval required. No delegation/contact, commit/push/archive, desktop Release package/install/service/live registry/browser/clipboard/history/config operation. User-authorized extension ZIP is generated as a deliverable; current installed desktop is not replaced.

## Source and owner boundaries
Read ABDM BrowserMessageRequest.kt(identity seam only), then legacy BrowserExtensionPackageService ExportCrx/ExportFirefox/CreateCrx3Bytes/PEM ownership, current reference/locked UI sources. Original local identity/signature/Firefox must survive. Baseline `chrome-store-fix-baseline/` stores10 source files and initial dirty status. Preserve concurrent Archive/floating assistant/notification/release work.

- C# BrowserExtensionPackageService: additive `chrome_store` format normalized by original native adapter and save dialog. Plain ZIP/root manifest copy removes only `key`, forces .zip, does not call signer/read/write PEM or change source. CRX/XPI remain original. No permission/CSP/media/privacy changes.
- Core DownloadSettings.chrome_web_store_extension_id: defaultempty/legacy-compatible, trim then exactly32a–p or empty. Malformed loaded value clears to local fallback. UI rejects invalid draft before persistence.
- Existing DownloadSettingsView General/browser group: original official Settings custom slots, one32DIP Input; third official Small outlined export Button, noToolbar.child ghost adaptation. Existing draft/Cancel/Save owner; empty ID does not block ZIP export. Config durable section merge preserves other sections; failed save never registers new ID.
- Existing EnsureBrowserNativeMessaging payload carries configured store ID or preserved local default. No protocol fields/new command. Unchanged C# registrar unions exact command/pinned/source/existing-exported IDs; unchanged BrowserHost validates registrar origins. No wildcard, page-supplied auto trust or invented storeID.
- Register on durable store-ID save/retry and reconnect. Pending caller ID installed before observable dispatch. Matching current ACK/disconnect/15s logical deadline retires pending; superseded ACK/deadline ignored. Failure preserves saved intent with deferred original notification guidance. Native work already executing is not claimed cancelled. Existing low-noise correlation/stage/nativecode/elapsed diagnostics only; no ID/key/path/payload telemetry.

## Reds and causal intermediate failures
- `chrome-store-red-export.log`: exact existing audited signing fixture1RUN/0PASS/1FAIL, additive storeformat incorrectly fell through CRX signer and created fixture PEM.
- `chrome-store-red-registration.log`: actual application reconnect1testFAIL; configured syntheticstoreID still dispatched pinned localID.
- Initial first-build formatting stopped before compilation for misplaced parenthesis in new button chain; narrow corrections applied. Rust then reported submodule helper visibility4errors, fixed to pub(super). Core full settings test required explicit new default field, corrected. These were causal in-scope construction errors, not successful tests.
- Export UI intermediate1FAIL/1PASS: added thirdbutton worked but final historical notificationcount remained2; updated to3distinctinvocations. Final actual3button test passes.

## Checks actually run
All Rust commands CARGO_BUILD_JOBS=1/CARGO_INCREMENTAL=0/RUSTFLAGS=-C codegen-units=256, serial preflight. No process killed/contacted. No review delegation; direct baseline source review.

| Check | Actual result |
| --- | --- |
| C# PlatformHost.Tests build |PASS,0warnings/0errors |
| Exact reviewed browser signing fixtures |LIST7/RUN7/PASS7/SKIP0/FAIL0 |
| Core optionalID/legacy/validation/normalize/roundtrip |1/1PASS |
| Core full --lib |83/83PASS |
| Desktop chrome_store_ |4/4PASS |
| Desktop browser_export_ |2/2PASS (three actual exportbuttons plus invocation/result isolation) |
| BrowserHost full --lib |31/31PASS incl exactregistrar-origin acceptance/rejection |
| Strict Core/Desktop Clippy --tests -Dwarnings incldeps |PASS |
| Workspace check --workspace --tests |PASS |
| One final fullDesktop |**1106PASS/1FAIL**,1107total,0ignored,147.20s; NOT GREEN |
| Static docs/provenance/lifecycle |**10PASS/1FAIL** of11; remaining listed below |
| Actual storeZIP independentverification |PASS:rootmanifest/nokey/onlykeyremoved/sourceunchanged/63otherfilesbyteidentical |

FullDesktop failure: downloads::tests::production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle at tests.rs1376, expected next-day2026-10-25 but observed2026-10-24. That pre-existing date-picker test body and production schedule code were not changed by this repair. No isolated/full rerun, date fix or green hunting. Keep this full result failing; do not borrow prior1042PASS.

Static failure: browser-native-messaging.ps1 exact literal `focus: launch_mode.main_window_focus()` missing from its `$desktopMain` source assertion204–205, message `A browser confirmation cold start must not let the main window claim focus.` That main/startup source is concurrent/unmodified by this repair. Did not weaken/fix gate or infer native focus behavior from regex. Other10 actually ran/passed: spec-contracts,component-catalog,gpui-release,markdown-editor,signing-isolation,test-selection,download-ui-lifecycle,ui-state-lifecycle,desktop-shell-lifecycle,ui-consistency.

## Deliverable and independent validation
`artifacts/browser-store/NxTools-Chrome-Web-Store-0.7.0.zip`
- 206089bytes,64entries,root manifest has no key.
- SHA256 **AAC79CF98E6F30028E8CBEE882ACB2CD9365623F013676F574C7AD6FDB8D4633**.
- Parsed manifest equals current source manifest after removing ONLY key.
- All other63entries equal source bytes; noPEM/PFX/P12/EXE/DLL.
- Current source manifest byte-identical to this repair's saved baseline; local identity retained.
- Generated using scratch one-shot caller of actual production public exporter; no host/native registration/product main invoked. Scratch caller emits WindowsBase reference-conflict warning on its build only; actual export succeeds and independent bytes validated. Production C# testproject build has0warnings/errors. Regenerated ZIP afterREADME update, exact finalhash above.

## Remaining and instructions
Upload the ZIP now, without any store ID. When Chrome creates the item and supplies its ID, use the new desktop version's Download Settings -> General -> Browser Extension -> Chrome Web Store ID and Save; then reopen extension/test connection. Empty optionalID stays local-compatible. No actual ID can be prefilled today and no store acceptance claim is made. Desktop source is repaired but installed executable unchanged; no desktoppackaging/deployment standingpermission. Other auditissues explicitly untouched. Physical/UIA/DPI/uservisual/storeupload/native installed connection acceptance NOTRUN.

Logs in current scratch: chrome-store-red-{build,export,registration}.log; focused-core.log/focused-identity.log/focused-export-ui.log; focused-chrome_store_-final.log/focused-browser_export_-final.log; signing-build.log and actual7fixturestdout; final-{core-full,browser-host,clippy,workspace,desktop-full}.log; static-results.json/pergate logs; chrome-store-zip-evidence.json, verify-chrome-store-zip.py; same chrome-store prefix unless already explicit. Original broad readonly audit/tests are not repaired beyond two items.
