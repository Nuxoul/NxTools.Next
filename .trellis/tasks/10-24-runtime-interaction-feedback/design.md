# Runtime feedback design

## Approved boundary
Follow `.pi/plan/runtime-interaction-feedback-20261024.md` (user approved). Rust owns state/layout and generation, GPUI rendering, C# Windows API. Existing ClipboardPanel/Application/menu/PreviewSession, floating controller/native STA, main creation and Settings owners remain.

## Clipboard
Small ghost official Toggle immediately left of existing Small ghost Recording. Per-opening bool, no Core config. Skip only automatic foreground/pointer/paste hides; explicit Esc/summon/hide stays. Successful paste retains existing native target restoration and never uses CopyOnly panel activation. Menu open cancels preview; generation/session/query/admission checked before delayed/cache reveal. Source-panel vs external dismiss classification belongs to native_context_menu, retiring pending before deferred cross-owner updates. Consume source dismiss gesture.

## Picker/startup
Trace legacy notes and full current native chain. Isolated failing tests must reach actual failure seam before causal patch. Retain one bounded STA/dialog owner, distinguish cancellation/timeout/busy/native source failure, no claim logical retirement cancels native work. Main-window logical bounds/display/create/show/DPI tests preserve user resizing; no vendor patch authorized.

## Settings/UI
Reuse full Settings/Form/Input/Button/ListItem compounds. System snapshot is read-only, generation-scoped and independent of manual draft and immutable NetworkPolicy. C# owns native read. No PAC network evaluation for display. Shared tile status uses original actual enabled states/theme tokens. Reorder blacklist fields at existing owner. Firefox Small outlined button uses existing correlated Shell open, exact supplied URL. Download screenshot must be identified before layout modification; Toolbar child still ghost+compact.

## Diagnostics
Low-noise correlation/session/generation accepted/start/terminal/elapsed/native code. Pin transitions; menu inside/external reason; picker cancel/timeout/source/native result; bounded startup dimensions/DPI transitions; proxy result kind/count. No payload/path/URL/query/header/cookie/raw exception logs. Pending-before-dispatch, current identity on apply, defer cross-owner effects including cache hits.

## Compatibility/rollback
No schema changes for pin. New typed proxy command only if no reusable boundary exists; keep protocol bounds and absent-capability failure. Revert only owned hunks if necessary, never baseline restore over peer edits. No deployment.
