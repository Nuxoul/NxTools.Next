# Floating assistant native boundary

## 1. Scope / Trigger
Changes to passive floating renderer/input/OLE sessions must preserve native receiving, hidden-frame preparation and peer identity. Desktop compilation and production-boundary integration now pass; real GPUI opacity/physical/mixed-DPI/user acceptance remains unverified, not release-ready certification. Evidence: `repo:csharp/NxTools.PlatformHost/Windows/FloatingAssistantCoordinator.cs`, `repo:csharp/NxTools.PlatformHost/Windows/FloatingAssistantInputSurface.cs`, `repo:rust/crates/nxtools-protocol/src/floating_assistant.rs`. Legacy reference: [assistant](../../../docs/reference-code/assistant.md); current evidence/limits: [checkpoint](../../tasks/10-23-floating-assistant/implementation-checkpoint.md).

## 2. Signatures
Outer protocol1 command `floating_assistant` -> `FloatingAssistantContract.Parse(ProtocolEnvelope)` -> per-connected-peer `FloatingAssistantCoordinator.Dispatch` returns `Task<WindowsCommandResult>`. Native input `FloatingAssistantInputSurface(renderer,peerPid,receive,input)` is created/disposed on the same fixed STA/OLE pump. Command operations: prepare/update/show/hide/retire/release_drop/pick_image. Rust `FloatingAssistantCommand`, `FloatingAssistantEvent`, `FloatingAssistantBounds` carry typed projection.

## 3. Contracts
- Command version1, nonempty session_id/generation>0, source_window_handle/source_process_id authenticated against connected pipe peer; renderer identity additionally required except picker before enable. Same exact HWND/session/generation binds later native operations; source can be hidden.
- x/y are device screen pixels, size20..300 is logical, opacity10..100percent, click_through/position_locked booleans. Optional edge_hidden defaults false: prepare/ordinary apply fully clamp, only explicit subsequent edge-hide retains8device pixels. Existing outer1MiB frame stays. Data response is JSON STRING with final client x/y/width/height/dpi/work/monitor bounds; picker returns selected_path nullable.
- One fixed native STA, bounded16queued commands; no pipe/sharedEventWindow blocking, unbounded thread replacement, callback injection, cross-process SetParent or extra GPUI patch. Input-only owned WS_POPUP uses NOREDIRECTIONBITMAP, NOACTIVATE, TOOLWINDOW and circle region. GPUI remains sole renderer.
- Prepare MUST stay natively hidden for every intermediate mutation: no SWP_SHOWWINDOW even if followed immediately by Hide. Only explicit show after GPUI correct-size frame reveals. Configuration saved, command accepted, SetWindowPos BOOL and final frame are distinct observations.
- One native IDropTarget captures Unicode/URL/HTML/PNG/DIB/GDI/local paths at Drop; DragEnter is query-only. Drop retires admission before callback and releases every STGMEDIUM. Text<=64KiB nativeUTF16, HTML<=256KiB, URL<=8192UTF8, image<=16MiB, files<=32 with bounded paths. Rust owns route and operation result, C# never decides download/search/copy behavior.
- One live staged drop token; release_drop cleans only exact per-session owned staging. Accepted staging survives hide until exact release, but rejected delivery/retire/disconnect clean it. Renderer stays passive/click-through; input receiving stops on hide. OLE offer and emitted event share original correlation; command lifecycle carries correlation/session/generation/elapsed/native code without payload or high-frequency logs.
- Existing500msobserver deduplicates monitor/work-area/DPI/fullscreen signatures; failed event publication does not advance the delivered signature. Rust owns recoverable anchor/layout and defers reapply. No real display configuration is mutated by synthetic signature tests.
- Rust action admission is bounded/once-only and pending-before-dispatch. Cancellation retires deferred/preparation work, while already admitted copy-only Windows work retains normalized image through actual result/deadline; it is not falsely reported as retractable. Main visibility success gates correlated activation.

