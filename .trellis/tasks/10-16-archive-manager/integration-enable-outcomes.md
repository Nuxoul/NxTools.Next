# Explicit archive product integration enablement

Date: 2026-10-27. Same **Phase F — Explorer integration and genuine native drag**, P206/P207 source repair only.

## Authority and preserved invariants

The user explicitly requests usable product integration, superseding the historical permanent
`ProductApplyAuthorized=false` source constraint. Product explicit Apply/Revert is now enabled;
this is NOT permission to execute Registry, COM, Shell, service, package, installer or ordinary
product-launch operations on this workstation. Earlier outcomes/acceptance remain historical.
No live integration, Windows 10/11 Explorer or first-release acceptance is claimed.

- Save/status/close/cancel is not Apply. Rust remains desired state/layout owner; C# owns Windows APIs.
- Original desired revision and durable observed ledger stay separate; original transaction captures
  all values before writes, records attempt before mutation, compensates reverse, and reverts only
  exact kind/data matches. Modified owned values and foreign/shared values survive unregister.
- HKCU Software\\Classes only; no UserChoice/HKLM, default forcing, recursive key-tree removal.
- Original Desktop resolver and retained `OpenArchiveRelease` helper identity lease remain product
  prerequisites. Empty/unknown/missing/tampered/null identities refuse before Registry capture;
  no fallback from grouped selection to `%1`, no new trust/registration/install/transaction system.
- Original serial dispatcher detaches integration; original bounded STA runner has one active slot,
  15-second reply bound, and retains its slot through actual cleanup after timeout/cancellation.
- Pipe cancellation flows into pre-admission/capture/write checks. Interrupted Apply uses the original
  reverse compensation. An admitted Revert finishes original exact cleanup and ledger retirement.
  Timeout/cancel replies are failed/unconfirmed, never applied or a promise that Windows is unchanged.
- Existing correlated accepted/terminal diagnostics now include started; bounded unconfirmed replies
  log fixed state/cause/elapsed only. No payload/path/secret or high-frequency logs.

## Reference and actual owner chain

Read AGENTS, archive-shell reference, implement.md P206/P207/amendments, shell-settings outcomes,
platformhost native/diagnostic and shared contracts before source changes. Re-read external
`D:\Code\Archiver-main\gui\src\shell.rs:56-138` (HKCU cascade shape, per-file `%1`, unsafe whole-root
unregister deliberately not copied) and `installer\archiver.iss:81-114` (ProgID/OpenWith vocabulary).
Legacy NxTools has no general archive-settings owner per the existing reference index; no native
engine/source claim is inferred.

Verified current chain: strict `PlatformCommandParser.ParseArchiveShellIntegration` ->
`PlatformPipeClient` original command route -> `WindowsPlatformAdapter.ArchiveShellIntegration` ->
`ArchiveIntegrationService.CreateProduct` -> original `ResolveApprovedDesktopExecutablePath`
(current verified sibling or approved protected Desktop) + `IndexerServiceInstaller.OpenArchiveRelease`
(original stable-handle five-member helper/worker approval) -> `ArchiveIntegrationPlan` /
`ArchiveSelectionHandlerPlan` -> original `ArchiveIntegrationTransaction` ->
`WindowsArchiveIntegrationStore` / original durable `FileArchiveIntegrationLedger`.
Build/install/COM-helper selection/forwarding owners are unchanged and were not executed.

## Root causes and source changes

1. Product composition unconditionally passed `ProductApplyAuthorized=false`; therefore every real
   Apply returned `disabled/apply_not_authorized` before identity/capture/write. This was not a
   missing Registry implementation or a UI copy problem. Product policy is now true for explicit
   requests, with identical native identity and transaction prerequisites.
2. The same command was absent from `PlatformCommandSequenceDispatcher.DetachedCommands` and the
   adapter called synchronous Execute on the reader. Once enabled, release hashing/Registry/refresh
   could block later commands. It now reuses original detached reader and bounded STA owner; no
   parallel registration or transaction path was added.
3. Cancellation was not passed from the pipe into integration. The route now carries the existing
   token into bounded dispatch and original transaction checkpoints; compensation remains original.

The native factory delegates to one overload that accepts only effect implementations, not a test
policy flag. Fixtures exercise that same product policy without constructing a Registry store or
using the default real-user ledger. Approved fixture release bytes use original read-only identity
verification; they are not signed/production release proof.

## Verification

Evidence directory: `C:\Users\Administrator\.pi-desktop\scratch\2b4b441f-ea46-480a-b5b5-84691667d060`.
All dotnet invocations use the reviewed harness with `--no-build --no-restore -- --fixtures` after
`dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -v:q`.
Each executed filter was first LISTed with the identical filters, no native authorization switch.

- **Observed red 1**: `archive Phase F P206 product composition`: LIST selected1/run0; RUN selected1/
  run1/pass0/fail1/skip0, exit1: explicit Apply permanently disabled. Build 0 warnings/errors.
  Logs: `integration-red-{build,list,run}.log`.
- **Observed red 2**, after policy enabled but before dispatch repair:
  `archive Phase F P206 product dispatch`: LIST selected1/run0; RUN selected1/run1/pass0/fail1/skip0,
  exit1: later serial reader command blocked. Logs: `integration-dispatch-red-{build,list,run}.log`.
- Initial green integrated set: build 0 warnings/errors, LIST selected19/run0, RUN selected19/run19/
  pass19/fail0/skip0, exit0. Logs: `integration-green-{build,list,run}.log`.
- Final verification after extending actual timeout/during-write cancellation coverage and explicit
  session-scratch identity files: build 0 warnings/errors; LIST selected19/run0; same-filter RUN
  selected19/run19/pass19/fail0/skip0/not_run0, exit0. Logs: `integration-final2-{build,list,run}.log`.
  Exact union filters: `archive Phase F P206`, `archive Phase F integration`,
  `archive association settings fixture`, `archive Phase F P207 future registration plan unapplied`.
  Command: `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj
  --no-build --no-restore -- --fixtures [--list] <the four exact filters>`.
  Cases cover strict wire, selected commands/layout/double-click, grouped compensation, current and
  empty/wrong Desktop identity, missing/tampered/null helper, explicit Apply/Revert, desired/observed
  revisions, cancel/timeout/one-slot actual cleanup, reverse mutation compensation and privacy.
- `python tests/architecture/archive-phase-f-shell.py`: PASS, exit0 (source ownership only).
- `python tests/architecture/spec-contracts.py`: PASS, exit0, 114 documents/14 owners (documentation
  and links only). Neither gate is native or visual acceptance.


No Cargo, production Generated identities/hashes, dependencies/vendor, browser/preview/Rust files,
actual Registry/COM registration/activation/Shell/service/package/install/product launch/commit/push
were changed or executed by this delegated lane. Real native and user interaction acceptance remains
separate and NOT RUN. No overall Phase F/first-release completion claim.
