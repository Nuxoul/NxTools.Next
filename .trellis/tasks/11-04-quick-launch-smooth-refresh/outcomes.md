# Quick Launch refresh and selection

## Delivered code

- New ordinary input resets default-first selection immediately. Current batches keep rank zero selected until an explicit keyboard/pointer choice; deliberate choices follow their result identity within the query, including an explicit first-row choice.
- The original panel owns selection for extension publication as well as ordinary results.
- Retained old-query rows preserve normal text, committed-query highlights and first-row background. Execution, preview, action-mode entry and pending navigation remain blocked until current results arrive.
- Application rows without an explicit description use one centered14DIP title. File/directory subtitles show parent paths. Explicit descriptions, complete accessible labels and action targets remain unchanged.
- Original40DIP row body,26/48rails,4-or0external gap, window geometry, current-result early publication, provider ranking and native focus/preview owners remain.

## Verification

| Check | Result |
| --- | --- |
| Pre-change `cargo test -p nxtools-desktop quick_launch::tests::refresh_tests -- --test-threads=1` | Six tests executed; five asserted the reported gaps, explicit-selection baseline passed. |
| `cargo clippy -p nxtools-desktop --bin nxtools-desktop -- -D warnings` | Passed for the implemented production code. |
| `tests/architecture/quick-launch-lifecycle.ps1` | Passed; title/optional-subtitle source assertions use the revised contract, fixed geometry and lifecycle protections remain. |
| `python tests/architecture/gpui-release-contracts.py` | Passed. |
| `python tests/architecture/markdown-editor-contracts.py` | Passed. |
| `python tests/architecture/spec-contracts.py` | Passed:125documents/14owners. |
| Scoped rustfmt and git diff whitespace check | Passed. |
| Post-change `cargo test -p nxtools-desktop quick_launch:: -- --test-threads=1` | Blocked before tests execute by archive module compilation. No green interaction result claimed. |

All Cargo commands used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

## Remaining acceptance

The latest attempted test build reports E0433 at `rust/apps/nxtools-desktop/src/archive/drag_progress.rs:87`: `super::progress::DRAG_TITLE` cannot resolve `progress` in that parent module. Archive/protocol sources are under concurrent modification. Resume the exact Quick Launch test command after that owner compiles, then desktop full tests and strict test-target lint. No further archive behavior changes are part of this task.

The focused regression file is `rust/apps/nxtools-desktop/src/quick_launch/refresh_tests.rs`; existing heterogeneous-row and retained-selection fixtures use the actual selection API and revised title height. Installed application, physical keyboard/IME, native/DPI and user visual acceptance remain unperformed. No package, install, service restart or product launch was performed.

## Additional build hygiene

Removed one duplicate Serde default annotation on ArchiveWorkObservation.zip_inflate_us and imported the existing Envelope type in archive/application_tests.rs. Neither change alters archive behavior or data fields. Other pre-existing and concurrent workspace changes were preserved.
