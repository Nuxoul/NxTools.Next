# GPUI Kit0.7.1 migration source boundary

Complete component/compound discovery starts at the mandatory
[official component index](gpui-kit-component-index.md). This release-boundary note
is not a substitute for that full catalog or proof that a page has adopted it.

User2026-10-10 explicitly authorizes full desktop migration to exact GPUI Kit0.7.1. Crates.io version API/package checksum verified; Apache-2.0. Published Kit facade depends on matched gpui-pre=0.3.8, Base/Component/Assets0.7.1. Direct app entry is gpui_kit::application/init, Base Root per window; test-support exposes real native-property ElementSnapshot. Official coding/design guides read for bootstrap/ownership/theme/focus/geometry/test contracts; external guide prose kept only in scratch (do not copy documentation without attribution/license).

This authorization supersedes earlier pinned-stack/no-upgrade restriction specifically for this migration, not domain/native/budget/service ownership. Old direct gpui0.2.2/component0.5.1 vendor patch leaves the dependency graph. Registry edits remain prohibited; the later explicit user authorization permits only the repository-local Windows0.3.8 WM_MOUSEACTIVATE exception below, not an unrestricted fork. Preserve all preexisting dirty source/callback/test behavior via session baseline pre-gpui-kit-071/source-baseline.tar (SHA256F85374162D30C62CFE1C8881B4E9DB2FA5569F19EDC4F3DD9D00541E804FF4D6).

Window/Root/focus and test API maps are being verified against published release source, not guessed from ShellRS0.6.1. Windows hidden HWND native bounds may still be CW_USEDEFAULT; C# final bounds/show/noactivate/readback, source HWND OLE and single serial-reader-independent owner remain. Existing deferred leased-entity/generation/pending/cancel policy preserved. Real snapshot/bounds test adapters must observe actual element IDs/geometry; no inert debug methods or suppressed tests.

Phase plan: .trellis/tasks/10-10-gpui-kit-071-migration/implement.md. No product packaging/installation/service operations or native/screen-reader acceptance inferred from this source migration. Toolchain compatibility checked through released source/compiler evidence; no automatic system-default switch. No GPL ShellRS implementation copied.

Verified Windows/test release entries: gpui-pre0.3.8 app/test_context.rs retains add_window_view/VisualTestContext/debug_bounds/simulate_resize and down-only simulate_keystrokes; existing explicit key releases and actual callback viewport simulation remain. Kit open_window adds Base Root and returns AnyWindowHandle+Entity; no double Root. Windows0.3.8 window.rs:549–587 hidden creation still CW_USEDEFAULT/pending placement; raw HWND extraction valid. C# bounds/noactivate/show/readback/single-worker owner retained. events.rs:90–96 returns MA_ACTIVATE on WM_MOUSEACTIVATE: passive preview pointer behavior requires actual owned-HWND gate, not style-only proof. AccessKit UIA and focus_visible now exist, but real provider/reader acceptance remains separate. Upstream Zed snapshot toolchain1.98.1 is evidence, no declared crate rust-version; local1.96 compiler compatibility checked before pin change.

Verified component release paths (published0.7.1): component/theme/mod.rs Theme::update reconciles Hsla colors, ThemeTokens backgrounds and Base projection; independent button/button_primary/button_secondary/button_danger palettes must be mapped. Base button.rs owns interaction/disabled/focus/roles, gpui-pre elements/div.rs focus_visible reads Window::last_input_was_keyboard; Base root.rs owns Tab/ShiftTab. InputBaseState::set_value is intentionally silent; TextareaState replaces multiline/auto-grow InputState. Product owner paths must explicitly preserve query/discovery intent without synthesizing cross-entity callbacks under a lease. Material/caption/brand aliases retain custom-first asset precedence with Kit Assets fallback.

Historical Kit gap: component/menu/popup_menu.rs0.7.1 relative child placement and unconditional ring/shadow do not expose a public flat/absolute-position override. The actual300px test first reproduced x228,width186,right414. User-approved project boundary now replaces Clipboard history's styled ContextMenu/PopupMenu path with a single Base Root plugin/Kit Base Button presentation, no upstream changes or parallel menu owners. Standalone Kit dropdowns remain unchanged.

Historical migration evidence: production/test cargo check passed on Rust1.96.0; shared action5/5 verified keyboard modality/one release-one dispatch/disabled/readable Apply. First integrated501 tests:490 pass/11 failures. Fixed Base Button child stretch,real Root in Tab fixtures,metric divider budget,half-open Bounds rectangle containment,persistent resize-hit fixture andreal-worker allow_parking. Replaced obsolete vendor-source submenu assertion with actual Kit0.7.1 menu/project192–240px shell in300×440 viewport; it reproduced x228,width186,right414 with both window.resize andsimulate_resize. Released PopupMenu placement/flat appearance is private. User-approved project replacement and actual green regression below now supersede this blocker; earlier behavior-red remains historical evidence, not native HWND acceptance.

