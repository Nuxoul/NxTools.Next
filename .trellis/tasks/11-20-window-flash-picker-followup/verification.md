# Verification

## Outcome

About uses a fixed420x480 modeless Normal window and one existing client-caption Close action. Its creation factory is the actual open_about input; it no longer enters the framework modal parent-disable/re-enable/forced-activation branch. The transparent caption and native drag-only area remain; client close retires update ownership before one GPUI removal. Copy/license/update owners and other dialog kinds are preserved.

Blacklist selection hides captured visible top-level windows of the originating Desktop process after native preparation/hook admission and restores surviving captured windows on terminal/rollback. Bounds, normal/minimized/maximized state and originally hidden/foreign/overlay windows are preserved by the visibility-only policy. Source focus/input refill remains Rust-deferred, with no automatic rule addition or persistence. The two input-row preset Checkboxes are removed; row scope controls and original new-rule defaults remain. Editing/Update retains the scopes just changed in that row.

## Main-window flash evidence

The user identifies the background main window, not About itself, as the flashing surface. The inspected locked WindowsWindow::new Dialog branch calls EnableWindow(parent,false); handle_destroy_msg calls EnableWindow(parent,true) and SetForegroundWindow(parent). About had selected that branch despite having no parent-blocking decision workflow. Its Normal creation policy removes this concrete parent-state transition. Existing Global Settings already uses Normal.

The production constructor test and source/native-policy gate protect that decision. The caption pointer regression closes the actual TestWindow once and verifies retired update ownership. The shared settled entrance remains stable through repeated ordinary notification/full-refresh invalidations, so no production animation shutdown was added. Read-only native-path investigation found no ordinary foreground route that deliberately resizes/recreates main buffers; existing Direct3D log records were INFO, without device-loss tokens in the bounded search. These facts do not prove physical frame stability or identify every reported page/data-update flash.

## Compiled and controlled checks

Every Cargo invocation was serial with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

| Check | Result |
| --- | --- |
| Desktop `network_settings::` | 51 passed |
| Desktop `window_feedback_` | 6 passed; subset of the listed owner groups |
| Desktop `main_window::about::` | 11 passed |
| Desktop `ui::components::titlebar::` | 2 passed |
| Desktop `ui::motion::` | 4 passed |
| C# test project build, --no-restore | Passed; 0 warnings/errors |
| Exact effects-reviewed process window pick fixtures | selected/run/pass12; skip/fail/not_run0 |
| About source/native policy gate | Passed |
| Desktop shell lifecycle source gate | Passed; includes9 approved-icon regressions |
| Spec/component catalog, GPUI release and pinned editor provenance | Passed |
| Scoped Rust format and git diff --check | Passed |

Desktop command form: `cargo test --locked -p nxtools-desktop --bin nxtools-desktop <filter> -- --test-threads=1`. The final related owner groups contain68 unique Desktop tests. The six feedback cases cover modeless/fixed constructor capabilities, Close-only caption, one close/update retirement, removed editor presets with local Add/defaults, both live-row scopes surviving Update, and no settled-animation replay. Busy layout still checks a real disabled rule-row Checkbox and locked navigation rather than deleted controls.

Exact C# commands/names and injected effects are recorded in [native visibility evidence](native-picker.md). Its12fixtures include19native lifecycle variants and8startup rollback variants through production controller/native adapter seams. Facts/hooks/process/clock are injected; one GUID-owned pipe is used. They do not enumerate/hide/show real windows or install global hooks. One bounded source review found no concrete defects in native restoration/client close/row-scope boundaries.

## Known limitations

- Native GPUI pixels, parent-window flash, actual Win32 hide/show/state/focus and mixed-DPI interaction remain unexecuted. TestWindow and injected native facts are controlled evidence, not physical/user acceptance. Broader page/data-update flashing is not declared fully resolved without real reproduction.
- Desktop strict production Clippy remains blocked by the five existing unrelated findings: about/emblem.rs68, extensions_page/activity.rs16, markdown_reader/history.rs170, settings.rs110 and mod.rs1153. No new in-scope production diagnostics or warning suppressions were added.
- An OS-rejected restore reports restore_failed/native code and continues attempting other captured HWNDs; a rejected HWND can remain hidden. The real boundary is documented in native-picker.md.
- No full Desktop/workspace suite, commit, Release package, installation, installed-program replacement, actual configuration mutation, Registry/service operation or live global hook was performed. The running installed program is unchanged.
