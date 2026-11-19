# Download IPC quality

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#tests`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadHostReadiness`.

Exercise actual temporary pipe/typed frames, incompatible Hello version, matching command identity, event/snapshot sequence and host draining/readiness. A successful pipe connect isn't Ready, nor does an Accepted command mean applied/finished. Caller tests preserve CLI ConnectOnly and browser acceptance identity. Existing readiness fields have serde defaults for compatibility. Run scoped library/lint/check/fmt per [verification](../../shared/verification.md), synthetic data only.

The task/completion contract uses download command12 and IPC handshake5. The stable pipe name returns explicit ProtocolIncompatible to old Hello before commands; CLI and BrowserHost consume shared constants. B4 DestinationExists/RetryWithAutoRename, required paused_url_revision/expected_revision, exact validator and cancellation/draining contracts remain. Replace is mutating and rejected while draining; candidate cancellation remains available. Keep real held-probe/SQLite/artifact/socket/lease-drain regression and correlated client results. Owning behavior: [download recovery](../../nxtools-download/backend/quality-guidelines.md#scenario-nonblocking-events-and-no-replace-destination-recovery) and [paused source replacement](../../nxtools-download/backend/quality-guidelines.md#scenario-explicitly-paused-source-replacement).

## Scenario: Normal host stop and physical IPC drain
### 1. Scope / Trigger
Normal/background Desktop cleanup of the original Download host. Evidence: `repo:rust/crates/nxtools-download-ipc/src/completion_tests.rs`, `repo:rust/apps/nxtools-desktop/src/runtime.rs`.
### 2. Signatures
`stop_download_host(pipe_name:&str,activity:&DownloadHostActivity)->Result<()>`; original run_download_host retains/joins client handles. `DesktopRuntime::shutdown_downloads_and_wait(engine,correlation_id)` closes admission, joins engine work, then stops/joins the original host. No shutdown wire command or second transport.
### 3. Contracts
Draining rejects new mutations but keeps reads/candidate cancellation. Stop uses the original lifecycle gate to retire current PipeRetirement handles and close cloned subscription receivers, then wakes blocked accept with an existing local connection without Hello. Registration shares the stop gate; no accepted client escapes retirement. Host joins all actual client workers outside locks and publishes Stopped only afterward. Runtime retains and joins the original host handle and verifies clients0/Stopped.
### 4. Validation & Error Matrix
Idle blocked reader/subscription -> retirement wakes both. Join/client failure -> error, not successful drain. Stopped -> reconnect fails. Stop request alone -> not physical resource completion.
### 5. Good/Base/Bad Cases
Good: original Hello/Subscribe/QueueOnly/draining/stop then actual joins. Base: reads remain available during draining. Bad: mark Stopped before joins or leave perpetual accept as evidence of completed cleanup.
### 6. Tests Required
Nonzero LIST→identical RUN for normal_stop_retires_idle_and_subscribed_clients_before_host_join; assert mutation rejection/read allowance, both caller joins, host join, client0/Stopped and post-stop refusal. Retain B4 held replacement cancellation and shared-constant caller tests.
### 7. Wrong vs Correct
Wrong: process exit eventually kills the accept loop, therefore cleanup is complete. Correct: stop/wake/retire/join actual Download IPC resources; physical application exit remains separate.