Historical checkpoint:500/501 desktop tests passed with real Kit narrow-child behavior-red. This was not a complete migration. Latest source closeout below supersedes that unresolved-menu status; earlier logs remain evidence.

Current menu source closeout: project_menu.rs owns one root/child session, absolute viewport clamps/bounded scrolling/24px reachable parent edge, deferred opening/dismiss-before-callback and original-focus restore. Actual frame prepaint liveness handles Kit keyed-state caching; cleanup is next-turn/session-guarded and never synchronously notifies Root while preparing/rendering. Local ProjectMenu Tab binding avoids private Base Root action types; one armed release owner handles Enter/Space, pointer rows cannot dispatch a second keyboard click. Fixed marker/label/arrow lanes/full label tooltip follow independently implemented ShellRS composition precedent. Menu8/8,real history four paste/copy transforms+four management+two category callbacks and failed-hide search focus pass. Full desktop510/510,strict production/tests lint,workspace --tests/fmt pass. No native/provider certification or deployed package follows from these source results.

Released GPUI/Base disabled-state limitation: Button.disabled rejects pointer/keyboard and removes Click accessibility action, but there is no public aria-disabled builder; ElementSnapshot.disabled may be None even for a disabled control. Preserve actual no-dispatch and eligibility tests, do not fake a metadata setter or claim reader acceptance.

Historical native red2026-10-10: isolated actual Kit Debug HWNDs at96DPI (single2560×1440 display), production noactivate followed by bounded WM_MOUSEACTIVATE returned1 rather than3 in both modes. Quick requested400×410 yielded384×402;Clipboard460×406 yielded444×398. All Rust callers use client/content device pixels; C# now compensates live frame insets, preserves x/y and verifies final client. UIA observes synthetic named Edit50004/focusable and Button50000. Physical clicks,reader and full product-tree acceptance remain separate.

Released WindowsPlatform::open_window/PlatformWindow has no public passive mouse-policy callback; out-of-process C# cannot safely install a callback pointer. User explicitly approved a minimal repository-local backend0.3.8 patch instead: existing GPUI WM_MOUSEACTIVATE owner reads live GWL_EXSTYLE and returnsMA_NOACTIVATE iff WS_EX_NOACTIVATE,otherwise preservesMA_ACTIVATE. Source/license/revision are recorded in vendor/gpui-pre-windows/NXTOOLS-PATCH.md;27 release files compared,only src/events.rs differs. Kit0.7.1/backend0.3.8 graph retained,no registry edits,injection,click-through/disabled previews or focus stealing.

Current native All: Quick2/2 and Clipboard2/2 PASS,actual HWND policy3/ordinary1/disabled-policy1/re-enabled3,exact400×410/460×406 clients,reuse and synthetic UIA. Clipboard creates hidden and confirms noactivate→bounds→topmost→show through caller-ID pending-before-dispatch; each step and reused-policy confirmation expires after3s. Success/failure/timeout checks saved HWND; generation/cancel/late result cannot resurrect content. Controller8/8,desktop519/519,strict production/tests lint/workspace/fmt/Debug build and10 static gates pass. Physical/mixed-DPI/reader/installed acceptance remains unverified.

Component-first Foundation distinction: user rejected15:12 technical/API package'slegacyappearance andapprovedoptionA. Actualrelease default16px/mono13px/radius6,published SmallButtons/Switch andMediumInput retained,notoldglobal12px/radius3. StyledButtonGroup preserveschildClickEventwhen group callback is absent(button_group.rs:230-264);equal-widthcontainerlayoutisproject-owned. SidebarMenuItem TreeItem has no publicfocus-handle/tabbinding;realSidebar+styledButtonsusepublicsame-scopekeyedfocusseam(button.rs:629-632,816),withactualTab/release/unmounttests. ToolssettingsadoptsSmallForm/Field;fixed legacylabeladaptercompositionremainsowning-stageworkafterlayoutregression,notcompletedmigration. RealKitadoption9newtests/full528passand96DPI productioncapturesinspected;userFoundationapprovalpending,otherfamiliesremainplanned. SeecomponentFoundationaudit,notearlierAPIcloseoutforvisualstatus.

## Foundation numeric focus regression recovery (2026-10-11)

