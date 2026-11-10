# Implementation

1. Read owning specs and reference notes, verify ABDM then legacy source, and update download reference evidence before migrated interaction changes.
2. H1 vertical slice: bounded engine Batch Remove regression red, nonblocking emission and lock release, regression green.
3. U1 vertical slice: production TestApp batch responsiveness/duplicate/feedback/confirmation red, asynchronous execution and official busy controls, green.
4. M2/U2 vertical slice: no-replace and failed-name reservation regressions red, safe publication and compatible typed recovery, production failed-window action/bounds regressions, green.
5. Verification in order (one Cargo process, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256):
   - cargo test -p nxtools-download --lib
   - cargo test -p nxtools-download --test engine_contract -- --test-threads=1
   - cargo test -p nxtools-download-ipc
   - filtered affected/new nxtools-desktop downloads/download_page/task_window tests -- --test-threads=1
   - cargo clippy -p nxtools-download -p nxtools-desktop --tests -- -D warnings
   - cargo fmt --all -- --check
   - git diff --check
   - required GPUI release/Markdown provenance gates; source lifecycle/spec checks where affected.
6. One directed review at most; fix only concrete in-scope defects and rerun their checks. Record outcomes, compatibility and limits. No commit/archive/push/product launch.

## Evidence
Each check must detect a named failure (deadlock, lost snapshot, overwrite, wrong reservation, stale/duplicate callback, clipped/disabled action, incompatible frame, lint or whitespace); failures change the corresponding implementation or are recorded as unrelated blockers. Cargo timeout requires process inspection before any new build.
