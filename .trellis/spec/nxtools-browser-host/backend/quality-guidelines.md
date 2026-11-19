# Browser host quality

Evidence: `repo:rust/apps/nxtools-browser-host/src/lib.rs#tests`, `repo:rust/apps/nxtools-browser-host/src/blob_upload.rs#tests`.

Test bounded fields/version/acceptance identity at real typed-command seam, malformed/oversized frames, batch IDs and temp socket ownership/cleanup. No real browser or registry operations. Library/extension/source checks follow [verification](../../shared/verification.md); registration/native presentation are separate owners. Preserve lease contract below.

## Scenario: Exclusive Blob Receiving lease

### 1. Scope / Trigger
- Changes to upload claim, socket body IO, idle expiry or cleanup/capacity release.

### 2. Signatures
- UploadState::Receiving; `reap_expired_uploads(registry, staging_root, now)` excludes it; MAX_RECEIVING_DURATION = two minutes.

### 3. Contracts
- One owner per Receiving chunk, at most 1 MiB; duplicate claim rejected. Idle reaper cannot delete owner state/staging during its write.
- Owner enforces total body deadline and per-read timeout min(remaining, 30 seconds), then owns finish/failure cleanup/capacity release.
- Pending/Uploading/Complete idle expiry semantics remain; no payload or capability token logging.

### 4. Validation & Error Matrix
- Duplicate Receiving -> reject without replacing owner; abandoned body/deadline -> bounded failure/cleanup.
- Write/rename failure -> preserve typed failure, release state; idle expired non-Receiving -> cleanup and capacity release.

### 5. Good/Base/Bad Cases
- Good: claim, idle-reap attempt, body completes with state still owned.
- Base: idle partial upload expires normally.
- Bad: TTL removes Receiving mid-write or active exemption has no owner-side deadline.

### 6. Tests Required
- Synthetic expired-Receiving/duplicate claim and real socket claim/reap/finish; assert exclusive ownership and output.
- Keep write/rename cleanup failure and idle-capacity tests; BrowserHost library and extension contracts.

### 7. Wrong vs Correct
```text
Wrong: expires_at <= now -> remove every state.
Correct: state != Receiving && expired -> reap; Receiving owner enforces bounded IO.
```

## Passive status and EOF lifetime
`get_snapshot` only connects an existing Download IPC engine; it must never call connect_or_start or spawn a desktop. Explicit task intake retains existing startup policy. Extension transport owns pending requests/Blob lifetime and emits stdin EOF when idle; host run returns on EOF. Add passive-status guard and extension fake-port concurrent/timeout/disconnect/stale/Blob/idle regressions. Native EOF/process exit and exact installed-file updater lock release need separately authorized fixtures, not an unrestricted real browser/service launch. A Windows service may outlive desktop by design; do not kill it by process name.

## Completed Blob ownership and request-context compatibility
Engine confirmation moves accepted complete staging into the original root's `pending` subdirectory; BrowserHost startup expiry remains top-level blob/uploading only and never recurses into engine ownership. Host forgets the upload registry after command outcome; original failure/duplicate cleanup touches only its unclaimed candidate. Engine owns pending import/discard/startup orphan cleanup. Required cross-owner cases are `blob_upload/ownership_tests.rs` and the Download quality scenario.

Native protocol2 remains256KiB. BrowserDownloadPayload defaults missing headers/cookie/referrer and tolerates unknown legacy headerText; host never consumes headerText. New extensions omit it and send Cookie only in the dedicated field. Old extensions remain accepted; original host filters duplicate Cookie/Referer and transport-owned headers before DownloadRequest. `browser_wire_tests.rs` checks legacy/reduced/default payloads plus32items×4KiBCookie through actual length-prefix framing and typed batch intake. Do not increase header16KiB/cookie64KiB/frame256KiB limits to hide duplicated context.
Optional camelCase `sourcePageUrl/sourcePageTitle` default to empty on legacy protocol2 payloads. `validate_payload` applies the16KiB URL and4096byte UTF-8 title bounds before the shared `build_browser_download_request` maps the URL/title to `DownloadRequest.source_page`; absent/blank URL means None regardless of title. Single, batch and controlled prepare use the same builder; the field never replaces submitted URL or frozen Referer/Cookie. Framed native intake, owned IPC pipe and a fresh SQLite engine reopen must retain the page independent of a real redirected probe and user-edited filename. Do not log URL/title values or change protocol2/frame256KiB.

