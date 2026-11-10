# B3 coordinator acceptance

## Verdict

**Accepted at source, production TestPlatform and owned-native fixture boundaries. Further implementation remains paused.**

REF-17 and REF-18 satisfy the scoped B3 acceptance requirements. B3-01, B3-02, B3-E1 and B3-E2 are closed. No open B3 product finding remains in the reviewed repair. Archive remains **Phase E — Format expansion and ZIP editing / in_progress**; this verdict does not close the parent archive or release gates.

SessionTask acceptance for original delivery `a2fad500-827e-40aa-a117-7186d0df09a7`, turn `5c029aec-3329-40ee-ac97-7db7266f1317`, was recorded at `2026-11-10T07:35:24.203Z`, incorporating the user-supplied [repair handoff](acceptance-repair.md). B1/B2 retain their recorded acceptance. No B4 or further implementation task was dispatched.

## Closed acceptance items

| Item | Verified production boundary |
| --- | --- |
| B3-01 — Validation reason | [Unit selection](../../../rust/apps/nxtools-desktop/src/archive/forms_volume.rs:85) preserves parsing, minimum-size and range errors; only valid nondivisible bytes receive exact-unit guidance. [Actual-control regression](../../../rust/apps/nxtools-desktop/src/archive/forms_tests.rs:2572) covers zero,63KiB,fractional/overflowing input,65KiB nondivisibility and correction to2048KiB→2MiB. Rejection retains text/unit/InputState and does not submit. |
| B3-02 — Keyboard confirmation | [Official confirmation Button](../../../rust/apps/nxtools-desktop/src/tools/raw_join.rs:283) dispatches public Confirm only for keyboard ClickEvent, retaining DialogAction's pointer owner and the original deferred generation-scoped on_ok. [First keyboard activation tests](../../../rust/apps/nxtools-desktop/src/tools/raw_join/tests.rs:719) observe actual semantic focus and prove first Enter/Space each queues one matching Start; Escape/back queue none, repeat queues no duplicate. Original mouse/cancel/close/late-result tests also pass. |
| B3-E1 — Preset and Step interaction | [Real preset/+/- test](../../../rust/apps/nxtools-desktop/src/archive/forms_tests.rs:2655) operates the actual preset Select and official increment/decrement hitboxes at680×600 and480×360. It verifies100MiB→101MiB→100MiB, exact104857600/105906176/104857600bytes, stable original input/focus, no outer Submit and reachable fixed32DIP footer. |
| B3-E2 — Submit to immutable Job/wire | [Original Application test](../../../rust/apps/nxtools-desktop/src/archive/application_tests.rs:3747) types65KiB and clicks actual Submit, then verifies one original JobRecord and one outbound ArchiveJob with the same entire command and66560bytes. Changing the retained submitted draft to128MiB leaves accepted/outbound commands unchanged and queues no second Start. The narrow mutation probe remains inside [cfg(test)](../../../rust/apps/nxtools-desktop/src/archive/forms.rs:145); it does not create admission evidence. |

The scoped [six-file source delta](evidence/coordinator-repair-source.diff) contains two product fixes, original test-module additions and the test-only probe. It does not alter writers, native IO, queue/transport implementations, dependencies/vendor or the accepted layout/visible variants. The supplied original RED logs were read: invalid input0 produced divisibility guidance, and first Space left the Start queue empty; first Enter already passed. E1/E2 are actual coverage additions, not claimed failing product cases.

## Independently executed repair verification

Every selection was LISTed and then RUN with the same package/target/filter. All listed cases ran; none failed or were ignored. Counts overlap and are not a unique cumulative total.

