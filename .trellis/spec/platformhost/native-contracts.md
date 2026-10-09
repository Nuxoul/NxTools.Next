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
