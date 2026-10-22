# GPUI Kit component and visual migration

## Goal
Replace the legacy-looking desktop control layer with actual GPUI Kit 0.7.1 styled components and coherent composition. The user must see and interact with the component migration, not merely receive a dependency/API upgrade.

## Background and confirmed facts
- The user rejected the 15:12 formal acceptance package because the interface still resembles the previous UI, and referenced https://gpui-kit.com/zh-CN/component/.
- The earlier exact Kit 0.7.1 dependency/API migration is real, but its claim of complete UI migration was incorrect. The existing 519 passing desktop tests do not certify component adoption or visual results.
- Production uses Kit Button/Input/Textarea/Select/Checkbox/Popover/Tooltip and standalone menus. Verified adoption gaps: `field.rs:111-114` disables Input appearance/border; `action.rs:121-156` reconstructs custom button states; `theme.rs:101-154` remaps extensive Kit colors and typography/radius; `settings.rs:336-407` draws a custom toggle; `list.rs:17-142` draws custom rows. Complete consumer evidence is in `adoption-ledger.md`.
- Source-backed feature/native contracts, uncommitted work and the exact Kit/backend version graph must survive. The explicitly approved minimal local Windows 0.3.8 passive-policy patch remains in place.

## Requirements
- **R1 — Actual adoption:** Use released Kit 0.7.1 styled controls where compatible equivalents exist. Remove redundant legacy appearance suppression/custom chrome, rather than changing imports only. Do not force unrelated showcase components into product workflows.
- **R2 — Coverage and exceptions:** Inventory every desktop control/surface as adopted, needing replacement, or a justified custom exception with a source-backed reason. A native lifecycle incompatibility is a narrow exception, not permission for blanket custom wrappers.
- **R3 — Feature preservation and order:** Original stage titles stay Foundation/Downloads/Quick Find/Clipboard + Tools/Global settings + supporting surfaces. User2026-10-11 explicitly reorders execution to Foundation → Quick Find → Downloads → Clipboard + Tools → Global settings + supporting surfaces. Preserve every field, action, shortcut, state, save policy and fixed native budget unless the later Quick Find geometry brief explicitly changes it.
- **R4 — Ownership and lifecycle:** Preserve Rust state/layout, GPUI rendering and C# Windows ownership; one activation dispatcher, pending-before-dispatch, deferred cross-entity effects, generation/HWND identity, cancellation, failure/retry and passive-preview behavior.
- **R5 — Approved option A:** Kit styled-component geometry, typography, padding, radius, borders and interaction-state grammar are authoritative for ordinary controls. Only the user-approved neutral semantic colors are remapped. Use official compact size tiers in small windows; adjust composition/scroll instead of removing features, enlarging fixed clients or reconstructing legacy control chrome. Historical presentation-only control/type/radius constants are not automatic exceptions.
- **R6 — Observable acceptance:** Establish actual component/state/geometry and visual evidence. No completion from dependency names, imports, source tokens, compilation or test totals alone. The user reviews a representative production-render Foundation slice before broad rollout.

## Acceptance criteria
- **AC1 (R1, R2):** The complete adoption ledger has no unexplained legacy appearance suppression or duplicate ordinary-control chrome. Each retained custom exception records its exact contract/capability reason and regression.
- **AC2 (R1, R5, R6):** Representative production controls visibly retain agreed Kit appearance/state grammar. Default, hover, pressed, keyboard focus, selected, disabled/loading/error states are checked, with no clipped CJK labels or missing controls.
- **AC3 (R3, R4):** Existing feature/native lifecycle regressions remain green. No deleted/ignored assertions, window inflation or domain/persistence rewrite is used to make components fit.
- **AC4 (R6):** Production rendering and relevant actions/bounds are verified per surface; visual inspection, user approval and native/physical/DPI/reader status are recorded separately. Final UI completion requires agreed visual acceptance, not framework-upgrade success.

## Out of scope
New product features; domain/engine/storage/indexer changes; dependency/toolchain upgrades; unrestricted upstream edits; copying GPL ShellRS code/assets; system DPI changes; service deployment; automatic commit/push. Packaging/installation requires explicit instruction after component validation.

## Decisions and technical notes
The user explicitly selected A after comparison with retaining legacy appearance. This supersedes conflicting presentation-only constants, not native dimensions or feature behavior. Official website content is a discovery reference; the locked published 0.7.1 source determines available APIs. Research and capability gates are in `research/kit-release-contracts.md` and `design.md`. No blocking product question remains; a concrete incompatibility discovered during execution must be documented rather than silently reverting to old UI.

