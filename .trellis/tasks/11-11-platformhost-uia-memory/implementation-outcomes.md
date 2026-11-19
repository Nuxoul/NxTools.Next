# A — Native OneCommander UIA source delivery

Implementation: ready_for_user_verification. Acceptance: not_accepted. Verification owner: user.

## Delivered repair

The ordinary host and elevated helper no longer initialize a global managed OneCommander focus tracker at startup. The affected OneCommander event, query, cache, raw-tree, condition, comparison and ValuePattern paths use native CUIAutomation8/IUIAutomation throughout.

The existing MTA owns the native client and strong pane references. Original named120ms capture/1500ms navigation STA callers retain physical admission until the MTA operation returns. Callback references coalesce to one pending element; there is one pending request, not an unbounded event/work queue or replacement worker.

Actual directory-source demand uses original foreground/destroy notifications and existing enabled features. Explicit capture works before overlay even with automatic observation disabled. Elevated OneCommander freezes its focused pane through the original180ms authenticated capture channel before panel publication. Native focus registration, when enabled, is honestly global and filters cached PID/HWND before traversal.

Captured panes are64native owner/identity/window/PID leases with original10minute expiry. They survive focus-cache replacement. Consume/fatal/capacity/expiry/window/pipe/scope retirement releases them on the originating MTA; helper outer-context removal retires the inner context. CaptureHandoff releases a result abandoned by a timed-out caller.

Stop closes admission/generation before exact unregister and callback drain, native reference/buffer release, COM teardown and thread exit. Failed native removal is reported with its HRESULT and does not permit successful re-registration over that failure. A bounded500ms wait does not claim physical exit.

## Precise product paths

The [source manifest](implementation-changed-paths.txt) lists10C# paths, not the entire dirty HEAD diff:

| Path under csharp/NxTools.PlatformHost | Original owner / delivered change |
| --- | --- |
| Windows/NativeUiAutomation.cs | New direct native ABI binding and deterministic interface/cache/BSTR/VARIANT/callback ownership. |
| Windows/OneCommanderUiaSession.cs | Native resources on the original MTA, strong pane leases, native queries/navigation and demand/retirement. |
| Windows/OneCommanderFocusTracker.cs | Existing focus/cache/thread owner gains dormant start, one physical operation, coalesced native focus and explicit cleanup. |
| Windows/OneCommanderFileManagerOperations.cs | Original adapter/process/main-window/native activation and bounded STA entry; all affected managed UIA replaced, abandoned capture released. |
| Windows/ThirdPartyFileManagerAdapter.cs | Original static adapter/session registry now owns scope and pane-lease retirement. |
| Windows/ElevatedDirectoryNavigation.cs | Original authenticated helper loop and outer context registry dispose inner contexts/native lifetime on all terminal paths. |
| Windows/WindowsPlatformAdapter.cs | Original startup/dispose/reconnect scope; no startup UIA WarmUp. |
| Windows/WindowsPlatformAdapter.EventWindow.cs | Existing foreground/destroy/configuration owners drive actual demand; elevated OneCommander capture precedes panel publication. |
| Protocol/PlatformPipeClient.cs | Original pipe disconnect closes current directory context work. |
| Program.cs | Original role log passed to helper native lifetime without another sink. |

Reference/contract updates: docs/reference-code/platformhost-uia-memory-repair.md, docs/reference-code/system-shell.md and .trellis/spec/platformhost/native-contracts.md. Task PRD/design/implementation state records user-owned validation. Current-batch before-copies and the156path pre-existing dirty baseline remain attribution material; no whole-HEAD attribution.

## Verification and remaining acceptance

| Boundary | Result |
| --- | --- |
| C#/Rust compilation | not_run |
| Formatting, lint/check and architecture gates | not_run for this implementation |
| Unit/integration/native LIST/RUN | not_run; no selected/run/pass/skip counts invented |
| Behavioral RED/GREEN | not_run |
| Actual unregister/COM/handle/thread/process retirement | not_run |
| Sustained no-target/active-target memory comparison | not_run |
| Physical OneCommander/elevation, human visual, installed/Release | not_run |
| B6 REF-13/14/15/16 timing | not_started |

The user owns validation. Source implementation does not prove that the reported private-memory growth has disappeared. The complete allocation/CCW share remains inference without native allocation stacks. If growth or an interaction defect remains, it must be investigated at the actual native owner.

No test/native GUI/UAC/product/reference launch, memory sampler, forced GC/trim/restart, packaging/install, service/registry mutation, dependency/vendor upgrade, commit/push or real power/current Desktop exit was performed. No B7 or whole-batch acceptance is claimed. B1–B5 accepted behavior, Rust download owners and unrelated native services were not changed by this repair.
