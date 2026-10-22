# Purpose and usage UI slice

## Scope and invariants
- Owner: only `extensions_page/workspace.rs`, `extensions_settings.rs`, new `extensions_page/usage.rs`, and the usage module declaration in `extensions_page.rs`. Parent owns activity, application/runtime, clipboard/history and verification serialization.
- Mission: explain what a package declares, where each existing handler is used, and the saved readiness/opt-in state before presenting access decisions. Manifest descriptions are author-provided, not host-verified capabilities. Missing descriptions say so.
- Preserve Kit Default Light/Dark, installed CJK/mono fonts, 1060x650/default, 920x480/minimum, 176px main rail, six detail page indices (usage/access 0; environment 5), original InputState identities/drafts, Save/Cancel, command actions and deferred callbacks. No new run dispatcher, universal Run button, environment probe, automatic subscription or session activity UI.
- Preserve original lifecycle/native boundaries: presentation does no filesystem/native IO or cross-owner updates; existing callbacks still defer. Back retains drafts; Cancel explicitly discards. Trust hash equality is only a saved-record projection, never proof of a live verified interpreter.
- Layout: natural-height text, 4/8/12/16px existing rhythm, elastic `min_w_0` text and nonshrinking Medium32 action rail. No forced line clamp, clipping workaround, new palette/type/radius or decorative motion. Existing Kit controls retain keyboard focus/disabled/hover grammar; no new animation/timer.

## Mandatory reference and component review
Complete `docs/reference-code/gpui-kit-component-index.md` (377 lines) read before implementation. Reference note `docs/reference-code/winshun.md` read first, then read-only `D:/Code/WinShun-main/src/app/PluginSettings.cpp` card description/keywords projection and `src/plugin/Manifest.h` declarations. No reference code copied, plugin executed or new native lifecycle introduced. The user-requested human trigger explanation is new behavior; Next's public schema and actual dispatchers, not WinShun's different schema, determine the copy.

Published release source inspected under `C:/Users/Administrator/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.7.1/src/`:

| Surface | Exact released API / chain | Decision and compatibility |
| --- | --- | --- |
| Detail hierarchy | `setting/settings.rs::Settings::{new,pages,default_selected_index,header_style,sidebar_width,sidebar_size_range}` -> internally keyed SettingsState/filter -> `SettingPage::render` -> `SettingGroup::render` -> `SettingItem::render_item` | Retain actual Settings compound; add usage as separate groups to page 0. `default_selected_index` remains initialization only, never a fake controlled setter; environment stays page 5. Keep existing public hidden search-header refinement. |
| Usage groups | `setting/page.rs::SettingPage::{new,resettable,groups}`; `setting/group.rs::SettingGroup::{new,title,item,items}`; `setting/item.rs::SettingItem::{render,keywords}` | Adapt through public custom-content items for read-only purpose/handler instructions; independent searchable items, not entire legacy page in one slot. `SettingItem::description` does nothing for custom Element items, so render actual text and supply keywords. No reset semantics. |
| Read-only metadata candidate | `description_list.rs::DescriptionList::{new,vertical,columns,bordered,label_width,item}`; `DescriptionText::AnyElement` | Reviewed before free composition. Retain existing metadata DescriptionList. Narrative purpose and per-handler instructions are not a key-value table; place wrapping text in the existing Settings custom-item seam, without fixed label columns or duplicating a surface. No unsupported-capability claim. |
| Package overview | `list/list_item.rs::ListItem::{new,accessibility_label}` + ParentElement children; released render owns hover/list grammar, natural content | Retain stateless ListItem and original search InputState/controller. Add author description and each handler's actual trigger plus intent/readiness. No new List/Command controller or row activation. |
| Actions | `toolbar.rs::Toolbar::{new,content,child}` and `Sizable::with_size(Size::Medium)`; released toolbar applies final size to child controls | Retain Medium at compound owner and original Buttons/deferred handlers. Overview row action area does not shrink. All existing text actions remain default32. |

No new Kit dependency, Base fallback, page-wide exception, custom leaf control or native owner.

## Verified public product schema and dispatch copy
- `nxtools-extensions/src/lib.rs::Manifest::description` and `Handler::{feature,entry,title,keywords,url_matches}`; `registry.rs::InstalledPackage::keywords` includes saved keyword overrides.
- `script.rs::UrlMatch` publicly exposes `host`, `include_subdomains`, `path`, `priority`; `UrlMatch::matches` matches HTTP(S) host/subdomains plus wildcard path, not a path prefix. `resolver_routing.rs::PackageRegistry::resolver_route` applies saved host chains/preferences before declaration ranking. Show declared host/path rules and explain that an explicit download chain can select an otherwise unmatched handler; never infer supported media/output.
- `quick_launch/extension_providers.rs` accepts exact keyword or keyword followed by a space; ordinary provider participation independently checks `enabled_providers`. Do not claim that this opt-in blocks explicit keywords.
- `ui/history.rs` actual menu title is `扩展处理…`; required instruction is exactly `剪贴板历史 → 条目菜单 → 扩展处理…`, then the declared action title. Do not invent accepted content types or derived output.
- Events are localized by feature/entry: download created/completed/failed and clipboard committed. Intent comes from `enabled_events`; circuit pause uses existing `event_paused`, not guessed runtime status. Tools show existing named actions, including Items titles.