| Selection | Result | Retained run evidence |
| --- | --- | --- |
| Desktop `ref17_` | 8/8 | [REF-17](evidence/b3-reaccept-ref17.log) |
| Desktop `ref18_` | 11/11 | [REF-18](evidence/b3-reaccept-ref18.log) |
| Desktop `archive::forms::tests` | 48/48 | [Forms](evidence/b3-reaccept-forms.log) |
| Desktop `archive::draft::tests` | 12/12 | [Draft](evidence/b3-reaccept-draft.log) |
| Desktop `archive::application::jobs::tests` | 129/129 | [Jobs](evidence/b3-reaccept-jobs.log) |
| Desktop `archive_polish_quick` | 3/3 | [Quick](evidence/b3-reaccept-quick.log) |
| Desktop `tools::polish_tests` | 11/11 | [Tools](evidence/b3-reaccept-tools.log) |
| Core `--lib ref17_` | 2/2 | [Core](evidence/b3-reaccept-core-ref17.log) |
| Protocol `--lib ref18_` | 1/1 | [Protocol](evidence/b3-reaccept-protocol-ref18.log) |
| IPC `--lib` | 62/62 | [IPC](evidence/b3-reaccept-ipc.log) |
| Six repair files, scoped rustfmt | Exit0 | [Format](evidence/b3-reaccept-rustfmt.log) |
| Spec contracts | PASS129documents/14owners | [Specs](evidence/b3-reaccept-spec-contracts.log) |
| Component catalog | PASS | [Catalog](evidence/b3-reaccept-gpui-component-catalog.log) |
| Kit release/provenance | PASS | [Kit](evidence/b3-reaccept-gpui-release-contracts.log) |
| Pinned editor contracts | PASS | [Editor](evidence/b3-reaccept-markdown-editor-contracts.log) |

[Exact commands, exit codes, counts and timings](evidence/coordinator-reacceptance-results.json) and matching `*-list.log` files retain this round's executed results. All Rust invocations inherited jobs1/incremental0/codegen-units256 and coordinator scratch TEMP/TMP from the same PowerShell execution. Tests use `cargo test --locked --offline` and `--nocapture --test-threads=1`; scoped format uses `rustfmt --edition 2024 --check --config skip_children=true` on the six repair files.

Job129 consumed the repair delivery's actual C# exports, not fabricated JSON or newly launched native operations. Retained copies are [progress wire](evidence/reaccept-job-exports/b3-repair-job-progress-wire.json), [ordinary ZIP](evidence/reaccept-job-exports/ordinary.zip.json), [split ZIP](evidence/reaccept-job-exports/split.zip.json) and [split7z](evidence/reaccept-job-exports/split.7z.json). The execution used `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE` for the progress file and `NXTOOLS_OPT03_CREATE_WIRE` for the directory containing the three create exports.

## Retained independent native evidence

The prior coordinator round independently LISTed/RAN genuine native split3 and raw-join4 with zero skip/fail/not_run. The repair leaves their native/writer boundaries unchanged; this repair round did not rerun those native selections.

- [ZIP/7z split3](evidence/b3-coordinator-split-native.log): real65KiB volumes and independent7-Zip extraction/byte readback, existing-family refusal,1024-volume bound, partial publication and pre/late-cancel facts.
- [Raw join4](evidence/b3-coordinator-raw-join-native.log): exact later-part join, numeric/source/family/output refusals, one64KiB streaming buffer, write112 failure, source-handle protection/release, cancellation/actual cleanup and original serial reader responsiveness/drain.

These establish owned-native/readback facts, not comparative throughput, total-process peak memory or installed acceptance. Generic raw discovery still cannot prove that an unknown final part exists.

## Limitations

Strict Desktop Clippy was independently rerun and returned101 with only the two unchanged download baselines: [browser_handoff_impl.rs:436](../../../rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs:436) `nonminimal_bool` and [settings_task_rem_tests.rs:612](../../../rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs:612) `unwrap_used`. [Exact diagnostic log](evidence/b3-reaccept-desktop-clippy.log) is retained. There is no full strict-lint or whole-workspace green claim; historical failures outside this scope remain separately attributed.

Physical Windows IME/DPI/provider/OS-close/hotkey/Shell, human visual, installed/Release and full-workspace acceptance remain unperformed. TestPlatform semantic focus and owned native fixtures do not certify those layers.

## Coordination state

B3 is accepted and completed at the stated boundaries. Canonical archive Phase E remains in_progress. Final process scan found zero Cargo/rustc/clippy/link, Desktop test, fixture-apphost or archive-worker verification processes. The coordinator changed only acceptance/task/coordination records and retained evidence, not product source. No packaging/install, product/reference GUI launch, system mutation, commit or push occurred. B4–B7 remain paused until an explicit user instruction.
