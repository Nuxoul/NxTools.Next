# Audit #7 — Desktop cursor semantics

## Scope and current status
User authorizes direct current-session repair, no delegation, in D:/Code/NxTools.Next-main. Ordinary Button/Checkbox/Menu/Tab/native command controls use Arrow. Genuine external links retain PointingHand. Text/Resize/Grab/Prohibited cursors describe actual manipulation only. This is not a blanket pointer removal or a request to restyle controls.

**INCOMPLETE: blocked before effective production paint-hit red.** Independent Cargo/test activity prevented natural-idle Rust admission. No production source, active test, vendor or reference was edited. #7 remains unclosed. Earlier #6 and Queue08/09/10 incomplete states remain unchanged.

## Authoritative rule
Original saved design guide, Pointer conventions lines470–475: default arrow for buttons, checkboxes, menu items, tabs and native controls; pointing hand for links/link-like content; text/resize/grab/prohibited only for active manipulation. Lines507–532 distinguish application commands from external URL/web/email resources. Source: https://gpui-kit.com/docs/design-guides/ (GPUI Kit; documentation CC BY4.0, software Apache2.0). The original saved text is retained unchanged in evidence/design-guide-original.txt; this record is a task-specific interpretation, not a new website audit.

Read AGENTS, component-first mandatory/current0.8.0/table/API, applicable Desktop/shared contracts and exact released source. Current Kit/Base/Component0.8.0/pre0.3.9 remains pinned; no dependency, vendor, theme, typography, rem-role or native-policy change.

## Smallest behavior gap and candidate boundary
Actual source declarations are in ui/components/action.rs::focused_action, path_list.rs::path_row, volume_scope.rs automatic/volume rows, custom_command_editor.rs::command_master_row; QuickLaunch result/action and attached default/result rows; Downloads NewDownload category choices; Clipboard add-category control. They are commands or selection nodes with explicit hand declarations, not proven runtime cursor results yet. ui/components/list.rs is file-level cfg(test), not production defect evidence.

Potential edits after red are cursor declarations only at those exact sources, using existing Styled cursor semantics; no helper API/controller is necessary merely to set Arrow. Actual paint/hit must determine whether inherited default or explicit cursor_default is required, including inner child declarations. Do not force Arrow across nested real links or text/resize regions. Candidate deletion is not predetermined before effective red.

Preserve appearance, names/labels/variants/sizes/IDs, same FocusHandle/InputState, disabled eligibility, single original click/KeyUp owner, handlers/callbacks/defer, drag capture/release/cancel, #1 keyboard/#9 picker/#10 history stable-ID protections, all accepted rem work and fixed clients. No #8 wording/#11 projection/#12 IO/#13 String work.

## Agreed regression seam
User expressly requests actual production elements and paint-hit cursor, normal/hover/disabled plus link/drag exceptions. A source-string test or model of the cursor resolver is not a substitute. Observe the real framework's current painted/hit result after fresh frame and synthetic movement only; no real OS pointer/native operation. Preserve actual pointer/Tab/Enter/Space/held/modifier/disabled/release-cancel and once-only behavior with existing fixtures. External-resource control may be rendered but must not launch URLs/network/Shell.

The inspected Frame::cursor_style and TestPlatform.active_cursor are private. A compliant public/test observation seam has not yet been established or compiled. Do not invent a getter, edit vendor or install a competing cursor resolver to satisfy this requirement; continue exact public API investigation when execution is available. Lack of a found seam is not a declaration of universal framework incapability.

## Verification and safety
Every Rust invocation including rustfmt/direct binaries inherits CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256; natural idle and serial full-timeout execution. No process kill/pause/yield coordination/lock bypass/duplicate own build. No complete Desktop run.

After effective red/minimal repair: relevant UI/QuickLaunch/Downloads/affected consumers, first-seven gesture/focus protections, production/test strict Clippy, workspace check, scoped/workspace fmt and source gates. No real pointer/native/HWND/user data/clipboard/paste/network/archive/picker/Shell/Host/service/product launch, package/install/commit/push. No native/DPI/IME/reader/visual/user/release acceptance implied.

## Live dirty baseline
Captured2026-10-26T01:50:53.945476+00:00:380 current source/contract/released-source files and2264 historical task/evidence files. Basis is live bytes, not HEAD. Whole-file protection preserves all old callbacks/prefixes/rem changes for this no-source-edit checkpoint. Prior manifests/outcomes remain immutable.
