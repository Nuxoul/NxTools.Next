# Windows API, lifecycle and mutation contracts

Evidence: `repo:csharp/NxTools.PlatformHost/Protocol/PlatformCommandSequenceDispatcher.cs`, `repo:csharp/NxTools.PlatformHost/Windows/BoundedStaOperationRunner.cs`, `repo:csharp/NxTools.PlatformHost/Windows/CenteredResizeCoordinator.cs`, `repo:csharp/NxTools.PlatformHost/Windows/BrowserRegistrationTransaction.cs`, `repo:csharp/NxTools.PlatformHost.Tests/Program.cs`.

## Ownership / observed runtime
Program initializes PerMonitorV2, log/cancellation/adapter/pipe lifecycle. Protocol v1 GUID envelope has1MiB frame limit; BoundedFrameReader honors cancellation/limit. PlatformPipeClient outgoing queue256; serial reader dispatch detaches blocking Shell/picker/OLE/process launch/service work before synchronous native calls can block. EventWindow owns the clipboard/WinForms STA, window messages and timers; LowLevelKeyboardMonitor owns a separate blocking keyboard-hook thread. Domain/GPUI remains Rust. Native methods partials are real code boundaries; prefixed P/Invoke needs actual EntryPoint.

## Scenario: Bounded native acceptance and applied result
### 1. Scope / Trigger
HWND resize/replay, STA/OLE or potentially blocking cross-process Windows call.
### 2. Signatures
`CenteredResizeCoordinator.TrySchedule(IntPtr root, Action operation) -> bool`; Windows command result carries success/error/native_error_code/data. Pending resize transaction includes request generation; source HWND is required for C# drag in a separate process.
### 3. Contracts
Default2 workers/64 HWND slots, one in-flight and one coalesced next per HWND. Native work outside scheduler lock; stuck call retains slot, never unlimited timeout replacements. Reader/EventWindow enqueue. Immutable generation replay/CAS writeback/retire; repeated empty IDs still distinct generation. Positive client device-pixel target, final correction/readback after live-center correction, no activate/Z-order change. Auxiliary preview hidden->noactivate->final bounds->show; primary search focus restored. OLE one dedicated STA loop begins while initiating button pressed after capture release, not another gesture runtime.
Retirement re-reads native client after commit interleavings; replay owns requested/readback dimensions. Queued requested-size WM_SIZE is forbidden because GPUI trusts stale lParam. Same-size sync derives the live client on the HWND owner and proves notification delivery. SendMessageW keeps the marshalled WINDOWPOS alive until processing returns; SendMessageTimeoutW with a temporary ref parameter is forbidden because timeout does not finish the receiver. Blocking stays in the fixed resize workers, never serial reader/EventWindow; a hung HWND retains its slot without replacement workers. Terminal diagnostics separately reject a current-client mismatch instead of success/final176.
### 4. Validation & Error Matrix
Admission false -> bounded reject; accepted/applied=false -> pending, not pixels-ready. Stale generation -> no commit/retire/terminal ownership. Destroyed HWND/API error -> native fixed code. Logical timeout/cancel -> no late result, doesn't claim uncooperative native call interrupted. Escape OLE cancel -> non-error cancelled, distinct from dropped.
### 5. Good/Base/Bad Cases
Good: final client readback matches and moved center retained. Base: one native gesture/window owner. Bad: SetWindowPos on serial reader, cached GPUI origin, timeout spawns overtaking thread or native API BOOL equals user outcome.
### 6. Tests Required
Custom WM_NCCALCSIZE HWND size/center/readback; same-size/current-client notification and delayed-receiver parameter lifetime; A/B generation barriers including Guid.Empty; delayed HWND worker/read responsiveness; fixed-budget/coalescing; native drag later-command responsiveness/cancel status. Exact harness filters `centered worker admission`, `client resize synchronization`, `browser registration compensates`, `native file drag does not block later platform commands`; real pointer/DPI/Shell must be separately accepted.
### 7. Wrong vs Correct
```text
Wrong: timeout -> create replacement worker -> old native call later overwrites new target.
Correct: fixed worker retains in-flight -> coalesced generation -> CAS result/retire only if current.
```

## Scenario: Browser registration compensation
### 1. Scope / Trigger
Native Messaging manifest/registry/legacy-key multi-step mutation.
### 2. Signatures
`BrowserRegistrationTransaction.Apply(IBrowserRegistrationStore store, IReadOnlyList<BrowserRegistrationChange> changes)`; store Capture/Write/Restore; registrar validates extension identity/packaged host hash.
### 3. Contracts
Capture all prior bytes/registry values+kinds/subkeys before any mutation. Record attempted index before Write, since Write can mutate then throw. Compensate every attempted step in reverse even if another rollback fails; aggregate incomplete compensation. Exact installer/service ownership and protected release tuple/hash guard any stop; service name alone insufficient. No SQL layer.
### 4. Validation & Error Matrix
Capture failure -> no mutation. Partial write failure -> attempted step included in rollback. Restore failure -> AggregateException, never claim recovered success. Unapproved host/extension/path/hash -> fail closed.
### 5. Good/Base/Bad Cases
Good: mutation-then-throw step restores original type/children/file bytes. Base: all changes succeed. Bad: rollback only successful writes or swallow compensation errors.
### 6. Tests Required
Synthetic store failure at every step with exact prior-state comparison; ownership/hash path tests and source gate. Real registry failure injection/compiled installer/service acceptance separate and unauthorized in docs task.
### 7. Wrong vs Correct
```text
Wrong: Write(); rollback.Add(target).
Correct: capture all; attempted=index; Write(); failure -> reverse Restore every attempted target.
```

No blanket native exception suppression. Isolated runner contains callback failures to keep thread alive, but owning operation reports bounded typed terminal failure; detached dispatcher drain/timeouts are not permission for infinite work. [Shared engineering](../shared/engineering.md).

## Fixed new-download presentation boundary
`PresentFixedWindowOverAnchorAsync` runs detached from serial command reader through the bounded STA runner (max2,5-second logical wait). New hidden GPUI HWND may retain CW_USEDEFAULT dimensions; apply the explicit device-pixel client size once at predecessor center, show nonactivating, read back once. Bind source display in Rust; if native DPI differs, prepare anchor monitor without sizing before deriving client/frame insets. Never resize predecessor, queue guessed WM_SIZE, poll dimensions or start timeout replacement workers. Failed/cancelled/dead-anchor presentation stays hidden/returns failure; Rust owns old/new retirement and retry. Diagnostics carry same caller-ID/correlation, start/terminal/elapsed/native code and HWND/size only. Owned100% native fixture is not mixed-DPI/physical-pointer acceptance. [Fixed-stage audit](../../../docs/audits/2026-10-09-fixed-download-stage-handoff.md).

## Ordinary shell settings versus protected indexer approval
Current-user startup/tray apply validates the configured desktop as exact running-PlatformHost sibling NxTools.exe with stable-handle embedded current DesktopSha256 match, or uses the existing fully approved protected fallback. Legitimate installed upgrades may coexist with an older protected service tuple; that tuple must not block ordinary shell settings on a valid current package. Outside-directory/env-only, tampered, missing and invalid-hash candidates fail closed. Protected service deployment/pipe admission keeps its original atomic tuple/registry approval; shell fallback is not service authorization. Preserve pending/correlation, saved-versus-applied failures, and synthetic temporary-file upgrade regression; do not mutate real services/Run registry just to test path selection.

## Explicit GPUI client bounds (Kit0.7.1 native gate)
SetWindowBounds preserves existing outer screen x/y but width/height are client/content device pixels. Current callers are Quick Launch panel/preview,Clipboard popup/link panel. NativeMethods derives outer/client frame insets, validates widened sizes before mutation, applies one SWP_FRAMECHANGED|NOACTIVATE|NOZORDER and reads final client. A SetWindowPos BOOL alone is not success; mismatch returns failure. Actual owned96DPI GPUI fixture first yielded384×402 from400×410 and444×398 from460×406; corrected client gates now pass exact400×410/460×406 including hide/reapply/reuse. Do not infer mixed-monitor frame/DPI acceptance from these96DPI results.

Passive mouse boundary: historical released GPUI HWND returned1 despite production WS_EX_NOACTIVATE. User explicitly approved the repository-local Windows0.3.8 WM_MOUSEACTIVATE branch: read current GWL_EXSTYLE,return3 iff WS_EX_NOACTIVATE,otherwise1. C# still owns style policy; GPUI's existing in-process message owner interprets it. No callback-pointer injection,click-through/disabled preview or post-activation focus stealing. Kit remains exact0.7.1,registry unmodified. tests/native/gpui-kit-acceptance.ps1 All now passes2/2 in both modes at96DPI,including primary1 and live flag toggles1→3;bounded message query is not physical-click acceptance. Clipboard initial hidden/noactivate/bounds/topmost/show steps require acknowledgements; reused policy must also confirm before staging. Pending identity precedes dispatch,initial/reuse-policy deadlines are3s,stored HWND/generation gates discard stale replies without hiding a replacement. Physical/mixed-DPI/reader/installed acceptance remains separate.

## Browser extension signing identity and isolated tests
`repo:csharp/NxTools.PlatformHost/Windows/BrowserExtensionSigningIdentity.cs` owns legacy PEM location and pure SPKI→SHA256first16→Chromium a..p identity, plus read-existing lookup only. Export owns LoadOrCreateSigningKey; registration cannot create/write keys. Public export retains production path; internal ExportWithSigningKey carries explicit path through actual signer, and all package tests must use a GUID-owned temporary path. Missing PEM contributes no exporter origin; malformed PEM is not replaced and retains original failure categories. Command/pinned/exported origins retain union/order/dedup; Firefox identity and distinct exporter(current-dir fallback)/registrar(no current-dir fallback) source policies remain. Required tests: `repo:csharp/NxTools.PlatformHost.Tests/BrowserExtensionSigningTests.cs` fixedSPKI/tempPEM/CRXsignature/read-only/union/error/cancel/fallback; source gate `repo:tests/architecture/browser-signing-isolation.ps1` forbids test calls to production-key entry. Synthetic artifacts are not browser installation or native registration acceptance.

## Scenario: Explorer target success separate from foreground
### 1. Scope / Trigger
OpenShellTarget SelectInExplorer, completed-download close and list Reveal.
### 2. Signatures
`ExplorerShellActivation.LaunchAndActivate`, `ObserveTarget(target,isDirectory,IExplorerTargetObservationPlatform)`; unchanged correlated WindowsCommandResult success/error/native_error_code/data.
Evidence: `repo:csharp/NxTools.PlatformHost/Windows/ExplorerShellActivation.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ExplorerRevealFeedbackTests.cs`.
### 3. Contracts
Production active-view readback uses ShellWindows/IServiceProvider/IShellBrowser.QueryActiveShellView/IShellView.GetItemObject(background); require valid visible view under matching root and normalized exact directory. File requests additionally require exact selected filesystem path, not LocationURL/shared-root HWND alone. Bounds256windows/4096selected items, existing8attempts/75ms waits/onefallback; existing detached STA, no new poll thread. COM declarations preserve exact IIDs/preceding IOleWindow vtable/HRESULT and balanced references. Target confirmation wins independently of best-effort restore/foreground; success Data foreground_confirmed or target_confirmed_foreground_not_confirmed. Native activation code can accompany success; Error=null. No fallback solely to steal focus once target confirmed.
### 4. Validation & Error Matrix
Missing/deleted/wrong selection/hidden or wrong active tab/unobserved target/destroyedHWND -> failure, no target success. S_OK/process launch/folder-only/foreground-only -> insufficient. Confirmed target plus denied activation -> success, correlated current completion window can close. Timeout/cancel/stale result -> original Rust retain/discard lifecycle, no unconditional click-close.
### 5. Good/Base/Bad Cases
Good: requested file observed selected, Windows denies activation, success has target-confirmed data. Base: target+foreground confirmed retains existing data. Bad: directory exists equals file located, or failed foreground turns successful target action into false error.
### 6. Tests Required
Exact audited Explorer feedback6 + existing safe compatibility7 (13total), real-entry red before fix, fake active tabs/selection/removal/delayed/error/COM metadata. Rust actual keyed-window close/failure/stale/cancel/disconnect tests. No live COM/Explorer implicit authorization; actual Windows11/UNC/physical acceptance separate.
### 7. Wrong vs Correct
Wrong: native request accepted -> assume selection; foreground denied -> operation failed.
Correct: observe exact target, attempt foreground independently, publish correlated truthful result.

Explorer reader ABI regression (2026-10-13): active.Folder returns base Folder dispatch, which does not declare Self. `ExplorerShellActivation.ReadFolderSelf` explicitly queries SDK IID_Folder2 f0d2d8ef-3890-11d2-bf8b-00c04fb93661 and calls get_Self at zero-based native slot17 (IUnknown3 + IDispatch4 + Folder10). Preserve HRESULT/returned reference cleanup. Decoded view snapshots cannot test this boundary: the owned COM table fixture exercises actual QueryInterface/slot/identity/failure/refcounts and reproduced missing Self before repair. No live Shell enumeration or installed-success claim follows from this fixture.

