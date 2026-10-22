# GPUI Kit0.7.1 native acceptance checkpoint

## Outcome
Executed owned Debug GPUI HWND acceptance at current96DPI on a single2560×1440 display. The C# client-vs-outer defect and historical released-backend passive-policy failure are repaired. User explicitly approved one repository-local Windows0.3.8 WM_MOUSEACTIVATE branch; Kit stays exact0.7.1. Current Quick/Clipboard All each2/2 PASS. Clipboard first-show/reuse confirmation controller8/8 and complete desktop519/519 PASS. Physical gestures,125/150/mixed-monitor,screen-reader/full product-tree and installed acceptance remain unverified.

## Scope and isolation
`ui/native_acceptance.rs` is Debug-only and enters through the existing explicit `--ui-preview foundation` interception before ordinary startup. Environment mode must be quick or clipboard. One primary with synthetic Input/Button and one hidden auxiliary reuse production QuickLaunchPreview/ClipboardPreview rendering with invalid application handles. No DesktopRuntime,real config/history/Clipboard capture,indexer,platform supervisor,network operation or service. This is production rendering/native adapter evidence, not complete product preview-generation/controller/IPC lifecycle acceptance.

`PassivePreviewAcceptanceTests.cs` validates exact fixture HWND/PID/executable and synthetic titles before Windows operations. Exact filters are registered in the existing C# harness; no unrestricted harness or DOpus test. `tests/native/gpui-kit-acceptance.ps1` validates real exit codes/exact RUN/PASS, bounded waits, environment restore and cleanup only of created exact-path processes. Initial scratch script serialized PowerShell string metadata and failed to retain process exit handle; the checked-in script fixes these evidence issues. One checked-in-script attempt hit shared-log ReadAllText IOException before native RUN; changed live-read to Get-Content, retained plain strings. These wrapper issues are not product failures or acceptance passes.

## Historical red / initial geometry green (superseded by approved repair below)
| Boundary | Before | After |
| --- | --- | --- |
| Quick preview final client400×410 at96DPI |384×402 |400×410 PASS |
| Clipboard synthetic text preview460×406 |444×398 |460×406 PASS |
| Same HWND hide/reapply/re-show | not initially tested | PASS,exact client and no foreground change |
| Invalid zero-width budget | not initially tested | reject PASS |
| Windows UIA input | API source only | actual Edit50004 named Synthetic search input,focusable PASS |
| Windows UIA action | API source only | actual Button50000 named Synthetic acceptance action PASS |
| Passive WM_MOUSEACTIVATE after production noactivate | concern from source | Historical FAIL:1/MA_ACTIVATE,expected3 in both modes; current approved repair PASS below |

The bounded mouse message proves the real HWND's activation response, not that a physical pointer click was executed. `foreground_unchanged` during query/show is not proof that clicking remains passive. UIA checks are actual Windows provider/tree evidence for synthetic controls, not reader or complete product-tree acceptance. No new images were captured or visually inspected.

## Client-size correction
Current Rust SetWindowBounds callers all send GPUI content width/height in device pixels: Quick panel/preview,Clipboard popup/link panel. C# previously sent those values directly as SetWindowPos outer dimensions. NativeMethods now compensates live outer/client frame insets, preserves existing outer x/y,validates widened native ranges,applies one frame-change/noactivate/no-zorder and uses final client readback for success. No new resize loop/worker/window budget/native ownership. Protocol documentation records the client-size contract without wire/schema/version changes.

Existing command dispatcher owns correlated terminal diagnostics; the native boundary returns fixed payload-free failures/native error codes. No animation/pointer tick instrumentation or temporary probes added. Current96DPI result does not establish changed-monitor frame inset behavior.

## Historical blocker and explicit authorization
Released gpui-pre-windows0.3.8 events.rs returnedMA_ACTIVATE unconditionally; no public Windows passive callback was available. Out-of-process C# cannot safely install an in-process callback pointer. User subsequently explicitly approved a narrow repository-local Apache-2.0 backend patch,not injection,click-through/disabled previews or focus stealing. Existing GPUI message owner now reads live GWL_EXSTYLE and returnsMA_NOACTIVATE iff WS_EX_NOACTIVATE;ordinary activation remainsMA_ACTIVATE. C# remains policy owner. Compared27 copied release files:only src/events.rs differs;license retained,registry untouched,Kit/backend versions unchanged.

Clipboard now creates first preview hidden,confirms noactivate→final device-pixel bounds→topmost→show using caller-ID pending-before-dispatch. Reuse reconfirms passive policy before existing offscreen/render staging,so failed initial setup cannot be bypassed. Each initial/reuse-policy step has3s deadline. Matching success/failure and timeout check saved HWND;replacement HWND is never hidden by old work. Generation/cancellation invalidates show and queues hide behind a possibly in-flight show. Displayed item is committed only after native show confirmation. No second owner/window or cross-entity synchronous update.

## Current executed repair checks
- Regression red: reused-policy timeout retained pending;passive-reuse-timeout-red.log records exit101. Shared bounded dispatch fixes it.
- Controller8/8 PASS:ordered acknowledgements,every initial native-step failure,cancel/stale show,initial/reuse timeout,reuse-before-move,old-HWND success/failure/timeout isolation.
- Desktop519/519 PASS,0 failed/ignored;strict production and --tests Clippy,workspace --tests,fmt and Debug build PASS.
- C# build0warnings/errors;Quick All2 RUN/2 PASS and Clipboard All2 RUN/2 PASS,exit0.
- Both passive HWNDs return3;primary and disabled-policy HWND return1;re-enabled policy returns3.
- Exact96DPI clientsQuick400×410/Clipboard460×406,hide/reapply/same-HWND reuse,invalid width rejection and synthetic UIA Edit50004/Button50000/focusable PASS.
-10 affected architecture gates PASS. Source gates are not physical or installed acceptance.
- No package/install/commit/push,ordinary product launch,user history/service mutation or system DPI change. Owned fixtures/harness exit and environment restores.

Logs: session scratch passive-presentation-focused-final.log,passive-repair-final-{full,lint,production,workspace,fmt,build,csharp}.log,native-acceptance-{quick,clipboard}*.log/evidence.json. Historical baseline logs and earlier check counts remain historical,not current certification. Reproduce using tests/native/gpui-kit-acceptance.ps1 -Mode quick|clipboard -Check All after rebuilding. Current message-policy green is not a physical pointer click,screen-reader/full-tree,mixed-DPI or deployed-product lifecycle claim.
