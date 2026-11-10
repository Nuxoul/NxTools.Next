# B1 BrowserHost outcomes

## Browser handoff

`rust/apps/nxtools-browser-host/src/lib.rs` retains the current-user `DownloadIpcClient`, existing desktop connect/start, dialog readiness retry and engine/store owners. `submit_download_with_client` verifies the engine command-result request ID before interpreting any outcome. A matching durable `BrowserAccepted` returns `accepted=true`, `data.handoffState=persisted`, `data.reason=desktop_accepted`; `data.queued=true` denotes a persisted QueueOnly task. A completed confirmation window returns `accepted=false`, `queued=false`, `awaiting_confirmation` / `confirmation_pending` and claims no task.

Refusal, result-identity mismatch, unavailable desktop, command timeout/disconnect, configuration and request-validation failure retain the original request ID and parsed handoff ID in a fixed payload-safe response. Low-frequency host events carry correlation/handoff, fixed state/reason, elapsed time and owning typed/native code, not raw errors or request payload. `handoff_state` diagnostics require UUID correlation, whitelisted state/reason and nonnegative integer elapsed; optional boundary/compensation values have fixed whitelists. Existing event-only logs remain valid, arbitrary extension text is not logged.

No protocol bump, Cookie-store IPC field, credential store, transport/engine/UI/settings replacement or dependency change.

## Production-boundary fixtures

Nine `b1_` tests in `browser_wire_tests.rs` cover native little-endian framing, production submission, independent current-user DownloadHost pipes, real engines and temporary SQLite. They verify distinct same-URL Cookies, durable replay after a new engine opens the database, zero tasks for confirmation-only admission, conflicting replay refusal, unavailable-desktop identity retention, mismatch after an actual pipe command and captured tracing metadata without secrets.

`tests/harness/native-host-roundtrip.cjs` loads the production extension scripts. Five scenarios exchange seven actual request/reply frames with the Rust fixture:

- Firefox container/private download events deliver the exact source Cookie to the engine confirmation consumer; matching pending replies leave the browser item untouched.
- A manual drop runtime request receives the real host confirmation-pending reply as ok=true/accepted=false with matching handoff ID, no task and the explanatory pending message. Exact resource/page Cookie queries stay in its originating container; the engine still has zero tasks and the browser item remains untouched.
- A real QueueOnly receipt feeds the original extension ownership policy. Browser cancellation fails; correlated `task_action(cancel)` travels over the real pipe/engine using the existing command envelope and task-action reply projection. The browser item is not erased and the stored task becomes Cancelled.
- Same-handoff replay returns that original task with duplicate=true.
- Conflicting same-handoff input produces the actual desktop-refused reply, retaining identity and leaving exactly one task.

The test connector never uses the product/default pipe or launches a desktop. QueueOnly avoids network transfer. SQLite reopen is a new engine/store, not an installed process restart.

## Verification

Every Cargo invocation sets `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, and scratch TEMP/TMP in the same PowerShell execution. One Cargo lane.

| Command | Final result | Scratch log |
| --- | --- | --- |
| Nine `b1_` production-boundary cases, included in the full library | **9 passed, 0 failed** | `b1-repair-host-full-final.log` |
| `cargo test -p nxtools-browser-host --lib -- --test-threads=1` | **47 passed, 0 failed**, including the extended real manual-pending/compensation/replay fixture | `b1-repair-host-full-final.log` |
| `cargo clippy -p nxtools-browser-host --all-targets -- -D warnings` | **Passed** | `b1-repair-host-clippy.log` |

Before the semantic fix, the nine tests produced one pass/eight expected failures: confirmation falsely accepted, absent persisted metadata, ignored handoff diagnostic categories, mismatched engine result accepted, lost refusal identity and unavailable-connection identity; the cross-language extension test independently rejected the false confirmation receipt. `b1-host-red.log` records that witness.

## Limitations

Installed browser/stdio process lifetime, registered Native Messaging, product/default pipe, actual confirmation HWND, physical/native UI, packaging and release acceptance were not performed. Payloads that cannot deserialize as a `BrowserDownloadPayload` retain the pre-existing generic error path; B1 identity guarantees apply to decoded submissions and their subsequent failures. Aggregate verification and remaining workspace blockers are recorded in `outcomes.md`.