## Scenario: Enrolled signed protected updates
### 1. Scope / Trigger
EnsureIndexerService sees a healthy newer package tuple; installer finalization prepares an enrolled update. Source implementation only; privileged deployment acceptance remains separate.
### 2. Signatures
`repo:csharp/NxTools.PlatformHost/Windows/TrustedReleaseManifest.cs#VerifyPackage` takes root, pinned SPKI and minimum sequence, returns disposable VerifiedRelease with locked source handles and CopyTo(empty protected staging). `repo:csharp/NxTools.PlatformHost/Windows/TrustedUpdateTransaction.cs#ApplyVerifiedAsync` owns prepared/deploy/commit/recovery. `repo:csharp/NxTools.PlatformHost/Windows/TrustedUpdateBroker.cs#RequestAsync` carries local package root and Guid correlation to the fixed NxToolsUpdateV1 bootstrap; existing EnsureIndexerService wire remains unchanged.
### 3. Contracts
RSA-PSS/SHA256 authenticates raw bounded NxTools.release.json before JSON parsing; detached NxTools.release.sig. Schema1/product/nontemporary Guid/positive Int64 sequence/pinned-key hash; complete seven executables plus helper DLL/deps/runtimeconfig, licenses and extension paths; max1MiB manifest/4096files/2GiB artifacts; stable-handle reparse/path traversal rejection. Publisher PEM is explicit/off-client; generated UpdateTrust contains public SPKI only. Release package requires signing key+sequence before Cargo, legacy unsigned is an explicit non-trusted exception. Resident bootstrap is separate from replaceable tuple; old-bootstrap allowlist migration, trust rotation/bootstrap repair remain explicit maintenance, not proven by staging. Caller-identity source opens, fixed protected staging/deployment, serial bounded requests, correlation and low-noise terminal diagnostics. Once prepared, finish/rollback independent of client loss. Sequence commits after actual tuple/all-file validation; signed last-good recovery and cross-process deployment gate precede retry.
### 4. Validation & Error Matrix
No broker + configured trust -> one enrollment authorization, even same release. Enrolled newer valid release -> broker, no runas. Same signed identity -> idempotent. Invalid/missing signature, wrong key, partial package, reparse, duplicate, replay/downgrade -> reject without UAC fallback. Deployment failure -> signed prior release recovery; failed recovery retains journal and reports failure, never claims successful rollback. Missing publisher key -> package preflight error, no unsigned automatic fallback.
### 5. Good/Base/Bad Cases
Good: installer prepares trusted update before ordinary original-user launch. Base: same approved tuple starts without authorization. Bad: accepting a writable hash/config as trust, deploying arbitrary unsigned binaries as SYSTEM, or describing moving UAC into installation as one-time lifetime consent.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/TrustedUpdateRoutingTests.cs`, `repo:csharp/NxTools.PlatformHost.Tests/TrustedReleaseManifestTests.cs`, `repo:csharp/NxTools.PlatformHost.Tests/TrustedUpdateTransactionTests.cs`: real Ensure dispatch red/green, tamper/key/path/size/sequence/locked copy, signed commit/idempotence/rollback/restart/frames. `repo:tests/architecture/installer-runtime.ps1` trusted-success/failure uses fake self-owned host and production callbacks, no live service. Require separately authorized disposable-VM SCM/enrollment/medium-user update/crash-child/first-launch and packaged Indexer restart/search/idle CPU before release claim. Production key is not provisioned by tests.
### 7. Wrong vs Correct
Wrong: new tuple -> runas every release; authentication failure -> ask UAC to bypass. Correct: one enrolled protected public key -> authenticated staged complete release -> verified deployment -> durable sequence commit; failures preserve recovery and reject.

Installed peer correction(2026-10-15): `repo:csharp/NxTools.PlatformHost/Windows/TrustedUpdatePeer.cs#Verify` binds GetNamedPipeServerProcessId to fixedNxToolsUpdateV1 SCM Running own-process PID, after unchanged protected registration/LocalSystem/bootstrap hash/ACL validation. Ordinary client must not OpenProcess SYSTEM just to query image: actual installed caller returned AccessDenied5 before sending request. SCM_CONNECT1/SERVICE_QUERY_STATUS4/QueryServiceStatusEx only; safe handles close. Wrong/zeroPID,stopped/sharedprocess or queryerror reject before write, no UAC/signature/ACL fallback. Actual self-owned pipe fixture red→green plus wrongstates rejects; repaired actual service readonlyPASS/no deployment sent, C#245fixtures. Full installer deployment remains user acceptance; see owning installed-peer-failure.md. RequestAsync optional log carries fixed clientphase/nativecode/correlation/elapsed, no path/errorpayload.

Recovery refinement: `repo:csharp/NxTools.PlatformHost/Windows/TrustedUpdateEnrollment.cs`
records pinned initial identity before bootstrap/SCM mutation and marks completed last;
same-identity retries resume, changed key/release fails before deployment. Publisher
`repo:scripts/reserve-release-sequence.ps1` reserves a durable per-key sequence before
Cargo; repeated/lower values fail against ledger and previous output. Installer preserves
BrowserHost backup until trusted preparation succeeds; `repo:tests/architecture/installer-exact-file-rollback.ps1`
proves wrong hash retains file, correct hash deletion uses the same inspected handle.
The trusted-failure replacement fixture proves exact old-host restoration before exit9.

Phase F release-chain source amendment (2026-10-23): actual Rust Desktop output -> generated helper
DesktopSHA -> framework-dependent helper publish -> generated five-member archive approval ->
PlatformHost publish -> original signed full manifest. No reverse embedded hash edge. Original
ArchiveReadHelper fixed sibling/retained source lease verifies the approved worker before suspended
Job admission. Protected deploy validates all five archive members before service mutation and after
copy; health/approval and Desktop redirect reject partial/mixed current releases. Installer original
suspend/finalize owns twelve root binaries/runtime/sidecar files, predeclares new hashes from extracted
sidecars, records attempted before mutation, compensates every attempted member in reverse and only
deletes exact hash-verified files. Foreign/mismatched files and incomplete recovery retain the journal.
After durable `committing`, the complete verified new set is retained: cleanup errors never run Undo;
explicit commit retry validates new bytes and durable per-backup cleanup evidence before continuing.
GUID library-only regression is not actual installer/crash acceptance. In that historical amendment
`ProductApplyAuthorized=false` and production P208 rows stayed unchanged; file deployment never
applies Shell registration. Explicit product enablement below supersedes only the permanent source gate.

## Scenario: Staging-key cleanup and installer preparation reentry (F-RC01/F-RC02)
### 1. Scope / Trigger
Failure after isolated TEST-key creation, or a new installer preparation finding an old journal.
### 2. Signatures
`scripts/package.ps1 -IsolatedStaging` owns `$ownedTestKeyPath` inside its main try/finally.
`Prepare-UpgradeBrowserHost(InstallRoot, StateFile, OperationId, NewManifestPath, NewSignaturePath)`
is the default helper call reached from Inno PrepareToInstall; original Suspend/Undo/Complete remain owners.
### 3. Contracts
Claim the exact fresh test-key path before creation; create/trust/label/preflight failures all unwind
through its finally. Never infer cleanup from a directory scan or production SigningKeyPath.
Preparation validates fixed sibling journal path, root/schema2/operation/phase/entries and plain paths.
Old journal + actual file facts alone authorize recovery; new manifest is used only after recovery.
committing/committed -> original Complete with committed=true (validate new set, cleanup only).
Precommit -> original reverse Undo, then verify complete old set/no backups before journal retirement.
Inno marks UpgradePreparationReady only on helper success; failed preparation cannot Finalize an old journal.
### 4. Validation & Error Matrix
Unknown/malformed/mismatched/oversized journal or unproved .pending -> refuse, retain, no new transaction.
One precommit restore fails -> attempt all other reverse restores, retain journal and refuse fresh Suspend.
Successful old recovery -> remove only original journal, then original fresh Suspend creates next transaction.
### 5. Good/Base/Bad Cases
Good: second preparation resumes failed committed cleanup before new backup. Base: no journal -> Suspend.
Bad: fresh Suspend forever rejects retry, cleanup before try leaks key, or new hashes authorize old deletion.
### 6. Tests Required
`tests/architecture/staging-key-cleanup.ps1`: unchanged wrapper copy + external tool stubs, key/trust/label/
preflight failures, exact key absent, foreign file intact, no Cargo. `installer-preparation-recovery.ps1`:
two actual preparation calls for committed/committing, precommit aggregate recovery, foreign/malformed reject.
Original owned-set632 assertions and installer-update-lifecycle source wiring remain relevant, not live acceptance.
### 7. Wrong vs Correct
Wrong: CreateTestKey; WriteTrust; try Build finally DeleteKey. Correct: try CreateTestKey; WriteTrust; Build finally DeleteExactOwnedKey.
Wrong: new Prepare -> fresh Suspend(old journal) -> reject. Correct: Prepare -> validate/recover OLD facts -> verify -> fresh Suspend.


Explorer observation topology refinement (2026-10-14): IWebBrowser2.HWND may be a tab
child, so resolve native GetAncestor(GA_ROOT) before CabinetWClass/ExploreWClass
classification; still require actual active visible view under that root and exact
folder/selected file. `repo:csharp/NxTools.PlatformHost.Tests/ExplorerObservationTests.cs`
uses owned HWNDs/raw RCWs through actual QueryService/QueryActiveShellView/GetWindow/
GetItemObject chain; child input red was HWND1/service0, green reaches all5calls and
preserves injected E_ACCESSDENIED/hidden/wrongroot/refcount contracts. This proves
classification traversal, not successful full automation or installed Reveal. Fixed
observation diagnostic stages are emitted once per command, not per retry/window.
Passive user-session observation found0ShellWindows and0Explorerroots, so the specific
installed recurrence cause and complete selected-target confirmation remain unverified.

Extension runtime discovery: empty typed InspectExtensionRuntime selection allows bounded process PATH search (256segments/32767chars), fixed python/node/powershell/pwsh.exe names only. Skip relative/malformed/WindowsApps entries; explicit invalid/missing selections never fall back. Normalize only local canonical drive prefix for Rust-persisted paths before existing validation; device/UNC/ADS remain rejected. Original ordinary Job3s/256MiB/2processes/4KiB, image hash before/after actualcleanup and metadata-only diagnostics retained. Discovery on explicit trust/environment open is informational, never approval or dependency install; no system/user PATH mutation. Owned apphost/process-local PATH fixture and Desktop fakebridge/current approval regressions: [trust/PATH contracts](../../tasks/10-15-extension-trust-path/code-spec.md).

## Native context menu session (2026-10-15)
`repo:csharp/NxTools.PlatformHost/Windows/NativeContextMenuCoordinator.cs` owns one global admitted menu/STA slot until real native cleanup. Typed ShowContextMenu takes sourceHWND/pipe-peerPID and source-client DEVICE-pixel anchor; C# PMv2 ClientToScreen and OS menu work area/RTL placement, no Rust Win32 calls. Reject invalid/invisible/foreign-session/PID-mismatch source, root>128/total>256/depth>4/label>256UTF16/total>32768, duplicate/nonpositive leaf IDs and malformed separators before allocation. Worker-owned WS_POPUP tool HWND is owned by foreign source root; TrackPopupMenuEx cannot use foreign-thread source as modal owner. Never block serial pipe reader/EventWindow; bounded synchronous admission precedes subsequent cancel so cancel cannot overtake creation. Cancellation/disconnect/120s lifetime posts exact worker message -> EndMenu; retain slot until menu handles/surrogate destroyed. Track state at actual TrackPopupMenuEx boundary: TPM_NONOTIFY did not deliver assumed WM_ENTERMENULOOP, actual owned fixture reproduced cancel timeout. Existing EventWindow pointer sampler marks outside and bounded200ms flush covers menu-return tick interleaving, no second hook/reader. Terminal data only selectedID/outside flag, callback Rust deferred/current; conditional source foreground restore never after actual observed external click. Low-noise correlated accepted/start/ready/terminal/nativecode/count/elapsed only. `repo:csharp/NxTools.PlatformHost.Tests/NativeContextMenuTests.cs`: safe wire/admission/cancel/nonblocking fixtures plus explicitly selected owned300x440 native#32768 overflow/UIA cascade/exactcancel test pass. Same-process owned fixture is not installed foreign-process/physical/mixed-DPI acceptance; [outcomes](../../tasks/10-15-extension-action-feedback/ux-outcomes.md).

