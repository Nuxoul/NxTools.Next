# B5 normal-exit outcomes

Acceptance: `not_accepted`.

## Original normal-close chain

`CompletionActionCoordinator` consumes the successful one-shot Exit intention → `DownloadApplication::execute_shutdown` retains its correlation and calls original `cx.quit()` → GPUI application run returns → `DesktopRuntime::shutdown_downloads_and_wait(engine,correlation_id)` → production `shutdown_download_resources` → original engine actual joins → Download IPC stop/retirement and actual host/client joins → original extension and clipboard cleanup → runtime/supervisor release.

The original runtime cleanup attempts Download IPC retirement even if engine drain failed. Main attempts extension and clipboard cleanup before returning an error. `desktop.shutdown.feature_resources_drained` is emitted only after successful relevant cleanup; failures emit `desktop.shutdown.failed`. There is no process kill, second shutdown transport or new cleanup manager. The background host uses the same original engine/IPC lifecycle before returning from its process entry.

## Download engine

Admission closes before pause/cancellation. Original transfer writers, probes, resolution and the existing managed operation are retained under one JoinSet. Actual futures retire before drain reports success; worker panic remains an error after the other handles finish. Task records and resumable artifacts survive. The same Exit correlation flows through Rust/runtime/engine lifecycle events.

## Download IPC

- Hello5 and command12 retain the stable original pipe and command envelope.
- `DownloadHostActivity` uses one lifecycle gate for current connection retirements and cloned subscription receivers. Accept registration is serialized with stop.
- `LocalPipeConnection::into_retiring` retires blocked reads/writes; closing a registered subscription receiver wakes its original event wait.
- `stop_download_host` requests retirement and wakes the original blocked accept with an ordinary local connection. No shutdown wire command is added.
- The original host rejects admission after stop, joins retained clients outside locks, then publishes `Stopped`. Actual client-worker drop removes its retirement/subscription record.
- Draining rejects mutations while reads and B4 paused-link candidate cancellation remain available. Stopping and draining are distinct.
- `DesktopRuntime` retains the original host JoinHandle and activity. Cleanup consumes the handle and checks zero clients/Stopped after the actual join. Errors remain errors.

## Real-boundary evidence

`nxtools-download-ipc/src/completion_tests.rs::normal_stop_retires_idle_and_subscribed_clients_before_host_join` uses a unique current-user pipe and production Hello/client/host. It covers idle read, Subscribe wait, one accepted QueueOnly task, draining mutation rejection/read availability, stop retirement, actual caller/host joins, clients0/Stopped and post-stop connection refusal. `b5-ipc-stop-pass2`: LIST1/RUN1/PASS1/FAIL0/SKIP0/not_run0; exits0/0.

`b5_normal_runtime_drain_joins_original_writer_then_pipe_resources` uses real SQLite, held local HTTP writer and unique Hello pipe through the same production `shutdown_download_resources` function. It verifies actual writer retirement before IPC stop/join, post-stop refusal, durable retained task state, rejected post-drain admission, and retired listener/sockets.

`b5_exit_requests_normal_testplatform_quit_without_claiming_resource_drain` exercises original completion Exit through production TestPlatform, then checks actual worker cleanup separately. Locked TestPlatform `quit()` is a no-op; no current Desktop process is terminated.

Final LIST/RUN counts, commands and logs are in `verification.md` and `verification.json`.

## Remaining acceptance layers

A quit request is not resource drain; resource drain is not physical process exit. These tests establish Download futures and Download IPC retirement, not Indexer/Platform process-lifetime reader joins, a real user Desktop exit, physical power, installed/Release or human visual acceptance. No product/reference GUI, service/registry mutation or package/install operation is performed.
