# Audit #3 only — names on actual Buttons

## Outcome, phase and boundary

Implemented only original audit #3. Ready for parent independent review, **not self-accepted**.
Foundation shared action contract correction; direct consumers stay in canonical Downloads / Quick Find /
Global settings + supporting surfaces. No stage reordering/renumbering or fifth item started.

Per-file current dirty baselines were saved before editing (not git HEAD). Existing unrelated changes
remain untouched. No recursive delegation, commit/push, package/install, product/PlatformHost/service,
real clipboard/picker/Windows Shell, external network, real user config/database or registry operation.
New fixtures use original production GPUI views with temporary config and in-memory engine/bridge.
Existing synthetic test suites exercise their original temporary/loopback seams, not external services.

## Reference, exact API and chosen composition

Read AGENTS, applicable Desktop/shared spec topics, current component selection/table/API index,
Kit/Base/Component0.8.0/pre0.3.9 release and download/system-shell/Quick Find notes before edits.
Exact locked Component `button/button.rs::render` computes explicit accessibility_label.or_else(label).
It does **not** infer a Button name from arbitrary child content, icon or Tooltip. Public
Button::accessibility_label names the original Button semantic node, with no layout/focus/activation owner.
Snapshot assertions inspect the node whose path ends in the actual Button ID and require Role::Button;
no outer div or visible child label substitutes for that node.

Reference read-only recheck, ABDM first for Downloads:
- LocationTextField.kt:45–105 names its folder icon with download_location contentDescription; original
  callback applies the chosen directory. MainActionButtons:444–505 uses Add/Download/Cancel text and original
  close/submit callbacks. No new picker or button framework is taken from that reference.
- Legacy SettingsWindow.axaml:998–1006 and .axaml.cs constructor15 -> WindowAnimationService.Attach,
  OnOpened21 -> ConfigureForCurrentScope, SaveButton_Click68 -> ApplySettingsCommand -> Close77,
  CloseButton_Click56 -> CloseSettingsCommand -> Close65. VideoSettingsWindow.axaml174+ names FFmpeg
  select/install/refresh actions. Avalonia native Window base is unavailable, not claimed inspected.
- The Quick Find note correctly records no exclusion UI in old NxTools. SwiftList ExclusionsTab.xaml
  path collection -> Templates.xaml::RuleItemTemplate16–40 has one trailing RemovePathCommand + object.
  This is behavioral evidence only, not adoption of WPF styling or bulk/edit features.
New verified API/reference invariants appended to gpui-kit-080.md; prior prefix unchanged.

Requested shape, official parts and adaptations remain exactly original: Small secondary icon Button
(optionally outline at callsite), original external Tooltip div hover owner, Small primary custom-content
Button with readable disabled content, and original custom40px path-delete hit budget. Original Toolbar
parent adaptations unchanged. No label geometry, ellipsis, visible copy, palette, font, border, size,
hover/focus ring, Tooltip strategy or callback dispatcher changed.

## Minimal helper contract and complete direct production consumers

`primary_action_button(id, accessible_name, content: impl IntoElement, theme, disabled, cx)` now requires
semantic name independently of visual content. It retains the same child(div.child(content)), disabled
readable colors, official primary style/geometry and sole original Button release-click owner.
`icon_action_button(id, icon, accessible_name)` requires name even when an outer Tooltip owns hover.
`icon_action` forwards its existing tooltip text into this required argument once and retains its original
Button Tooltip; no duplicate accessible parent or second activation/focus owner. Technical IDs never
become user-facing names. No broad ARIA/provider framework, dependency/vendor/protocol/C# edits.

Complete direct callsites found by scoped source search and compiled signature adaptation:

