# Verification

## Outcome

Four corrections are implemented in their original owners. The supplied Sarasa archive opens read-only at the original 64 MiB work limit. Controlled archive, application-icon, About/downloader and Markdown regressions pass. Sarasa UI SC remains an assessment, not a bundled or selected font. Version 0.7.3 and unrelated window-picker work are preserved.

## Reference evidence

- Bandizip 7.32.0.1 and stock 7-Zip 26.00 independently list the supplied archive. Bandizip's retained shipped-binary/Ghidra reconstruction separates catalog Open from later grouped payload decoding; proprietary original native source is not available. Exact references are in [archive design](archive-design.md) and `docs/reference-code/archive.md`.
- Listary's verified managed chain retains an AppsFolder PIDL, calculates icon size and calls IShellItemImageFactory/GetImage ICONONLY on STA lanes. This is managed icon-boundary evidence, not native search-engine source. See [icon design](icons-design.md).
- Winc heading navigation and locked Base Tree 0.8.0 establish separate navigate versus toggle behavior. The production Root pointer regression reproduced parent collapse before the correction and passes with descendants visible after repeated clicks in Preview and Split.

## Compiled and controlled checks

All Cargo commands were serial and inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`.

| Check | Result |
| --- | --- |
| `cargo test --locked -p nxtools-archive --features isolated-codecs --test read_work_contract sevenz_ -- --test-threads=1` | 4 passed |
| `cargo test --locked -p nxtools-archive --features isolated-codecs --lib worker_codecs::sevenz_budget::tests -- --test-threads=1` | 2 passed |
| Independent 134-byte CPython/liblzma fixture with stock 7-Zip listing/integrity | Passed; 2 files, 11 bytes, solid LZMA2:192m |
| `cargo build --locked -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --target x86_64-pc-windows-msvc --target-dir target` | Passed; standalone Debug worker, no package overwrite |
| Supplied archive through rebuilt worker, Open at 64 MiB | Passed; complete, 60 entries, 1,476,080,380 declared bytes, zero output |
| Desktop `windows_apps_` | 6 passed |
| Desktop `main_window::about::` | 8 passed |
| Desktop `installer_download_` | 1 passed |
| Desktop `tools::markdown_reader::` | 27 passed |
| C# test-project build, `--no-restore` | Passed; 0 warnings/errors |
| Effects-reviewed C# `--fixtures "packaged application icon fixture"` | Selected/run/pass 3; skip/fail 0 |
| Strict archive production Clippy, isolated-codecs | Passed |
| Scoped Rust formatting and `git diff --check` | Passed |
| Spec navigation/contracts, GPUI release and pinned editor provenance gates | Passed |

The final Desktop test run used `cargo test --locked -p nxtools-desktop --bin nxtools-desktop <filter> -- --test-threads=1` for each listed filter. C# list-only review selected exactly three Fixture cases with run=0 before execution. Fixtures replace only the raw Shell COM leaf and own their bitmaps/cache files; resolver, bounded STA, factory lifetime, PNG encoding and original cache remain real. No live AppsFolder enumeration, activation or icon extraction was performed.

About tests use an owned SQLite store with its default queue paused before opening the engine. They exercise deferred real admission, task identity, actual durable request defaults, repeat/stale/closed/unavailable owners and changed-default reuse without external network traffic. Compact list snapshots intentionally omit full request options; durable assertions use the original engine task lookup.

## Supplied archive boundary

`D:\Temp\SarasaUi-TTF-1.0.43.7z` is 162,676,868 bytes, 60 files, 10 solid blocks, LZMA2:192m, without encryption. The encoded-header dictionary is 6,144 bytes. Original Open charged the unused payload dictionary plus 4 MiB overhead and returned ResourceLimit below 196 MiB. Metadata Open now validates declarations without charging payload workspace; encoded-header allocation admission remains mandatory.

The rebuilt worker's bounded read-only private loop returned complete with 60 entries at 64 MiB, catalog_loads=1, output_begin_calls=0 and output_write_bytes=0. Its source-acquire response was diagnostic-only: it did not supply the real C# source lease or native Job owner. An independent header-only reader regression rejects any read overlapping packed payload; work counters alone are not an independent ordinary-decoder oracle. Test, extract, selected and retained requests still refuse insufficient workspace before payload/output, and oversized encoded headers still refuse Open. The user archive was not extracted, tested for payload integrity, altered or copied into Git.

## Known limitations

- Desktop strict production Clippy remains blocked by five existing unrelated findings: `main_window/about/emblem.rs:68` unnecessary lazy evaluation; `main_window/extensions_page/activity.rs:16` large enum variant; `tools/markdown_reader/history.rs:170` and `mod.rs:1153` nonminimal Boolean expressions; `tools/markdown_reader/settings.rs:110` unit-value binding. No new correction diagnostics were reported; no warning suppression was added.
- No full workspace/Desktop suite, package, installation, installed-program replacement, real Shell icon appearance, physical pointer/mixed-DPI or user visual acceptance was run. These controlled checks do not certify those levels.
- No installer was downloaded from the network or executed. Accepted update tasks use the existing downloader; protected installation/update authorization is unchanged.
- No fonts were copied, installed, embedded or selected. [Font assessment](fonts.md) records the local weights/names/raw sizes, SIL OFL 1.1 and current private DirectWrite support; actual installer size and 600-weight rendering require the separately chosen bundle.

No commit, Registry/service mutation or live global-hook operation was performed.