## Scenario: Browser receipt means durable acceptance (B1)
### 1. Scope / Trigger
REF-02/03 browser create_task replies, engine identity validation and bounded extension lifecycle diagnostics.
### 2. Signatures
`submit_download` retains connect_or_start; `submit_download_with_client` accepts the original connector/command operations for isolated tests. Browser protocol2 adds `data.handoffState` / `data.reason` to existing replies, not a new transport or task store. `BrowserExtensionLogPayload` permits only fixed handoff metadata beside event/correlation identity.
### 3. Contracts
`BrowserAccepted` follows engine durable persistence and produces accepted=true, state persisted, reason desktop_accepted; queued=true may describe a persisted QueueOnly task. `BrowserDialogAccepted` proves only confirmation-window creation, produces accepted=false, queued=false, state awaiting_confirmation and reason confirmation_pending. It cannot transfer browser ownership. Check engine result request_id and dialog handoff_id before projecting any acceptance. Preserve original requestId and known handoffId on refusal/unavailable/mismatch. Same handoff replay after reopening SQLite returns the same task and duplicate=true, never a second task.

Extension event handoff_state requires UUID correlation, fixed handoffState/reason and nonnegative integer elapsedMs. Optional boundaryCode/compensation are fixed categories documented by the extension contract. Arbitrary URL/Header/Cookie/message fields are ignored, never logged; existing event-only diagnostics remain valid. Rust records state, reason, elapsed and owning fault/native codes without raw error or payload. Existing engine storage, native frame limits and client lifetime remain.
### 4. Validation & Error Matrix
Durable BrowserAccepted -> persisted acceptance. UI/channel-only -> awaiting_confirmation, no acceptance. Engine request/dialog identity mismatch -> rejected/identity_mismatch with original identities. Engine rejection -> rejected/desktop_refused. Failed desktop connection -> rejected/desktop_unavailable. Unknown or incomplete handoff diagnostic metadata -> reject; no arbitrary text ingestion.
### 5. Good/Base/Bad Cases
Good: same URL's two dedicated Cookie values traverse native framing -> production submission -> owned current-user pipe -> separate engine requests. Base: QueueOnly receipt survives a new engine opened from SQLite. Bad: confirming a window is treated as creating a task; mismatched engine result is accepted; a failure loses known handoff identity.
### 6. Tests Required
Nine b1_ tests in browser_wire_tests.rs exercise actual length-prefix request/reply framing and production submission seam, real temporary DownloadHost/client, engine/SQLite reopen, zero tasks after dialog-only completion, mismatched result after a real pipe command, refusal/unavailable identity retention, and captured tracing fields with payload secrets absent. The Node native-host-roundtrip fixture loads production extension scripts and exchanges framed messages with the Rust fixture: two Firefox container/private download events check engine Cookie input, matching replies and retained browser ownership for confirmation-only admission; real queue-only receipts feed the original extension handoff policy, and failed browser cancellation sends correlated Cancel through the actual pipe/engine, leaving one Cancelled task across replay and conflicting-refusal checks. The fixture uses the existing command-envelope and task-action reply projection. Run full BrowserHost library and strict Clippy, Download IPC, browser engine contracts and versioned message tests. Fixture pipes are unique/current-user and never use the product default pipe/start closure. New-engine SQLite reopen is not an installed process restart or native confirmation acceptance.
### 7. Wrong vs Correct
Wrong: BrowserDialogAccepted -> accepted=true; return any engine outcome without matching request ID.
Correct: only durable matching receipt is accepted; UI-only confirmation is pending, and every terminal reply retains known browser identities.
