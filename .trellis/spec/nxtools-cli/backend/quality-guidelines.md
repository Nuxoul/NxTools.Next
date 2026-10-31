# CLI quality

Evidence: `repo:rust/apps/nxtools-cli/src/lib.rs#tests`, `repo:rust/apps/nxtools-cli/src/lib.rs#parse_invocation`.

Test actual parse_invocation/parse_args combinations, optional download prefix, --connect-only placement, invalid schedule/IDs/headers, exit mapping and JSON projection. Watch uses authoritative generation/sequence and bounded caller policy; do not fake completion from launch success. Fixtures use synthetic tasks/temp directories, no product autostart. `cargo test -p nxtools-cli --lib` and strict scoped checks follow [verification](../../shared/verification.md).

Rejected schedule/add/list output uses one private reported_fault_error(request_id,fault,json_output,request_rejected):emit through unchanged print_fault, map stable exit, clear already-reported message. Add alone passes true; schedule/list false. Empty-message convention prevents outer JSON fallback/main stderr from duplicate reporting; do not merge watch/batch/connection policies. `repo:rust/apps/nxtools-cli/src/output_tests.rs` executes real command/result handling over GUID-owned synthetic named pipes in test subprocesses and captures stdout/stderr/exit.36cases cover text/JSON, exact response request ID,sixfault categories and single emission. Temporarily removing schedule message.clear produced two stderr lines(red),then mutation removed. Child test entry without explicit fixture environment does no operation; no real engine/start/download. Preserve source-equivalence evidence and strict checks.

Archive public-entry tests: `repo:rust/apps/nxtools-cli/tests/archive_cli.rs`; original-router
receipt/prompt/terminal regressions: `repo:rust/apps/nxtools-cli/src/archive/execution/tests.rs`.
Opt-in real matching-build gate `repo:tests/integration/archive-cli.py` generates GUID-owned
small inputs/output and retains evidence: exact metadata/CRC/bytes/empty+implicit tree, all four
conflict policies, destination permission, traversal/missing/unsupported and single JSON failure.
Normal child EOF must not race event-channel completion into a false host failure: stop token
precedes original session Stop. Actual success plus childexit0 regression guards this boundary.
No real Desktop/installed/GUI/Explorer/performance acceptance or recursive output cleanup.

Lifecycle-only opt-in gate: `repo:tests/integration/archive-cli-lifecycle.py` and scratch build
`repo:tests/integration/prepare-archive-cli-lifecycle.ps1`. New Windows console must contain exactly
own driver+CLI before any CTRL_C_EVENT; never signal current tool console. Original output/cleanup
checkpoints use finite waits and small literal samples. `repo:tests/integration/ArchiveCliLifecycleFixture.cs`
is compiled ONLY into the scratch source-copy, not a product test mode; disclose that evidence level.
Real signal -> original cancel -> no premature JSON before cleanup -> one cancelled result, commits
retained, exact owned children exit and source/helper exclusive reopen. Parent death cannot return
JSON; EOF branch versus parent-monitor branch is observed separately. No same-name process kills.
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveCliCleanupLifecycleTests.cs` proves original runner
Drain(false) keeps slots/leases/no terminal credit; release then exact cleanup. Not public timeout
completion or a reason to rewrite inherited session disposal. Default diagnostic capture is scoped
by own PID/date; no old-log cleanup, speculative sink failure or complete-runtime reliability claim.
