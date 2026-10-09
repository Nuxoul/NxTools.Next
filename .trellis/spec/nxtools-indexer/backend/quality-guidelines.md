# Indexer quality and release boundaries

Evidence: `repo:rust/apps/nxtools-indexer/src/tests.rs`, `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`, `repo:rust/apps/nxtools-indexer/tests/process_lifecycle.rs`.

Synthetic scanner/poller/cache tests prove last-good publication, cursor/version and unique bounds. Admission tests precede shared state. Some suites scan real volumes; never unrestricted under scan/service prohibition. Native release requires authorized packaged LocalSystem idle/restart/base+delta/search/hash, not Ready/source gate. [Verification](../../shared/verification.md); retain admission contract below.

## Scenario: Authenticated bounded session and truthful Ready

### 1. Scope / Trigger
- Changes to Windows service admission, initial Hello, snapshot state or recursive USN work budgeting.

### 2. Signatures
- `windows_service_host` obtains LocalPipeConnection actual client_process_id/client_session_id and calls authorized_packaged_desktop before shared state.
- `LocalPipeReader::set_initial_frame_timeout(Duration)`; service uses three seconds; MAX_PENDING_SESSIONS = 4.

### 3. Contracts
- PID/session and registered protected desktop path/hash authorize all service commands; Hello payload is not authority. Reject unknown/zero session/unapproved identities before shared mutex. Elevated broker keeps separate checks.
- Four authenticated pending slots; decode Hello before shared mutex. Initial read deadline is not extended by partial bytes; newline disarms it for healthy long-lived sessions.
- No snapshot after first failure means not Ready. Last-good snapshot remains Ready/searchable through failed maintenance. Unique USN candidates consume recursive budget, not duplicate raw entries.
- Unregistered development/Validation clients fail closed; do not weaken authorization to infer compatibility.

### 4. Validation & Error Matrix
- Unidentified/unapproved/session zero -> reject, no lease; capacity exhausted -> bounded reject.
- Silent/partial first frame -> TimedOut and released admission; non-Hello -> reject before state/ack.
- First build failure -> no false Ready; maintenance failure -> preserve last-good search. Genuine large subtree -> bounded RebuildRequired maintenance, not payload leakage.

### 5. Good/Base/Bad Cases
- Good: approved client Hello -> session; later idle reads remain alive.
- Base: duplicate-heavy direct changes remain incremental.
- Bad: trust client_name, lock state before Hello, reset timeout per byte, or publish Ready with no snapshot.

### 6. Tests Required
- Admission denial/cap release; preadmitted non-Hello; first-failure/last-good and duplicate-heavy/large-subtree regressions.
- Temporary named pipe partial deadline and healthy-session tests; no unrestricted real-volume suite when scanning is prohibited.
- Release gate remains packaged LocalSystem idle CPU with RestartService/AssertRegression, persisted base+delta restart, real service search and approved/runtime hash consistency. Never infer release acceptance from synthetic tests.

### 7. Wrong vs Correct
```text
Wrong: pipe ACL -> service mutex -> read unbounded Hello -> trust name.
Correct: actual PID/session/path/hash -> bounded admission -> deadline Hello -> service mutex.
```

Reference: SwiftList UsnServicePipeServer and Listary managed boundary only; Listary native source unavailable. Evidence: `docs/audits/2026-10-08-sol-review-remediation.md`.