Rechecked the published release snapshot retained under prior session scratch
`b54c35cc-1a3b-45d3-ba44-64d6f226efc4/gpui-kit-071-reference/`:
- `gpui-pre-0.3.8/src/window.rs:3392-3431` dispatches frame focus transitions;
  `WindowFocusEvent` substitutes empty previous/current paths for inactive frames.
  `FocusHandle::is_focused` alone does not establish window activation.
- `gpui-pre-0.3.8/src/platform/test/window.rs:398-408`: `activate` calls the test
  platform's `set_active_window`; initial `is_active` returns false. Synthetic
  fixtures must activate and drain the application turn before asserting Blur.
- `gpui-base-0.7.1/src/input/base/state.rs:708-709,3397-3417` registers retained
  focus/blur subscriptions in input construction and emits `InputEvent::Blur`.
- `gpui-component-0.7.1/src/select.rs:168-247` defers list confirmation, commits
  selection, emits `SelectEvent::Confirm`, closes and restores trigger focus;
  cancelling restores the committed selection instead of persisting the cursor.

Next call chain: `main_window/tools_page.rs::open_tool_settings_window` creates
one keyed normal 460x320 window; `ToolSettingsWindowBody::ensure_inputs` creates
stable Input/Select state once; actual Blur defers last-valid-value restoration,
valid changes and Select confirmations defer application persistence. Preserve
2-60 seconds/step 2, invalid/intermediate edits not committed, silent hydration,
cancel not saved, close presentation-only and existing native activation policy.
This tool has no matching legacy NxTools settings interaction (see gpui.md and
link-detector.md); no passive-preview precedent is an activation override.

The original regression failed, then passed by adding only synthetic-window
activation with production unchanged. First-render initialization was therefore
not the cause; no constructor rewrite or second focus listener was introduced.
Permanent tests exercise Input-to-Select pointer blur before Escape and actual
Select keyboard confirmation through a temporary config persistence boundary.
These are GPUI test-platform evidence, not physical Windows input acceptance.

## Foundation role/tier composition review (2026-10-11)

Rechecked published component0.7.1 `input/number_input.rs::new/render`,
`select.rs::SelectOptions/SelectState::render/Select::render`,
`switch.rs::new/render` and `form/{form,field}.rs::render` before composition edits.
NumberInput/Select retain their released default or official Small frame grammar;
Switch defaults to Medium (36x20 track), Small is28x16. Form Small selects6px row
spacing, not a mandate to force child controls into24px boxes. Horizontal Field
supports a label-width choice; that is application composition, not Kit default
layout acceptance. Default-vs-Small production comparisons and fixed-client
containment are required alongside visual inspection.

Official NumberInput documentation HTML was fetched from
https://gpui-kit.com/zh-CN/component/number-input into session scratch; the embedded
examples expose actual NumberInput and InputState step/min/max APIs. Browser
inspection is currently unavailable because the calling session panel is not
visible; fetched HTML is not visually inspected evidence. No behavior conversion
to built-in numeric clamping is approved: keep last-valid restoration and step2.

Revisited supplied ShellRS `src/workspace/title_bar.rs::build_title_bar` title
command grouping and no-drag boundary and `src/host/host_dialog.rs` Form/Field
construction as composition precedent only; no GPL implementation copied.
This slice preserves title/status/Switch/close first-line alignment and separates
feature-state grouping from window-close action without moving native ownership.
Tools stays460x320; no callbacks, save policy, window activation or entity
initialization owner changes. Legacy NxTools has no matching Tools settings
interaction; existing Next production behavior remains the source of truth.

## Feature-description Tooltip amendment (2026-10-11)

User explicitly changes optional explanatory copy from inline rows to Kit Tooltip
across desktop settings families, preserving names/state/validation/risk copy.
Rechecked released component tooltip.rs::Tooltip::{new,element,build}/render and
button/button.rs::Button::{tooltip,accessibility_label}, Base button focus and
GPUI div tooltip owner lifecycle. Official Tooltip HTML examples fetched into
scratch: Tooltip::new(...).build and Button::tooltip. Default activation is hover/
long-press, not keyboard-focus automatic reveal. Help Button is keyboard-focusable
and carries complete text in its accessible name; do not invent a parallel popup
manager or assert focus-triggered display. Use official Small/XSmall tiers without
painting duplicate tooltip chrome or changing theme fonts.

Reviewed indexed legacy MainWindow/SettingsWindow controls/tooltips and ABDM
DesktopSettingsComponent appearance reference first for Downloads. Legacy inline
help is intentionally superseded by this user amendment, not an unchanged behavior
claim. No settings save, native show/hide/cancel or feature callback changes.
Dynamic scan/registration/install state, paths, errors, units/ranges and warnings
remain visible through explicit status rendering. Shared labels and per-family
entry points preserve complete descriptions and existing owner identities.
