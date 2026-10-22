# CLI reported-fault deduplication (2026-10-13)

## Outcome and scope
Completed narrow behavior-preserving refactor of run_schedule/run_add/run_list rejected outcomes. Existing broad dirty work preserved. No real engine connection, download, host autostart, package/install/service/native user-state operation or commit. CLI specs/shared architecture/engineering/verification and whole-project report read before changes; this is Next-specific CLI output ownership, not a migrated UI/native interaction.

One private reported_fault_error(request_id,fault,json_output,request_rejected) now owns the exact repeated sequence:print_fault -> error_from_fault -> clear already-reported message -> return error. Only3callers use it. Add retains true;Schedule/List false. Existing exits3/4/5/6/7/8, JSON schema/response identity/error fields, text stderr and empty-message convention unchanged. Watch,batch/Action,fetch/connection/startup policies deliberately not merged.

For output testing, the existing post-connection command dispatch and outer JSON terminal guard moved into private run_connected_command. Public run_with_host_policy retains original connection/error handling then calls it. No new trait/manager/formatter/error framework. Same command delegates/order and error guard; actual main.rs unchanged. Production-client selection/startup/default pipe unchanged.

## Real output fixture and red-capability
New cfg(test) output_tests.rs executes run_connected_command -> run_schedule/run_add/run_list -> real DownloadIpcClient over a GUID-owned synthetic current-user named pipe. Fixture peer verifies the actual command kind and returns a fixed request ID and typed Rejected fault; it is not a real engine and never fetches the synthetic URL or validates/creates task files. Test-only existing workspace nxtools-ipc dependency exposes current-user temporary pipe construction; no external dependency.

Parent test launches the test executable with only output_tests::reported_fault_child --nocapture plus child-specific fixture environment. Child connects only its unique temporary pipe, executes real command/result handling, then applies the unchanged main.rs empty-message final print rule and actual process exit. Parent captures actual stdout/stderr/exit with10second owned-child timeout; bounded3second handshake/frame/client waits. Child entry without explicit environment does no command and returns. No environment override added to production CLI, no stdout test-library capture tricks or copied command/error logic.

Matrix36combinations:3commands×2modes×6faults(NotFound,InvalidState,Protocol,Unavailable,InvalidRequest,Network). Asserts exact exit mapping(Addinvalid/network6,others7;common4/5/8/3), JSON stdout one record/stderr empty/schema_version1/fixedrequest_id/code/message/retryable, text stderr exactly one message/newline/noJSON. Test harness banners are not treated as product records.

Baseline matrix passed before helper refactor. Temporarily replaced Schedule's error.message.clear with uniquely tagged DEBUG-CLI-OUTPUT-RED comment:the real matrix failed exit101 at text output with two identical stderr lines versus one expected. This proves regression catches duplicate final reporting rather than merely testing a helper. Temporary mutation removed in actual refactor; no debug marker/ignored test/warning suppression remains. Logs:cli-output-baseline.log,cli-output-duplicate-red.log.

## Preservation evidence
Compared dirty pre-task cli-reported-fault-before.rs,not HEAD.11methods byte-identical:run_show,run_action,fetch_task,connect_with_policy,connect_with_policy_named,start_background_host,print_fault,print_json,safe_fault_value,error_from_fault,cli_error_code. Expanding the new helper into each caller reproduces original3method bodies after whitespace normalization with exact false/true/false arguments. main.rs unchanged. Output/exit comparison independently verifies36captured results.

## Actual final checks
- cargo test -p nxtools-cli -- --test-threads=1:12/12lib tests(10original+output matrix+child entry),0failed/ignored;main/doc0tests. Matrix itself executes36owned subprocess cases; child entry is not misrepresented as another independent assertion.
- cargo clippy -p nxtools-cli --all-targets -- -D warnings:exit0.
- cargo check --workspace --tests:exit0.
- cargo fmt --all -- --check:exit0.
- Temporary mutation removed;11unchanged methods/3equivalent expansions confirmed by deterministic comparison.
- Final spec-contracts/diff check and surviving build inspection recorded at closeout.
- All Rust commands jobs1/incremental0/RUSTFLAGS=-Ccodegen-units256,oneCargo at a time,21600second operation timeout,locked1.96.0 command-local PATH. Complete logs:cli-reported-fault-final.log;source comparison verify-cli-reported-fault-preservation.py in session scratch.

No source/compiled/output-test blocker remains. Actual installed CLI/real engine/download behavior was not exercised and is not claimed; this slice preserves existing source/output contracts through isolated real IPC/terminal execution only. No product release artifact delivered.

Final closeout:spec-contracts87documents/12owners passed;git diff --check exit0;no surviving Cargo/rustc processes. No additional runtime operations.