| Production consumer / Button ID | Explicit semantic name | Visible content and owner |
| --- | --- | --- |
| downloads/settings_view default-directory picker | 选择默认保存目录 | Same icon/outline/external Tooltip and request_directory |
| settings FFmpeg picker | 选择 FFmpeg 可执行文件 | Same icon/outline/Tooltip/deferred original request |
| settings config Copy/Open | 复制配置路径 / 在资源管理器中显示配置文件 | Original DownloadSettingsPathKind.presentation values, original deferred action |
| settings logs Copy/Open | 复制日志路径 / 打开日志目录 | Same presentation and original copy/open owner |
| settings database Copy/Open | 复制数据库路径 / 在资源管理器中显示数据库文件 | Same presentation and original copy/reveal owner |
| new download refresh-probe | 重新探测下载链接 | Same icon/outline/submitting guard/begin_probe |
| completed task drag (window_support) | 拖拽下载文件 | Same outline icon, pressed native drag owner untouched |
| primary settings FFmpeg auto-download | 自动下载 FFmpeg | Same 自动下载 / 正在下载… and original install owner |
| primary settings taskbar edit | 编辑任务栏布局 / 结束任务栏布局预览 | Same 编辑布局 / 结束预览 and original edit owner |
| primary settings Apply | 应用下载设置 | Same 应用 / 正在保存… and original save |
| primary new-download footer StartNow | 立即下载 | Same supplied dynamic content, only primary StartNow branch |
| primary new-download schedule-confirm | 确定下载计划 | Same 确定, deferred original selection commit |
| primary new-download probe | 探测下载链接 / 重试打开下载确认 | Same 继续 / 正在准备… / 重试打开确认, original begin_probe/handoff policy |
| primary task failed-retry | 重试下载 | Same 重试 and original retry callback |
| primary task resume | 继续下载 / 重试下载 | Same 继续 / 重试 and original status branch |
| primary task pause | 暂停下载 | Same 暂停 and original pause callback |
| primary schedule Apply | 应用下载计划 | Same 应用 / 正在应用… and original save |
| primary completed Open | 打开文件 | Same spec.label and original Open action |
| primary Global Apply | 应用全局设置 | Same 应用 / 正在应用… and original section transaction |
| direct path_list delete | 移除排除目录：{actual path} | Original icon/custom40px/Tooltip and remove(index,frozen path), no log |

There are **12 primary production callsites**, **6 external icon helper callsites** (settings copy/open
parameterize three path kinds), plus already-named icon_action's forwarding. All were inspected; no
other consumer was added to scope. The existing canvas contrast fixture gets a semantic parameter only.
Unchanged icon_action users require no callsite edits. Object paths belong to requested accessibility
content, not telemetry; no new logging or cross-process operations/events are introduced.

## Red-green and seven acceptance tests

All Rust commands inherit CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256; serial full-operation timeouts, natural idle checked, no kill/duplicate build
or request for another session to stop/reserve/reorder.

`cargo test -p nxtools-desktop accessible_name_ -- --test-threads=1`:
- Valid pre-fix red (`names-red-behavior.log`): compiled, **0 PASS / 7 FAIL**. All seven got the actual
  Button and passed Role::Button, then failed expected name vs None. Global Apply, default directory,
  FFmpeg, probe, shared primary/icon and path delete all have concrete real-node red evidence.
- Initial fixture compile attempt used wrong path-kind variants/nonexistent cleanup methods. Fixed
  only new fixture enum/cleanup; that attempt is **not** counted as behavioral red evidence.
- First green candidate: 6/7, all names repaired; new Downloads test incorrectly demanded equal widths
  for pre-existing 应用 and 正在保存… text. Kept original product natural width; test now asserts original
  height/top/right anchor and footer containment, not a changed product layout.
- Second green candidate: 6/7 after added icon keyboard coverage lacked a real focused target. New
  fixture now follows original Tab path and verifies observed focus before Enter/Space; product unchanged.
- Final same seven tests **7/7 PASS** (0 ignored). Final complete Desktop also includes all seven PASS.

| Test | Actual production evidence |
| --- | --- |
| accessible_name_icon_is_on_button_not_outer_tooltip | Actual icon Button name in normal/disabled/loading; pointer single dispatch, actual Tab focus + Enter/Space release once; disabled/loading pointer inert; existing icon_action creates one named Button, no duplicate wrapper |
| accessible_name_primary_custom_content_states_keep_original_dispatch_and_bounds | Ordinary text and actual canvas/custom content each normal/disabled/loading name; stable96x32 bounds; pointer and Tab/Enter/Space original release once, disabled/loading no dispatch |
| accessible_name_global_apply_is_actual_button_and_stable_while_saving | Actual Global settings view, original Apply Button name stable while saving, disabled pointer cannot persist temporary config |
| accessible_name_download_settings_apply_and_default_directory | Actual Downloads settings and directory Button; Apply name stable busy, original text-dependent width preserved, footer/hit height stable, busy no save |
| accessible_name_download_settings_ffmpeg_and_diagnostic_objects | Original official Transfer/Diagnostics navigation, actual FFmpeg select/install and all6 config/log/database copy/open Button nodes named distinctly; no action invoked |
| accessible_name_new_download_probe_and_refresh_are_actual_buttons | Actual compact probe and synthetic confirmed form refresh/StartNow names; no network probe or task submission |
| accessible_name_excluded_path_delete_buttons_identify_each_object | Two actual production path rows, each trailing Button name includes its own path, original40x40 hit bounds; observation never removes |

