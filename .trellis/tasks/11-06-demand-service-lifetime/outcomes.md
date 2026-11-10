# Demand service lifetime — delivery

## Result

Indexer and trusted updater registrations use Demand start. Persistent registration/trust is separate from running processes.

- Indexer startup waits for an approved desktop identity, bounded Hello and retained process-object lease before discovery, cache restore, scans or USN work. With no authenticated owner, startup expires after15seconds without index work. The last desktop process exit stops the coordinator and native sessions before SCM Stopped. Reconnect does not stop a live owner's service; multiple desktop owners remain independent.
- Existing mmap base, durable delta and USN cursor recovery remain unchanged. Reopening Desktop does not itself schedule a full rebuild; original missing/invalid cache or journal conditions retain their maintenance behavior.
- Updater registration does not start it. Actual requests verify enrollment and signed package, then start the exact approved service. Cancel before startup performs no service start. One update/probe/rejection/EOF/timeout ends the listener; BrokerService calls normal Stop. Accepted transaction settlement and rollback complete independently of client/SCM cancellation.
- Existing Auto indexer registration uses a verified same-release configuration-only authorized operation once; successful configuration then starts it. Both legacy Auto and current Demand retain native ownership validation; unsupported startup modes remain rejected. Auto remains migration-needed rather than a current configuration.
- Existing Auto updater requires the original explicitly authorized same-key bootstrap maintenance even if its historical version marker is valid. Maintenance verifies a one-shot readiness probe and actual Stopped, preserving enrollment, sequence and release recovery. No key, ledger, service registration or index cache is deleted/reset.
- Close to tray retains a running Desktop. Explicit Exit ends its service ownership. Ordinary PlatformHost parent monitoring remains unchanged.

## Verification

| Boundary | Result |
| --- | --- |
| C# production and original test-project build | Zero warnings/errors |
| Original C# registration/upgrade/routing/update/maintenance/rollback fixtures | Exact LIST22/RUN22/PASS22/SKIP0/FAIL0 |
| Rust Indexer executable tests | 13passed: process-object owners, native admission, lazy start/concurrency/stop and helper refusal |
| Shared runtime native retirement and coordinator shutdown | 2passed |
| Cache reopen/delete/restore/cursor replay | 1passed, temporary storage only |
| Indexer all-target strict Clippy | Passed with `-D warnings` |
| Installer bootstrap migration | 50assertions passed; controlled registration and actual x86/x64 detection entry |
| Indexer lifecycle/updater demand/installer complete-update source gates | Passed |
| Scoped Rust formatting, diff whitespace, spec navigation | Passed |

C# fixtures use the original production owners with fake SCM/transaction operations or owned pipes, signed temporary packages and injected settlement. Rust process fixtures use only self-owned child processes; cache tests use temporary files. Counts are separate boundaries, not a claim of installed behavior. Every Rust invocation used jobs1, incremental0 and codegen-units256. No real volume scan, live SCM/Registry mutation, installed process termination, package, installation or commit was performed.

## Remaining acceptance

The installed protected tuple and updater bootstrap have not been replaced. Formal same-key signed upgrade/maintenance is required to apply the code and old-registration migration. Packaged LocalSystem restart/base+delta recovery/idle CPU+I/O/real search plus explicit Desktop Exit and updater process disappearance remain unperformed. These gates must retain an authenticated approved Desktop rather than use an unapproved standalone PowerShell Hello as a service owner. Controlled unit/pipe/source gates do not certify that release acceptance.

## Main owners

- `rust/apps/nxtools-indexer/src/windows_service_host.rs`, `desktop_lifetime.rs`, `desktop_lifetime_tests.rs`, `session_runtime.rs`, `main.rs`.
- `csharp/NxTools.PlatformHost/Windows/IndexerServiceInstaller.cs`, `IndexerServiceInstaller.Lifetime.cs`, `TrustedUpdateBroker.cs`, `TrustedBootstrapMaintenance.cs`, `TrustedBootstrapMaintenanceWindows.cs`; original Program role routing and reviewed tests.
- `installer/bootstrap-migration.ps1`, `tests/architecture/installer-bootstrap-migration.ps1`, `indexer-lifecycle.ps1`, `trusted-update-demand-lifecycle.py`.

Reference sources: legacy NxTools MainWindow Hide/Exit boundary; SwiftList Service/Program.cs and Service/UsnService.cs for native service ownership/stop only; Listary managed boundary and service metadata, not native index-engine source. Current contracts are in `docs/reference-code/system-shell.md`, `quick-launch-research.md`, and the Indexer/PlatformHost specs.
