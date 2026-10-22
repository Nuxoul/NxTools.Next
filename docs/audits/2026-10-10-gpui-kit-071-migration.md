# GPUI Kit0.7.1 migration checkpoint

## Outcome
The local exact GPUI Kit0.7.1 migration compiles on existing Rust1.96.0. Menu adaptation and explicitly approved passive-preview repair are implemented;current full desktop519/519 pass. Owned96DPI Quick/Clipboard native All each2/2 pass,message-policy/client/reuse/synthetic UIA checked. Physical/mixed-DPI/reader/installed acceptance remains separate,not blanket product-release certification. Preexisting dirty fixes retained;no commit,push,package,install,ordinary runtime launch or service deployment.

## Integrated implementation
- Desktop's only direct GPUI-stack dependency is exact gpui-kit0.7.1. Cargo tree resolves Base/Component/Assets0.7.1 and one gpui-pre/platform/windows0.3.8 stack. Old gpui0.2.2/Component0.5.1 vendor patch is outside the graph; dormant vendor files are not proof of shipped behavior.
- Kit application/init,one Base Root,custom-first assets/Kit fallback. Registry unmodified;later explicit authorization permits only local Windows0.3.8 WM_MOUSEACTIVATE branch honoring WS_EX_NOACTIVATE. NoGPL ShellRS code/assets copied.
- Theme::update synchronizes colors/renderable tokens/Base projection and explicitly maps independent button palettes. Readable primary actions preserve foreground/background pairing; existing neutral roles and fixed budgets remain.
- Application-shaped actions use Base Button and native focus-visible/keyboard modality; retired window-keyed project modality registry removed. Production Tab fixtures mount real Base Root. One pointer/KeyUp callback, disabled and held/modifier checks remain.
- Request headers and Clipboard multiline editor use TextareaState/Textarea. Silent programmatic set_value remains hydration; user paste/clear and owner-specific query/discovery paths explicitly retain intent. Directory/FFmpeg picker publication remains deferred, generation-scoped.
- Window/focus/Anchor/scroll/DisplayId/test-event APIs adapted across surfaces and test files. Tests retain real bounds, generations, native command queues, fixed download handoff/completion, persistence and callback assertions.

## Executed checks
| Check | Result |
| --- | --- |
| Production cargo check | PASS |
| Desktop --tests cargo check | PASS |
| Shared action behavior/paint suite | 5/5 PASS |
| First full desktop integration | 490/501 PASS,11 failures |
| Prior full desktop checkpoint | 500/501 PASS,1 actual menu failure (historical) |
| Current full desktop integration | **519/519 PASS**,0 ignored;menu checkpoint510/510 retained historically |
| Strict production Clippy | PASS |
| Strict --tests Clippy | PASS |
| cargo check --workspace --tests | PASS |
| cargo fmt --all -- --check | PASS |
| Document contract gate | PASS87 documents/12 owners |
| Affected static architecture gates | PASS10/10 after equivalent Kit API/source-shape adaptation |
| git diff --check | PASS |

All Cargo invocations used CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; one build at a time after writers stopped. Full logs are in session scratch `kit071-final-{full,clippy-tests,clippy-production,workspace,fmt}.log`. Compilation is not native lifecycle or visual acceptance.

## Concrete failure and resolved regressions
Initial failures were diagnosed, not removed: Base Button's centered child shrank full-width settings navigation; explicit content-width wrapper restores147px; Tab fixtures needed the actual Root binding; equal metric columns needed equal reserved divider width; new Bounds::contains is half-open so rectangle containment assertions compare all boundaries directly; draw-only resize fixture now stays mounted with actual simulated viewport; one real SQLite-worker fixture enables the scheduler's explicit allow_parking seam without changing production worker behavior. New Kit asset test uses actual released close.svg path.