Representative contract + complete direct-callsite inspection covers task pause/resume/retry, schedule,
completed Open/Drag and taskbar layout controls; existing geometry/dispatch regressions cover these
original owners. No claim that every such additional state was independently inspected by a reader.
Existing contrast test retained normal/hover/disabled paint assertions and unchanged bounds/no-dispatch.

## Final verification and protected logic

| Check | Result |
| --- | --- |
| Names focused red/green | 0/7 red -> 7/7 PASS |
| `cargo test -p nxtools-desktop downloads:: -- --test-threads=1` | **185/185 PASS** |
| `cargo test -p nxtools-desktop network_settings:: -- --test-threads=1` | **35/35 PASS** |
| `cargo test -p nxtools-desktop ui:: -- --test-threads=1` | **231/231 PASS**, includes shared/path/Clipboard identity |
| strict `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS |
| `cargo check --workspace --tests` | PASS |
| `cargo fmt --all -- --check` | PASS; only own files formatted, skip_children |
| Scoped `git diff --check` | PASS (existing gate CRLF normalization warning, no whitespace error) |
| Exactly one complete Desktop run | **FAILED:1191 PASS / 1 FAIL**, 0 ignored/filtered,209.37s |

Sole Desktop failure remains
`archive::application::tests::production_password_application_original_dialog_ticket_and_deferred_grant_never_start_ole`
at archive/application_tests.rs:954, `official unlock Button`. Archive was not modified or rerun to hunt
for green. Final integrated run explicitly contains accepted #1 keyboard **8/8**, #9 picker **14/14**,
#10 identity **11/11**, all names **7/7** passing.

Static gates **10 PASS / 1 FAIL**:
PASS download-ui-lifecycle (one precise old signature assertion updated to require named drag helper),
clipboard-ui-lifecycle, clipboard-lifecycle, desktop-shell-lifecycle, ui-state-lifecycle,
ui-token-boundaries, clipboard gate mutation **5/5**, gpui-release-contracts,
markdown-editor-contracts, spec-contracts (**114docs/14owners**).
FAILED extra quick-launch-lifecycle.ps1:1765. Bounded read-only diagnosis evaluated its74 predicates;
sole failing predicate is `$buildScriptSource -notmatch 'write-release-binary-manifest\.ps1'`.
Current build.ps1 calls publish-release-hosts.ps1 instead. This task does not edit build/release/elevated
helper code, and cannot certify that security boundary. Failure retained, not weakened or repaired.

Protection checks: six whole-file baseline hashes unchanged (directory_picker implementation/tests,
new_download_keyboard_tests, Clipboard history.rs/panel mod.rs/identity tests). Three additional exact
byte-prefix comparisons unchanged: shared focused_action/focusable_content before SegmentedOption;
NewDownloadView entire constructor/picker/keyboard/probe/submit implementation before footer action enum;
DownloadSettingsView constructors/picker/save before render_setting_control. **9/9 exact checks PASS**.
Accepted files legitimately changed here are called out above: shared action naming only; new/settings
view render/helper calls only. Do not compare whole accepted file hashes and pretend they are unchanged.
No prior parent-review file changed.

## Delivery and remaining acceptance limits

Parent-readable task `evidence/own-delta.json` contains exact per-operation baseline/delivery text and
line positions for all12 existing files, relative to this task's dirty baseline, not HEAD. Manifest
records baseline/current hashes and new files. Logs include valid red, all intermediate fixture failures,
final green/scoped/strict/full/static results and bounded gate diagnosis; hashes permit integrity checks.
Evidence files are review artifacts, not patches to apply. Ad-hoc scripts stayed in session scratch.

No task-specific source/compiled/synthetic blocker remains; parent acceptance still pending. Complete
Desktop and extra Quick Launch gate remain FAILED above. Actual Windows UIA/provider/screen reader,
physical keyboard/pointer/IME, real native picker/clipboard/Shell, actual popup/native lifecycle,
DPI matrices, screenshot/visual comparison, installed product and user acceptance were **not executed /
not accepted**. Semantic TestSupport snapshots do not certify those evidence levels. Stop after #3
handoff; no fifth item without a new single-item assignment.
