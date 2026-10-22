# Installed UI screenshot feedback correction — 2026-10-12

## Scope
User screenshots authorize unclipped selected module borders; readable full Downloads filter titles/counts; proxy selector/input height and title-start alignment; title-adjacent help; right-aligned standalone Switch rows; bounded sort popup; no Quick Find search focus highlight; Clipboard search/history breathing room; Tools tile grid. Existing domain, save/cancel/busy, focus/input/IME, keyboard/selection/drag and native owners remain. No feature, dependency, palette, native size, Release package or installation changes.

## Implemented source
- Sidebar consumer menu adds4px outline clearance within original176px rail, real Kit Buttons/original keyed focus/click retained.
- Downloads filter minimum576px with content-based reserve and bounded horizontal scroll only when necessary; sort remains adjacent and uses released PopupMenu160..180px public limits. Six filters/count formatting/selection unchanged.
- Shared `aligned_setting_row` in Settings public render slots: title then original help Tooltip; independent control lane aligned right. Downloads/Global/Quick settings/Clipboard use it, visible risk/status copy retained. Controls retain natural geometry, no stretched Switch. Input-adjacent proxy ButtonGroup uses releasedMedium32px; compact toolbar keepsSmall24px.
- Proxy address starts at mode-title edge; blacklist retains separate original padding/budget.
- Quick Find primary search uses real released Input::prefix and focus_bordered(false), same InputState32px/42pxheader/client. InputGroup::focus_ring(false) alone did not suppress border brightening; narrow public Input adaptation meets explicit feedback. Outer inset pointer focus retains icon-click focus; no new query/key dispatcher.
- Clipboard search retains realSmall24px InputGroup and adds8px list separation.
- Tools only registered Link Detection uses180x160 tile/top icon/title/status/Configure, left-start wrapping grid. Original sole focused_action/keyed settings owner retained. No fake DNS/network/etc tools added.
- Structural gates updated from obsolete SettingItem::new/SettingField source recipes to actual public render/shared-row contracts, retaining feature/native checks.

## Verification
Pre-edit genuine production geometry reds: Downloads Switchx540/28x16; GlobalSwitchx372/36x20 violate user right-edge requirement. Initial full622pass/3fail resolved: blacklist padding regression; superseded24px proxy assertion; prefix focus regression. Later625pass/1fail caught elastic filter lane stranding sort; max-width restores adjacency. Compilation/import errors and stale/unseen Edit anchors are not behavior-red evidence.

Final fullDesktop626/626,0failed/ignored; strict desktop--testsClippy,workspace--testscheck,fmt,Debugbuild0.11affected sourcegates0. Actual production tests include proxy32px/rightedge/title-help adjacency/hoststart, naturalSwitchwidth, Clipboard>=8pxgap, sixfilter>=95px/readable sortMenu160..182px, tile geometry/order/centers/keyedaction and original broad lifecycle regressions.

Owned nativeQuick2/2 andClipboard2/2 pass at96DPI, noSKIP; performed before final filter-only change. Validates owned client/UIA/passive mouse-message policy, not physical/mixed-DPI/installed behavior. No real business or service operation.

45unique PNG captured from samefinalDebugbuild SHA256 `E754B1EDC2A47400F9F4EA8D878718FDD1BF0ECCD973320DBEBFD56E9D4FDB36`, each checkedPID/HWND/client/DPI andimagehash. Gallery: [actual screenshots](../../artifacts/ui-installed-feedback/index.html); [evidence](../../artifacts/ui-installed-feedback/evidence.json). All45 are **uninspected**: Browser reports `Browser view did not become visible for the calling session` onboth attempts. User asked toswitchback; no visual success/userapproval claim. Source/test/ownednative evidence is not aesthetic certification.

Specs87docs/12owners pass; scoped diff check emits only existing CRLF normalization warning, no whitespace error output. No Cargo/rustc/ownedDebugfixture survivors. No Release repack/install/currentdesktopshutdown/service mutation/commit/push. Existing installer still predates these UI edits.

## Remaining blocker
Actually inspect updated default/minimum/settings screenshots and fix any concrete visual defect once Browser becomes visible. Sort-open native screenshot, physical focus and supported DPI matrix remain separate. Do not call the UI fully accepted until inspection/user feedback.
