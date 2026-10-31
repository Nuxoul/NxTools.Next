# Audit #6 — ordinary rem-relative roles

## Authorized boundary
This is only original audit finding #6 (eighth serial implementation). The first seven accepted slices and their immutable task evidence are prerequisites, not editable history. No cursor, ellipsis/copy, projection performance, storage scheduling, String ownership, native/vendor, dependency, palette, user zoom setting, packaging, installation, or product launch work is included.

The bounded consumers are (1) the daily Download Schedule form and (2) the Download workspace task rows and matching table header. Existing ordinary px elsewhere, including New Download, are explicitly not claimed compliant by this slice.

## Behavior gap and owners
The production Component Root prepares `window.rem_size` from Component Theme.font_size. Product `theme::apply_component_theme` loads bundled themes and changes font families, not a separate scale. Existing schedule spacing/rails/body type and download row heights/text/column widths nevertheless use fixed px, so official rem-based controls and these ordinary layout roles diverge when the base font changes.

Use existing GPUI scale builders where exact default density matches. A small `ui::metrics` role conversion may express approved custom default-pixel ratios as `Rems` against a fixed, verified design base; it must not divide by the current rem or introduce runtime state. Existing constants and font-role values stay unchanged. The expected source default is 16, but an actual product-theme/Root render assertion must confirm it before adopting that baseline. Test default plus 14 and 18; these are controlled base-font fixtures, not Windows DPI evidence.

## Candidate paths and necessity
- `ui/metrics.rs`: one explicit ordinary-role-to-Rems conversion if standard scale builders cannot express custom existing roles.
- `downloads/schedule_view.rs`: relative ordinary spacing/type/rails and responsive Form public label width; one bounded body scroll if needed, fixed title/footer retained.
- `downloads/polish_tests.rs`: append production Root geometry/paint/long-content regression; preserve all prior bytes as a prefix.
- `main_window/download_page.rs`: only task row and matching table-header relative roles, with the existing uniform_list measurement of the actual rendered row. No new selection/list/controller owner.
- `main_window/download_page/kit_migration_tests.rs`: append actual production Root/list geometry/scroll/stable-ID selection regressions, preserving the original prefix.
- `docs/reference-code/download.md`: minimal English append recording this verified layout/reference/API boundary.
`ui/typography.rs` and `gpui-kit-080.md` were captured as investigation candidates; do not edit unless necessary.

## Component selection / exact released API
- Schedule: current Component0.8.0 Form::horizontal().small()/Field, independent Small outlined Toggle, default Switch and default H23/minute TimeField; direct Small outlined Cancel and primary Apply, not Toolbar.child. Form::label_width accepts Pixels (resolve a Rems ratio at the window boundary); label_text_size accepts Rems. Styled scale builders and TimeField input_h use rem. Official public scroll composition retains one body viewport and no action dispatcher.
- Download rows: original custom domain cells/uniform_list are retained because the existing external stable-ID multi-selection/context/drag owner must not be replaced. Official Icon/Progress and actual XSmall ghost compact sort Button remain. The sort Button's full-parent-height and zero-padding adaptation remain; only its custom text role becomes relative.
- Exact sizing.rs shows mixed units: official semantic Button/TimeField heights and type use rem, while Switch track uses fixed 36x20px and no public arbitrary scaled-track slot. Keep this official track unchanged; adapt only its ordinary containing rail. Do not repaint or patch it, and do not claim its track scales.

## Fixed exceptions
Retain native default/minimum/fixed clients (Schedule430x326, MainWindow default1060x650/min920x480), titlebar36/footer48, native insets/hit coordinates/OLE/monitor placement, persistent geometry, physical hairlines and minimum hit tolerances. Keep the existing shell bitmap20px unchanged while its ordinary icon rail and vector fallback may scale. Do not touch Tools180x160 or Clipboard card/image190/files62/gap8/gradient/shadow roles. Existing download tooltip popup caps and unrelated details/filter/native budgets remain unchanged.

## Preserved invariants
ABDM QueueInfoComponent configuration orders weekdays before independent start/stop controls and hides time editing when disabled; its Table keeps stable item keys and one LazyColumn/list scroll. Legacy SettingsWindow.axaml has the scheduler section and cross-midnight explanation. SettingsWindow constructor attaches WindowAnimationService; OnOpened configures scope; ApplySettingsCommand/CloseSettingsCommand precede Close, and OnClosed discards otherwise. The external Avalonia native Window implementation is unavailable; no native acceptance is inferred.

Next retains the keyed open_schedule_window constructor/default/minimum, Root, on_window_should_close/deferred identity cleanup and close_schedule_window boundary. Draft toggles/time editing/validation, #5 held/plain Enter eligibility, save/defer/persistence/close/failure methods and all callbacks stay identical. Layout must preserve normal Tab/focus-visible/Enter/Space/IME/cancel/busy behavior. Download row pointer/context/native drag callbacks, immutable ID selection, sort algorithm and accepted actual Button action remain unchanged. Preserve original behavior sections separately even when the same file has layout edits.

## Red/green seams (already authorized)
1. Actual product theme + production Component Root + DownloadScheduleView: assert runtime default rem, default density, then14/18 ordinary gaps/rails/line bounds and official control bounds. Take the valid original fixed-px failure before implementation. Exercise long Chinese/path validation in the fixed client, real body scroll/end, fixed footer and actual button hit reachability; restore global Component Theme per fixture.
2. Production Root + MainWindow Downloads + uniform_list: default and two variations at default/minimum client; assert actual row/header/column/text/vector proportions and bitmap/hairline exceptions. Verify measured item height updates when rem changes, scrolling reaches a real last row, and pointer selection still addresses its immutable ID.
No source-string or math-helper-only test counts as a production zoom regression. Paint/bounds evidence is synthetic, not visual/user/native/DPI/reader acceptance.

## Verification and evidence
Record each effective red/green separately from compile/fixture/shared-tree errors. Run relevant Schedule/MainWindow production tests and accepted audit05(15), new-download keyboard(8), picker(14), identity(11), names(7), sort(3), prompt display(6) regressions. Strict Desktop Clippy --tests -D warnings, workspace --tests check, fmt/scoped diff and applicable UI/release/catalog/editor/spec/lifecycle gates remain. Run the complete Desktop suite at most once, after convergence; preserve every failure. Known extra QuickLaunch gate1765 is outside scope.

Every Rust shell uses jobs1/incremental0/codegen-units256, naturally waits for other Cargo/rustc to become idle, and uses full-operation timeout without killing/duplicating builds. Only temporary config/SQLite, memory bridge/engine and synthetic content are allowed. No real desktop/Host/service/Shell/picker/clipboard/network/archive/registry/user data.

Each existing path has a current dirty byte baseline and hash (not HEAD). Stop a path on concurrent change; preserve external deltas independently. Publish reversible own deltas, source/docs/log hashes, old-test-prefix and behavior-section comparisons plus immutable prior-history hashes in this new task. Do not rewrite older manifests or accepted/failed checkpoints. Notify the parent and stop after this item; only an explicit later task permits the next audit item.