The old submenu test read retired vendor source and falsely passed after dependency replacement. A real Kit menu in300×440 reproduced x228,width186,right414; this executed red was retained and migrated to the approved shipped project boundary. project_menu.rs uses one Base Root plugin for root/child and Kit Base Button pointer rows; local menu key context owns one armed release dispatcher. Absolute client positions/bounded scroll/24px reachable parent edge preserve narrow clients without changing native budgets. Existing standalone dropdowns remain Kit-owned; history no longer runs the old styled ContextMenu/PopupMenu path. No upstream source patch,feature deletion or second active menu framework.

## Remaining work
Current canonical phase is **Integrated validation and documentation** after source adaptation converges. Menu8/8 regressions and real history action/category callbacks pass; full510/510,strict production/tests lint,workspace --tests/fmt pass. One bounded optional review found concrete child-overlap/pointer-keyboard-level/retired-focus/hide-failure-focus issues; fixed and retested without repeated broad reviews.

Native passive Clipboard/Quick Launch preview still needs actual owned-HWND pointer/focus verification: released Windows WM_MOUSEACTIVATE behavior cannot be certified by WS_EX_NOACTIVATE source alone. Full100/125/150/mixed-DPI, physical gestures, live motion, OLE and AccessKit/UIA provider/reader acceptance remain open. No new native captures were acquired in this checkpoint. Existing installed formal package predates this migration and local upgrade-Apply fixes; service state was not changed.

Reference and canonical task: `docs/reference-code/gpui-kit-071.md`, `.trellis/tasks/10-10-gpui-kit-071-migration/implement.md`.

## ShellRS-backed menu/lifecycle closeout
Revisited D:\Code\shell-rs-main src/explorer/pane_menu.rs,src/history/history_panel.rs,src/shared/segmented_control.rs andsrc/app/mod.rs before adjusting layout. Independently implemented fixed14px leading marker/icon, flexible clipped/full-tooltip label and14px arrow rails, semantic neutral selected/hover/pressed hierarchy with unchanged28px row/240px maximum/flat boundary. No referenceGPL code/assets copied, no16px/rem/window sweep. Actual alignment/disabled/checked/hit-bound tests supplement existing behavior, not palette-only claims.

Menu tests cover arrows/Home/End/right/left/child Escape/root Escape/Tab/ShiftTab, one release incl held/modifiers/stale release, pointer-entered child keyboard ownership,focus-out/original focus/reopen/trigger unmount/host hide,short220×140 viewport with40-category real scroll and parent edge. History menu regression executes four paste/copy transforms,four edit/pin/favorite/delete callbacks andtwo checked category add/remove callbacks; nontext Edit/WithoutBreaks reject pointer actions. GPUI exposes no public aria-disabled setter; record snapshot None limitation, do not fabricate semantics. Actual callback reads closed menu state outside its lease. Failed native-hide handler restores search focus; immediate enqueue-failure refocus is deferred after owners release and guarded by session/is_open.

Final logs: session scratch menu-shellrs-final-{full,lint,production,workspace,fmt}.log;510/510,0 ignored. Affected static gates10/10 including shared token policy;spec navigation/diff checks recorded at final closeout. No new native captures or screen-reader acceptance; installed formal package remains older and persistent service untouched.

## Native acceptance follow-up
See [actual native checkpoint](2026-10-10-gpui-kit-native-acceptance.md). Historical MA_ACTIVATE1 failure was repaired through user-approved repository-local Windows0.3.8 live-style branch. Both native All modes now2/2 PASS:passive3/ordinary1/live policy toggles1→3,exact Quick400×410/Clipboard460×406 client/reuse and synthetic UIA. Clipboard hidden-first acknowledged noactivate/bounds/topmost/show chain and reused-policy3s deadline have8/8 controller regressions;full519/519/strict lint/workspace/fmt/Debug build/10 source gates pass. Physical/mixed-DPI/reader/installed/service acceptance remains separate;formal package unchanged.