## Scenario: Official styled menu's independent GPUI HWND
### 1. Scope / Trigger
User rejects the historical white HMENU renderer while requiring true source-HWND overflow.
### 2. Signatures
ShowContextMenuCommand adds optional popup_window_handle:i64(default0). Nonzero + empty items selects NativeContextMenuCoordinator.PrepareStyledSurface; returns width/height/anchor_x/anchor_y device pixels, not a menu selection. Rust styled_menu_surface owns official PopupMenu/Root; native_context_menu owns one source/session/trigger callback lifecycle.
### 3. Contracts
Validate connected source PID/session/visible root and hidden popup in same source process; distinct HWNDs. C# PMv2 ClientToScreen -> MonitorFromPoint/GetMonitorInfoW -> owner WS_POPUP/tool style -> exact work-area final client bounds/readback, detached from serial reader. Do not invoke HMENU on this production route. Rust pending before creation/IPC, unique phase IDs, current scale/model placement then freshly submitted next-frame reveal and acknowledged native activation; only later deactivation dismisses. Session first-wins terminal retires before deferred popup removal/original callback/current source and trigger. Cancel/disconnect/stale native result cannot show a retired popup. No native APIs in Rust, labels/payload logging or fallback clipped source menu.
### 4. Validation & Error Matrix
Foreign/visible/same popup or invalid source -> reject before placement; readback mismatch/native error -> failure with source retained. Duplicate phase/old session/retired trigger -> no show/callback. Outside -> original source hide, Escape -> source native/GPUI focus restoration only while original source/session remains current.0popup remains historical native fixture compatibility, not the current product renderer.
### 5. Good/Base/Bad Cases
Good: hidden independent popup final viewport contains root/cascade beyond300px source. Base: same-source/session model and keyboard callback. Bad: paint HMENU, show hidden CW_USEDEFAULT before prepared frame, source-window clipping or update leased source/menu synchronously.
### 6. Tests Required
Actual official row/cascade/LightDark bounds, disabled ancestor and take-once deferred/stale callback; C# wire/admission/nonblocking plus owned HWND hidden/owner/work-area/readback/overflow/cancel fixture. Physical/foreign-process/mixed-DPI/user acceptance separately required. [Current outcomes](../../tasks/10-16-menu-notification-refinement/outcomes.md).
### 7. Wrong vs Correct
Wrong: SetWindowPos success -> immediately visible -> direct business callback. Correct: native final readback -> GPUI place/frame -> current-phase show/activate -> take session -> deferred still-current callback.

## Scenario: Memory-capped archive worker process
### 1. Scope / Trigger
Approved Phase A worker isolates unbounded codecs; bounded product open/test now uses [read lifecycle](../nxtools-archive/backend/product-read.md). Installed-codec/release acceptance remains separate.
### 2. Signatures
`ArchiveWorkerRunner.ForPackage(publish,log)` fixes sibling `NxTools.ArchiveWorker.exe` and only `--archive-worker`. Internal `ForFixture` accepts exact current test apphost only. `Start(ArchiveWorkerRequest)`, `Write(id,JsonElement)`, `Cancel(id)`, `Stop`, `DrainAsync` retain per-operation ownership. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Extensions/ExtensionNative.cs#Spawn`.
### 3. Contracts
At most2Jobs/aggregate2GiB; process ceiling=approved codec work+metadata+64MiB overhead, default832MiB, max1GiB, one process/kill-on-close/no breakaway. Work below64MiB rejects ResourceLimit; invalid/unknown/duplicate budget fields reject invalid_budget; explicit MemoryBytes mismatch rejects budget_mismatch before spawning. Suspended create with stdio allowlist -> assign Job -> current admission -> resume. No secret/path argv. Private u32LE1..1MiB+strict UTF8JSON; schema2 start negotiates source_lease_v1/budget_reservations_v1, then source_acquire before Rust source reads. ArchiveSourceLease holds no-reparse parent chain without delete sharing and read-only source with FILE_SHARE_READ only, including both UnRAR path opens. Source handles survive actual process/pump/Job cleanup. Schema1 ZIP stays compatible; schema1 isolated codecs explicitly reject UnsupportedOperation. stdout8MiB/stderr64KiB are finite transport bounds, not million-entry acceptance. Cancel version1/id/op=cancel;200ms grace then own-Job termination; slot retained until real cleanup. Publish consumer must be bounded/nonblocking before product routing; never codec work on serial reader/EventWindow.
### 4. Validation & Error Matrix
Duplicate/current/retired ID or excess slot -> reject; invalid frame/schema/id/UTF8/count -> fixed failure; missing executable/admission failure -> native code at owner, suspended child cannot execute; cancel/timeout/disconnect -> own-Job cleanup, no unrelated process kill; failed log sink -> unchanged result. `exited` with CleanupComplete=false is not a free worker. Saved/read capability is not codec success.
### 5. Good/Base/Bad Cases
Good: current-apphost fixture observes Job flags/memory/process cap before marker can execute. Base: framed complete then real process/pump exit. Bad: Process.Start then assign after execution, timeout replacement thread, arbitrary executable from IPC, log child stderr/frame/password or infer real Rust codec acceptance from fake child.
### 6. Tests Required
Preserve original native/codec/output fixtures plus ArchiveProduct read fixtures. Full deployment/FIFO/resource/native-user acceptance remains open; exact current results live in the task checkpoint.
### 7. Wrong vs Correct
Wrong: unrestricted codec in desktop, or thread timeout pretends native memory/call stopped. Correct: bounded fixed worker admitted to owned OS Job before execution -> private correlated frames -> cooperative stop/own-process termination -> real cleanup -> slot release.

Evidence: `repo:csharp/NxTools.PlatformHost.Tests/ArchiveRemediationTests.cs` source/Job bounds; `repo:csharp/NxTools.PlatformHost.Tests/ArchiveExtractionWorkerTests.cs` encrypted outputs. Results/history: `repo:.trellis/tasks/10-16-archive-manager/overwrite-remediation.md`. No native-user acceptance implied.

## Scenario: Archive handle-output rendezvous
### 1. Scope / Trigger
Isolated Rust ordinary known-destination extraction writes only through C# held-directory capabilities. This includes button/dialog/context,Smart/Here/Name/Desktop/custom and destination-directory drag across admitted formats. Virtual cache,archive Create/Edit/split candidates and HZ5 retain separate output contracts. The current ZIP parallel slice follows the completed original synchronous-pump baseline. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs`; approved targets: [ordinary output alignment](../../tasks/11-02-archive-workflow-optimization/ordinary-extraction-output.md), [bounded parallel ZIP](../../tasks/11-02-archive-workflow-optimization/parallel-extraction.md).
### 2. Signatures
ArchiveWorkerRequest.OutputRoot stays in C#. Ordinary begin/write/commit/abort uses the existing native-call envelope and matching native result; accepted bulk v1 must include private `extraction_output_v1`. Private `ParallelExtraction=true` additionally requires `parallel_extraction_v1` before mutation and serializes as `parallel_extraction:true`; absent/false remains serial. Production ordinary Jobs request parallel capability by default, not through a new public Job/UI schema. Begin returns a token for a new final-name handle or `success:false,error_code:target_exists,existing_bytes`. `begin_overwrite` directly truncates the same relative path. Binary write records are <=32KiB and receive no per-record native result. `commit{token,metadata}` is ordered metadata+drain+close.
### 3. Contracts
OutputSession holds non-share-delete safe parent handles and resolves relative single components with no reparse. Initial ordinary begin opens the final name with `CREATE_NEW`; no exclusive GUID/sibling partial,rename/replacement,FlushFileBuffers,per-write/content/file-ID fingerprint,per-file VerifyDirectory or published reopen. Existing-target failure mutates nothing and Rust owns Ask/Overwrite/Skip/AutoRename/current-attempt ApplyAll. Overwrite has no target signature/CAS and directly truncates the same path; AutoRename repeats bounded CREATE_NEW attempts.

Rust sends coalesced <=32KiB records through the original anonymous stdout pipe without ACK. The original independent input/stdout pumps keep the existing synchronous `CreatePipe` handles: input uses `FileStream` buffer1 and cancellation-prechecked synchronous `Write`; stdout `ReadFrames`/`ReadBulkWrite` use synchronous `Read`/`ReadExactly`. The pipes supply bounded backpressure. Keep the first native write failure/code sticky. The stdout pump consumes preceding payload before calling `session.Commit`; logical commit remains drain -> metadata/MOTW -> close, but `native_commit_ms` measures only that post-drain native metadata/handle-close call. Rust `output_close_wait_us` measures the full await including queued payload consumption. No new pipe/runtime/handle authority or native-flag change. Unnegotiated directions retain one pending control; negotiated ordinary parallel extraction uses at most32 keyed controls and bounded native tasks on the original Run.

Codec/source integrity controls when Rust may commit. Delayed solid/format integrity may retain a bounded set of live final-name handles. Cancel,CRC/codec/size/source/native failure closes and deletes only incomplete owned output(s); successfully closed files/finalized directories and pre-existing skipped targets remain. Journal stores bounded result paths,created counts and completion/cleanup state,not ordinary file hashes or IDs. Worker stdout pump owns native work; no serial command/event blocking or second runtime.

Only independent ordinary begin/begin_overwrite/commit/abort may overlap after parallel negotiation. Clone retained JSON before its document is disposed; keep accepted native work bounded32 and await every accepted operation before terminal,Cancel/Dispose cleanup or source/destination lease release. Replies use the existing bounded stdin queue without treating temporary fullness as native failure. Payload remains on the original ordered stdout consumer; close cannot overtake earlier token payload and later payload cannot follow close. Protect shared admission/journal/cache with short locks and per-file ordering, never a session-wide lock across native open/metadata/close. The first native category/code survives EOF,cancel and cleanup; every incomplete owned file is deleted while completed closes remain journal facts.
Parallel native/worker operation faults may replace only a provisional `cancelled` result; a real category/code remains first-wins. Remove completed native tasks before admission/high-water accounting. `archive.worker.work` forwards `parallel_workers_planned`, `parallel_workers_high_water` and `parallel_members_completed` as bounded numeric aggregates.

Archive Create/Edit output continues using exclusive candidates,complete verification/readback,target checks and verified replacement. HZ5 continues append/flush/readback/recovery. ArchiveEntryCache continues Seal/verified cursor checks.

Native timing exports are inclusive: `native_begin_ms` covers the whole bridge begin action; `native_parent_resolution_ms` is validation/cache parent resolution within it; `native_final_open_ms` covers ordinary open allocation, syscall and inspection; `native_create_ms` covers only `NtCreateFile` plus P/Invoke marshalling; `native_open_inspection_ms` covers post-open inspection; `native_directory_ms` and `native_directory_metadata_ms` cover their native actions. These overlap and are neither additive nor exclusive IPC. A duplicated-output-handle boundary remains only a causally supported authorized option, not the current design. The [current genuine Release nine-trial result](../../tasks/11-02-archive-workflow-optimization/ordinary-extraction-pump-results.md) is byte-correct at4241/4214ms button/Root medians versus Bandizip1476.011ms,2.873×/2.855×; the required<=1.5× gate remains open.
### 4. Validation & Error Matrix
Unsafe/name/reparse -> typed refusal. Existing CREATE_NEW -> target_exists+bytes,no token. Direct overwrite wrong-kind/access/reparse -> exact native code,no observation retry. AutoRename target_exists -> next bounded candidate; exhaustion -> OutputCollision,no overwrite. Short/native write -> first code returned by commit. Metadata/close failure -> delete current incomplete owned output. Cancel with prior completed files -> completed journal retained and cancelled-with-output semantics. Unknown close/delete -> explicit cleanup ownership,no blind retry or rollback.
### 5. Good/Base/Bad Cases
Good: independent payload+metadata comparison through actual Job/Rust/native final handles. Base: new file begin,payload,commit. Bad: unrestricted unpack,lexical-only path proof,per-file ACK/partial rename/fingerprint,or ordinary direct-close semantics applied to archive candidates/cache/HZ5.
### 6. Tests Required
Boundary coverage retains final-name/no-partial/rename/flush/fingerprint/reopen, two JSON controls, zero write ACKs, <=32KiB backpressure, sticky write error, conflicts/direct overwrite/AutoRename, commit metadata, delayed-integrity cleanup, prior completed retention, journal truth and actual cleanup. It additionally fixes synchronous original-pump behavior and exact native begin/parent/open/create/inspection/directory timing boundaries, including the distinction between post-drain `native_commit_ms` and full Rust `output_close_wait_us`. Preserve Create/Edit candidate,HZ5,cache,path/source/integrity and operation-isolation suites. Current genuine Release bytes/cleanup pass; the required performance gate and installed-package acceptance remain open.
Parallel coverage adds actual native-begin overlap, deterministic32 pending-task cap, multiple-incomplete cancellation/completed retention, native/worker failure after provisional cancellation, nonaccumulating pending diagnostics, exact token ordering and source-lease release only after joins. `repo:csharp/NxTools.PlatformHost.Tests/ArchiveParallelExtractionTests.cs` exercises the native/runner seam; `repo:csharp/NxTools.PlatformHost.Tests/ArchiveParallelExtractionTests.ActualWorker.cs` combines actual Rust Stored/Deflate decoding with original native output for exact bytes, dual-incomplete cancellation and CRC cleanup. Genuine Release nine-trial acceptance remains separate.
### 7. Wrong vs Correct
Wrong: integrity -> FlushFileBuffers -> sibling rename -> output fingerprint/reopen for ordinary extraction. Correct: held safe final-name open -> bounded original-pipe writes -> integrity -> metadata+drain+close -> incomplete-only delete on failure; candidate owners remain separate.

