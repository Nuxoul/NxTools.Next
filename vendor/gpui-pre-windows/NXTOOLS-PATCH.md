# NxTools narrow Windows backend patch

User explicitly approved this exception2026-10-10 after the actual owned-HWND test returned MA_ACTIVATE despite WS_EX_NOACTIVATE. Source is the published Apache-2.0 `gpui-pre-windows`0.3.8 (Zed revision279fe070bb389b79652e52065b2f001edcc0b11b); LICENSE-APACHE is retained. GPUI Kit remains exact0.7.1 and its matched backend remains0.3.8. Cargo registry cache is never modified.

Only upstream behavior change: events.rs WM_MOUSEACTIVATE reads the HWND's current GWL_EXSTYLE and returns MA_NOACTIVATE iff WS_EX_NOACTIVATE is set. Otherwise preserves MA_ACTIVATE including ordinary/custom-titlebar activation. C# continues owning the policy flag; the existing in-process GPUI message owner interprets it. No focus stealing, disabled/click-through preview, cross-process callback injection or new Windows framework. No payload/high-frequency logging.

Regression: tests/native/gpui-kit-acceptance.ps1 checks real owned Debug HWND response3 with the flag; primary/unflagged window and disabled flag continue response1. Current Quick/Clipboard All each2/2 pass at96DPI. Other27 copied release files were hash-compared:only src/events.rs differs. This is a performed source-delta check,not a checked-in whole-vendor gate or physical-click certification. Keep this patch minimal when updating the backend; do not carry an unrestricted component fork.
