# Installed Explorer Reveal: observation outcomes (2026-10-14)

## Outcome / acceptance boundary

Repaired a reproduced native observation defect: the reader treated IWebBrowser2.HWND as a top-level Explorer HWND, silently discarding a browser child before QueryService. It now classifies GetAncestor(GA_ROOT), then preserves the real active visible view/root and exact directory/selected-file readback. Owned native-chain regression is red before this change and green after it. **The installed user's Reveal is not certified fixed:** no existing Explorer windows were available in this session, so the user's actual HWND topology could not be observed.

Supplied incident evidence (not read from real logs by this worker): Oct14 10:44:40, correlation5f7aec30-a882-4c96-afe5-8b3650f9eb70/PID15888, accepted native selection -> readback -> one fallback -> readback -> target_not_confirmed,853.7093ms,null native error. Supplied protected/artifacts host SHA2565BCD8D283765169C106269256092484DA37216C95A6016880BDA6A68CB3BEDAE already includes Folder2 repair. No binaries/hashes/deployment were modified or independently revalidated.

## References / preserved contracts

Required AGENTS, platformhost index/native-contracts/diagnostics, shared architecture/engineering/verification, download.md tail and previous10-13-explorer-theme-recurrence/native-outcomes.md were read. Reference sources:

- ABDM first: D:\Code\ab-download-manager-master\shared\utils\src\desktopMain\kotlin\ir\amirab\util\osfileutil\WindowsFileUtils.kt:24-39,57-96: native folder/selection request then one executable fallback; acceptance is not selected-target evidence.
- Legacy D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs:10-35,56-75: Avalonia Window construction, animation attach/OnOpened, owner file/folder action then Close.
- Legacy Views\MainWindow.axaml.cs:2193-2226 -> ShowIndependentSecondaryWindow:2844-2884 -> window.Show():2867; passive retention:2663-2666. Services\WindowAnimationService.cs:13-44 attaches Opened/Closing and animated Close. Avalonia native Window implementation is not in the inspected tree; do not claim native internals were available.
- Legacy ViewModels\MainWindowViewModel.cs:1583-1616,1636-1658: existing file selection/existing directory open/parent recovery/caught launch failure. Next deliberately retains stronger success-only correlated close instead of legacy unconditional close.

No layout/placement/focus policy changes. Existing native request, detached STA, eight attempts/seven75ms waits/one fallback,256-window/4096-selection bounds remain. Folder2 exact COM contract unchanged. File success still requires exact selected filesystem path from active visible view, directory success exact folder. Foreground is independent best effort, never target evidence. Rust originating HWND/generation/current-result close/cancel remain untouched. No launch-only/unconditional success, timing inflation, service/registry/download/log/Cargo/package operations.

## Three ranked hypotheses before repair

1. Browser HWND topology/classification silently rejects a tab child before QueryService. Best fit for native_error=null and no target evidence, but supplied log cannot prove the topology.
2. IServiceProvider/QueryService/QueryActiveShellView/GetItemObject ABI or provider failure prevents automation decoding. Existing Folder2-only and decoded-snapshot fixtures do not exercise this prefix.
3. GetWindow visibility/root test rejects a real active view (or selection publication occurs outside the preserved budget). Do not widen timing or accept folder/foreground as a file without causal proof.

Passive read-only observation: Shell.Application.Windows count0. EnumWindows/EnumChildWindows counts: CabinetWClass0,ExploreWClass0,ShellTabWindowClass0. Output contained only stage/count/class metadata; no folder/file/title. No user window was opened/navigated/activated/selected/closed. These observations establish the environment limitation, not that the user's Explorer was absent when the incident occurred.

## Actual seam regression / causal repair

ExplorerObservationTests.cs:9 BrowserChildTraversesNativeActiveViewChain creates its own offscreen NOACTIVATE/TOOLWINDOW HWNDs and raw unmanaged COM tables. Only ShellWindows collection creation is injected; each candidate is an actual RCW. Production reader performs IWebBrowser2 IDispatch HWND(-515/VT_I4), native GetAncestor/GetClassName/visibility reads, IServiceProvider.QueryService(SID_STopLevelBrowser,IID_IShellBrowser), actual browser slot15 QueryActiveShellView, view slot3 GetWindow and slot15 GetItemObject(SVGIO_BACKGROUND,IID_IDispatch).

