# A source review

Scope: pre-repair source evidence. Current source delivery is recorded in [implementation outcomes](implementation-outcomes.md); compiled/native/memory validation is user-owned and has not run.

## Reference authority

Read `docs/reference-code/platformhost-uia-memory-repair.md` and `system-shell.md` before actual SwiftList entries:

- `D:\Code\SwiftList-main\Plugins\OneCommander\OneCommanderPathCollector.cs:27`: process rejection before UIA, focus refresh before overlay.
- `D:\Code\SwiftList-main\Plugins\OneCommander\OneCommanderInlineSearchAdapter.cs:53`: refresh the selected pane before search UI takes focus; execution reuses the captured scope.
- `D:\Code\SwiftList-main\Plugins\OneCommander\Automation\UiaPathAccessor.cs:22`: strong per-window latest focus, including HWND-less WPF children; nearest ancestor with CurrentPathGet/CurrentPathSet rather than a first-pane search.
- `D:\Code\SwiftList-main\Core\Hook\Commands\InlineAdapterCommandHandler.cs:16`: writes use STA execution. Its fresh-thread/replacement shape is not adopted; Next retains named bounded admission and physical cleanup ownership.

The searched legacy NxTools checkout has no OneCommander implementation. Listary evidence is its managed boundary, not available native-engine source. No reference implementation code/assets/comments/tests are copied.

## Pre-repair owners and exact boundaries

| Source entry | Verified behavior |
| --- | --- |
| `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs:102` | Unconditional third-party WarmUp before elevated channel and EventWindow startup. The full constructor also starts unrelated clipboard/native hooks and is not an isolated fixture. Dispose at1612 stops EventWindow and channel, but does not own third-party tracker/session cleanup. |
| `Windows/ThirdPartyFileManagerAdapter.cs:39` | Static registry constructs four adapters. WarmUp48 enters OneCommander static tracker. Registry100 captures process/HWND/context and strong PaneState;64entries/10minute TTL. RemoveSession399 only removes map/order entries. Native pane leases need explicit release on consume/fatal/expiry/capacity/stop. |
| `Windows/OneCommanderFileManagerOperations.cs:45` | Process and main-window gates precede capture. Capture121 uses named120ms STA; Navigate159 uses1500ms STA and original native activation. |
| `Windows/OneCommanderFileManagerOperations.cs:221` | Managed ElementFromHandle, raw walker, conditions, subtree searches, ValuePattern and comparisons are routine paths, not just registration. Nearest pane search334 is25ancestors; navigation279 verifies getter up to1second. PaneState93 holds a strong managed element. |
| `Windows/OneCommanderFileManagerOperations.cs:424` | Global managed focus subscription and initial current-focus sample; process gate before64ancestor ascent. ProcessExit is the current static disposal owner. |
| `Windows/OneCommanderFocusTracker.cs:1` | One existing MTA owner;64latest focus anchors;60second native window/process sweep without UIA idle traversal. Exact-observed-anchor removal protects asynchronous replacement.500ms Dispose wait is not physical native completion. |
| `Windows/BoundedStaOperationRunner.cs:1` | Named admission survives logical timeout until actual call/finally and associated Dispatcher shutdown. Do not release or replace that slot early. |
| `Windows/WindowsPlatformAdapter.EventWindow.cs:1222` | Existing foreground hook feeds ProcessForeground1247 and local directory capture1298. Reuse this source for ordinary-role demand; no additional global hook. |
| `Windows/WindowsPlatformAdapter.EventWindow.cs:2550` | Local capture occurs before QuickLaunchPanelRequested publication2560. Elevated capture currently occurs later in Task.Run2581. A native demand-only helper must freeze OneCommander focus before publication, not sample the now-focused overlay. |
| `Windows/DirectoryNavigationAdapter.cs:145` | Original local/helper routing, foreground integrity selection, capture attempt and context identity; third-party call at200 and navigation246. |
| `Windows/ElevatedDirectoryNavigation.cs:49` | Original authenticated channel has serialized request gate,180ms capture/2500ms navigate, bounded frames and correlated reply. Reuse capture for the pre-overlay OneCommander freeze. |
| `Windows/ElevatedDirectoryNavigation.cs:396` | Client Dispose closes exact connection. Helper Run631 validates rendezvous/elevation, calls WarmUp640, then authenticates/serves capture/navigation. EOF675 and all terminal paths currently lack third-party cleanup. |
| `Windows/ElevatedDirectoryNavigation.cs:431` | Helper registry64entries/10minute TTL maps public context to original local context. RemoveSession559 removes only mapping; cleanup must retire the underlying OneCommander lease, not leave it in the inner registry. |
| `Protocol/PlatformPipeClient.cs:255` | Session disconnect cancels original commands/resources and drains tasks. Directory session retirement must join this actual session boundary without affecting Shell/clipboard/archive cleanup. |
| `Windows/WindowsPlatformAdapter.EventWindow.cs:3139` | Existing stop removes its hooks/timers and exits the owning event thread. UIA stop must compose with this order, not replace unrelated cleanup. |

Paths abbreviated with `Windows/` are under `csharp/NxTools.PlatformHost/`.

## Native API evidence

The installed Windows SDK `C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um\UIAutomationClient.h` was read directly. IUIAutomation AddFocusChangedEventHandler at19009 takes only cacheRequest and handler; it has no process/root parameter. RemoveFocusChangedEventHandler at19013 removes the exact handler. ElementFromHandleBuildCache is at18880; IUIAutomation2 connection/transaction timeout setters are at19679/19685. CUIAutomation8 is declared by the same SDK.

Use native COM ABI definitions and explicit owned references; do not invoke managed System.Windows.Automation in the affected OneCommander path. Native global focus registration, if enabled, is honestly global while target demand is active. A cached PID/native HWND permits rejection before live tree/property/pattern traversal. Validate interface method layout/signatures and actual native calls in the owned fixture before relying on the binding.

## Existing test evidence and gaps

`OneCommanderFocusTrackerTests.cs` has seven synthetic-element cases for latest anchors, native-window retirement, bounded recency, invalid/stale queries, reuse/stop and an in-flight callback. `StaDispatcherFocusTests.cs:16` uses delegates/reflection to test process rejection before tree walk. These preserve policy but do not prove native COM retirement or memory behavior.

`NxTools.PlatformHost.Tests/README.md` and TestEffects require body/callee review before exact catalog registration and nonzero LIST→identical RUN. New tests must exercise real UIA registration/access against owned providers/ListView/dialog windows. Full adapter construction, real helper Run elevation validation, product/reference app launch, UAC, foreign HWND and global input are not implicitly authorized.

## Invariants

Layout/placement/rendering stays unchanged. Preserve focused/split pane, strong HWND-less WPF anchor, pre-overlay capture, nearest getter/setter, request-bound pane independent of later cache replacement, path/navigation/inline-search and native focus restoration. Retain process/HWND/generation identity, bounded failure budgets, logical-cancel versus physical-return distinction and exact unregister/drain/release order.

The incident's near20GiB private memory, small managed heap and managed ListView/COM OOM stacks are reported observations. Full native allocation/CCW share has not been established by allocation stacks; the controlled work must not claim otherwise.
