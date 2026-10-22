# B2 password portion — implementation evidence

2026-10-20. **B2 password portion implemented and scoped checks passed; pending independent acceptance. Conflict/apply-all portion is NOT complete.** This does not complete B2, ENG03/ENG09, Phase A or A–F. B1/delivered C4 remain [independently accepted](zip-encoding-acceptance.md), not rerun. [Source boundary](research/password-operation-sources.md); [results](password-operation-results.json). Only [checkpoint](phase-a-checkpoint.md) owns current state.

## Minimal ownership and behavior

Original Rust worker is the operation coordination entry: optional private schema2 `password_interaction:true`, open/test only, after ONE original source acquisition. `worker_password` is a fixed password-only loop using existing InteractionPort, not a new task/process/retry framework. No ArchiveRead v1/v2 field/version/permission change; schema1/read/extract/create opt-in rejects. Old explicit-password/no-option paths retain original behavior. This slice is all-entry read-only, so it cannot silently replace/delete published output; selected extraction/conflicts are not claimed.

Each actual attempt receives a fresh OperationControl ID sharing original cancel/lifetime; password_state exposes attempt_id/number under original logical operation_id. Same original request retains path/codepage/op/finite limits. At most3 actual attempts (initial+2manual answers), no guessing/retry without a current answer. Existing PasswordRequired and explicit UnRAR WrongPassword alone enter installed `Prompt::Password` via InteractionPort; AuthenticationOrDataError is terminal. Prompt ticket job/prompt/generation is installed before private notification; stale/duplicate answers stay ignored by the existing port. Null typed answer is user Cancel; original control Cancel/EOF also wakes. Before a new attempt original cancellation is checked.

C# original runner/Job/source/image/slots stay owners while waiting; no respawn or slot release per answer. Typed `RespondPassword(id,ticket,string?)` only enqueues the bounded original private input; Rust decides acceptance/retry. Existing process30s timeout/budgets/cancel-grace unchanged. New opt-in capability password_interaction_v1 is required before use. Typed Cancel preserves original runner category cancelled; first native/timeout failure still wins. No public PlatformCommand/UI/FIFO/risk/password persistence.

SecretPassword/Zeroizing owns answers in Rust; temporary WorkerRequest password and raw serialized buffers are zeroed. C# private UTF8 packing/input queue buffers are zeroed after copy/delivery; no password in argv, settings, metadata, password_state or terminal. Existing C# secret strings are not claimed OS-locked/GC-zeroized. Transition diagnostics carry only original correlation, fixed state/error, attempt ID/number, elapsed; no payload/per-byte logging. Independent fixture passwords already documented upstream are synthetic.

## Real red → green / independent input

- Existing unrar0.5.8 upstream `rar5-header-encrypted.rar` is the main input. Password and exact18byte `target\nCargo.lock\n` oracle are from published tests/independent7-Zip evidence in tests/fixtures/upstream/README.md. No NxTools-created archive. Actual test runs full UnRAR integrity/data validation and asserts1file/18bytes; open verifies decrypted .gitignore header/18byte size.
- Original first actual C# runner/Job/worker missing-password test: LIST1/RUN1/FAIL1/skip0/exit1, no pending prompt (`b2-red-run.log`). After opt-in coordination main chain1/1green (`b2-green-run.log`). Expanded six then seven tiny scenarios are not an all-format matrix.
- Seven final scenarios: missing->correct full test; missing->correct encrypted open; missing->WrongPassword->manualcorrect with old-ticket reply injected; typed Cancel; original operation Cancel; third failure remains WrongPassword/no4th attempt; correctly passworded damaged RAR remains AuthenticationOrDataError/no prompt. Every operation starts ONE real worker, distinct attempt IDs, source writer denied while waiting, helper image cannot be opened exclusively while running; terminal cleanuptrue/ActiveCount0/filescommitted0, source+helper exclusively reopen afterward, no output created, metadata/log secret sentinels absent.
- Later added exact outer-cancel assertion reproduced another real red LIST1/RUN1/FAIL1/skip0 (`b2-cancel-red-run.log`): private Cancelled had projected worker_failure. Fixed opt-in-only C# projection; final same4fixture union passes with correct cancelled.
- One intermediate four-case run failed with Sequence contains no matching element (RUN1/FAIL1/not_run3). Source inspection established the test's unsafe assumption: runner removes active slot BEFORE publishing exited, so DrainAsync alone cannot certify an enqueued terminal. Fixture now waits actual exited then checks drain. Tagged one-case probe passed but is NOT considered explanation/whole-union PASS; corrected full union ran and passed, with old failed log retained. Early test line-placement corrections were preparation errors, not product red evidence.