## Regression order and parent handoff
Added production-render regressions to `extensions_settings.rs` BEFORE changing presentation or filtering:
1. `extension_purpose_and_clipboard_usage_render_in_overview_and_first_detail_page`: synthetic existing manager fixture, no service/broker; asserts content-bearing debug bounds of real purpose/clipboard instruction, purpose-before-use ordering, default32 manage action, page0 usage label and stable environment page5.
2. `extension_overview_search_matches_author_description_at_real_render`: description-only query, asserts the actual current-frame Button snapshot remains rendered (not cached debug-bound presence).
3. `extension_usage_copy_uses_actual_entries_and_independent_intents`: missing description, exact clipboard path, localized event trigger, independent event/provider intent and pause, saved keyword overrides, named Tools Items and no unverified runtime-ready claim.
4. `extension_usage_resolver_describes_public_host_path_not_invented_output`: absent-rule guidance and public host/subdomain/wildcard-path projection without fabricated media/output capabilities.

These regressions have NOT been run. Behavioral red is pending parent serial Cargo verification; source addition is not red evidence. Parent can run the new tests against the pre-presentation files to obtain a real red, then restore the scoped changes. Do not mistake compile failure for behavioral red.

## Verification status
- No Cargo, product launch, real plugin execution, user clipboard, package/install/service operation performed by this slice.
- Parent must run focused new render/copy/search tests, existing W9 trust/settings/automation/chain and draft-retention tests, strict compilation/lint, fmt, and isolated actual GPUI default/minimum Light/Dark captures. Existing gallery is baseline only, not a post-change screenshot.
- BrowserPreview cannot render this Rust/GPUI change or supply DOM/keyboard/reduced-motion interaction. No HTML mock or out-of-scope gallery rewrite is introduced. Actual changed-render visual, keyboard, reduced-motion, DPI and installed acceptance remain pending parent.

## Scoped handoff outcome
- Added `usage` module declaration only to `extensions_page.rs`; no other edit to that parent-owned file. Shared read-only methods use existing public `InstalledPackage`, `Handler` and `event_paused` APIs; no parent application API changes are needed.
- Overview retains original actions but uses two wrapping lines per handler (localized heading plus state, then actual entry instruction), after purpose/readiness. Description, localized entries and instructions participate in the original search predicate. First detail page is `用途与启用`; usage is before the unchanged warning/action group. Automation titles/descriptions now use the same localized entry projection; callback bodies are unchanged. Commands keep named Tools actions and keyword editing with added entry instructions.
- Existing test caller in `extensions_page.rs` still asks for `信任与启用`; the owned Settings test helper maps that historical title to page0 rather than editing the parent file. Actual production sidebar label and indices are explicitly asserted.
- Non-Cargo checks PASS: direct `rustfmt --edition 2024 --config skip_children=true` formatting and `--check` on the four Rust paths; `python tests/architecture/spec-contracts.py` (93 documents / 13 owners); scoped UTF-8/trailing-whitespace, six Settings page-order, usage-before-warning, exact clipboard path and presentation-only boundary assertions. Initial rustfmt caught a closing-parenthesis typo in the added test; fixed before final syntax/format check. No claim of typecheck/test execution.
- `git status --short` confirms these extension source paths were already untracked in the parent's dirty tree. `git diff --check -- <owned paths>` therefore cannot establish their content cleanliness by itself; the explicit file whitespace check and direct rustfmt check cover this slice without staging or disturbing the tree.
- BrowserPreview requested existing `artifacts/extension-trust-ui/index.html` after the first visual edit. Tool returned a tab-open request only; it did not expose a screenshot or render the changed Rust. Baseline gallery is not post-change evidence, and no actual current-render visual inspection is claimed. Actual GPUI default/minimum Light/Dark capture plus keyboard/reduced-motion checks remain parent-owned/pending because this slice is forbidden Cargo/product launch and has no browser-E2E target.

Parent verification commands (serialize, do not run in this subagent):
```powershell
$env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
cargo test -p nxtools-desktop extension_purpose_and_clipboard_usage_render_in_overview_and_first_detail_page -- --test-threads=1
cargo test -p nxtools-desktop extension_overview_search_matches_author_description_at_real_render -- --test-threads=1
cargo test -p nxtools-desktop extension_usage_ -- --test-threads=1
cargo test -p nxtools-desktop main_window::extensions_page -- --test-threads=1
```
Keep existing strict-lint/workspace/fmt and isolated preview verification in the parent's sequence. No activity widget/slot was added; parent can insert its separate activity module without coordinating a second state owner here.
