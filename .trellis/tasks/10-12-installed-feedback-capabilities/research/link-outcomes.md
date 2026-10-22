# R6 Link identification and explicit actions

## Authority and reviewed references

R6 in the approved `prd.md:19,27` supersedes the historical copy-only/browser-gate policy in `docs/reference-code/link-detector.md` and shared UI history. There is no legacy NxTools link detector panel: old Clipboard popup/preview is lifecycle precedent only, not a source for Link action policy.

Reviewed source chains before implementation:
- `D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs:61-167,845-905`: constructors -> Opened passive/tool styling and search focus -> one reused preview -> placement -> Show(owner) only when hidden -> Hide with hover cancellation.
- `D:\Code\NxTools\Views\ClipboardPreviewWindow.cs:29-95`: no decorations/taskbar/activation, topmost, Opened applies passive native style. No link-popup equivalent or new legacy action claim.
- `rust/apps/nxtools-desktop/src/clipboard.rs:924-1315`: capture identifies/deduplicates/echo-suppresses, show/reuse calculates 420px collapsed bounds, native topmost/shadow/position/bounds, session generation, original row-close/next-turn application route. Existing browser route used Rust `cmd /C start`; local Shell route used correlated PlatformHost commands. Browser dispatch now uses the existing C# Shell Open URL capability, so synthetic queues intercept every open without launching a live browser.
- `csharp/NxTools.PlatformHost/Windows/ClipboardShellTarget.cs:94-98` accepts HTTP/HTTPS for Open; `WindowsPlatformAdapter.cs:929-1015` validates, runs native launch on STA, logs correlated start/stage/terminal elapsed/native code. Existing result/disconnect handling in ClipboardApplication remains owner of terminal logging; no payload URLs/paths logged.

## Selection record (released 0.7.1)

Complete official component index reviewed. Existing Link compound selection is stateless ListItem + StatusBar, not a ListState/controller transplant or window-internal Popover. Reviewed published `gpui-component-0.7.1/src/list/list_item.rs:44-139,170-276`: children/suffix slots permit a real action control without replacing session/focus; ListItem on_click is intentionally unused, original outer row remains sole opening dispatcher. Reviewed `src/button/button.rs:371-395,447-452,807-851`: Button label/accessibility_label, one on_click, keyed native focus/release and no implicit nested-click propagation suppression. Adopt XSmall ghost Button with explicit stop_propagation for each Copy link/path action; no appearance/default geometry suppression. Existing compact settings Form/Field retained; remove only the two approved obsolete gates, not unrelated controls or tile changes.

## Invariants

- Capture only identifies and presents; never dispatches open or writes clipboard.
- Opening: Web -> Browser (existing C# Open URL), LocalFile -> SelectInExplorer, LocalDirectory -> Open. Opening never notes a copied value or writes clipboard; neither legacy flag influences runtime.
- Explicit Copy link/path is a separate named XSmall button, one release/pointer callback, stops bubbling, removes window before next-turn echo mark/session forget/TestPlatform clipboard write. Copy never dispatches an open.
- Keep 420px width, 28px rows, 24px expander, 22px footer and exact 60/84/116px budgets; one keyed row-focus release owner and independent Kit copy-button focus. Long names truncate before the bounded copy action.
- Keep existing placement/topmost/shadow/options and one session window. Native/passive behavior is preserved, not certified by headless tests (existing show options say focus/show=true; no new native-policy claim).
- Keep default six-second configurable auto-hide and restaging. Timer ownership must not let an old capture timer remove a newer restaged capture; superseded timers retire without closing.
- Configuration retains readable obsolete fields under parent ownership; Link view/settings projection/setters no longer expose them.
- Failures keep original close-before-route policy; rejected/disconnected open does not fall back to clipboard mutation or reopen. Pending command/result/disconnect diagnostics remain correlated and payload-free, accepted is not native completion.

## Verification

Production regressions and final source/static results recorded below after implementation. Cargo/build/native/live Clipboard/browser/Shell/package/install/commit are prohibited for this subtask and remain parent-coordinated.