## Scenario: Runtime clipboard capture pause
### 1. Scope / Trigger
Temporary privacy pause, resume, deadline, pipe reconnect or event-owner shutdown.
### 2. Signatures
`SetClipboardCapturePausedCommand { paused: bool }`, wire command set_clipboard_capture_paused; `EventWindow.EnqueueClipboardPause(command, correlationId, cancellation)`. Success Data remains a JSON STRING containing exactly capture_paused:boolean, not an object. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.ClipboardPause.cs`, `repo:rust/apps/nxtools-desktop/src/clipboard/capture_pause.rs`.
### 3. Contracts
Independent of ConfigureClipboard/hotkey and persisted enabled. Event owner applies gate before ACK and clears retry/source/attempts/timer. Guard update,retry and real ReadClipboardContent before sequence/source/native formats/Contains/Get. Resume samples current sequence metadata only and excludes that sequence from queued updates; never backfill paused content. Admission is bounded32, synchronous before reader advances; completion is detached through existing dispatcher so later commands remain responsive and pause/resume cannot reorder. Pending ID precedes Rust dispatch; only exact ACK marks applied. Runtime15minutes/manual, generation-scoped deadline/reconnect, no restart persistence. Rust rejects in-flight captures before management deferral and removes staging.
### 4. Validation & Error Matrix
Missing/nonboolean/unknown fields -> reject. Queue full/stopped/post failure -> terminal failure/no applied data/no queued leak. Cancel before application removes request; application wins late cancellation. Apply/resume-baseline failure remains fail-closed with native code. Timeout/disconnect -> explicit unknown/failed UI, retry reachable, late ID cannot retire new intent. Stop immediately retires queued tasks then owner clears native retry state.
### 5. Good/Base/Bad Cases
Good: bounded accepted -> owner gate -> correlated applied ACK; paused content never read/replayed. Base: history/paste remain usable. Bad: persist enabled=false, report pause from enqueue, block serial reader awaiting owner, or resume by reading current clipboard content.
### 6. Tests Required
Six exact ClipboardPauseTests fixtures use production WndProc/read/retry/application with injected OS operations: no real clipboard/HWND/hooks. Assert gate-before-ACK/no sequence read while paused, retry cleanup/no-backfill/idempotent resume, bounds/post cleanup, cancellation/disposal, native failure/privacy/throwing log and application-cancel race. Desktop capture_pause regressions assert pending/applied/failure/reconnect/deadline/generation/staging plus actual official controls. Live Windows listener/physical gesture/DPI/user acceptance stays separate.
### 7. Wrong vs Correct
Wrong: ConfigureClipboard success -> claim privacy pause; resume -> capture existing content. Correct: independent correlated owner command -> gate ACK -> value snapshot; resume records sequence and waits for a fresh change.

## Directory Reveal semantic correction (2026-10-22)
Approved follow-up corrects the preceding file-only selection wording: `ExplorerNativeFolder.Open` PIDL-selects ordinary directories as well as files; `ExplorerShellActivation.ObserveTarget` requires the parent activeview AND exact selected targetpath for both. Entering the requested directory is ordinaryOpen, not Reveal. A parentless drive root is the explicitexception: openitself/confirmitsownactiveview; fallbackordinarydirectory uses `/select,"<exacttarget>"`. Existing8attempts/75ms/onefallback, activeview COM/Folder2 ABI, borrowedrelativePIDL lifetime, nativeHRESULT and foreground-independent success remain. Actualentry syntheticred accepted directoryitself;9exactreviewedfixtures nowpass/skip0. No liveExplorer operation or proof that this completely explains the user's installedtimeout. Rustsuccess-onlyclose/stale/disconnect owners unchanged. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/ExplorerNativeFolder.cs`, `repo:csharp/NxTools.PlatformHost/Windows/ExplorerShellActivation.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ExplorerRevealFeedbackTests.cs`.