Before repair, owned top-level input reached all five calls and the deliberately failing background seam E_ACCESSDENIED. Child input failed with `native chain child=True counts=1/0/0/0/0 failures=0`: actual HWND read succeeded but QueryService was never called. Exact single filter selected1/run1/pass0/skip0/fail1/not_run0/exit1. This is a root/class gate failure, not a Folder2 or decoded-snapshot failure.

After GetAncestor(GA_ROOT) repair both top-level and child inputs reach counts1/1/1/1/1 and preserve E_ACCESSDENIED. Hidden view and view under another root still return no evidence before GetItemObject. Assertions verify no wrong native slot and all RCW references return to baseline1 on success-to-failure and rejected branches; diagnostic callback exceptions do not change behavior. GetItemObject intentionally fails, so this new fixture proves traversal/omission repair, **not a successful complete folder/selection automation observation**. Existing Folder2 and exact-selection fixtures remain separate coverage. Initial fixture setup failures (cross-assembly dynamic collection accessibility) were corrected before the meaningful red; no product contract was weakened.

Product changes in csharp/NxTools.PlatformHost/Windows/ExplorerShellActivation.cs:

- :300-314 exposes internal NativeTargetObservationPlatform constructor with optional Func<object> createWindows and Action<string> observationStage; production default still creates ShellWindows on owning STA.
- :336-342 resolves native root before Explorer class gate, retaining active view validation:376-387 and exact automation/matcher unchanged.
- :43 wires the existing stage callback into native observation; :326,340-344,377-387 add fixed omission/failure stages. :478-488 deduplicates each fixed stage once per command (at most10 distinct new stages, not window/attempt logs); diagnostics contain no payload. Existing terminal carries native HRESULT and elapsed under same correlation.
- :550-551 shares GetAncestor(GA_ROOT) read through ReadRootWindow; no new Windows mutation API.

Test changes:

- csharp/NxTools.PlatformHost.Tests/ExplorerObservationTests.cs:9 new public Task BrowserChildTraversesNativeActiveViewChain; :63 OwnedCollection is public for production dynamic binding; :71 ChainFixture owns raw COM tables/HWNDs; :188 nonactivating offscreen creation. No CoCreateInstance/live Shell/registry operation in fixture.
- ExplorerRevealFeedbackTests.cs:132 calls the new method from existing NativeComContractsHaveExactVtableSlots. Program.cs/TestEffects.cs **not edited**. No new catalog entry required to run this coverage; parent can optionally register this exact symbol separately after effect review, but avoid reporting duplicate calls as extra independent fixtures.
- docs/reference-code/download.md:2781-2784 append verified sources/seam/limits; no earlier text rewritten.

## Verification

`dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore`: passed0warnings/0errors (also compiles referenced PlatformHost). Single existing filter `Explorer reveal feedback fixture native COM exact vtable contracts`: red1/1 failed before root normalization; focused green selected1/run1/pass1/skip0/fail0/not_run0 after repair.

Final build after bounded diagnostics/assertions: passed0warnings/0errors. Exact safe integrated filters:

```powershell
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- `
 "Explorer reveal feedback fixture" `
 "installed native fixture Explorer native first and fallback" `
 "installed native fixture Explorer readback retry and pending" `
 "installed native fixture Explorer relative PIDL lifetime" `
 "Explorer activation selects files despite an existing matching folder window" `
 "Explorer activation opens existing directories directly" `
 "Explorer activation rejects a window destroyed before foreground readback" `
 "Explorer activation rejects unrelated foreground and denied activation"
```

Exit0 selected13/run13/pass13/skip0/fail0/not_run0. Six feedback entries (native contract entry now also calls owned chain) plus seven compatibility entries. No native suite/native-authorization switches. Scoped git diff --check passed. No Cargo overlap initiated. Existing dirty source changes preserved.

## Remaining limits / handoff

Precise blocker to confirming the installed recurrence cause: ShellWindows and native Explorer class enumerations returned0, leaving no real user COM provider/HWND chain to inspect. Child-HWND gate defect is concretely repaired but whether the installed provider returns a tab child is unverified. Live complete automation readback, Windows11 tabs/UNC/timing/physical foreground and originating completion-window close remain untested; do not report the user's Reveal fixed from synthetic traversal or compilation. No package created. Future authorized installed attempt can distinguish no ShellWindows/interfaces/root rejection/service/view/window/visibility/background failure through bounded new observation stages without exposing target data; preserved truthful failure should remain if exact selection is still unobserved.
