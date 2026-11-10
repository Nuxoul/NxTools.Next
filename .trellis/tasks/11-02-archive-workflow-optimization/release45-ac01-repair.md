# Release 45 — R45-AC01 output abort contract

Parent: **Phase E — Format expansion and ZIP editing / in_progress**. Scope: the multi-output abort finding in [independent acceptance](release45-independent-acceptance.md).

## Change boundary

`worker_codecs::run_sevenz` keeps original output tokens until complete decoder integrity and source verification. Failure cleanup aborts them in creation order. `PrivateOutput::call_payload` must treat every abort as a discard boundary, including an older live token while the current token has buffered bytes. An abort never flushes another member and is not suppressed by the coalescer's sticky write failure. The original cancellation/pipe checks and native actual-disposal authority remain.

Production scope is the abort branch in `worker_output.rs`; regressions belong to `worker_output_bulk_tests.rs` at the original ArchiveOutput → binary/JSON → ReplyPort boundary. Tests use two nonempty members and the real forward abort order, including rejected writes and a preexisting sticky failure. Original successful begin/metadata/source_precommit/commit barriers, first failure, token/sequence/ACK and budgets are preserved. No UI, protocol, codec scheduling, native ownership, performance policy, packaging or physical receiver change.

The saved Bandizip solid/output reference distinguishes per-member result and buffered close/drain; it does not replace Next's explicit failed-output discard contract. No new reverse engineering or reference execution is involved.

## Implementation and verification

`PrivateOutput::call_payload` handles `action == "abort"` independently of the current extraction token. It clears pending bytes/current-token staging and proceeds through the original checkpoint and native call; it does not reset `write_failure`. Successful barriers are unchanged. The production change is one condition plus its ownership comment. The C#7z fixture now reports its synthetic sample, expected/actual category, native code and cleanup result on failure without changing assertions.

Three checked-in regressions use two nonempty members through real begin/write/ACK/abort. All three failed on the original implementation and pass after the repair:

- `cross_member_abort_discards_unverified_tail_without_new_write`: A's3 acknowledged bytes remain the only write; B's13-byte tail is discarded; both aborts reach the peer.
- `cross_member_abort_write_failure_does_not_skip_earlier_abort`: a peer configured to reject B's write receives no cleanup write and still receives both aborts.
- `cross_member_abort_preserves_sticky_failure_without_suppressing_cleanup`: an actual failed32KiB B write does not suppress A/B aborts, replay bytes or permit a later commit; the original IoFailure remains.

| Check | Result |
| --- | --- |
| Output boundary, including3 new cases |10/10 pass |
| Actual-process binary transfer |8/8 pass |
| Actual-worker extraction contracts |10/10 pass |
| Archive strict Clippy with isolated-codecs/tests |Pass, `-D warnings` |
| Final Debug worker / C# test host build |Pass; C#0 warnings/errors |
| Spec navigation/contracts; scoped rustfmt and diff whitespace |Pass |
| Original C#7z handle-session fixture |LIST1/RUN1/PASS0/FAIL1/SKIP0; low-budget category mismatch described below |

All Cargo commands used offline mode, one job, incremental disabled and codegen-units256, waiting for the other session's build lane. Test inputs are checked-in attributed fixtures or owned synthetic buffers/output; the user's ZIP is untouched. Evidence: [regression failure](../../../.diagnostics/archive-root-drag/20261104-r45-ac01/r45-ac01-red.log), [Rust verification](../../../.diagnostics/archive-root-drag/20261104-r45-ac01/r45-ac01-green.log), [native final](../../../.diagnostics/archive-root-drag/20261104-r45-ac01/r45-ac01-native-final.log), [old/new native comparison](../../../.diagnostics/archive-root-drag/20261104-r45-ac01/r45-ac01-native-comparison.log), [source gates](../../../.diagnostics/archive-root-drag/20261104-r45-ac01/r45-ac01-source-gates.log).

## Remaining verification boundary

The native7z fixture's `lzma.7z` case with `max_output_bytes=1` expects ResourceLimit but receives worker_failure/IoFailure with no native error code and CleanupComplete=true. The retained pre-coalescing Debug worker and final repaired worker both reproduce that result. Earlier plain/solid/non-solid/encrypted success and password cases complete; the fixture stops at low-budget, so its later collision/corrupt cleanup assertions are not claimed as executed. This existing native preservation failure remains a separate blocker to full output-slice acceptance; no assertion or production error mapping is relaxed here.

R45-AC01's original-output-boundary regression is repaired; the historical independent partial verdict is retained, not rewritten into independent acceptance. No fresh performance measurements, physical DOpus/Bandizip/GUI, Release package/install, service/registry changes or commit/push occurred. Prior sequential/ABBA results keep their original scope and do not imply stable throughput. Parent Phase E remains in_progress.

## Directed independent acceptance

[Current independent verdict](release45-ac01-independent-acceptance.md) closes **R45-AC01 at the original output boundary**: the unchanged original probe passes2/2, checked-in output/bulk/extraction tests pass28/28, and sticky first-failure behavior is preserved. Full output-slice acceptance remains partial: the exact native7z selection still fails its low-budget ResourceLimit-versus-IoFailure assertion, and later collision/corruption cases remain unexecuted.

## Native preservation follow-up — 2026-11-05

The low-budget blocker recorded above has [directed repair evidence](release45-sevenz-limit-repair.md): native source admission's explicit resource_limit now remains ResourceLimit through the original worker. Full7z fixture LIST1/RUN1/PASS1/SKIP0 completes all10cases, including collision and both corrupted-output cleanup scenarios. R45-AC01's3regressions remain green. The earlier failed logs and independent verdicts retain their original scope.
