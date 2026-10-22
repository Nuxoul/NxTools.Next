# GPUI Kit post-gallery feedback — 2026-10-12

## Decisions and implemented scope
This amendment supersedes the matching queue03/05/07/category-master deferred decisions, not canonical stage order or unrelated dirty work.

- Downloads task grid: original virtual list temporarily retained by user, for a later separately authorized package review. No packaging now.
- Daily schedule: actual released TimeField/TimeFieldState, H23/minute, original enable flags/weekdays/Apply validation/cross-midnight/Cancel/busy/error owner. User explicitly supersedes raw-invalid HH:MM text retention. Actual focused digit KeyDown/KeyUp2340 edits23:40; programmatic01:15 + missing-weekday error + Monday Apply passes. Complete disabled/focus/edit state matrix remains unverified.
- Downloads Settings category master: real stateless ListItem/Label on original draft/selection/order/actions. Existing full production editor test passes including destination invalidation/full Tooltip, enabled state, pointer selection, order/add/remove/persistence. Original40px row and native984x650 retained. Test-only old compact row remains cfg(test), no production caller.
- Clipboard categories: actual XSmall underline Tab, original stable keyed focus, sole on_click selects once and restores pointer focus. BaseTab stops Left MouseDown; drag initiation is moved to the same real Tab, original drop/strip wheel/context remain. Pointer-focus/repaint/Space down-no-action/up-one-generation passes. Real pointer-down/move starts CategoryDrag and cancel passes; long categories wheel/right menu passes. Complete Tab traversal/held/modifier/actual drop-reorder acceptance remains unverified.
- Clipboard search: actual official Small InputGroup24px with original InputState/IME/query owner; no cropped32px control/global font scaling.
- Layout correction: actual old Global General PNG was inspected and duplicate left SettingItem/right Switch labels identified. Switch now uses accessibility_label only, preserving complete semantic names and original callbacks. Real36x20/control-width regression red -> green. No blind whole-app spacing rewrite.
- Browser-restored visual follow-up: Downloads ordinary field lane440px and Global lane360px bring short controls nearer labels without stretching Switches; wide category/request/FFmpeg/path content stays unchanged. Official Form uses4columns/address span3/port1. Schedule32px controls now occupy32px rather than44px slots, reducing measured time-row gap20px to8px. Palette/type/radius/fixed clients/action/save/native owners retained.

## Calendar sizing — measured public recipes
Released0.7.1 Calendar supports Styled outer sizing/padding and official Small/Medium/Large. Date cells are28/32/40px; custom Size uses the default32px branch.

`feedback_calendar_public_size_and_padding_measure_natural_geometry` measured natural geometry (no cross-axis stretch):

| Public composition | Natural size |
| --- | --- |
| Small,12px padding |220x224|
| Small,0px padding |220x200|
| Medium,0px padding |248x228|
| Custom Size20px,0px padding |248x228|

Thus it **can** change outer size/padding and official tiers, but these APIs do not arbitrarily shrink date cells. Zero-padding Small still exceeds original156px date area in304x240 schedule popover. Scroll/reallocation/DatePicker alternatives remain unverified, not universally rejected. Original compact date grid is unchanged; no window inflation, font/rem scaling or dependency patch.

## Notification explanation
The existing manager must receive a close intent for the correct ID/generation so an older notification cannot dismiss a same-ID replacement and timer/list state stays synchronized. `on_close` is not inherently required; it is one possible adapter seam. Prior standalone official dismiss recipe's private completion behavior is narrow evidence, not universal Notification incompatibility. Current Alert presentation/original bounded manager remain unchanged; no second notification manager.

## Verification and causal failures
All Rust commands serial, jobs1/incremental0/codegen-units256, timeout21600s. Latest tests/strict/Debug build logs `ui-balance-final2-*`;14gates in `ui-balance-final2-gate-*`.
- Full Desktop **625/625**,0failed/ignored. Global10/10, full category editor1/1 and new Download control position1/1 pass.
- Strict tests/production Clippy, workspace --tests check, fmt --check, Debug build: exit0.
- Fourteen affected source architecture gates pass. Only retired Clipboard font-expression and user-superseded schedule Input/raw-policy anchors updated; native/action/lifecycle requirements retained.
- Safe exact-filter native Quick All2/2 and Clipboard All2/2 pass, no SKIP. Owned96DPI passive message result3, ordinary/policy-disabled1, client400x410/460x406, foreground unchanged/UIA controls. Not physical/high-DPI/reader/installed certification.
- Production regression reds: Tab pointer not selecting; pointer focus stayed search; Global duplicate Switch geometry. Compile/API errors and unsupported focus observers are excluded from causal behavior-red evidence.
- Full initial622/623 failed retired category marker, then full622/623 reached missing Tooltip observation. Stable full-directory Tooltip restored, real hover timer frames registered; disabled-label precise-bottom edge assertions changed to inclusive containment without weakening40px budget. Full editor and final623 pass.

## Updated captures and acceptance limits — Browser restored
[Feedback gallery](../../artifacts/ui-kit-feedback/index.html), [evidence](../../artifacts/ui-kit-feedback/evidence.json): **30 unique real owned HWND PNGs, all30 actually viewed** in bounded Browser screenshot groups after latest changes.14Downloads/10Clipboard/6Global,96DPI exact client/pixel sizes; single Debug SHA256 `7A40B129F70B3FAF7967339C7EE3870BE4D17BDBAE54005EBD4D98DF7A8B714F`. Scratch/memory/disconnected fixtures only; no business action clicked. Prior30-build baseline preserved in session scratch, old50 gallery unchanged.

Browser visibility blocker is resolved after user switched back. Actual inspection confirmed improved control lanes/host-port proportion/schedule rhythm and intact fixed footers/complex content. Clipboard24px search reviewed; bright outer edge is official InputGroup focus styling, not a custom second border. Remaining missing states, full pointer/keyboard/drag matrix, mixed/highDPI, physical/reader/installed acceptance and user visual approval are still open. Not whole-migration completion.

Old [50-image gallery](../../artifacts/ui-kit-final-acceptance/index.html) remains historical build `FA591A...`; it is not current after these authorized product changes. Whole migration, missing states, mixed/physical DPI, reader and installed acceptance remain open. No Release/package/install/services/indexer/real-volume/user-data/ordinary startup/delegation/commit/push.

Visual follow-up causal reds: GlobalSwitch x668, DownloadsSwitch x924, time-row gap20px; bounded lanes and32px slots fix them. Intermediate GlobalSwitch wrapper stretched332px; restored natural flex width without weakening original36px assertion. Final625/625/strict/build/14gates pass; original business methods untouched by these layout-only edits.

