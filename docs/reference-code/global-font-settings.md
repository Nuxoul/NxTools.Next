# Global UI font settings

## Reference and lifecycle

User item 5 authorizes an installed English/Chinese UI-family choice. The existing defaults remain Microsoft YaHei UI and diagnostic/code Consolas; font sizes and the selected theme graph do not change.

Verified legacy sources in `D:\Code\NxTools`:
- `ViewModels/MainWindowViewModel.cs:627-639`: OpenSettings -> OpenGlobalSettings -> RequestSettingsWindow.
- `Views/MainWindow.axaml.cs:1774-1804`: reuse one SettingsWindow, or construct it with the existing view model; Closed clears the reference and restores main-window foreground when visible.
- `Views/SettingsWindow.axaml.cs:11-24,27-54`: Avalonia Window constructor initializes and attaches animation; OnOpened configures scope/appearance and initial section.
- `Views/MainWindow.axaml.cs:2844-2879,2965-2978`: measure/place -> Window.Show -> foreground; reuse shows only if hidden, restores minimized state, Activate/Focus. Avalonia's native Window implementation is not available in this checkout.
- `Views/SettingsWindow.axaml.cs:56-78,185-194`: explicit close calls CloseSettingsCommand; Apply calls ApplySettingsCommand then Close; unhandled close calls cancel once. `ViewModels/MainWindowViewModel.cs:653-698` owns close status, validation, runtime appearance and durable state save.

Next preserves one registered settings window, existing placement/activation/native close owner, local drafts, pending-save close/duplicate guards and fixed scroll/footer budgets. Cancel/close has no font publication or persistence. Successful durable Apply publishes on the next application turn, independently of Windows startup/tray acknowledgement. Failure retains the draft and old active font. No live global preview, new HWND, native font enumeration or dependency is introduced.

## Released APIs and presentation

- GPUI pre 0.3.9 `TextSystem::all_font_names() -> Vec<String>` (`src/text_system.rs:281-288`) returns sorted unique platform-available families. Use `cx.text_system().all_font_names()` outside render, at draft construction and Apply validation.
- Component 0.8.0 `select::{Select, SelectState, SearchableVec, SelectItem}`: `SelectState::new(delegate, selected_index, window, cx).searchable(true)`, `selected_value()`, `Select::new(&state).accessibility_label(...).search_placeholder(...).menu_max_h(...).disabled(...)`. SearchableVec performs case-insensitive substring filtering, including Chinese text. One single-family selector is enough; the platform text system retains glyph fallback. No CSS comma list or configurable fallback chain.
- Released Windows backend `vendor/gpui-pre-windows-039/src/direct_write.rs:544-569` creates DirectWrite text format/layout and only overrides FontFallback when explicit GPUI fallbacks exist. This keeps the existing system glyph-fallback path for an English primary family; the preference does not construct or persist a fallback list.
- Compose `Settings -> SettingPage -> SettingGroup -> SettingItem -> SettingField::render` with the existing aligned-setting row. The selector uses released Medium/default 32 DIP appearance, a 240 DIP lane, bounded 240 DIP menu and no Toolbar adaptation. No hand-painted control, extra focus controller or global preview.
- Component `Theme::update` synchronizes the public font family to Base typography; Component Root's `WindowState::style` (`vendor/gpui-component-080/src/root.rs:542-548`) supplies it to ordinary inherited text. Custom product roots must not override that family with the default literal. Explicit monospace remains Consolas.

## Preference, availability and transaction

`SystemSettings.ui_font_family` is a serde-default exact single family. Core bounds it to 256 UTF-8 bytes, nonblank, no control characters; Desktop validates actual membership against available names. Persist the requested family, not a display label. A missing saved font remains visible as unavailable and is retained on unrelated Apply; rendering falls back to Microsoft YaHei UI until it is available again or explicitly changed. Unknown new selections are rejected. The original atomic global-settings section transaction owns persistence and unrelated section preservation.

Diagnostics belong to the existing correlated settings save boundary: record font_changed and font_available booleans with accepted/start, persisted, next-turn presentation applied and terminal success/failure/elapsed events. Never log family text, the installed catalog, config paths or raw errors. Font fallback is a bounded presentation status, not a Windows shell failure; no native operation or IPC is needed.

## Desktop integration

`network_settings.rs` constructs one `FontSettingsDraft` from the saved system preference. The individual 界面字体 field belongs to the official Appearance page. The original view validates its selected family before entering busy state and includes it in the atomic SystemSettings transaction; validation errors use the existing inline owner.

After durable success, the application defers `theme::set_ui_font_family` alongside theme publication. Startup calls the same setter before windows show. The setter reports availability without replacing unavailable persisted intent. Production transaction regression covers Cancel, Apply, reopen and failed-save draft/active-font retention.

## Verification

Core tests cover old JSON defaults, English/Chinese roundtrip, bounded invalid preferences and original atomic global-settings persistence preserving other sections and bytes on failure. Typography tests cover available/missing/default/CSS-list selection. Theme tests exercise the production application boundary and theme switching, assert Component/Base typography and actual rendered inherited text, and preserve Consolas and sizes. Run Core and Desktop tests with the mandated memory environment in one coordinated Cargo session. Source/rustfmt checks do not establish physical keyboard/pointer, mixed-DPI or user visual acceptance.
