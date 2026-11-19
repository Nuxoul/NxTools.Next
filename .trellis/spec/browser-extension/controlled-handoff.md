# Controlled browser ownership and external resolution

Evidence: `repo:browser_extension/nxtools-interceptor/bg/browser-handoff.js`, `repo:rust/apps/nxtools-browser-host/src/controlled_handoff.rs`, `repo:rust/crates/nxtools-download/src/engine/controlled_handoff.rs`, `repo:rust/crates/nxtools-download/src/storage.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`.

## Scenario: Pause, durable hold, cancel, activate

### 1. Scope / Trigger
Automatic Chrome/Firefox download interception and external Add Download intake. This controlled lifecycle supersedes ordinary one-shot acceptance/compensation for automatic browser items; manual context/menu/resource/Blob/batch commands retain their original identity-bound semantics.

### 2. Signatures
- Native protocol2/256KiB: `prepare_browser_handoff` takes the original URL payload including `handoffId`, `openDialog`, `startImmediately`; `get_browser_handoff`/`abort_browser_handoff` take `handoffId`; `commit_browser_handoff` additionally requires `taskId`. Capability: `controlled_browser_handoff_v1`.
- Download command protocol14 and IPC protocol6: `BrowserPrepareHandoff { handoff_id, request, open_dialog, mode }`, `BrowserAcceptHandoff { handoff_id, request, mode, resolution }`, `BrowserHandoffStatus { handoff_id }`, `BrowserCommitHandoff { handoff_id, task_id }`, `BrowserAbortHandoff { handoff_id }`.
- `BrowserHandoffState::{Pending,Held,Committed,Aborted,Expired,Missing}` serializes snake_case. Outcome `BrowserHandoff { handoff_id, state, task_id: Option<DownloadTaskId> }`.
- SQLite schema15 adds `download_controlled_browser_handoffs`: identity/state/task, the existing resolution resource identity, start intent and Pending deadline. Task/transfer/queue/Held admission is one transaction. No second raw request or credentials store; the extension recovery journal remains secret-free. The task UUID receipt survives task removal/redownload independently of the task row.
- Private `BrowserDownloadDialogSource::ControlledDownload(Box<DownloadRequest>)` carries controlled intake through the existing NewDownloadView and fixed confirmation owner.