## Scenario: Successful Explorer readback and filesystem path aliases
### 1. Scope / Trigger
Both completed-download Reveal and main-list Reveal reported failure after Explorer opened. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/ExplorerShellActivation.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ExplorerObservationTests.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ExplorerRevealFeedbackTests.cs`.
### 2. Signatures
`NativeTargetObservationPlatform(Action<int> nativeFailure, Func<object>? createWindows, Action<string>? observationStage)` always reads selected paths. `NormalizeDirectory(string)` is shared by requested target, observed folder and selected paths. Wire/result schema unchanged.
### 3. Contracts
Ordinary directory Reveal requires parent plus exact selected directory, so native reader must not return an empty selection merely because target is a directory. Files and directories share the successful active-view/Folder2/SelectedItems chain. Comparison maps only explicit extended DOS drive and extended UNC prefixes to their ordinary filesystem aliases, on both sides; preserve other device/volume namespaces and exact case-insensitive full identity. No basename/LocationURL-only success. Keep8attempts/75ms/onefallback, detachedSTA, balanced COM, hidden/wrongroot/wrongtab/missing/selection guards and best-effort foreground. Original Rust correlated currentHWND+generation success-only close remains; list actions carry no window identity. Existing diagnostics remain fixed/correlated, no payload/new polling.
### 4. Validation & Error Matrix
Selected directory in parent -> reader retains selection and matcher succeeds. Directory entered itself/wrong selection/hidden or wrong-root view -> reject. ExtendedDOS/UNC versus ordinary samepath -> equal; wrongfilename/server/share -> reject. Native/API acceptance alone -> insufficient. Confirmed result -> close only originating current completed window; failure/timeout/cancel/stale -> retain/discard under original owner.
### 5. Good/Base/Bad Cases
Good: `\\?\C:\Downloads\file.bin` and Shell `C:\Downloads\file.bin` with exact selection confirm. Base: ordinary file paths unchanged. Bad: matcher requires directory selection but reader skips it; failure-only COM fixture falsely treated as successful observation coverage.
### 6. Tests Required
Owned native reader-success through actual HWND/QueryService/QueryActiveShellView/GetItemObject/Folder2 and synthetic selected path for both target kinds; preserve refcounts/HRESULT/hidden/wrongroot. Exact matcher DOS/UNC aliases and wrongname/server negative cases. Existing10reviewedfixture subset includes chooser cancel/nativePIDL/fallback/readback; actualGPUI buttonclick/result/currentwindow close and main-list isolation. These synthetic tests do not establish installed Explorer or networkshare acceptance.
### 7. Wrong vs Correct
Wrong: `if (isDirectory) return viewWithEmptySelection;`. Correct: always read actual selected items, then let the original exact-target matcher decide. Wrong: compare raw extended/ordinary path strings. Correct: explicit filesystem-alias normalization, never strip arbitrary namespace prefixes.

## Reveal through the observed replacement host (2026-10-24)
Evidence: `repo:csharp/NxTools.PlatformHost/Windows/DirectoryOpusTargetObservation.cs`, `repo:csharp/NxTools.PlatformHost.Tests/DirectoryOpusTargetObservationTests.cs`; [reference](../../../docs/reference-code/quick-launch-research.md). Windows Shell acceptance may open Directory Opus instead of Explorer. Shared NativeExplorerShellPlatform preserves Explorer-first observation, then an on-demand bounded DOpus observer; Rust Quick Launch/Downloads retain their original pending/result owners and no protocol changes. Exact parent plus selected target is mandatory for files and nonroot directories; drive roots confirm their own view.
Official DOpusRT `/info` `paths` and `listsel,<exacttab>` are read-only evidence, never `open/select/Go`. Resolve only dopusrt.exe sibling of the observed running same-session dopus.exe, not PATH. Require visible dopus.lister and visible same-PID child dopus.filedisplaycontainer, retained process starttime, XML active_tab=1/tab_state=1, exact filesystem path (not localized display_path), matching exported lister/tab and exact selected item. Re-export paths and native identity before accepting; inactive same-root/same-directory tabs cannot succeed. One command-local platform budget1500ms, each helper/output250ms; no observation retry worker or change to existing8attempts/75ms/onefallback. Bound XML2MiB,256tabs/4096selected and4matching candidates; prohibit DTD/external resolver. Unique fresh temporary result and cleanup, fixed diagnostic stage/nativecode only. Missing helper/bad XML/wrong selection/stale/hidden/foreign/budget failure stays unconfirmed; helper exit or lister appearance alone never authorizes success.
Three synthetic fixtures cover production shared Reveal red→green, wrong/inactive/stale/identity/path/selection and XML/selection budgets/diagnostic sink behavior. Separately authorized owned target passed actual product native entry and exact-selection positive/unselected-file negative on DOpus13.22. That is not broad third-party/mixed-DPI/release acceptance. Service-only Explorer COM candidates are independently guarded by actual IShellBrowser.GetWindow/root/view proof; initial QI probe alone did not establish the installed cause.

Export cleanup is an actual-lifetime owner, not part of observation retry. Process-wide capacity2
covers active exports plus pending cleanup; full admission returns unconfirmed without a helper,
directory or queue. Each owner retains only its exact GUID directory/result.xml and helper Process.
After helper timeout, the original <=25ms confirmation remains; unconfirmed actual exit keeps the
handle and slot. File.Delete or Directory.Delete failure retains ownership and retries those exact
paths every250ms asynchronously, with no prefix sweep or recursive deletion. Helper exit plus actual
directory removal returns the slot; deferred cleanup never changes the original result or serial
reader responsiveness. Fixed cleanup_failed/deferred/completed/busy stages use the original
correlation; no per-retry logging or paths/XML/child output. Cross-thread stage deduplication is locked.
`repo:csharp/NxTools.PlatformHost.Tests/DirectoryOpusExportTests.cs` covers real independent writers
after helper exit/timeout, file/directory sharing failures, bounded admission/recovery, actual XML
read-size limits and controlled termination with a still-live helper after writer exit. Owned
apphosts/files only; no live DOpus/Shell/GUI acceptance. Cleanup ownership is in-process, not restart-persistent recovery.

Chrome store addition uses original exporter/registrar owners: additiveformat chrome_store producesZIPmanifestcopywithoutkey, noPEMaccess,localCRX/XPIunchanged. Existing EnsureBrowserNativeMessaging command carries an optional configuredstoreID ascommandidentity; command/pinnedsource/existingexportedexactoriginunion preserveslocalcompatibility; no wildcard/newprotocol. Registrar remainsread-onlyforPEM; actualregistrytransactionunchanged. Existing7signingfixturecallgraphsre-reviewedwithstore ZIP tempfiles/lockedtempPEM only; no newTestEffects symbols/nativeeffects. Contract: [key-free upload and optional identity](../browser-extension/contracts.md#scenario-key-free-chrome-store-upload-and-optional-exact-identity).

## Hidden main-window startup preparation (2026-10-24)
`MainWindowPreparation` / `WindowsPlatformAdapter.MainWindow.cs` owns authenticated `prepare_main_window`: DIPwidth920..4096/height480..4096, hiddenpeerroot/samesession, detachedfixedboundedSTA5s. Primaryworkarea stagewithNOZORDER|NOACTIVATE, targetGetDpiForWindow48..960, scaleonce/roundAwayFromZero, liveouter/clientinsetcompensation, exactscreenclient/hidden/DPIreadback beforeData success. NoSHOW/activation/topmost/framepolicychange, noframeworkpatch. Pipepeer validatesPID independently ofDTO. Cancellationrechecks beforefinalapply/readback; nativeerrorcode retained; accepted/start/terminal fixedmetadata and onecorrelation, neverpayload. OriginalRustdispatcher deferspublication ofHWND/entrypoints untilACK; prepare isstartup-only, laterrestoremustnotresetusergeometry. TestEffects MainWindowStartupTests usesonlyownedSTATICHWND/fakenative/pureDTO/detachedsignals, no registry/tray/input/userwindow. Source/debugnativefixture acceptance isnotinstalledrebootacceptance.

## Extension settings path picker (2026-10-24)
`repo:csharp/NxTools.PlatformHost/Windows/ExtensionSettingPathPicker.cs` uses additive strict sourceHWND/sourcePID/kind/currentPath DTO. Original authenticated pipe peer must match sourcePID; native owner checks HWND/process before presentation and before accepting selection. Detached reader path, one bounded STA reservation retained until real modal exit, no human-wait deadline or replacement workers. Existing-file/folder only, successful empty dismissal, logical session cancellation drops late selection; no config/trust/write/execution in picker. Rust applies only matching current draft/request/package/input revision and final text, never saves automatically. Revision counts each official InputEvent::Change rather than coalesced observed values, so same-turn A-B-A user edits retire an older reply; silent set_value hydration emits no Change. Actual fakeIPC/GPUI regression and source acceptance limits are in the owning task. Diagnostics accepted/start/terminal fixedreason/elapsed/nativecode only, never paths/exception output. `repo:csharp/NxTools.PlatformHost.Tests/ExtensionSettingPathTests.cs` is an exact TestEffects fixture with injected owner/existence/presentation, not a real dialog or user-state acceptance.

## Scenario: Human picker ownership and machine execution

### 1. Scope / Trigger
Directory/program, clipboard archive/storage, extension manifest/runtime/settings-path, archive and browser-export destination selection. Use the existing detached dispatcher and bounded STA; state/draft ownership remains Rust. See [owned picker evidence](../../../docs/reference-code/archive-picker.md).

### 2. Signatures
Human command DTOs carry `source_window_handle: i64` and `source_process_id: u32` from the actual initiating GPUI Window. `pick_browser_extension_export_path { format, source_window_handle, source_process_id }` selects only a destination. `export_browser_extension { format, destination }` performs bounded machine work and requires nonempty destination. Existing archive picker version/vector shape remains unchanged.

### 3. Contracts
`PlatformPipeClient.ValidatePickerPeer` binds source PID to the authenticated Desktop pipe peer. `PickerOwner.Run` validates HWND/process before and after the native show. `BoundedStaOperationRunner` uses `Timeout.InfiniteTimeSpan` for human wait and retains admission through real modal exit and dispatcher cleanup. `StaDialogRunner.RunInline` closes only that STA's dialogs on cancellation or original-owner loss. Temporary Topmost suspension/restoration concerns only the same Desktop process; do not pin a chooser over unrelated applications. Router and feature owners impose no human elapsed deadline; machine operations keep bounded policies.

Browser export installs pending state before each send, uses distinct chooser/export command IDs and one logical correlation, and never starts export after source retirement or duplicate selection. A started exporter can complete after settings close. Directory and FFmpeg replies recheck original draft/input generation both before scheduling and at deferred application. Managed FFmpeg installation is not a manual selection origin. Closing settings retires a pending picker without retiring unrelated machine work.

### 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
| Normal Cancel | Successful empty selection; no settings write or export, no error toast |
| PID differs from authenticated peer | Reject before presentation; no selection |
| Missing/invalid/retired HWND | `picker_invalid_owner`, native1400; no selection |
| Cancel/disconnect during presentation | Matching native close request; late selection discarded, admission retained until cleanup |
| Picker cancelled | Native995; no successful late payload |
| Existing admission occupied | Existing busy result; no replacement STA/controller |
| Empty machine-export destination | Parser rejects; exporter never opens a second chooser |

### 5. Good/Base/Bad Cases
Good: choose after the old10second/two-minute/FFmpeg deadline, then apply once to the original unchanged draft. Base: Cancel leaves the draft untouched. Bad: use the worker-time foreground window, start another modal thread after a logical timeout, or broadcast an old FFmpeg choice into a reopened settings draft.

### 6. Tests Required
Protocol roundtrips assert every owner field and the selection-only export command. IPC controlled clocks assert all human commands remain pending while machine commands expire; disconnect/replaced-generation/duplicate replies cannot complete a replacement. `Opt07PickerTests` retains original parser/STA/owned HWND, Topmost restore, atomic-vector and real-reservation assertions. Extension manifest/runtime/path fixtures retain cancellation and detached-reader responsiveness. Desktop regressions assert edited/reopened draft isolation, quiet Cancel, two-stage export and one logical notification identity. Owned synthetic HWNDs do not certify live focus, physical keyboard or DPI.

### 7. Wrong vs Correct
Wrong: increase the global timeout or let `export_browser_extension` wait for a path. Correct: human destination selection has no elapsed deadline; only its nonempty result starts the existing bounded exporter. Wrong: an expired Rust request releases native admission. Correct: matching cancellation requests native close, and the original STA slot is released only after actual cleanup.

## Scenario: Archive association and classic-menu settings (2026-10-25)

### 1. Scope / Trigger
Rust owns persisted desired state/UI, C# owns fixed Windows Settings launch and the original native
integration planning/transaction. The 2026-10-26 explicit product enablement supersedes the historical
`ProductApplyAuthorized=false` constraint for source (`true` now); it grants no workstation execution.
Source and injected fixtures are not live Explorer/Registry/visual acceptance.

### 2. Signatures
`PlatformCommand::OpenDefaultAppsSettings(OpenDefaultAppsSettingsCommand {})` serializes exactly
`{"command":"open_default_apps_settings","data":{}}`. C#
`PlatformCommandParser.ParseOpenDefaultAppsSettings` -> detached sequence dispatcher/pipe ->
`WindowsPlatformAdapter.OpenDefaultAppsSettings` ->
`DefaultAppsSettingsOpener.Open(command, correlationId, cancellation)` uses an injected no-argument
`Action` or the fixed `ms-settings:defaultapps` Shell target; never a caller-supplied URI/path.
`ArchiveShellIntegrationCommand` adds `double_click_action`, `menu_layout`, `menu_commands` to
the existing version1/revision/action/extension contract; original Apply/Revert owns mutation.

### 3. Contracts
Double-click values: `open|smart|name`; layouts: `cascade|flat`; commands: `open`,
`extract_smart`, `extract_here`, `extract_name`, `extract_dialog`, `test`, `create_zip`, `create_7z`, `create` (unique, max9).
Missing fields and fresh defaults recommend open/cascade plus the original five commands and all
three creation commands; integration remains disabled until explicit Apply. Every explicit saved
array, including empty/custom/historical five commands, remains exact. The historical three-argument
plan overload retains five commands. Selected Create maps ordinary files/folders through `*`/`Directory`
and one complete grouped `--archive-compress` intent; ZIP/7z use fixed quick actions with the same owner.
`classic_cascade` retains its legacy menu-enabled meaning; layout is a separate field.
Smart/name ProgID commands use `--archive-extract smart-dialog|name-dialog "%1"`; the original
Desktop intent/form preselects a strategy but does not submit a job or approve risk/output budget.
Flat means separately owned classic HKCU verbs, not Windows11 top-level menu placement.
ZIP/7z classic verbs use independent LocalServer32 identities through `MUIVerb` and
`command\DelegateExecute`, with fixed labels `压缩为同名 .zip` / `压缩为同名 .7z` shared by
the settings labels. All commands retain IObjectWithSelection/IExecuteCommand grouping, complete
once-only IShellItemArray forwarding and original out-of-process identity/final Release lifetime.
Output uses the first ordered selected file stem or full folder name plus fixed extension;
the menu label is not a dynamic filename. Apply over an existing applied-facts ledger replaces it:
after identity/helper/plan gates pass, the original compare-before-restore Uninstall removes the
earlier revision (including old ExplorerCommandHandler values), the ledger is retired, then the new
plan is applied; the result carries the replaced removed/restored/preserved counts. A failed
replacement Uninstall reports `replace_revert_incomplete` and keeps the ledger. ExtractDialog
retains its stable identity.
`set_default` claims each selected extension's HKCU `Software\Classes\.<ext>` default value for
the owned `NxTools.Archive` ProgID (with its DefaultIcon/open command and OpenWithProgids), placed
before the OpenWithProgids value so reverse restoration empties children first. Explorer resolves
icons and double-click through `FileExts\.<ext>\UserChoice`, which outranks the class default; with
no choice and several candidates it shows "Pick an app" and a generic icon. Before saving the
facts, Apply records each extension's original choice (`PriorUserChoices`, never our own ProgID);
after they persist, `WindowsArchiveDefaultClaim` writes `ProgId` plus the Explorer proof computed by
pure `UserChoiceHash` from extension, user SID, ProgID, the key's last-write minute and the
shell32 experience string, refreshes once, and reads the live association back. Any extension
that does not resolve keeps the applied state and reports `failure=user_choice_locked`. Revert and
replacement write the recorded original back only where the choice still names NxTools; a choice
the user changed meanwhile is kept. Open With alone never claims the class default or a choice.
Quick argv correlation becomes the original native Job operation/session ID.
Default-app result success requires `success=true,data="opened"` (Shell accepted only). Rust keeps
original pending/correlation/timeout/disconnect and deferred publication; native diagnostics record
accepted/start/terminal, elapsed/native code, never request payloads. Saving/revision/request
acceptance is not applied state; only validated native facts may report applied, with revision.
Only the explicit Settings or [installed archive integration](installer-preparation.md#installed-archive-integration-phase-f-p206p207)
Apply/Revert boundaries mutate HKCU Software\Classes. Product composition always retains the
original approved Desktop resolver and `OpenArchiveRelease` helper lease; missing/null/unapproved
helper refuses without capture/write or grouped-to-per-file fallback. Original transaction owns all
pre-capture/attempt-before-write/reverse compensation/exact compare-before-restore. Detached reader
dispatch calls the original bounded STA runner (one active operation, 15s reply bound; actual cleanup
retains its slot after timeout). Pipe cancellation propagates before admission and mutation; Apply
observes cancellation between captures/writes and compensates attempted writes. Revert/replacement
first restore still-owned recorded UserChoices, durably recording attempts and confirming readback.
Incomplete restoration retains the ledger and prevents Classes cleanup/replacement; a later different
choice is preserved. Newer Windows choice keys are not deleted by the legacy writer. Only complete
restoration and exact Classes cleanup retire the ledger. Save/status/UI close is not Apply.

### 4. Validation & Error Matrix
Fixed-command null/array/extra field/URI/path -> strict parse failure, no native operation.
Unknown/null/duplicate/over-limit menu settings -> reject, never broaden/default silently.
OpenWith or selected archive verbs without extensions -> invalid intent; create-only menus need no
archive extension. Native cancellation/failure/invalid response/timeout/disconnect is not success.
Explicitly disabled injected policy -> `disabled/apply_not_authorized` before capture/write/helper lease.
Product's authorized policy -> identity verification then original transaction. Unknown identity ->
typed failure, zero Registry capture/write; bounded cancellation/timeout/occupied slot -> failed reply
(`cancelled` or `native_operation_unconfirmed`), never applied or an assertion that Windows is unchanged.
Existing transaction pre-capture, reverse compensation and compare-before-restore remain required;
no UserChoice writes except the explicit `set_default` claim and its recorded restoration, shared-tree deletion
or grouped-selection fallback to per-file `%1`.

### 5. Good / Base / Bad Cases
Good: smart+flat+[test,create] reaches the native plan; double-click smart opens the original
confirmation form with no started jobs. Base: missing fields recommend creation; explicit old arrays stay exact.
Bad: a default-app URI parameter, repeated command or forged applied result without revision.

### 6. Tests Required
Core save/reopen/default/invalid tests; Protocol empty-object-only fixed command and menu bounds;
IPC original router timeout; real production Settings controls/all/none/dropdown keyboard, minimum
Light/Dark reachability, failed save/changed-field merge; original request wire assertion and pending
terminal cleanup; original form route asserts strategy and zero jobs. Native exact fixture
`ArchiveAssociationSettingsTests` plus original Phase F plan/service fixtures assert selected-only
flat/cascade values, whole Create selection, strict parse, compensation and zero disabled-policy effects.
`ArchiveIntegrationServiceTests` additionally exercises real product composition with injected effects:
explicit Apply/Revert, desired/observed revisions, current/wrong/empty Desktop identities and retained
helper approval, responsive original dispatcher, STA admission, cancellation/timeout/actual cleanup,
one-slot retention and payload-free correlated logs. No native store/default ledger is used in fixtures.
Evidence/outcomes: `repo:.trellis/tasks/10-16-archive-manager/shell-settings-outcomes.md` (historical),
`repo:.trellis/tasks/10-16-archive-manager/integration-enable-outcomes.md` (current source enablement).

### 7. Wrong vs Correct
Wrong: relax OpenShellTarget to arbitrary URI, mark saved settings applied, or auto-submit extraction.
Correct: fixed empty-data command, original desired/observed owner, and confirming launch modes.
Wrong: classic layout change bypasses grouped helper or deletes shared registry roots.
Correct: public menu layout only changes owned plan values; identity lease and transaction stay owners.

## Extension v2 data outer budgets and backpressure (2026-10-26)
Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Extensions/ExtensionProcessProtocol.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Extensions/ExtensionProcessRunner.cs`, `repo:csharp/NxTools.PlatformHost/ExtensionHost/ExtensionHostBroker.cs`. Original authenticated demandStart optional data_plane=2 opts into finite combined25MiBstdin/28MiBstdout; omittedlegacy1MiB/4MiB,256KiBframe/stderr64KiB/token/Job-before-resume/global4+perpackage1/actualcleanup unchanged. Rust alone classifies exactcontrol/data/session semantics. Native v2stdout awaits originalbounded32slot outgoingchannel with2seconddeadline; controls and nativecleanup remain independent, no second receiver/unboundedbuffer. Rust reader has finite250msbackpressure, cancels only exactslowrun and retainscleanupslots. Real256progressburst reproduced NativeUnavailable beforeboundedpumpfix; lateractualscriptregression passes. Native fixture10group covers explicitv2input and28MiBoutputceiling pluslegacy/UTF8/Job/cancel/parentdeath. DrainAsync proves physicalslotdrain,not diagnosticcallbackcompletion: tests must await actualexited/terminaldiagnostic counts beforeprivacyassertions. Fixtures own currenttestapphost/temporaryfiles only,not ordinaryproduct/nativeUI/installedacceptance.

