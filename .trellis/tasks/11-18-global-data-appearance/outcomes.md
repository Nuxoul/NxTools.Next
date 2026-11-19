# Global data and appearance outcomes

## Delivered behavior

- Global Settings has a Data page showing the current software data directory. Selecting an empty, writable local directory starts normal writer shutdown, migration and automatic restart. Directory validation excludes system/install/runtime locations and overlapping source/destination trees.
- Configuration, application databases, managed Clipboard assets and managed extension paths move together. The locator is published last; failed copies or publication keep the old location authoritative. Source data remains as a backup. Downloaded files, separately configured Clipboard storage, runtime cache/logs, Windows integration material and indexer storage retain their existing locations.
- New or missing appearance preferences default to System. Explicit persisted theme choices are preserved.
- Appearance has an official searchable installed-font selector. Apply persists and publishes the selected family; Cancel or failed persistence leaves the active font unchanged. Theme changes preserve the UI font; monospace roles and typography sizes retain their existing behavior.
- Firefox classified response interception waits for durable Held acceptance, cancels the original response and requires terminal cancellation evidence before Commit. Chromium handoff is unchanged. Firefox exports receive the blocking permission without adding it to the shared Chromium manifest.
- Extensionless unfinished downloads request the existing generic Windows Shell icon resource, matching the generic completed-file fallback.

## Automated evidence — 2026-11-19

| Check | Result |
| --- | --- |
| Core library | 103 passed |
| Theme defaults integration | 2 passed |
| Desktop focused settings, migration, theme/font and icon regression set | 65 passed, 0 failed, 0 ignored |
| Recursive extension Node suite | 363 passed, 0 skipped |
| C# test-project Debug build | Passed; 0 warnings, 0 errors |
| Explicit browser signing/export fixtures | 2 passed, 0 skipped |
| `cargo check --workspace --tests` | Passed |
| Browser Native Messaging, download UI, shared UI state and desktop shell lifecycle source gates | Passed |
| GPUI release and Markdown editor provenance gates | Passed |
| Spec contracts | Passed; 131 documents, 14 owners |
| Scoped Rust formatting and whitespace diff check | Passed |

The desktop focused set covers directory-picker cancellation/disconnection/stale validation, copy and locator failures, managed Clipboard assets, approved EXE packages across two migrations, font draft/apply/cancel/failed-save transactions, production font painting through Settings and Base Root file cells, theme changes and the real generic-icon request boundary. Font inventory is injected at the platform enumeration seam in headless tests; these are not installed-font acceptance results.

The two C# fixtures are `browser signing fixture CRX signature and reuse` and `browser signing fixture missing corrupt Firefox cancellation`. No unrestricted native test selection was run.

## Known verification limitations

- The full desktop suite is not green: its full run reports 2005 passed, 31 failed and 1 ignored. The separate focused result does not replace that full-run status. Outstanding cases include missing AC/OPT03 external native exports, archive drag identity and tree label-height assertions, download notification/layout/validation and paused-task fixtures, extension-management action reachability and main-window menu/titlebar assertions. The tree-height case also fails with the production theme applied: 26DIP versus an asserted maximum of 24DIP.
- Strict Core/Desktop test Clippy remains blocked by 103 diagnostics in other existing work, including Markdown reader/history tests, extension activity and About rendering. No diagnostics remain in the new migration, font-selection, typography, theme or Core path implementations.
- Full-workspace formatting reports differences in 9 files, including existing Markdown defaults, archive integration and main-window work. Those unrelated blocks are not reformatted.
- The UI token source gate reports five existing values in category extensions and integration setup: one 16DIP radius, 20/22DIP text sizes and 14/24DIP radii.
- Firefox animation/history, authenticated/container downloads and native integration require installation of a newly exported Firefox extension and physical validation. Unclassified or scripted downloads retain the existing `onCreated` fallback and may still show browser download UI.
- Native directory-picker focus, real-data migration/restart, installed Chinese/English glyph fallback and DPI/visual acceptance remain unverified. No Release packaging, product launch, service changes or real-data migration was performed.
