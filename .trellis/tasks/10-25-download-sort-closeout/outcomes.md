# Download sortable-header integration closeout — executed results

## Outcome and ownership
User requested the next task after historical Desktop1193PASS/3FAIL. All three sort_keyboard regressions now pass, and this session's ONE integrated Desktop run actually completed **1196PASS/0FAIL/0ignored**,225.36s,exit0. Download-page Kit tests15/15, strict Desktop --tests Clippy and workspace --tests also pass. This closes the previously reported test blockers, not product/native/visual/installed acceptance.

This session made **no Rust/product source edits**. Existing sort owner was actively fixing the original production Button/test fixture. Current helper identifies actual Button element ID rather than mutable name; fixture mounts original MainWindow under official Root. Source already contains official ghost/XSmall/compact Button/public content/accessibility_label and unchanged original sort/scroll/persist owners. Changes are credited to that owner, not this verification session. Original task ../10-25-download-sort-keyboard/prd.md remains independently owned.

## Evidence and actual checks
Session scratch logs use download-sort-closeout-*; wrapper/result JSON retain exits and timestamps.

| Executed check | Result |
| --- | --- |
| Current sort_keyboard_ initial |3PASS/0FAIL/0ignored,13.84s,exit0 |
| main_window::download_page::kit_migration_tests:: |15PASS/0FAIL/0ignored,16.44s,exit0 |
| Desktop --tests strict Clippy -D warnings |PASS,31.45s,exit0 |
| cargo check --workspace --tests |PASS,25.20s,exit0 |
| ONE full Desktop |1196PASS/0FAIL/0ignored,225.36s,exit0 |
| Final current-source sort_keyboard_ |3PASS/0FAIL/0ignored,14.46s,exit0 |
| Final current-source strict Clippy |PASS,1.29s,exit0 |

Full run includes all3sort_keyboard and6production_password tests PASS. Previous1193/3,1159/1,1147/6 remain historical failed runs; no claim they were green then. No fullsuite rerun in this closeout. Current test coverage exercises actual semantic Buttons/names/direction, default/minimum column bounds/focus paint/Tab, held/down/up/duplicate-release/focus-transfer, Enter/Space/pointer equivalence, view-only ordering/stable IDs/selection/anchor, temporary config persistence/reconstructed application.

## Concurrent-source evidence
Observed product download_page.rs SHA256 DBB8CD5F614C5C4A16B7C2AA27905457DD222A1BD9373D3962893F6C1ABBBB7D and downloads.rs BADD97E84410B64E9B6B155EBCE6D94615FD0B29EB2EE7D8BC2408A168A7499F remained identical before/after checks. kit_migration_tests.rs changed during serial verification from E11FF5792D424DA77CFE604B0EBB62B5FCFFD6A9894B9C55B7DAE7D155B2B037 to D11A415C105B5858CAFD3A64A21111B67B3554C7F5E6589D261C998A4AB2BDDF. Exact opening/current diff saved: only2lines add all taskIDs to the selected set before existing preservation assertions, strengthening multi-selection coverage. No product change.

Accordingly the full1196 result is a measured run, not a frozen-whole-worktree certification. Full run compiled again after earlier checks, but build-time exact source identity is not assumed from timing. Final actual3test run and final strictlint verify current strengthened fixture; before/after focused hash matches D11A...BDDF, and all3observed source hashes still match after verification. No second fullrun or forced global freeze/peer pause.

Initial exit75 and final-focused exit75 were skips, tests NOTLAUNCHED. Every launched Rust check inherited jobs1/incremental0/codegen256. One own initial run observed Cargo package/build locks after preflight (natural-idle observation is not an atomic reservation). No peer process termination/contact/pause request, duplicate editor write, baseline restoration or warning suppression.

## Delivery limits
Only new closeout docs and previous task status links updated here. No package/install/binary replacement/service/Registry/COM/native Shell/clipboard/picker/user data/live window/product launch/delegation/commit/push/archive. No real pointer/WindowsIME/mixedDPI/UIA/reader/uservisual/installed acceptance. Existing extension capability gaps and broader Archive obligations remain outside this test closeout. No current failure observed in the executed test set, not a claim every project capability or release gate is complete.

## Final static/document checks
Executed7/7PASS: spec-contracts114documents/14owners, historical componentcatalog/current0.8.0authority, exact GPUI two-file release provenance, pinned editor57originalfiles, DownloadUI/sharedUI lifecycle source gates and extension-doc parity2tests. Scoped source rustfmt--check and docs gitdiff--check PASS. These do not certify native/visual/provider acceptance. Task JSON remains in_progress/source_completed_acceptance_pending/deployedfalse; no automatic commit/archive.