## Artifact and approval status
Task creation, Phase 1 planning and option A are approved. PRD convergence is complete. `design.md`, `implement.md`, `adoption-ledger.md`, research and curated context manifests form the execution proposal. They are not implementation approval: the latest final planning summary must receive a subsequent explicit approval before `task.py start`. No product code, build, package or service operation was performed during planning.

## Foundation rejection and measurable alignment requirement
User rejects the first Foundation captures: centered navigation clusters and tool header title/actions are not professionally aligned. Foundation remains current; no Downloads rollout. Add R7/AC5: title/status/Switch/close share one first-row vertical centerline within1 logical px; description is an independent bounded second row aligned to title start, long content cannot shift controls and full text remains available. Navigation icons occupy a fixed16px lane and labels a separate elastic left-aligned lane, same starts across all four rows at default/minimum clients. Use Kit type/state/geometry; these composition constraints do not authorize hand-painted controls or another focus/activation owner. Actual geometry and refreshed screenshot inspection are mandatory, user approval remains pending.

## User amendment — shared template and explanatory Tooltip migration (2026-10-11)

Supersedes R7/AC5's feature-state-in-titlebar requirement: small feature windows
use a title/close-only drag header and divider; enabled_label/status/Switch belongs
to the body before feature-specific controls. Template data supplies enable text.
Later user amendment explicitly requests Kit0.7.1 Tooltip for optional feature
descriptions across Downloads, Quick Find, Clipboard, Tools and global settings.
No permanent subordinate description row; use discoverable title-adjacent help.
Retain complete text, keyboard-focusable accessible help and existing callbacks.
Actual state, paths, errors, validation/ranges/units and risk warnings stay visible.

This authorizes one narrowly scoped cross-family explanatory-tooltip migration in
canonical family order. It does not advance their full component/layout phases or
waive Foundation user visual approval, native fixed budgets, reference-first
review or separate package/install/service authorization.

## Quick Find first / Steward reference amendment (2026-10-11)
User requests Quick Find UI before Downloads and alignment with supplied
D:\Code\steward-master, plus feature/performance standards review. Source review
and static screenshot complete; docs/reference-code/steward.md records verified
entries and nontransferable native/benchmark assumptions. MIT/reference stack
does not authorize dependency or toolchain change. User clarification narrows work
to UI layout and component-reference adaptation within ALL existing Next sizes,
visible behavior and functions. Preserve neutral theme/Kit controls/native and
entity ownership, input/IME/search/selection/action/preview/settings behavior.
Assistant's A/B size proposal is withdrawn; no geometry decision is outstanding.
Feature/performance review is informational only, not implementation authorization.
No engine, query grammar, service, storage or new-feature change.

## Official Settings behavior approval — 2026-10-11
User explicitly permits official Settings built-in search and related component
features: preserve every existing product function, not every old settings gesture
or visual. Use the actual released Settings hierarchy; do not patch/fork the library.
Official selection/filter/resizable sidebar/scroll presentation may replace the old
custom settings presentation; no automatic reset/persistence semantics are added.
Retain5topics, hotkey recording, index scope/exclusion keyboard actions, command
draft/picker/save and diagnostics actions. Keyboard scope follows the mounted/focused
business content rather than private SettingsState. Approved launcher UI is untouched.

## Post-gallery user amendment — 2026-10-12
Temporarily retain Downloads virtual task grid for later separately authorized package
review. Adopt official TimeField with original Apply business validation (raw-invalid
HH:MM retention superseded only here), migrate category master ListItem, continue real
Clipboard Tabs adaptation and shorten search with officialSmall24px. Explain/test
Calendar public sizing; do not infer universal incompatibility from directSmall.
Notification needs original ID/generation close intent, not specifically on_close.
Inspect actual visual imbalance/loose spacing and make targeted corrections within
existing palette/type/radius/native clients. No automatic packaging or user acceptance.

## Installed layout feedback — 2026-10-12
Latest user screenshots supersede short-control-near-title and continuousToolsrow presentation: standaloneSwitches alignright; help followsfieldtitle; input-adjacent selectors matchinputheight; no clippednavigation/fullfilterlabels; bounded sortpopup; noQuicksearchfocushighlight; Clipboard8pxsearch/listgap; Toolsregisteredfeatures useleftstarttilegrid as supplied. Preservefields/owners/nativeclients;no newtools/package/install authorized. ExactAPI/invariants: [selection record](research/installed-layout-feedback.md). Source626tests/strict/workspace/fmt/Debugbuild/11gates andownednative2+2pass;45finalbuildPNGs capturedbutBrowservisibility blocksallactualinspection. [Outcome](../../../docs/audits/2026-10-12-installed-ui-layout-feedback.md). Visual/installedapproval remainsopen.
