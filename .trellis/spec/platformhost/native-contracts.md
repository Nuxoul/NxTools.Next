# Windows API, lifecycle and mutation contracts

Evidence: `repo:csharp/NxTools.PlatformHost/Protocol/PlatformCommandSequenceDispatcher.cs`, `repo:csharp/NxTools.PlatformHost/Windows/BoundedStaOperationRunner.cs`, `repo:csharp/NxTools.PlatformHost/Windows/CenteredResizeCoordinator.cs`, `repo:csharp/NxTools.PlatformHost/Windows/BrowserRegistrationTransaction.cs`, `repo:csharp/NxTools.PlatformHost.Tests/Program.cs`.

## Ownership / observed runtime
Program initializes PerMonitorV2, log/cancellation/adapter/pipe lifecycle. Protocol v1 GUID envelope has1MiB frame limit; BoundedFrameReader honors cancellation/limit. PlatformPipeClient outgoing queue256; serial reader dispatch detaches blocking Shell/picker/OLE work before synchronous native call can block. EventWindow owns dedicated STA WinForms messages/hooks/timers and cleanup; domain/GPUI remains Rust. Native methods partials are real code boundaries; prefixed P/Invoke needs actual EntryPoint.

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
RSA-PSS/SHA256 authenticates raw bounded NxTools.release.json before JSON parsing; detached NxTools.release.sig. Schema1/product/nontemporary Guid/positive Int64 sequence/pinned-key hash, exact complete5executable+licenses+extension paths, max1MiB manifest/4096files/2GiB artifacts; stable-handle reparse/path traversal rejection. Publisher PEM is explicit/off-client; generated UpdateTrust contains public SPKI only. Release package requires signing key+sequence before Cargo, legacy unsigned is an explicit non-trusted exception. Resident bootstrap is separate from replaceable tuple; trust rotation/bootstrap repair is not a silent update. Caller-identity source opens, fixed protected staging/deployment, serial bounded requests, correlation and low-noise terminal diagnostics. Once prepared, finish/rollback independent of client loss. Sequence commits after actual tuple/all-file validation; signed last-good recovery and cross-process deployment gate precede retry.
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
Isolated Rust extraction writes only via C# held-directory capabilities. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs`, `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs`, `repo:csharp/NxTools.PlatformHost.Tests/ArchiveExtractionWorkerTests.cs`.
### 2. Signatures
ArchiveWorkerRequest.OutputRoot stays in C#. Rust ArchiveOutput directory/begin/write/commit/abort emits native_call{call_id,action,path?,token?,data?}; matching version1/op=native_result/call_id/success/token?/error_code?/native_error_code?. One outstanding call; chunk<=65536bytes, no payload in published events/logs.
### 3. Contracts
OutputSession uses held non-share-delete parent handles/relative single components/no-reparse/exclusive GUID partials. Flush/integrity before no-replace commit; exact-token abort. Cleanup new outputs only after identity/size/mtime/hash match; replacements retained. No implicit NFC. Current local-drive/depth64/4096file/finite directory limits are not broad format acceptance. RAR/7z stage partials through full codec integrity/source recheck. Worker stdout pump owns native work; no serial command/event blocking.
### 4. Validation & Error Matrix
Unsafe/reparse reject; no-replace appearance returns target_exists80/183. Approved2026-10-17 overwrite: full verified temporary→relative volume/full128-bitID/size/mtime/hash recheck→close only target check→single SetFileInformationByHandle(FileRenameInfoEx22,flags1|2). Parent/source/temp protections remain; TOCTOU accepted, no target-ID CAS. Win32 full name derives from held parent VOLUME_NAME_GUID, RootDirectoryNULL; counted UTF16 bytes exclude explicit WCHAR NUL. Relative RootDirectory returned87 here; unterminated name probe published wrong suffix. No delete/truncate/backup-first or MoveFileEx fallback. Native32/5/87remain real failures, not unsupported; only proved capability codes qualify. FAT/SMB/old systems unverified.
### 5. Good/Base/Bad Cases
Good: independent payload byte comparison through real Job/Rust/native handles. Base: verified no-replace commit. Bad: unrestricted unpack, lexical-only path proof, raw output logs or failed default suite described as green.
### 6. Tests Required
Preserve native path/cleanup and ZIP/RAR/7z integrity fixtures;7overwrite/ABI fixtures cover bytes, stale/missing/appeared/directory/reparse/busy, native unsupported/unknown, cancel before/during/after and cleanup. Historical strict-CAS32FAIL stays historical; current path contract supersedes it.
### 7. Wrong vs Correct
Wrong: own target lease blocks replace, or cancelled implies unchanged. Correct: integrity/flush→relative recheck→close target only→final cancel checkpoint→CommitInFlight→one rename. Native success sets Committed and idempotent FilesCommitted before close/hash. Postcheck/unknown never retry or partial-delete. Preserve overwrite_not_supported/native code to terminal; published cancellation says cancelled_with_committed_output. Log correlation/state/count/elapsed/native only.

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