## 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Wrong wire/version/range/type/extra field/peer | invalid_request or stale_or_busy before native mutation |
| Picker before renderer exists | renderer0 permitted; source/peer still validated; cancel changes no settings |
| Stale session/generation/closed source | no show/side effect, cleanup owned receiver |
| Native final client/policy mismatch | failure, hide rather than report applied |
| Duplicate Drop/DragLeave/inactive receiver | no repeated snapshot/action |
| Oversize/invalid medium | rejected, medium released, no consumer callback |
| Full outgoing result queue | await bounded transport off reader; never silently discard terminal result |

## 5. Good/Base/Bad Cases
Good: hidden native final bounds -> GPUI correct-size draw -> explicit passive show; own actual OLE route reaches target once. Base: image picker on disabled feature uses authenticated main HWND. Bad: prepare uses SWP_SHOWWINDOW then hides and reports success; direct managed Drop method called and claimed browser OLE acceptance; expose5FPS selector with no actual decode; native styles alone claimed transparent pixels.

## 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/FloatingAssistantCoordinatorTests.cs` checks strict DTOs/optional edge_hidden, picker renderer0, no temporary show via actual WM_WINDOWPOSCHANGED, owned clamp/edge-hide/recovery, source loss/drop staging and environment delivery retry. Input/Drop fixtures check pixels/passive/circle/foreign renderer/gesture/capture loss and COM representations/budgets/once-only/failing callbacks. Exact native `floating input native actual owned OLE routing` requires explicit authorization. Final13safe+1authorizednative and focused41pass; final fullDesktop1091pass/1unrelatedarchivefail. Latest global lint/fmt blocked by concurrent archive work and Quick Launch source gate still fails. Real GPUI opacity/physical/mixedDPI/user acceptance remains unverified. `repo:tests/architecture/floating-assistant-lifecycle.py` protects32source contracts; decode cache cancellation/re-enable has a production regression.

## 7. Wrong vs Correct
Wrong: `SetWindowPos(...SWP_SHOWWINDOW); ShowWindow(HIDE);` during preparation; test only final IsWindowVisible=false.
Correct: hidden-only style/bounds mutation and native transition-count assertion; explicit show is sent only after current-generation frame readiness. Wrong: `outgoing.TryWrite(terminal)` and ignore false. Correct: admitted off-reader completion awaits bounded `WriteAsync` with session cancellation.

## Runtime topmost/fullscreen recovery (2026-10-24)
Existing500ms observer now verifies WS_EX_TOPMOST on visible current renderer/input and repairs lost native topmost in renderer-then-input order with NOMOVE|NOSIZE|NOACTIVATE only. Applied policy includes TOPMOST readback. Hidden/retired/wrong-lease/destroyed surfaces never revive; click-through never shows a receiver. Bounds, foreground and show count stay unchanged. Repeated repair failure diagnostics are fixed-reason/correlated and rate-limited30seconds; environment events distinguish actual fullscreen/geometry change. Legacy `_topmostGuardTimer -> EnsureNativeTopmost` is the reference invariant. Fullscreen policy rejects Shell desktop/invalid/hidden/minimized and ordinary framed/maximized windows, preserves real borderless full-monitor hiding. Expanded self-owned HWND fixture plus8fullscreen cases pass in14safe fixture set. Historical fullDesktop/lint/fmt failures above are dated earlier evidence, not the current runtime task verdict; see task10-24-runtime-feedback-reveal-ui outcomes.

## Human-driven picker wait (2026-10-24)
`FloatingAssistantCoordinator.RunImagePicker` reuses the single bounded STA reservation with InfiniteTimeSpan for the human modal wait, coordinator Stop token and original StaDialogRunner.RunInline cancellation. A waiting user is not a background timeout: old2minute native/600second Rust deadlines caused isolated red regressions. No extra worker per timeout; cancellation cannot claim to kill uncooperative native code and original bounded slot stays held until actual modal exit. Rust retains one callerID until selection/cancel/disconnect; cancelled/retired is quiet, busy/invalidsource/real native failure is distinguishable. No selected path/raw exception diagnostics. Tests use owned synthetic STA signal, cancellation, Rust601s clock and stale disconnected reply; no real chooser/user files/clipboard. Native safe coordinator8+picker1 tests and Rust42related passed before final additions; exact cumulative results in task10-24-runtime-interaction-feedback. Other intermittent causes remain possible, not silently certified fixed.
