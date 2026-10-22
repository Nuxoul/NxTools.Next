# Implementation order

1. Coordinate peer file/verification lane, preserve dirty baseline and reverify reference/native/Kit entries.
2. Add red thumbnail worker/cache lifecycle tests; implement bounded derived-thumbnail production integration; scoped test run.
3. Add actual-panel keyboard regressions; implement number/page actions through existing owner and guard IME/modal/pending/held input.
4. Add summary query/full-by-ID tests; implement separate projection, integrate hydration for actions/edit/preview/extension/open targets; assert old full APIs and archive remain unchanged.
5. Add native pause read-guard/config plus actual-panel runtime tests; integrate runtime desired/applied/pending/deadline/reconnect state and explicit UI.
6. Run one integrated Desktop suite and affected Clipboard/Core/Protocol/C# safe fixtures; strict scoped checks, architecture gates, optional single directed review, outcomes and limits.

All Cargo commands initialize CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 and preflight all-machine build/test occupancy with full-operation timeout and scratch logs. After the user's correction, work continues only in this session: no delegation or messages/tasks to other sessions. Wait for an observed free lane without terminating another process. Preserve shared dirty files and do not start duplicates after timeout. No automatic commit/archive/package/install/service.

## Final source checkpoint
All four implementation slices and affected automated checks completed; see outcomes.md for exact counts, initial failures/fixes and remaining physical/visual/native acceptance. No real clipboard/history operation, package, install, service or commit was performed.
