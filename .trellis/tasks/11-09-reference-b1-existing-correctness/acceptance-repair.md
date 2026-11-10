# B1 acceptance corrections

Status: complete at the source/fixture level. All three targeted production-boundary corrections pass. Current results, logs and whole-workspace/DOM/native limits are recorded in [outcomes](outcomes.md).

## Corrected boundaries

1. **Originating Cookie context:** production webRequest/download listeners, bounded snapshots and `cookieContextForSource` retain observed Chromium normal/private identity after source-tab closure. Later native tab lookup supplements only compatible source facts. Chromium0/1 mapping uses an existing enumerated store on the filename-interception-capable browser path, never Firefox container inference or a bare unresolved tab ID. Header/Cookie gates, exact-store queries, redirects and resource/batch context remain.
2. **Manual confirmation:** existing `handoffOutcome` admits only valid original identity-bound `ok:true/accepted:false`, no task, `awaiting_confirmation/confirmation_pending` without queue admission. URL/Blob wrappers and context/drop/popup/resource/media consumers show **确认窗口已打开，尚未创建任务** as normal pending. Pending resource retries retain original identity without sent markers. Refusal/invalid pending/identity/timeout/disconnect fail. Automatic browser ownership still requires matching durable acceptance and successful cancellation; compensation and persisted replay are unchanged.
3. **Explicit Copy diagnostics:** the original Button closes locally before deferred owner dispatch. The window guard prevents stale writes and emits exactly one `copy_cancelled` with `panel_replaced`, original correlation and elapsed. Normal and ownerless Copy each complete once. No payload, extra turn/retry or synchronous cross-entity update.

## Production test seams

- Real background webRequest/tab-removal/download/context-menu/runtime events through `load-background.cjs`, exact Cookie API parameters, outbound payloads, pending feedback/status and browser ownership.
- Existing real extension/BrowserHost framing, isolated current-user pipe, engine confirmation, actual compensation, same-ID replay/refusal and temporary SQLite reopen.
- Actual GPUI explicit Copy click; replacement established by the window-closed observer before deferred work, original TestPlatform clipboard/typed platform command boundary and captured tracing.

## Component selection record

The requested surface remains one420-DIP link panel with28-DIP rows. Locked Component0.8.0 public `gpui_kit::component::list::ListItem::on_click` and `gpui_kit::component::button::Button::on_click`, `.xsmall().ghost()` are retained. Exact released `list/list_item.rs` and `button/button.rs` callback/render sources were reviewed after component-selection/table/API/release guidance. Existing Label/Icon/StatusBar and accessible names stay; direct layout children introduce no Toolbar ghost/compact adaptation. The correction changes diagnostics only, not layout, focus, IME, placement or timer/generation owners. Compiled interaction does not certify native/visual acceptance.

## Verification

RED was observed at each reported boundary before production correction. Final extension suite **170/170** includes21Cookie,44manual and32automatic reason tests. BrowserHost **47/47** includes the seven-frame/five-scenario cross-language fixture. REF-01 **4/4** and adjacent polish11/11/link-panel8/8 pass; Host strict Clippy, scoped Rust formatting and four architecture/spec gates pass.

Message contracts retain the expected8/product9 failure; Desktop Clippy retains two unrelated initial-dirty-file diagnostics; Clipboard source gate retains its old export-call token failure. Supplementary popup/content DOM did not complete. No full historical archive rerun, package/install/product/reference launch, registry/service change, commit/push or B2 implementation. Native/installed/physical/visual acceptance remains unperformed.
