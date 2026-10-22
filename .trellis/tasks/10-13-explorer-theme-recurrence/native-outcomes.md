# Explorer target-reader recurrence: native outcomes

## Scope and references (2026-10-13)

Owned files: ExplorerShellActivation.cs, ExplorerRevealFeedbackTests.cs, this report; ExplorerNativeFolder.cs only if necessary. Preserve dirty unrelated work. No live Explorer/COM enumeration, registry, services, downloads, real logs/user configuration or Cargo. C# exact reviewed fixture filters and build only.

References inspected before implementation: docs/reference-code/download.md including R5 lines2773-2776; ABDM D:\Code\ab-download-manager-master\shared\utils\src\desktopMain\kotlin\ir\amirab\util\osfileutil\WindowsFileUtils.kt:24-39,57-96 (native acceptance then one executable fallback); legacy D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs:10-35,56-75 (Avalonia Window constructor/animation/OnOpened; owner action then Close), ViewModels\MainWindowViewModel.cs:1583-1616,1636-1658 (file selection/directory opening/fallback and caught launch errors). Next deliberately requires observed exact target before correlated current-window close; do not restore unconditional legacy close.

Verified Next call chain: WindowsPlatformAdapter.OpenShellTargetAsync:931-1025 -> ShellTarget.Launch:123-133 -> ExplorerShellActivation.LaunchAndActivate -> NativeTargetObservationPlatform.ReadActiveViews -> IServiceProvider.QueryService(SID_STopLevelBrowser,IID_IShellBrowser) -> IShellBrowser.QueryActiveShellView -> IShellView.GetWindow / GetItemObject(SVGIO_BACKGROUND,IID_IDispatch) -> active.Folder -> Folder.Self -> selected filesystem paths -> ObserveTarget. ExplorerWindowAdapter.cs:319-337 owns reused IWebBrowser2 dispatch. ExplorerNativeFolder.cs:17-46 preserves native open/selection and borrowed child PIDL lifetime.

## Ranked hypotheses before tests

1. Background automation/interop contract gap at decoding actual COM objects (previous fixtures skip it).
2. Service/view IID or vtable mismatch.
3. Valid active-view HWND rejected by visibility/root checks.
4. Selection published after bounded attempts.

SDK verification: C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\ShObjIdl_core.idl:1565,1606-1645,2593-2641 and ShlGuid.h:275 agree with used Shell browser/view slots, SVGIO_BACKGROUND and service IID. ShlDisp.h:3817-3818 returns base Folder;1751-1791 has no Self;2000-2005 defines Folder2 IID f0d2d8ef-3890-11d2-bf8b-00c04fb93661 and its first additional method get_Self. Candidate1 was reproduced through an owned COM fixture with base Folder dispatch and explicit Folder2 QueryInterface, not a snapshot or live Shell enumeration.

## Invariants

No layout/placement/focus-policy changes. Existing detached STA owns COM initialization, RCWs and bounded8attempts/7waits75ms/onefallback; do not introduce workers/replacement loops. Query active visible view under matching root; normalize exact directory, additionally exact selected file for file requests. API/launch/folder-only/foreground-only success is never target proof. Once target confirmed, foreground is best effort. Missing/deleted/wrong/unobserved target remains failure. Rust retains originating HWND/generation/correlation, cancellation and close policy. Preserve payload-free stage/native-error/elapsed diagnostics; release COM references in finally. No native/user acceptance claim from synthetic proof.

## Results

### Root-cause proof and minimal change

The reader called dynamic Folder.Self even though the native getter returns base Folder. There was no explicit QueryInterface to Folder2. An owned unmanaged COM fixture exposes base IDispatch without Self and exposes IID_Folder2; the unchanged reader member access failed with `'System.__ComObject' does not contain a definition for 'Self'`. Previous decoded ExplorerTargetView snapshots and browser/view metadata assertions bypassed this member access. This establishes a native reader contract defect; it does not prove this is the only cause on the user's installed Explorer.

ExplorerShellActivation.cs:268-299 adds ReadFolderSelf and the SDK-exact Folder2 prefix (IUnknown3 + IDispatch4 + Folder10; get_Self zero-based native slot17), querying the correct IID and preserving HRESULT. ReadActiveView:372 now uses that helper. Existing per-provider/native-terminal error handling and finally release own failure/reference cleanup. ObserveTarget:254-265, bounded attempts/fallback:35,113-198, target-success/foreground independence:134-169 and75ms wait:519 remain unchanged. ExplorerNativeFolder.cs, Program.cs, TestEffects.cs and Rust were not edited.

ExplorerRevealFeedbackTests.cs:104-183 extends the existing audited NativeComContractsHaveExactVtableSlots fixture, including exact Folder2 metadata and a production ReadFolderSelf call on its own STA. The raw COM fixture:185-274 asserts QueryInterface, actual get_Self slot, returned COM identity, no calls to preceding slots, E_ACCESSDENIED preservation and balanced references on success/failure. There is no CoCreateInstance or live Shell enumeration. Native calls are limited to COM initialization and its owned in-memory tables/RCWs.

### Commands and truthful counts

Red build: `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` passed,0warnings/0errors. Before the fix, `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- "Explorer reveal feedback fixture native COM exact vtable contracts"` exited1: selected1/run1/pass0/skip0/fail1/not_run0, missing Self error above.

Initial focused green after the fix: same build and exact single filter passed selected1/run1/pass1/skip0/fail0/not_run0. An added HRESULT branch initially failed because the fixture caught COMException only, while Marshal.ThrowExceptionForHR(E_ACCESSDENIED) maps to UnauthorizedAccessException. Corrected the fixture to assert Exception.HResult; product behavior was not weakened or changed to swallow the error.

Final integrated build: `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore`,0warnings/0errors. Exact audited subset:

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

Exit0: selected13/run13/pass13/skip0/fail0/not_run0. This includes6feedback fixtures +7compatibility fixtures. The extended native-contract fixture includes2owned COM cases (success/native failure), not2extra harness entries. Standalone `dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore` passed0warnings/0errors. `git diff --check -- csharp/NxTools.PlatformHost/Windows/ExplorerShellActivation.cs` passed.

### Limits / handoff

No live/user Explorer or real COM enumeration ran; no package/product launch, registry/service/download/log/user-config operation or Cargo. Synthetic/native owned COM proof repairs a real contract gap but cannot certify Windows11 tabs, UNC, physical foreground, timing, installed completion close or that this is the sole live cause. Parent may resume C# after handoff. Before claiming the user symptom resolved, separately authorize a scoped live directory/file Reveal plus originating completion-window close check with payload-free correlated diagnostics; do not substitute launch acceptance or unconditional close if readback still fails. No further product changes are justified from the unavailable live observation.
