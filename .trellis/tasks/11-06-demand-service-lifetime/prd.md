# Demand-only indexer and updater lifetime

## User contract

Explicit product Exit must not leave index tracking or an update-request listener running. Closing to tray retains the running application and its ordinary features. Index startup restores the existing persisted base/delta and catches up changes while the desktop is running; no requirement to track changes while the application is absent. An actual authenticated update may finish or roll back its already prepared file transaction before its process exits.

## Change boundary

- Rust Indexer Windows service binds lifetime to approved desktop process objects after an admitted Hello. Pipe reconnect is not desktop exit. When the last approved process ends, the original service/runtime stops; an unclaimed startup has a finite admission deadline.
- C# original IndexerServiceInstaller registers/configures Demand start. A same-release existing automatic registration uses an explicit narrowly authorized configuration operation only after the existing complete protected tuple passes validation. It does not redeploy binaries or reset data. Later starts require no repeated authorization.
- C# TrustedUpdateBroker starts the fixed verified service on a real request, processes one operation, and normally stops after reply/transaction settlement. No-client and malformed/disconnected handshake have finite bounds. SCM Stop is normal cancellation, not a recovery-loop failure.
- Existing bootstrap maintenance upgrades approved older updater executors through its original signed same-key authorization and durable handoff. Ordinary application code cannot silently overwrite its protected bootstrap or rotate trust.

## Invariants

Preserve exact service/image/account/peer ownership, source/protected atomic tuple checks, existing authenticated-user start rights, standard-user Desktop, immutable source and signed update sequencing, prepared/committed recovery and rollback. No service-name-wide process termination, new publisher key, service removal, data deletion, cache version change or unconditional full-volume rebuild. Rust remains indexing/domain owner; C# remains SCM/native boundary. No additional permanent watcher/broker or feature flag.

## Diagnostics

Use original operation correlation where available and fixed lifecycle reasons, elapsed time and owning native code. Indexer lifetime starts/admitted/last-desktop-exited/stopped; updater requested/started/transaction terminal/service stopped. No paths, queries, payload text, package contents or credentials. No per-tick/per-file diagnostics.

## Acceptance

1. New and upgraded registrations use Demand, not Auto. Existing owned Auto registration requests its one explicit migration; decline/failure/unhealthy tuple does not start or redeploy it.
2. Authenticated desktop alive plus pipe reconnect retains one lifetime owner; unauthenticated connection acquires none. One owner exiting while another remains does not stop the service. Last owner exit or no-owner startup deadline ends actual service work.
3. Startup preserves persisted base/delta/USN recovery. Only established stale journal/cache conditions authorize existing rebuild maintenance.
4. No update request means no automatic updater start. A request starts the validated service once; success/rejection/failure, probe and no-client deadline complete one service invocation and release the listener/process.
5. Cancellation/client exit cannot strand a prepared transaction or keep the service waiting after settlement. Existing recovery/rollback and bootstrap-maintenance tests pass.
6. Original C# effect-reviewed fixtures and owned process/pipe Rust tests establish controlled behavior. Installed LocalSystem release acceptance requires separately authorized atomic packaging/deployment, idle/restart/base+delta/search and actual exit observation. Source tests are not installed acceptance.

## Exclusions

No packaging, installation, live SCM/Registry reconfiguration, service stop/start, real-volume scan, commit or push is performed by source implementation. No UI restyle or unrelated archive/download changes.