## Scenario: Keyboard hook ownership and observable recovery
### 1. Scope / Trigger
Double-Ctrl installation, configuration/recording changes, explicit recovery and shutdown.
### 2. Signatures
`LowLevelKeyboardMonitor.TryInstall(Guid correlationId) -> bool`; `Dispose()` requests WM_QUIT and joins up to2seconds.
`QuickLaunchKeyboardConfiguration(bool Enabled, bool Recording, QuickLaunchTriggerKind Trigger, long ResetGeneration)`
is published by EventWindow through Volatile. `QuickLaunchKeyboardHealthChangedEvent(Guid CorrelationId,
QuickLaunchKeyboardHealthState State, int NativeErrorCode)` maps to the matching Rust typed platform event.
### 3. Contracts
One background thread creates its message queue, installs WH_KEYBOARD_LL, blocks in GetMessage and unhooks in finally.
The delegate stays rooted for that lifetime. No replacement is started while a prior owned thread remains alive.
Only that thread mutates left/right tap detectors; each down/up callback consumes one immutable snapshot and resets
on generation change. Injected flags0x10/0x02 remain filtered; callbacks do no logging, IO, foreground capture or lock waits.
They post the existing gesture to a captured HWND; original EventWindow foreground/fullscreen/IPC ownership remains.
ConfigureQuickLaunch double Ctrl calls TryInstall before rejecting an unavailable monitor, preserving its command correlation.
No polling or heartbeat: state is install_failed/thread_exited/reinstalled, not proof of Windows silent-removal detection.
Outer protocol1 remains. Hello advertises quick_launch_keyboard_health_v1; only a HelloAck accepting it enables new health
event forwarding, because unknown event variants fail Rust decoding. The existing quick_launch_panel_hotkey Hello capability
is current availability on reconnect. Install failure/native thread exit clears IsInstalled; healthy retry is idempotent.
After HelloAck, discard old queued events before reading the monitor's immutable current health snapshot.
Send that snapshot only to an accepted health peer before normal forwarding; pass the same fact as the
reader's last-sent state. A transition between Hello's availability sample and the acknowledgement must
not disappear with the discarded queue. `ReadEventsWithKeyboardHealthSnapshots` samples retained health
before each queued event and emits only a changed fact; queued health records wake the reader but never
override the latest snapshot. Queue saturation therefore recovers within the current session when reading
resumes, without blocking the native publisher, polling or another queue. Ordinary events retain their order.
The same correlated lifecycle fact may be replayed on reconnect without a new native operation.
### 4. Validation & Error Matrix
Native installation failure -> unavailable/typed install_failed/native code. GetMessage failure or unexpected loop return ->
unhook/unavailable/thread_exited. Explicit retry success -> reinstalled with retry correlation/code0. Recording/custom hotkey/
disabled snapshot -> no double-Ctrl gesture. Reset after armed down but before up -> no stale summon. Intentional Dispose ->
same-thread unhook/no unexpected-exit event; a join budget does not claim forced termination of a stuck native call.
### 5. Good/Base/Bad Cases
Good: clipboard STA blocks while the hook loop still recognizes taps. Base: both released taps are processed by one owner.
Bad: move only the native callback while leaving detectors/config fields mutable across threads, or send an unnegotiated event.
### 6. Tests Required
`KeyboardMonitorTests` uses injected native queue/delegates, not global hooks or SendInput: different installation thread,
2second caller stall, same-thread loop/callback/unhook, actual bounded joined exit; reset-generation/recording/injection/
left-right controls; native failure/retry/unexpected-exit/correlation/no-key-log; paired wire and accepted-capability gating.
`HealthSnapshotSurvivesFullQueue` uses the production reader with512queued events and a rejected health
TryWrite, then asserts current-session recovery, ordinary ordering, stale-health suppression, handshake
deduplication and capability rejection. `ObservableFailureRetryAndThreadExit` also preserves the post-ack
snapshot's original correlation/native code after the event queue was consumed.
Keep original DoubleControlGesture and QuickLaunchContracts/diagnostic/fullscreen/paste regressions. Physical keyboard,
real clipboard delayed rendering, modifier release and user acceptance remain separate native evidence.
### 7. Wrong vs Correct
Wrong: EventWindow resets a detector while the hook callback mutates it. Correct: publish a new reset generation and let
the next hook callback reset both detectors. Wrong: nonzero retained HHOOK means Windows still delivers input.
Correct: report only observed native installation and owned-thread lifecycle.

## Detached process and service command lifecycle
`launch_process` and `ensure_indexer_service` use the original detached dispatcher/tracked-task drain.
Launch disposes the Process handle without terminating the application. The existing named indexer installation gate
serializes concurrent installers; Rust installs its bootstrap command identity before dispatch and matches the result.
Drain observes the tracked work within its existing2second budget; neither drain nor cancellation forcibly ends UAC/native work.
`DetachedCommandsKeepReaderResponsive` holds synthetic delegates for picker/probe/process/service, verifies a later
visibility command runs, then verifies drain remains pending until actual completion. No live launch/service effects.

## Scenario: Unlimited broker control lifetime with finite run budgets
### 1. Scope / Trigger
Changes to ExtensionLineReader, broker control construction or per-run stdin/stdout readers.
### 2. Signatures
`ExtensionHostBroker.CreateControlReader(Stream)` selects `ExtensionLineReader.Unlimited(stream,"control")`. `ExtensionLineReader(Stream,int totalLimit,string prefix,Action<int>? observeBytes=null)` remains the finite caller API.
### 3. Contracts
Unlimited is explicit nullable-budget state, not int.MaxValue or another sentinel. It never accumulates a lifetime byte counter. Fixed8KiBchunk/256KiBline buffers, strict UTF8, CRLF trimming and EOF-without-LF failure remain. Finite readers check `end > limit-finiteTotal` before accumulation and retain byte observation. ExtensionProcessRunner stays finite: legacy1MiBstdin/4MiBstdout, data_plane2 combined25MiB/28MiB, stderr64KiB. Protocol1, authenticated parent/Jobs/admission/cleanup unchanged.
### 4. Validation & Error Matrix
Control lifetime>2^31 valid bytes -> continue. Finite total exceeded -> prefix_total_limit. Current line>256KiB -> prefix_line_limit. InvalidUTF8 -> prefix_invalid_utf8. EOF with partial line -> prefix_truncated_line. No payload in diagnostics.
### 5. Good/Base/Bad Cases
Good:8193stream-generated262144byteframes read with bounded memory. Base: ordinary finite child output still rejects its run budget. Bad: int.MaxValue advertised as unlimited, or remove line/run limits with the lifetime limit.
### 6. Tests Required
`ExtensionProcessBrokerTests.ControlReaderStreamsBeyondInt32WithoutLifetimeBudget` uses the actual broker factory and generates2147745792bytes, retaining one frame template/current decoded frame only. `FiniteReaderBudgetsAndFrameErrors` asserts finite total/exactline/overline/UTF8/truncated failures; retain parent identity/malformed control fixture. Build then select these exact reviewed fixture names, not broad native tests.
### 7. Wrong vs Correct
Wrong: `new ExtensionLineReader(input,int.MaxValue,"control")`. Correct: named Unlimited for broker lifetime; original finite constructor for every per-run caller.

## Scenario: Virtual Root descriptor name units

