# Link panel and attached dialog material boundary

## Verified reference chain

- Legacy `D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs:127–167,206–259` wires Opened/focus and cancellation, and HidePopup reaches Avalonia Hide after retiring preview work. `Views\ClipboardPreviewWindow.cs:29–95,200–237` constructs a reusable passive topmost surface and reaches native Get/SetWindowLongPtr plus SetWindowPos(NOACTIVATE). There is no legacy Link tool equivalent; this is lifecycle precedent, not Link behavior or inspected Avalonia native-backend evidence.
- Listary managed `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary\Main\Views\DialogSearchWindowFactory.cs:10–12` constructs DialogSearchWindow; `DialogSearchWindow.xaml.cs:24–60` calls SearchWindowBase, then delegates ShowWindow and host-relative device-to-logical placement. `SearchWindowBase.cs:318–336,347–399` binds view-model Show/Hide/Close, calls WPF base.Show/base.Hide/base.Close, and keeps native foreground activation and requested input focus separate. Only the managed boundary is available; the proprietary native engine and WPF native implementation were not inspected. Legacy NxTools has no Quick Launch surface.
- Next Link: `ClipboardApplication::request_link_panel` -> cursor-monitor ACK -> reused hide/new hidden transparent popup -> NOACTIVATE -> offscreen bounds -> topmost -> stage show ACK -> next frame -> hide -> target monitor -> final client bounds -> reveal ACK. PendingLinkPlacement owns the original operation, generation, pause generation and three-second deadline. `show_link_panel` retains the original single view, row actions and hide timer.
- Next attached search: `handle_file_dialog_search_host` -> hidden transparent popup -> deferred initialize/topmost -> `apply_file_dialog_search_window_placement` -> SetWindowOwner/PlaceFileDialogSearchWindow/SetWindowVisibility. C# `WindowsPlatformAdapter.cs:762–831` delegates placement to AnchoredWindowPlacement and registers the existing EventWindow follow owner; visibility reaches NativeMethods.SetWindowVisibility. Material must precede this original reveal boundary, not replace attachment/follow or introduce another ACK owner.
- `csharp/NxTools.PlatformHost/Windows/AnchoredWindowPlacement.cs:95–178,180–249` keeps attachment placement/follow at the original SetWindowPos(NOZORDER|NOACTIVATE) boundary, without showing the HWND. Native visibility remains the separate original command.

## Preserved invariants

One retained popup per feature; Rust layout/state, C# native effects, GPUI rendering. Link retains current monitor/work-area/DPI, passive policy, stage/frame/ACK order, generation/pause cancellation, expansion and original three-second ceiling. Attached search retains host/visibility identity, hidden construction, original owner/placement/follow and host-destroy/foreground/source behavior, InputState, IME, query cadence, selection and scrolling. No material request targets the foreign file-dialog/Explorer or Clipboard paste HWND. Every visibility-true command needs a current exact-window material receipt; stale facts, retired generations, closed windows, disconnect and timeout cannot release it. Callback work occurs on a later application turn outside target-window leases; retained runtime preparation remains the sole material record.

## Component selection

No rendered controls/layout/variant changes. Link retains released Kit0.10.0 ListItem public row slots, XSmall ghost Copy/Download, Small ghost expander and StatusBar; attached search retains InputGroup::new/input/addon with Input::new on its stable InputState, original 500DIP desired width/36DIP bar/264DIP result budget. No Toolbar parent adaptation, replacement list/search controller, custom material renderer or vendor change.

## Verification scope

Synthetic production-boundary regressions cover Link offscreen admission, frame/final reveal, changed facts, retained reuse, cancellation, deadline, disconnect and wrong-HWND receipts; attached-search tests cover original attachment/reveal, changed facts, hidden retained reuse, pending host replacement, deadline, disconnect, native failure and close. They use temporary configuration/history and explicit TestWindow HWNDs.

## Implementation and source validation

- `clipboard/link_placement.rs` adds material admission at Topmost→Stage and FinalBounds→Reveal, with the original operation clock, pause/generation/native-window/connection checks. Deferred registration and completion both qualify the live owner. Runtime retains the same Preparation through reuse and pending updates.
- `quick_launch/file_dialog_search.rs` keeps its original attachment and native command loop behind `file_dialog_material.rs`. Host visibility/replacement, hide, close and disable retire the admission token. A three-second material wait fails hidden while retaining the existing popup; it never replaces query/selection/foreground/source or native-follow ownership.
- Source gates passed: `python tests/architecture/gpui-release-contracts.py` and `python tests/architecture/markdown-editor-contracts.py`. Scoped rustfmt parse/format and tracked-diff whitespace checks passed.
- Compiled test filters for the owning integration run: `clipboard::link_placement::tests`, `quick_launch::file_dialog_search::material_tests`, and existing `quick_launch::file_dialog_search::tests`.

## Limitations

The 12 new material regressions are not compiled or executed in this slice. Cargo/build/native/product/service/user-data operations remain deferred. Source checks do not establish actual Windows Acrylic composition, passive focus, mixed-DPI behavior or user visual acceptance. The C# placement and material writers are unchanged.