### 3. Contracts
Pause and verify the original browser task before credential/native demand. Failure leaves browser authoritative without desktop intake. Chrome retains its original one-shot filename callback until the terminal boundary. At most32 active journal identities; never evict active work for admission.
Chrome reports a paused download as `in_progress/paused:true`; Firefox reports `interrupted/paused:true/USER_CANCELED`. Both remain under browser ownership and are resumable on abort. The direct cancel and recovery paths share the same predicate: `interrupted/USER_CANCELED` without `paused:true`. See the [browser workflow reference](../../../docs/reference-code/browser-integration-polish.md#firefox-controlled-download-state).

Pending means confirmation only: no task, no accepted receipt. It has a renewable30second lease. Quiet1second status demand renews Pending while the extension owns control, with a10minute confirmation limit. Held means a persisted task that cannot transmit: ordinary task actions, resume/batch, queue, daily/start-all, schedules and engine reopen cannot bypass it. Held never expires automatically. Commit accepts only the same handoff/task and releases original StartNow/QueueOnly/schedule intent; duplicate commit returns the same identity. Abort cancels only uncommitted work. Unknown abort persists a tombstone; late prepare/confirmation cannot recreate it. Stale abort of Committed does not cancel active work.

Accept compares original URL, headers, Cookie, referrer and certificate policy before resolution or durable admission. Destination, filename, category, limits, schedule and parsed media selection remain confirmation options. Editing and re-probing another resource cannot turn the original browser handoff into a different download. Both client titlebar close and footer Cancel claim Abort before removing the owned window; native close uses the same ownership phase. Acceptance failure terminates Pending rather than renewing an ownerless confirmation.

Native valid states are RPC `ok:true`; `accepted:true` and task ID occur only for Held/Committed. Pending maps to `awaiting_confirmation`, Held to `persisted`, Committed to `transferred`; terminal inactive states map to `rejected`. `queued` is not completion. Verify request, handoff and task identity before ownership effects.

Before browser cancel persist `cancelling`; only verified cancellation advances to durable `browser_cancelled` and Commit. Query failure or missing history in an unverified cancellation phase retains Held without guessing. A previously verified cancelled phase can retry Commit after history removal. Failed activation retains the same Held identity for explicit retry; never create a replacement or resume a cancelled original. If the browser item remains paused after cancellation, abort Held before resuming the browser. Do not delete browser files; history erasure follows successful transfer only.

A secret-free `storage.session.nxtools_browser_handoffs` journal carries browser ID/start time, handoff ID, phase, task ID, timestamps and bounded options. No URL/Header/Cookie/body. Empty startup does not connect native transport. Each active operation retains the original demand-only native owner. Recovery and explicit connection tests reuse IDs; passive refresh never reconnects. Diagnostics contain fixed transitions/correlation/elapsed/native error codes, not request payload or per-poll logs. `desktop_activation_pending` and `browser_cancel_unconfirmed` must not claim the browser is still transferring.

Externally supplied Url/Download/controlled sources defer the original `begin_probe` after window construction or successful pristine-prefill reuse. Existing generations, receipt, cancellation, native placement/focus and compact-to-fixed boundaries stay. Fixed transition transfers controlled context without aborting the source handoff; cancelling/closing/probe or window failure aborts pending ownership. User confirmation accepts Held only and never commits. Manual `None`, clipboard prefill and Paste remain explicit; automatic parsing never means automatic confirmation or task creation. Populated, edited, external and in-progress drafts are preserved.

The noop resolver emits no receipt. Controlled confirmation uses the existing engine Resolve admission in that case; a published receipt remains mandatory to reuse its resolved request and is never replaced by Bypass. Both paths enter Held only, with no Desktop Commit or progress-window activation.

All Rust commands inherit `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; one Cargo operation at a time.

### 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Pause fails, paused flag false, original already complete or active cap reached | No prepare/desktop demand; leave browser ownership |
| Pending cancel/expiry, UI close or failure | Abort same identity, restore paused browser; no task activation |
| Desktop unavailable before browser cancel | Resume browser; Pending lease/abort rejects late activation; Held stays blocked until cleanup |
| Browser cancel fails or does not change original state | Abort Held before resume; no erase or Commit |
| Post-cancel query fails | Keep cancellation phase/Held for recovery; no Abort/Commit guess |
| Verified cancellation, Commit unavailable | Retain durable cancelled identity and Held; fixed retry guidance |
| Wrong reply/request/task identity | No ownership effect; keep recoverable state or browser fallback according to phase |
| SQLite transaction fails | No success/state/event leak; original task and queue remain unchanged |
| Draining host | Reject Prepare/Accept/Commit; allow Status/Abort cleanup |
| Late confirmation after abort/expiry | No new task, no transfer |
| Edited/re-probed resource differs from Prepare | Reject Held admission; Desktop aborts the original handoff and browser resumes |
| Committed task removed or redownloaded | Status/Commit/Abort replay retains the original committed task UUID |

### 5. Good / Base / Bad Cases
Good: paused browser → Pending confirmation → one durable Held task → verified browser cancel → Commit original task. Base: user cancels, same ID ends and original resumes. Bad: window-open/channel enqueue is accepted, a queue Resume starts Held, query failure discards Held after browser cancel, or external URL still awaits Continue.

### 6. Tests Required
`tests/controlled-handoff.behavior.test.js` drives actual production background Chrome/Firefox events, pause/verification, Pending/Held/cancel/Commit order, control failure, cancellation failure/unknown query, worker recovery, secret-free journal and no premature native demand. Existing method/Cookie/manual/Blob/batch suites remain required.

`browser_handoff_lifecycle_contract` uses real temporary SQLite and localhost HTTP: no transfer before Commit across resume/batch/queue/schedule/reopen; identity and replay; expiry/tombstone/late confirmation; SQL-trigger acceptance/commit rollback. BrowserHost `controlled_handoff` tests cross actual owned pipes and native framing. IPC tests assert draining classification. Desktop production GPUI tests exercise external auto-probe/fixed confirmation/no task, manual no-probe, dirty draft protection, controlled cancellation/close/fixed context and late result rejection. Source gates do not replace these boundaries.

Installed-browser Save As/pause behavior, authenticated sites, physical keyboard/pointer, mixed DPI, provider and human visual acceptance require separate evidence and authorization; fixtures do not certify release acceptance.

### 7. Wrong vs Correct
Wrong: `open window -> browser resumes -> desktop starts later`; `Held -> Resume/StartAll -> transfer`; `cancel succeeds -> query throws -> abort Held`.

Correct: `pause -> Pending -> durable Held -> verified cancel -> Commit`; all alternate admission reads durable Held state; unknown cancellation retains Held and identity until resolved.
