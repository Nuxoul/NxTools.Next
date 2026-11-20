# Verification

## Outcome

Only unmodified SarasaUiSC-Regular.ttf 1.0.43 is embedded, with default UI family `Sarasa UI SC`. Its original 24,049,996 bytes (22.94 MiB) were copied byte-for-byte from the supplied font and retained as borrowed static data. No SemiBold, Bold, italic or light face is bundled. Complete upstream copyright/SIL OFL1.1 is in the source asset and existing About license view. Runtime does not depend on the user's temporary font directory or system font installation.

New and missing UI-family preferences select Sarasa UI SC. Explicit saved families remain authoritative, including Microsoft YaHei UI; unrelated settings Apply/Cancel does not rewrite them. Consolas, sizes, typography roles, theme colors, existing controls and window lifecycle remain unchanged.

## Checks

All Cargo operations were serial with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

| Check | Result |
| --- | --- |
| Core `cargo test --locked -p nxtools-core --lib -- --test-threads=1` | 106 passed |
| Desktop `assets::tests::` | 5 passed |
| Desktop `theme::` | 13 passed |
| Desktop `network_settings::font_settings::` | 2 passed |
| Desktop `ui::typography::` | 2 passed |
| Desktop `main_window::about::` | 8 passed |
| Core strict production Clippy | Passed |
| `python tests/architecture/bundled-ui-font.py` | Passed; actual TTF family/style/400 weight, one face, embedded/default/startup/OFL wiring |
| Scoped Rust format and diff check | Passed |
| Spec/component catalog, GPUI release and pinned editor source gates | Passed |

Desktop tests used `cargo test --locked -p nxtools-desktop --bin nxtools-desktop <filter> -- --test-threads=1`. The Core default regression distinguishes the requested family from the previous implicit default and separately preserves explicit old/English/Chinese preferences. Existing theme paint/selection tests retain Component/Base inheritance, default fallback, monospace and font sizes. The original font draft transaction still covers Cancel, Apply, reopen and failed-save retention.

## Evidence limits

The default GPUI TestPlatform uses NoopTextSystem. Its registration test exercises the production public call but does not certify native DirectWrite registration or real glyph appearance. The resource gate independently parses the real TTF name and OS/2 tables; startup source ordering registers static bytes before Kit/theme/family/window use. The locked Windows backend's in-memory collection was inspected without changing it. Physical/native/mixed-DPI/user visual acceptance and a full Desktop suite were not run.

Desktop strict production Clippy retains the five existing unrelated findings in about/emblem.rs68, extensions_page/activity.rs16, markdown_reader/history.rs170, settings.rs110 and mod.rs1153. No new font diagnostic or warning suppression was added.

No Release packaging, installation, installed-program replacement, actual config mutation, Registry/service change, system-font registration or commit was performed. Compressed installer growth is unmeasured; raw font bytes are not an installer-size claim.
