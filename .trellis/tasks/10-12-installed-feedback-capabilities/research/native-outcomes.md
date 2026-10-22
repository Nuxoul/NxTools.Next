# R3/R4 native outcomes (2026-10-12)

## References verified before implementation
- Read `docs/reference-code/download.md:2447–2468` plus PlatformHost native/shared contracts first.
- ABDM `D:\Code\ab-download-manager-master\shared\utils\src\desktopMain\kotlin\ir\amirab\util\osfileutil\WindowsFileUtils.kt:20–40,57–117`: native selection/open first, executable fallback; Open With uses SHOpenWithDialog with ALLOW_REGISTRATION|EXEC. Its selection passes absolute child PIDLs and its COM cleanup is not a safe ownership template: Next must pass a borrowed relative child and balance COM initialization only on success.
- ABDM `desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\AppComponent.kt:722–805,881–909`: IO dispatch; final file -> incomplete file (not Completed) -> parent; keyed dialog reuse/dismiss. `shared\app\src\commonMain\kotlin\com\abdownloadmanager\shared\singledownloadpage\BaseSingleDownloadComponent.kt:33–49,107–134,149–177`: monitor updates same component; open/folder immediately dismiss. Approved R3 deliberately replaces immediate dismissal with correlated success.
- Legacy `D:\Code\NxTools\ViewModels\MainWindowViewModel.cs:1540–1617,1636–1658`: final existing file selection, directory open, otherwise parent (legacy creates missing parent); asynchronous open but unverified Explorer Process.Start. `Views\DownloadProgressWindow.axaml.cs:26–59,286–315` constructs an Avalonia Window, subscribes/unsubscribes task lifecycle, completed action calls Close. `Services\WindowAnimationService.cs:13–44` intercepts Closing and calls Close again after animation. Avalonia's native/base show/hide implementation is unavailable in this checkout; these leaf files are not native HWND evidence.

## Preserved boundaries/invariants
- Rust resolves final/temp/parent targets and owns keyed window, pending ID and generation; C# does not create missing directories, reinterpret Open With as parent open, or close a task HWND.
- Native file selection always executes even when a matching Explorer directory already exists. Relative child is borrowed from the live absolute PIDL; parent is cloned/trimmed; only owning allocations are freed, and all live through synchronous SHOpenFolderAndSelectItems.
- Native request acceptance is not terminal success. Existing/minimized Explorer must be restored, activated and re-read as a live matching foreground root. Lookup/readback retries and executable fallback are bounded. Failures remain retryable.
- Open With is synchronous on an existing detached STA Shell worker. Chooser cancellation is a separate terminal data value, never open success or an error toast; no early task-window close. Caller carries correlation through existing Shell dispatch lifecycle.
- Healthy protected tuple matching all three packaged identities is reused. Healthy older tuple requires explicit runas/UAC through InstallElevated. Unhealthy/unreadable tuple fails closed with repair instructions; no pipe admission/hash approval relaxation.
- Synthetic seams must never inspect/mutate real Explorer, registry, services, process shutdown, browser or user data. Compiled checks and real release/native acceptance are distinct.

## Implementation/verification
- Implemented native-first folder selection/open and one executable fallback on native rejection or missing foreground acknowledgement, within eight lookup/readback attempts. Fixed the pre-existing `IsIconicNative` P/Invoke export alias. PIDL ownership is covered by an injected allocation/borrow/free fixture.
- Added OpenWithShellDialog.Show returning `open_with_completed`, `cancelled` (Success=false, Error=null), or fixed actionable failure + native HRESULT. It runs inside the parent's detached STA Shell worker; it does not spawn another worker.
- Replaced unconditional existing-service reuse with production `DecideDeployment` / `EnsureCoreAsync`: healthy matching three-binary tuple reuse; healthy older tuple explicit UAC upgrade via existing atomic InstallElevated; damaged/unreadable tuple repair-required. Authentication/approval checks and exact-path shutdown remain unchanged. SHA256 identity is the release-version authority (not an AssemblyVersion label).
- Added seven synthetic test entries across InstalledNativeActionTests and IndexerUpgradeDecisionTests. No real Explorer/chooser/registry/service/process operations occur in these tests; file fixtures are isolated in session scratch and removed. Tests were written before production fixes where applicable; no executable red/green claim until registered and run by parent.
- Actual check: `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -nologo` passed with zero warnings/errors. Scoped `git diff --check` passed. Test execution deferred because Program.cs registration is parent-owned; no unfiltered/real DOpus suite run. Live Explorer/chooser/UAC/deployment/LocalSystem service gates, Rust build and packaging remain unauthorized/deferred.

## Required parent integration (blocking before live use)
1. ShellTarget: add OpenWith kind/action `open_with`, accept fully qualified existing files only, and at the beginning of Launch use:
   `if (Kind == ShellTargetKind.OpenWith) return OpenWithShellDialog.Show(Target, stageChanged);`
   Keep this inside the existing detached STA adapter path with caller correlation/start/stage/terminal/elapsed/native-code diagnostics. Process chooser cancellation before failure toast or success-close: Data=`cancelled`, Error=null, Success=false; keep the task window and clear pending identity.
2. Pass the IPC caller correlation into `IndexerServiceInstaller.EnsureAsync(_log, diagnosticCorrelationId, cancellationToken)` rather than the retained compatibility overload generating a new ID.
3. **Production Program.cs must route the new three-argument elevated invocation** (current parser only accepts two). Before ordinary IPC startup replace its install branch with:
   `if (IndexerServiceInstaller.TryParseInstallArguments(args, out var indexerPath, out var installCorrelation)) return IndexerServiceInstaller.InstallElevated(indexerPath!, log, installCorrelation);`
   The helper accepts legacy two arguments as well; the third is a GUID, not caller-controlled binary identity. No live upgrade should be attempted before this integration, otherwise the elevated child will miss install mode.
4. Register the seven entries with unique safe filter prefix (suggested `installed native fixture`):
   InstalledNativeActionTests.ExplorerNativeFirstAndFallback;
   InstalledNativeActionTests.ExplorerReadbackRetryAndPending;
   InstalledNativeActionTests.OpenWithCancellationAndFailure;
   InstalledNativeActionTests.ExplorerRelativePidlLifetime;
   IndexerUpgradeDecisionTests.ProtectedTupleUpgradeDecisions;
   IndexerUpgradeDecisionTests.ProtectedTupleHashChangeDetection;
   IndexerUpgradeDecisionTests.ProtectedTupleAuthorizationLifecycle.
5. Existing Program.cs Explorer fake defaults to E_NOTIMPL via interface compatibility, hence the old existing-directory no-launch assertion needs its fake to implement `OpenNative(target, isDirectory) => isDirectory ? 0 : unchecked((int)0x80004001)` or adopt the new native-first assertions. Rename/replace the obsolete `existing indexer service never requests second authorization` label/test; ShouldRequestInitialAuthorization remains only as a compatibility initial-install predicate, no longer the production decision.
6. Native tests cover terminal-return timing and retry, not Rust ID/generation close ownership; parent must retain/test final/temp/parent resolution and actual correlated completed-window pending/close/cancel behavior.