## Actual commands/results

All Cargo calls inherited JOBS1/INCREMENTAL0/RUSTFLAGS=-C codegen-units=256, offline, serial; isolated C#apphost built before matchingisolated worker. No100k/long-path matrix/1GiB/4GiB+17ZIP64/resource sample. Logs under session scratch `C:\Users\Administrator\.pi-desktop\scratch\949da288-1ef9-46b3-a698-90f543613cf5`, prefix b2-.

| Command | Actual result |
| --- | --- |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test interaction_contract --test worker_wire --test worker_cancel_contract -- --test-threads=1` |16pass/0fail/0ignored (3+12+1); b2-interaction.log |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --lib password_notification_tests -- --test-threads=1` |1pass/13filtered; installed-before-notify immediateanswer/cancel; b2-notification.log |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test zip_crypto_contract aes_damage_framing_crc_budgets_and_operation_failures_never_publish_invalid_bytes -- --exact --test-threads=1` |1pass/4filtered; existing AES ambiguous classification unchanged; b2-aes-classification.log |
| `cargo clippy -p nxtools-archive --features isolated-codecs --tests --examples --offline -- -D warnings`; same without feature |both exit0; b2-clippy-isolated.log / b2-clippy-default.log |
| `cargo fmt -p nxtools-archive -- --check` |exit0; b2-fmt-scoped.log |
| `cargo fmt --all -- --check` |exit1: unrelated unedited rust/apps/nxtools-desktop/src/main.rs:283 formatting. NOT whole-workspace fmt PASS; user changes preserved; b2-fmt.log |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` |exit0,0warnings/errors; b2-verified-dotnet.log |
| `cargo build -p nxtools-archive --features isolated-codecs --offline --bin nxtools-archive-worker` |exit0 after C#build/before execution; b2-verified-worker.log |
| C#apphost `--fixtures --list` then `--fixtures` exact same4filters below |LIST4=RUN4=PASS4,skip/fail/not_run0,exit0; b2-verified-list/run.log; exact names compared |
| spec-contracts.py /task.py validate/fixture-selector/git diff --check |exit0;106docs/14owners,context82/79; old native-contracts35497bytewarning only; b2-spec/context/selector/diff.log |

Exact C#filters: `archive password fixture actual wait retry cancel`; `archive product fixture actual open test privacy`; `archive product fixture strict wire validation`; `archive codec actual Rust password budget corruption cancel fixture`.
Environment: NXTOOLS_ARCHIVE_WORKER_FIXTURE=<workspace>/target/debug/nxtools-archive-worker.exe; NXTOOLS_ARCHIVE_CODEC_FIXTURES=<workspace>/rust/crates/nxtools-archive/tests/fixtures/upstream; NXTOOLS_ARCHIVE_PYTHON_ORACLE=existing Python313/python.exe. Product regression generates tiny3member ZIP via existing Python producer. No SKIP accepted.

Historical reports/results and103pre-existing task files unchanged before checkpoint update. No formal app/package/install/sign/registry/service/UNC/recycle/source-delete/dependency/commit/push action. No delegation/review loop. Remaining: independent password-slice acceptance and unrelated globalfmt difference; conflict/apply-all, risk/UI/FIFO and full B2/ENG03/ENG09/PhaseA remain unfinished. Stop this slice.