### 1. Scope / Trigger
Root names entering `ArchiveVirtualDragWire.Parse` from the actual Rust envelope. Owners: `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveVirtualDragContract.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualFileDataObject.cs`.
### 2. Signatures
`ArchiveVirtualPlan.Admit(items)` retains descriptor names/original ordinals; `IDataObject.GetData(CFSTR_FILEDESCRIPTORW)` emits the SDK descriptor HGLOBAL.
### 3. Contracts
Windows component<=255 UTF-16 code units; complete relative descriptor path<260 UTF-16 units, reserving the terminating WCHAR. Full serialized envelope retains its separate1MiB UTF-8 transport limit. No255-byte UTF-8 component veto. Noncolliding valid names remain verbatim; wrapper prefix is descriptor-only and does not change original member ordinals/cache paths.
### 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
|86 ordinary CJK units /258 UTF-8 bytes, safe short child|Admit unshortened wrapper and emit exact UTF-16 names|
|256 component units, full path>=260 units, Windows-invalid name or envelope overflow|Original invalid-request refusal before native descriptor publication|
### 5. Good/Base/Bad Cases
Good:86-CJK Root plus short child passes both languages. Base:ASCII Root follows the same limits. Bad:rejecting a legal Windows component solely because its UTF-8 encoding exceeds255 bytes.
### 6. Tests Required
Rust real serialized command fixture -> C# Parse/Admit -> native HGLOBAL; assert count, all names/NUL/attributes and original ordinals. `repo:csharp/NxTools.PlatformHost.Tests/ArchiveVirtualTreeTests.cs` also covers85/86/255 accepted and256 refused component units.
### 7. Wrong vs Correct
Wrong:component UTF-8 bytes<=255. Correct:component UTF-16 units<=255, complete descriptor path<260, separately bounded serialized transport. Progress identity and cache/receiver semantics follow [the original demand contract](../nxtools-archive/backend/product-read.md#scenario-demand-bound-drag-byte-progress).

## Scenario: Demand index registration and one-shot trusted update
### 1. Scope / Trigger
User2026-11-06 rejects index/update processes remaining after Desktop Exit. This supersedes
automatic/resident registration only, not signed complete-release, native ownership or UAC contracts.
### 2. Signatures
`IndexerServiceInstaller.RegistrationArguments` emits Demand for create/config. `IDeploymentPlatform`
adds NeedsLifetimeMigration/ConfigureLifetimeAsync; the exact current tuple alone may enter
`--configure-indexer-lifetime <GUID>` through its explicit authorized helper.
`TrustedUpdateBroker.RequestAsync/StartIfStopped/ServeAsync` retain original validated service,
peer and bounded frame owners; BrokerService owns actual normal Stop after one operation.
### 3. Contracts
Normal verified startup starts the Demand indexer. A same-release Auto registration requires one
configuration-only authorization, not binary redeployment; successful configuration then starts
the service. Cancellation/failure/unhealthy tuple never claims running or silently replaces files.
Updater enrollment registers Demand without starting it. A real request verifies enrollment and
signed package before service start; a cancelled request does not wake it. Running/StartPending
reuse the existing start, never duplicate a service invocation. Serve owns30s first-connect/10s
read bounds and one accepted request/probe/rejection/EOF, then returns instead of reopening a pipe.
An accepted transaction uses its original independent settle/recover/rollback boundary despite
client exit or SCM Stop; OnStop waits real completion with SCM wait hints, not self-wait deadlock.
Normal stop does not Environment.Exit(1) or trigger failure restart. Explicit same-key bootstrap
maintenance configures Demand and verifies one probe plus actual Stopped. Old Auto registrations
require that original approved maintenance path even with a valid historical version marker;
ordinary requests cannot overwrite protected bootstrap, rotate keys or reset the ledger.
### 4. Validation & Error Matrix
No update -> no start. Handshake timeout/EOF/malformed/terminal -> release listener and stop.
Prepared work -> settle first. Wrong peer/signature/registration -> original refusal, no fallback
UAC or arbitrary service. Configure declined -> actionable cancellation and no service start.
### 5. Good/Base/Bad Cases
Good: stopped approved service starts only for an update, returns result and stops. Base: later
normal index launches need no repeated authorization. Bad: demand registration with an infinite
Serve loop, stop only the desktop helper, or treat old marker as proof of one-shot execution.
### 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/TrustedUpdateBrokerLifecycleTests.cs` uses actual owned
pipes/injected settlement and original demand-start decision. Preserve original signature,
rollback/enrollment/maintenance and IndexerUpgradeDecision fixtures. Installer bootstrap-migration
gate covers approved Auto -> required, compatible Demand -> skip, identity refusal and x86/x64.
Source/owned fixtures do not establish installed SCM or packaged upgrade/exit acceptance.
### 7. Wrong vs Correct
Wrong: updater waits forever just because it is a service. Correct: durable registration and trust
remain, but listener/process are bounded to the original actual update/maintenance operation.
Evidence: `repo:csharp/NxTools.PlatformHost/Windows/IndexerServiceInstaller.Lifetime.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/TrustedUpdateBroker.cs`,
`repo:installer/bootstrap-migration.ps1`.

## Clipboard capture staging ownership

Evidence: `repo:csharp/NxTools.PlatformHost/Windows/ClipboardCaptureFiles.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/ClipboardStagingDirectory.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ClipboardStagingTests.cs`.

`ReadClipboardContent` owns each newly allocated path before the first write. Image promotion
cleans partial output and borrows its source. The publication tail takes the capture owner;
stop/pause, tail admission failure, partial format writes and publication failure dispose only
capture-owned files. Successful event publication transfers exactly its referenced files.
Source icons are copied from the shared IconCache into the capture run; neither producer nor
Rust consumer may delete the borrowed cache entry. Direct-paste fallback is a separate owner.

Each Host writes into a GUID run below `%TEMP%/NxTools.Next/clipboard-staging`. Its owner file
holds a non-delete-sharing Host lease and the Desktop PID plus process-start time supplied by
the existing `--parent-pid` startup argument. Closing the Host lease never deletes nonempty runs:
pipe write completion does not mean the Rust worker/deferred capture has finished. Startup
recovery requires an exclusive owner-file claim and confirmed retirement of both recorded
process identities. One pass examines at most64run directories and256files, deletes only owned
GUID PNG/BIN files, and retains locked files for a later pass. Unknown identities, unknown files,
live runs and legacy flat staging are retained; no age-based or whole-Temp cleanup.

Recovery start/completion share one correlation and expose only counts/elapsed time; cleanup
failure diagnostics never include paths or payload and cannot change capture results.
`ClipboardContentOperations` substitutes native reads/file writers only for isolated fixtures;
the production capture order, real publication tail and filesystem remain under test. Preserve
existing Rust policy rejection/serialized persistence cleanup and the unchanged wire contract.
These fixtures do not authorize live clipboard/listener/global input, product launch or packaging.
Publication tails capture the pause owner's epoch. Closing that gate retires the epoch, so
pause followed by resume cannot revive pre-pause queued work. The same production-tail fixture
covers pause, pause/resume, stop and count-budget rejection with real tiny temporary files.

## Clipboard image capture budgets

Evidence: `repo:csharp/NxTools.PlatformHost/Windows/ClipboardImageEncoding.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/ClipboardImagePromotion.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/ClipboardImageCompatibility.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ClipboardImageBudgetTests.cs`.

Keep the existing ClipboardCaptureLimits settings/defaults/clamps. Pixels and encoded PNG bytes
are independent budgets; compressed input length cannot substitute for either. Native capture
checks dimensions immediately after the Image API returns, before creating/encoding a PNG.
File promotion probes dimensions with eager GDI+ validation disabled, disposes that probe, then
performs the original full validation only for admitted dimensions. Known budget rejection must
not materialize another native bitmap or repeat text-path image promotion. Preserve independent
file/text representations, source files and existing capture-owned partial-output cleanup.

All production PNG writes use SavePng with the current encoded-byte setting. Its private output
stream rejects Write/Seek/Position/SetLength beyond that budget; a rejected codec write stays
rejected even if GDI+ wraps or consumes its exception. No scaling, truncation, new setting,
dependency or format parser. Successful captures preserve their original dimensions/pixels.
Canonical DIB conversion checks its existing32MiB pixel-buffer budget from the dimension probe
before full validation and bitmap/DIB allocation; publish both arrays only on success.

Limits: the initial Windows Image call, GDI+ dimension probe, full validation and codec buffers
can allocate internally. The32MiB canonical limit bounds one pixel buffer, not aggregate process
commit. These controls are not a process-wide decoder memory cap. Isolated fixtures use at most
1001x1000 bitmaps and640x640 PNG noise to cover early rejection, actual output caps, exact allowed
boundaries, lossless roundtrip, malformed input and conversion failure without live clipboard IO.

## Clipboard publication-tail byte admission

Evidence: `repo:csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs`,
`repo:csharp/NxTools.PlatformHost.Tests/ClipboardPublicationBudgetTests.cs`.

Retain the original64-entry serial ContinueWith tail. A single short gate admits count and
payload bytes atomically; no IO/native callbacks run under that gate. Charge native array
LongLength plus input text/path/format/source strings at UTF-16 width. Shared references may
be conservatively charged more than once; declared sizes and staged image file lengths never
substitute for resident input data. The normal budget is32MiB. One larger legal capture may
enter only when no tail work remains and excludes every subsequent capture until retirement.
Thus charged payload is at most32MiB, or exactly one oversized capture's charge, never64copies.

Count/byte rejection preserves owned-file cleanup and the existing throttled drop event.
Count-full remains publication_tail_full; byte-full is publication_byte_budget_full with only
numeric payload/retained/limit fields. Failed logging cannot affect rejection/cleanup. Keep
accepted order, pause epochs and full/closed event-queue cleanup. Cancel/stop must not release
the bytes of a blocked callback until it actually returns. Completion/failure clears the
continuation's input references and disposes its file owner before returning count/bytes.
Scheduling failure also unwinds reservation and ownership.

StageNativeFormats writes raw arrays to run-owned files; ClipboardCapturedEvent and the original
PlatformPipeClient path carry only references and metadata. No protocol/pipe capacity change;
serialization/write/session failure still uses the original exact-file cleanup. The quota ends
at tail retirement, not the Rust persistence acknowledgement. Initial capture, object headers,
native/IO buffers, downstream metadata and CLR collection/working-set timing are outside this
charged-payload bound. No forced GC, working-set trimming, new queue or replacement worker.
Scaled isolated fixtures block the actual consumer and cover bytes/text, order/reuse, exclusive
oversize, throwing logs, copy/write failure, pause/resume/stop and downstream retirement.

## OneCommander focus-anchor lifetime

### 1. Scope / Trigger
OneCommander UIA access and directory lifetimes in ordinary PlatformHost and elevated helper.
Evidence: `repo:csharp/NxTools.PlatformHost/Windows/NativeUiAutomation.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/OneCommanderUiaSession.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/OneCommanderFocusTracker.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/ThirdPartyFileManagerAdapter.cs`,
`repo:csharp/NxTools.PlatformHost/Windows/ElevatedDirectoryNavigation.cs`.

### 2. Signatures
`ThirdPartyFileManagerAdapter.BeginLifetime(IPlatformLog,string) -> IDisposable` creates no UIA
client/thread/subscription. ObserveForeground/RetireWindow reuse EventWindow; Stop closes the scope.
`OneCommanderFocusTracker<T>.Invoke<TResult>(Func<TResult>,CancellationToken)` admits one physical
native operation. `OneCommanderPaneState : IDisposable` is an owner/identity/window/PID/expiry lease,
not an AutomationElement or a pointer created on a retired STA. Helper Run accepts the original log.

### 3. Contracts
Use native CUIAutomation8/IUIAutomation for events and all affected routine queries/patterns/tree
access. One existing MTA owns COM initialization, interfaces, cache and request panes. Existing named
120ms capture/1500ms navigation STA admission survives logical cancellation until native return and
Dispatcher cleanup. One pending operation and one coalesced focus element; no replacement worker.
Native AddFocusChangedEventHandler is global, never a per-process API. Enable only for actual target
demand; cached PID/HWND filter before traversal, HWND0 WPF anchors remain valid. Ordinary background
capture/observation requires enabled directory features. Explicit helper capture freezes before the
overlay on the original authenticated channel. Frozen panes need no global focus subscription.
Keep64focus anchors/64native request panes and original10minute session expiry. Release on replace,
consume/fatal/admission failure/expiry/capacity/window/disconnect/stop. Existing60second native sweep
retires unavailable windows; no UIA idle traversal. Registry removal only signals native lease release;
COM release runs on its MTA. Stop rejects late work before exact unregister/callback drain/reference
release/COM teardown/thread exit. Actual physical retirement, not500ms wait or ACK, releases ownership.
Diagnostics onecommander.uia.lifecycle use correlation/role/fixed state/reason/pane-window count,
elapsed and native code only; no focus text/path or per-event stream.

### 4. Validation & Error Matrix
No target/no demand -> no UIA initialization or desktop-wide subscription. Unavailable pane or
unsupported ValuePattern -> bounded original failure. Cancel/timeout -> original caller result,
physical slot retained. Native registration/removal failure -> HRESULT diagnostic and owner teardown,
never successful re-registration over a failed removal. Late callbacks/old generation -> no mutation.

### 5. Good/Base/Bad Cases
Good: captured HWND-less pane survives cache replacement, then its lease retires on session consume.
Base: no OneCommander leaves both startup roles dormant. Bad: ordinary lookup calls managed UIA,
or timeout frees admission while a native call is still executing.

### 6. Tests Required
User-owned verification covers actual registration/removal/callback drain, no-target module state,
split/active/WPF pane reads/writes, timeout slot exclusion, HWND recreation, native leases and actual
handle/thread/process retirement. Existing synthetic tracker/process/STA cases remain policy-only;
they do not establish native memory or physical elevation acceptance. No checks were run for this
implementation at the user's request; memory behavior and full allocation attribution remain unverified.

### 7. Wrong vs Correct
Wrong: startup WarmUp creates a permanent managed global focus path and dictionary removal drops
only a wrapper. Correct: dormant lifetime, native target-demand owner and explicit lease retirement
with unregister/drain/release before owner exit.

## Native temporary-allocation ownership

Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs#OutputNative.Open`,
`repo:csharp/NxTools.PlatformHost/Windows/Extensions/ExtensionNative.cs#Spawn`,
`repo:csharp/NxTools.PlatformHost.Tests/NativeAllocationTests.cs`.

Initialize both temporary pointers to zero and enter the existing try before either native
allocation. Assign each successful allocation directly to its owned pointer; finally frees
only nonzero pointers, including when the next allocation or initialization fails. Open owns
its UTF-16 name/UNICODE_STRING through NtCreateFile and inspection; the returned SafeFileHandle
remains caller-owned. Spawn owns list/handle-value storage through CreateProcessW; delete the
attribute list only after successful initialization, before freeing its storage. Keep input
stdio borrowed, returned process/thread caller-owned, and original native flags/errors unchanged.

Per-call allocation/free delegates provide failure injection at those actual method boundaries;
production defaults remain Marshal.AllocHGlobal/FreeHGlobal, with no global allocator or new
resource framework. Isolated fixtures inject first/second allocation exceptions without system
memory pressure, track real tiny native buffers by pointer and require exactly-once cleanup and
the same exception instance. Later native failure/success fixtures preserve fixed native codes,
borrowed pipe usability, returned file-handle usability and post-create exception/handle cleanup.
Existing owned echo/suspended-admission fixture covers successful process and Job handoff.

## Scenario: Invocation-owned temporary .NET build trees
### 1. Scope / Trigger
Build, Release fixture verification and packaging complete or fail after creating temporary .NET trees.
### 2. Signatures
`scripts/build.ps1` owns `vh-*`; `scripts/test-platformhost-fixtures.ps1` owns only its new `vf-*`,
not its caller's HostBuildRoot; `scripts/package.ps1` owns `package-build-*`.
`publish-release-hosts.ps1 -GeneratedDirectory ... -BuildDirectory ...` borrows caller-owned outputs.
### 3. Contracts
Claim the exact unique path before any creation and remove that tree in the creating wrapper's outermost
finally, for PI_SCRATCH_DIR or Windows Temp. Package ownership begins before generated-directory creation
and sequence/trust setup and lasts through installer/distribution stages. No wildcard or whole-Temp sweep.
Borrowed publisher outputs survive until test builds/copies consume them. Keep published package files,
logs and explicit evidence outside transient ownership; retain original private-key/sequence/service policy.
Default fixture evidence lives in `.diagnostics/verification/fixtures-<unique-id>/`; custom roots remain
effective and all caller fixture environment values are restored even on failure.
Staging copies only DesktopIdentity, ReleaseBinaryManifest, ArchiveReleaseManifest and UpdateTrust public
C# inputs to `<output>.generated-inputs/` and the current managed Host assembly to
`<output>.TEST-PlatformHost.dll`. `.build-inputs.json` references retained generated_directory and
host_assembly, never a deleted build_directory. `ArchiveReleaseTests.StagedActualIdentities` loads
that retained assembly for its unchanged reflection/runner/approval/signature checks; its explicit
staging root uses the same scratch-or-Temp selection as packaging. TEST public trust remains beside
output, private PEM is never copied, and evidence sidecars are not production installation members.
### 4. Validation & Error Matrix
Partial directory/create, sequence/trust/key/preflight, verification, publish/test, copy/source/sign,
installer compiler or final distribution failure -> original failure, exact owned tree removed.
Success -> owned tree removed,
necessary public artifacts/evidence retained. Foreign same-prefix directories and caller inputs remain intact.
### 5. Good / Base / Bad Cases
Good: finally encloses allocation through the final consumer. Base: caller-specified evidence roots persist.
Bad: clean publisher output before its caller consumes it, retain dangling staging references, sweep Temp,
delete borrowed roots, or mistake a synthetic fixture key for production publisher material.
### 6. Tests Required
`tests/architecture/build-tree-cleanup.tests.py` runs whole unchanged script copies with external tools
stubbed in self-owned roots, both scratch/Temp, success/throw/native nonzero, early setup/late installer
and final distribution failure, native-shell/ISCC/distribution live-tree assertions, nested verification,
borrowed lifetime, retained managed/public-input references and matching consumer source bindings. Retain
`build-host-bindings.tests.py`, `staging-key-cleanup.ps1` and release-chain/memory-policy source gates.
No Cargo, .NET build, real signing/reservation, product/service/installer operation is authorized by these tests.
### 7. Wrong vs Correct
Wrong: mkdir temporary; reserve; try publish finally restore location. Correct: own exact temporary path;
try create/reserve/publish/consume finally remove exact tree, while durable outputs keep independent ownership.

## Scenario: Resolution-aware Shell icons
### 1. Scope / Trigger
File/type icon pixels for the existing download40DIP image slot and other original Shell-icon consumers. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/ShellIconExtractor.cs#ExtractHighResolutionShellIcon`, `repo:csharp/NxTools.PlatformHost/Windows/IconCache.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveTypeIconTests.cs`.
### 2. Signatures
Existing `ExtractShellIconCommand(Target, Size)` accepts16..256; `ExtractOnSta` owns5-second logical wait/max8 real operations; `Extract(..., cancellationToken, IconCache?)` returns the original success/native-error/PNG-path result. No command/schema or GPUI layout change.
### 3. Contracts
For ordinary Shell selectors with Size>32, read SHGFI_SYSICONINDEX then obtain SHIL_EXTRALARGE=2 for Size<=48 or SHIL_JUMBO=4 for larger requests on the original STA. If a jumbo canvas contains at most48x48 nontransparent artwork, fetch the48px list instead and release the superseded icon. Preserve SHGFI_USEFILEATTRIBUTES for reserved type targets, including a fixed generic file target for extensionless files. Each query owns and releases its exact IImageList native pointer; never share/FinalRelease a managed RCW across concurrent STAs. Destroy every acquired HICON. Cache native raster pixels under `/image-list-v2`, isolating old undersized entries. Existing small/indexed-resource paths remain. IconCache retains128memory paths/2048disk entries, per-key dedup/trim and disk PNG validation; both memory/disk hits refresh write time for recency. No global clear or new file hashes.
Before the first high-resolution query, one Lazy initialization on the original STA primes the Shell index/image list using a fixed attribute-only generic file. Concurrent callers share only that initialization, never query pointers or HICONs. The max-eight admission, five-second logical wait and actual thread-final cleanup remain unchanged; the parallel fixture starts cold without test prewarming.
### 4. Validation & Error Matrix
Invalid size/reserved target -> original rejection before native query. High-resolution lookup unavailable -> existing Shell-icon fallback. PNG/native failure -> failed result and no successful cache entry. ExtractOnSta preserves the original Win32 code/HRESULT with a fixed `shell_icon_*` failure category; diagnostics never contain target paths or raw exception messages. Cancellation -> original actual-operation/STA lifetime and no late successful publication. Native fallback does not guarantee high-resolution artwork when the Windows association itself supplies none.
### 5. Good/Base/Bad Cases
Good:48px file/type/folder requests use the48px list; a256px canvas carrying40px artwork falls back to the48px list even for larger requests. Base:32px type query and cancellation remain unchanged. Bad: shrink a mostly transparent256px canvas to40DIP, or serve it from the old cache key.
### 6. Tests Required
`ArchiveTypeIconTests.SmallArtworkOnJumboCanvasUsesExtraLarge` exercises real bitmap content bounds and the production image-list selector/handle release; `CacheHitRefreshesDiskRecency` covers memory/disk hits. `FolderPngUsesOwnedCacheAndOriginalSta`, `ParallelPngExtractionKeepsNativeImageListReferences`, `CorruptPngRegeneratesThroughOriginalSta`, `NativeFailureRetainsSafeCategoryAndCode` and `CompletedExecutableUsesOwnedCache` cover native raster/cache recovery, concurrent references, errors and executable files. Desktop icon regressions cover path-to-type and extensionless generic fallback, STA-busy retries without consuming the failure budget and real failed-image File fallback. Preserve fixed-target rejection, original bounded STA and IconCache lifecycle fixtures. No production cache, Shell open, shortcut COM, registry, service or external file mutation. Installed/mixed-DPI/user visual acceptance is separate.
### 7. Wrong vs Correct
Wrong: choose only SHGFI_LARGEICON for every Size>20 and enlarge its fixed raster in the list. Correct: resolve a high-resolution system image-list icon, retain its native pixels through PNG/cache and let the existing Contain renderer scale down.

## Scenario: Typed completion Sleep and Hibernate
### 1. Scope / Trigger
Completion coordinator requests Windows power transitions. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/CompletionSystemActions.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`.
### 2. Signatures
`suspend_system { hibernate:bool }`; SuspendSystemCommand(bool Hibernate); ICompletionSystemActions.RequestSuspend(bool) -> WindowsCommandResult. Rust PlatformCommand::SuspendSystem carries the same boolean. Existing ShutdownSystem remains applicable.
### 3. Contracts
Original parser→generic dispatch→WindowsPlatformAdapter→ICompletionSystemActions owns the native boundary. Sleep checks IsPwrSuspendAllowed, Hibernate checks IsPwrHibernateAllowed; unsupported returns50 without substitution. Enable only SeShutdownPrivilege on the original token, preserve actual open/lookup/adjust errors, call SetSuspendState(hibernate,false,false), restore prior privilege state and close the token. Original detached sequence dispatcher keeps shutdown/suspend off the serial reader; drain waits for actual delegates. Correlated action/start/terminal/elapsed/native-code diagnostics contain no payload. Windows acceptance is not a claim that a physical transition completed.
### 4. Validation & Error Matrix
Unsupported -> false/code50. Access denied or privilege not held -> false/exact5 or1314. SetSuspendState failure -> false/exact native code. Successful injected request -> typed accepted result only. Failure never substitutes another power action or force-critical/wake policy.
### 5. Good/Base/Bad Cases
Good: Sleep/Hibernate reach distinct typed native calls and preserve exact result codes. Base: existing sound/Shutdown routing. Bad: unsupported Sleep schedules Shutdown, reports success before native result, or blocks the serial command reader.
### 6. Tests Required
Effects-reviewed exact B5 completion fixtures must LIST nonzero then RUN identical filter. Inject at ICompletionSystemActions through production generic parser/routing/result envelopes; cover Sleep/Hibernate,50/5/1314/87, diagnostic correlation/action/terminal and held operation with later-command responsiveness plus actual dispatcher drain. Ordinary fixtures never call capability/token/power APIs; physical Windows acceptance is separate.
### 7. Wrong vs Correct
Wrong: DTO roundtrip or mocked success certifies machine suspension. Correct: typed production route/result lifecycle with injected OS boundary, and separate physical acceptance.

## Scenario: Elevated foreground input
### 1. Scope / Trigger
Quick Launch/Clipboard activation and administrator-target paste/outside-click dismissal. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/ElevatedInputHost.cs`, `repo:csharp/NxTools.PlatformHost/Windows/ElevatedInputHost.Pointer.cs`, `repo:csharp/NxTools.PlatformHost/Windows/ElevatedInputHost.Activation.cs`, `repo:csharp/NxTools.PlatformHost/Windows/ElevatedDirectoryNavigationClient.Input.cs`, `repo:csharp/NxTools.PlatformHost/Windows/WindowsPasteCoordinator.cs`; indexed references `repo:docs/reference-code/quick-launch-research.md` and `repo:docs/reference-code/clipboard.md`.
### 2. Signatures
The existing authenticated same-session helper private channel owns configure_input(ElevatedInputConfiguration), paste_input(ElevatedPasteInput) and activate_window(ElevatedWindowActivation). Configuration carries ordinary destination HWND, keyboard generation/correlation and DesktopProcessId bound from the connected Rust pipe's actual server PID. Paste carries only target/focus HWND and correlation; activation carries only target root HWND and correlation. Public Rust wire/executable roles remain unchanged.
### 3. Contracts
NxTools.exe and ordinary PlatformHost stay unprivileged. Original protected LocalSystem broker/linked-admin token/peer authentication owns helper launch. Cache/coalesce latest configuration and replay after authentication. Claim gesture ownership only for current successful configuration and installed helper hook. Current-generation failure dominates a later ACK; old PID/generation cannot alter ownership. Observe actual process exit without heartbeats; all connected-channel failures share a finite automatic reconnect budget.
Hook callbacks reuse released double-tap detection and only PostMessage. Foreground/session/elevation checks, destination PID validation and forwarding run on helper message thread. Ordinary and helper sources partition by confirmed ownership. Both hooks carry the same native key timestamp so a queued ordinary message cannot toggle the panel after helper summon changed foreground; do not time-debounce distinct gestures. Recording/configuration resets are generation-scoped. Keep ordinary keyboard-health reporting independent.
Ordinary coordinator writes clipboard exactly once before input delegation; copy-only and write failure never dispatch. Helper validates live same-session elevated target and focus PID/root. Shared input-only execution creates its thread message queue, performs original bounded activation/modifier release and rechecks foreground before Ctrl+V. WM_PASTE alone remains unconfirmed failure. Helper absent before dispatch permits ordinary behavior; a dispatched unknown outcome returns failure and never replays input. EOF/disposal retires hook, hidden message HWND and directory contexts.
Helper activation admits only a live same-session root owned by the configured Desktop peer, then uses the original native activation/focus readback. Desktop disconnect clears that binding; detached activate_window dispatch does not block the serial reader. An undispatched unavailable helper permits the original ordinary path; a dispatched unknown result is a failure without native fallback replay.
The helper's existing message thread owns one40ms primary/secondary state monitor, stopped before its HWND is destroyed; no added hook/thread. Only press transitions on same-session administrator roots are forwarded, not movement/hold/release streams. The message retains root HWND, authenticated helper PID and native tick sample. Ordinary client admits only the current successful configuration/helper, rejects pre-configuration, duplicate, future or older-than1second samples, converts sample time to UTC and assigns the original platform event sequence. Original Rust pin/menu/paste/session/preview guards decide hiding. No per-poll diagnostics.
Normalize the32-bit native sample against the current64-bit uptime before comparing configuration/last-press baselines; a resident session or long idle must not reject fresh clicks after signed-delta overflow or native tick wrap. The controlled-clock original-channel regression covers both while retaining the1second stale-sample limit.
### 4. Validation & Error Matrix
Wrong target owner/root/session/destination -> no activation or input. Stale config/helper/click sample -> ignored; current helper click -> original GlobalPointerPressedEvent. Recording disables keyboard gestures, not outside-click observation. Native activation readback failure -> real failure, never success from topmost. Hook/thread/process failure -> clear ownership, finite recovery. Dispatched timeout -> unknown failure, no input/activation replay. Diagnostics carry correlation, role, state, elapsed/native error metadata, never clipboard data or key/pointer streams.
### 5. Good/Base/Bad Cases
Good: elevated helper forwards one administrator-foreground gesture and sends input after a single ordinary clipboard write. Base: ordinary windows and RegisterHotKey retain their original owners. Bad: elevate the whole app, send clipboard payloads to the helper, re-enable a dead hook from a late ACK, or replay timed-out paste.
### 6. Tests Required
Effects-reviewed ElevatedInputTests, ElevatedInputChannelTests and ElevatedInputHostTests exercise original coordinator, self-owned pipes/hidden HWNDs, fake native identity/input/timer boundaries, activation owner/root/readback, current/old/duplicate click routing and real cleanup. Retain keyboard ownership/injected/repeat/recording, original paste failure/correlation and directory authentication/session regressions. LIST before execution. Original installed administrator outside-click/activation failures have owned-window traces; updated protected binaries still require separate post-package native/physical/linked-token acceptance. Safe fixture success is not installed acceptance.
### 7. Wrong vs Correct
Wrong: successful compilation or synthetic HWND proves administrator-window interaction. Correct: native-safe production-boundary regressions plus explicitly separate installed/native acceptance.

## Scenario: Shell action invocation waits for human completion
### 1. Scope / Trigger
Quick Launch invokes a command from the live Windows Shell context-menu session.
### 2. Signatures
`ShellContextMenuSessionManager.InvokeAction` -> `ShellContextMenuSessionWorker.Invoke` ->
`ShellContextMenuSession.Invoke` -> `IContextMenu.InvokeCommand`; the existing session
`CancellationToken` remains the logical cancellation owner.
### 3. Contracts
STA startup, QueryActions and QuerySubmenu remain machine phases with the existing five-second
logical wait. The queued Invoke completion wait has no elapsed-time deadline: `InvokeCommand` may
synchronously open a system or extension window and return only after the user finishes. Cancellation
returns the logical caller promptly and retires the worker, but never claims to interrupt native code.
Retirement also wakes every queued logical waiter, and a dequeued request checks retirement before
entering its operation; an Invoke queued behind a machine phase cannot run after that machine phase
times out and retires the session. The already-active worker thread, COM session and cleanup remain
owned until the active operation actually returns. Do not create a replacement thread, parallel
invocation queue, feature switch or late successful result.
### 4. Validation & Error Matrix
Invoke running beyond five seconds -> still pending. Invoke returns -> original success/failure.
Cancellation while native code is running -> prompt cancelled result; old worker is disposed and not
reused, while physical invocation and cleanup continue to actual exit. Query/submenu/startup timeout ->
original five-second machine-stage failure; queued requests fail promptly without execution, while the
timed-out active native operation remains owned until actual exit.
### 5. Good/Base/Bad Cases
Good: a Shell verb waits for user input and succeeds after more than five seconds on the original STA.
Base: a short verb completes normally. Bad: report five-second failure while the native verb remains
live, force-stop COM/native code, or cache/reuse the worker before actual return.
### 6. Tests Required
Effects-reviewed fixtures enter the real worker Invoke method with an injected STA action, hold it
beyond the old five-second deadline, and then release it to success. Cancellation proves prompt logical
return, no worker reuse before actual exit and bounded watchdog cleanup. A held machine `TryInvoke`
queues a real worker Invoke, reaches the original five-second timeout, proves the queued wait fails
promptly without executing, then releases and observes the active operation's actual exit. No fixture
opens a real Shell window or touches user files, Registry, services or `IContextMenu.InvokeCommand`.
### 7. Wrong vs Correct
Wrong: every queued Shell operation waits at most five seconds. Correct: machine setup/query waits are
finite; the user-owned Invoke completion waits until return or logical cancellation while the original
physical owner continues to cleanup.
