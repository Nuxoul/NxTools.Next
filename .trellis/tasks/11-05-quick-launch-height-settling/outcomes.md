# Quick Launch height settling

## Result

Ordinary search results no longer cause native viewport contraction from intermediate local/index/script batches. Current rows remain immediately actionable and growth is immediate. Once current providers finish, contraction requires the same popup session, query generation and target height for180ms. Expiry re-reads current state; newer input, changed height, closing and disconnect invalidate earlier intent. Clear input and action/recent modes retain immediate sizing.

The behavior lives in the existing QuickLaunchApplication resize dispatcher. Extension invocation completion updates generation-scoped running counts and reevaluates that dispatcher. No ranking, provider result publication, row geometry, theme, component, native adapter, protocol or preview/focus ownership change.

## Cause

The previous content sizing applied every published result count immediately. A current local result could replace a tall prior list before the index finished, request a short native window, then request a tall one when indexed results arrived. Result availability and permission to contract now have separate conditions.

## Verification

- `cargo test -p nxtools-desktop quick_launch:: -- --test-threads=1`:188passed/0failed. Native command regressions cover intermediate batch suppression, no shrink/grow pair, immediate growth, quiet final contraction and supersession on new input; extension completion remains generation-scoped.
- `cargo clippy -p nxtools-desktop --tests -- -D warnings`:passed.
- Quick Launch lifecycle, GPUI release, pinned Markdown editor and spec-contract gates:passed.
- Scoped rustfmt and diff whitespace:passed.
- All Rust commands used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

## Limitations

The subsequent `cargo check --workspace --tests` was blocked by concurrent archive edits: missing `edit_request_current`/`submit_drop_add` methods and changed call signatures in archive application/tests. No archive changes were made for this task. A new whole-Desktop test run and release acceptance are not claimed.

The passing regression exercises the production typed native resize dispatcher with controlled input/provider states and test clocks; physical HWND/DPI interaction and user visual acceptance remain separate. No package, installation, service operation, product launch, commit or push was performed.
