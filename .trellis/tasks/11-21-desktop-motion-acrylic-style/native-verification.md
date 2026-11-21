# Native material verification

## Outcome

Actual GPUI/DirectComposition windows display native Acrylic on local Windows11 build22631. The native window and GPUI client share one visible contour, without an external glass collar. This is an isolated representative rendering proof; installed/full-product theme integration has not occurred.

User visual acceptance: the native Acrylic appearance and single, collar-free contour are approved for the current proof.

[Run guide](../../../docs/ui/native-material-preview.md) · [Reference boundary](../../../docs/reference-code/desktop-materials.md)

## Checks

- Native DirectX source-over blend state:1 test passed.
- Actual GPUI rendering/input/selection/theme/close fixture:5 tests passed, including unchanged original search-surface regressions.
- Private candidate strict Clippy, including tests: passed. Standalone Debug executable build: passed.
- C# native boundary:114 assertions passed on build22631, exit0. Includes Accent apply/reset, native dark in both directions, wrong identity, actual captioned fixed-frame normalization, exact client origin/size, style/SysMenu/Z-order/foreground preservation, DPI context restoration, repeat stability and pre-write THICKFRAME rejection.
- Production framework/editor provenance gates passed; production vendor sources and root lock remain unchanged.

## Actual native evidence

| Surface | Client and visible window | External insets |
| --- | --- | --- |
| Workbench | 900x600 | 0/0/0/0 |
| Clipboard | 480x540 | 0/0/0/0 |

The fresh Clipboard launcher receipt reports `FrameClientGeometryPreserved=true`, `FrameForegroundPreserved=true`, `FrameClientOnly=true`; requested Accent4 accepted with native error0, system backdropNone1, corner preference2 and all DWM HRESULTs0.

The controlled desktop-pixel check on the client-only Workbench reports stripe-edge P95 **0.286/255** and broad background luminance range **13.104/255**. Criteria are edge<=4 and transmission>=3. Fine pattern smoothing and retained broad color are both established; ordinary transparency and fully opaque fills are rejected by this check.

- [Workbench desktop composition](native-workbench-client-only.png)
- [Clipboard desktop composition](native-clipboard-client-only.png)

The real stdout/native session received F2 requests and independently read back the corresponding native dark values. A fresh Search instance retained its existing UIA input value through F2 and exited normally with code0 after owned normal-close cleanup. The normalized Clipboard's actual X-button click closed its exact HWND, with child exit0 and clean session disposal.

## Limitations

- These are fixed-size representative views, not the full production layouts or settings integration. Production resizable-frame, saved-bounds and passive-window lifecycle acceptance remains separate.
- Windows10, other builds, mixed DPI and all supported material fallback conditions are not certified locally.
- Literal native automatic text injection did not match its sent string; full IME/physical-input acceptance is not recorded. Existing content retention through F2 was observed independently.
- Native Escape automation was blocked by the tool's missing cancellation tree and sent no key. GPUI Escape tests pass; actual normal-close/X behavior was verified separately.
- Native observation tools returned image metadata only; saved evidence uses foreground/PID/title-checked desktop composition capture.
- CPU/private memory/DWM/GPU/frame-time/power comparison is not measured. No installed package, configuration, service, clipboard capture, indexer connection or business runtime was started.
