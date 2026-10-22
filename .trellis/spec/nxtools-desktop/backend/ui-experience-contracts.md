# Unified document and experience contracts

Authority: approved2026-10-22 six-item snapshot; framework remains Kit0.8.0/defaultpre0.3.9. This supersedes historical no-editor/built-in-default-only presentation restrictions, not existing action/native ownership. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/markdown_document.rs`, `repo:rust/apps/nxtools-desktop/src/ui/markdown_edit.rs`, `repo:rust/apps/nxtools-desktop/src/ui/preview_image.rs`, `repo:rust/apps/nxtools-desktop/src/theme.rs`, `repo:rust/crates/nxtools-download/src/http.rs`.

## Scenario: Unified passive documents and explicit save
### 1. Scope / Trigger
Clipboard list/full preview/edit and Quick Launch text/Markdown presentation changes.
### 2. Signatures
`MarkdownDocument`, `DocumentMode::{Markdown,Literal(language)}`, `document`, `cached_view`; pinned editor `BlockEditor::passive(bool)` defaults false. `ClipboardPanel::{prepare_markdown_edit,markdown_edit_value,save_edit}`; `edit_generation`, `edit_save_pending: Option<Uuid>` guard async ownership.
### 3. Contracts
Same editor projection/Kit renderer across four surfaces. Passive skips focus/input/action/caret/grip editing ownership; links propagate to original list action, never browser/network. ImagePolicy::Placeholder, existing64KiB/1200line/depth32/node8192 safety budgets, mounted400char list summary/210DIP whole text card/8gap preserved. Overbudget falls back literal original text, never old TextView. Original action payload independent from summary/canonical editor source. No-edit/cancel preserve exact raw bytes; source save preserves CRLF, changed rich save may canonicalize with visible notice. Same content/mode cache avoids per-frame parse; source_snapshot is revision-cached. Cross-owner effects next-turn including cache hits. Save pending installed before original worker dispatch; duplicate pending save ignored. Completion must match edit generation/pending ID/item. Cancel/reopen retires UI ownership but does not undo an accepted durable worker write. Newer typing remains open after submitted value succeeds. Failure retains draft and retry; no payload/error Display in diagnostics.
### 4. Validation & Error Matrix
Unsafe/deep/oversize -> bounded literal fallback, no resource loader. IME composition -> no save. No changes -> close without write. Duplicate -> one operation. Old completion -> no new editor close/focus. Worker failure -> pending cleared/draft retained. Rich-mode format normalization is explicit, not applied to readonly data.
### 5. Good/Base/Bad Cases
Good: saveA, cancel/reopen draftB, A completes and B remains. Base: readonly Markdown/table/code displays and original full paste unchanged. Bad: readonly flag treated as passive, summary pasted, canonical text silently overwrites CRLF, or old save closes new draft.
### 6. Tests Required
Editor passive focus/default editing and Fenwick regression; four real host seams/full payload/link action, source/no-edit/cancel/save/failure/duplicate/stale, actual focus/IME/undo, long last-block scroll and resource privacy. Final editor234/Desktop1002 source tests; exact native Quick/Clipboard2each at96DPI separate from physical WindowsIME/mixedDPI/reader/user acceptance. Provenance57files allows only element.rs and exactly3cull.rs lowbit substitutions; no nightly/bootstrap/vendor expansion.
### 7. Wrong vs Correct
Wrong: `set_readonly(true)` in a result row; worker success unconditionally clears editing_item_id. Correct: explicit passive renderer plus original owner; current edit_generation/pending/item/draft gate before terminal UI publication.

## Scenario: Durable image modes and external launcher gaps
### 1. Scope / Trigger
Core presentation settings, Clipboard image layout and Quick Launch result/action rows.
### 2. Signatures
`ClipboardImageViewMode::{Auto,Fit,Panorama,Light,Mini,Compact}`, serde-defaultAuto; `QuickLaunchRowStyle::{Continuous,Separated}`, serde-defaultSeparated/gap0or4. `image_plan(source_pixels,available_DIP,dpi_scale,mode) -> ImagePlan`; `set_image_view_mode`, `set_row_style`.
### 3. Contracts
Official Settings/Page/Group/Item/dropdown field32DIP, original deferred app setter. Core locked section-field merge preserves latest storage/other owners; failed commit never publishes live mode. Auto uses workarea and aspect-aware overflow, Fit full/no scroll/no unnecessary upscale, Panorama sourcepx=devicepx, light/mini/compact whole-window shortedge<=800/500/300devicepx. Viewport/content separate; reset scroll on item/mode; bounded decode unchanged. `visible_bounds` uses rcWork, original adjacent right/left/clamp. One passive HWND through original hide/offscreen/generation/boundsACK/frame/reveal. Image list190/files62/sourceiconhue unchanged. Launcher interactive content40DIP, slots40/44; gap outside click/focus/hover, ordinary/recent/action rows share owner and scroll stride. No indexing/native service configuration for either presentation setting.
### 4. Validation & Error Matrix
Legacy missing fields -> Auto/Separated. Corrupt/unwritable config -> priorlive, retry notice. Tiny workarea -> existing bounded placement behavior, not claimed mixed-DPI acceptance. Overflow axis only scrolls; Fit never retains offset. Gap click -> no action/selection/preview. Old result/ACK -> original generation rejection.
### 5. Good/Base/Bad Cases
Good:125%DPI Panorama2400sourcepx produces2400devicepx content. Base:4DIP default can return to0. Bad: innerpadding as gap, caplongedge instead ofshortedge, full cached config overwrite or index restart for presentation.
### 6. Tests Required
Sixmodes×aspect/DPI math, actual GPUI longimage scroll/Fitreset/settingsdurablefail, Coredefaults/merge;0/4actualgap/noaction, persistfail/noIPC, fullQL153. Preserve preview lifecycle/native readback tests. Physical mixedDPI/actual wheel/native image-mode matrix remains separate.
### 7. Wrong vs Correct
Wrong: fixed440x340/fullscreen bounds or uniform40stride with4padding. Correct: sourcepixel/DIP/workarea plan plus original staged HWND;40content+external0/4slot.

## Scenario: Passive notification cadence and coherent themes
### 1. Scope / Trigger
Notification auxiliary WindowOptions and default theme selection.
### 2. Signatures
`notification_window_options(display_id)` sets inactive_frame_intervalNone. `tests/native/notification-cadence.ps1 -AuthorizeNative` requires existing Debugbinaries and scratch; Debug `preview_cadence` captures<=512paints/8milestones. ThemeRegistry::load_themes_from_str + Theme::update/change; project GraphiteLight/Dark config names.
### 3. Contracts
Official Toast remains sole animation/clock/stack/close owner. No new production frame clock/log; idle stops. Native hiddenprepare/readback/finaldraw/reveal and exit-beforehide retained. Debug legacycap only when explicit isolated Cadence global+env enabled; Release lacks sampler. Native production preference calls App::set_reduce_motion; fixture must do same, not only businessglobal. Theme registration does not replace Registry defaults; app explicitly chooses project configs for Light/Dark/System. Optional official themes unchanged; current Kitcolors -> renderabletokens/Base/businessprojection. Keep fonts/radius/geometry/sourceiconRGB.
### 4. Validation & Error Matrix
Default33333us -> source/config regression. Invalid native readback/stalegeneration -> no reveal. Missing effective refresh/sample -> report unavailable, not smoothnessPASS. Unknown keys -> schema gate fail. RGB/HSL roundtrip -> channel tolerance1e-5; original Kitrole/token equality exact.
### 5. Good/Base/Bad Cases
Good:180Hz currentpaint~5.5ms vslegacy~34ms, idle0; reducedentrance1frame. Base: same optionaltheme retains wholepalette. Bad: recursive frame sampling drivesidle, hardcode60FPS, recolorcustomcanvas while KitInput/Menu staysold, or screenshots treated as userapproval.
### 6. Tests Required
Notification35 plus actual ownednative boundedcadence/reduced/foreground/readback. Theme11+optional1,112schema/statecontrastroles each, actual96DPI dark/light0/4GPUIcaptures. Source/compiled/native/AIvisual/user/installed evidence reported separately. Upstream editor serialize.rs Clippy nonminimal_bool remains unmodified/outsideapproval; strict projectpackages use --no-deps and disclose unrestrictedlintfailure.
### 7. Wrong vs Correct
Wrong: configtest provesperceivedsmoothness; is_defaultJSON replacesregistrydefault. Correct: productionoptions + boundednativepaintcomparison; explicitregisteredconfigselection via publicThemeupdate/change.
