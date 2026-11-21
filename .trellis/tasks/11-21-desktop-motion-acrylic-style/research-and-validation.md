# Source research and resource validation

## Current source facts

Paths are relative to the repository root.

| Owner | Verified entry | Consequence |
| --- | --- | --- |
| Theme | `rust/apps/nxtools-desktop/src/theme.rs:8`, `:214` | Current projection exposes color roles and applies the saved Kit theme/font; no separate product material selection here. Preserve this owner. |
| Windows background | `vendor/gpui-pre-windows-039/src/window.rs:916` | Public Opaque/Transparent/Blurred/MicaBackdrop/MicaAltBackdrop branches exist. Blurred calls accent state 4; it is not the documented DWM transient-backdrop branch. |
| Accent implementation | `vendor/gpui-pre-windows-039/src/window.rs:1628` | Build <17763 returns; dynamically resolves SetWindowCompositionAttribute, sends WCA 0x13, and discards BOOL. A selected enum does not prove successful native material or complete fallback behavior. |
| Mica implementation | `vendor/gpui-pre-windows-039/src/window.rs:1604` | Build <22621 returns. Sends DWM types 2/4; no implemented fallback in this function despite its comment. Material transitions/reset also require verification. |
| Product native owner | `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs:550`, `:728` | Existing shadow and optional window region adapters are separate from GPUI background selection. New production material policy must not create competing HWND writers. |
| Shared motion | `rust/apps/nxtools-desktop/src/ui/motion.rs:51`, `:68` | Deferred weak observer; shared 120ms entry and ease_out_quint. Exit/resize constants are not evidence that every surface uses them. |
| Quick results | `rust/apps/nxtools-desktop/src/quick_launch/result_motion.rs:50` | Query/identity-owned finite 120ms motion; virtual-scroll/reorder must not replay it. |
| Quick preview | `rust/apps/nxtools-desktop/src/quick_launch/preview.rs:442` | Generation-scoped 130/90ms transitions; preserve passive HWND preparation and search refocus. |
| Clipboard | `rust/apps/nxtools-desktop/src/ui/mod.rs:2561`, `rust/apps/nxtools-desktop/src/ui/preview.rs:199` | Main renderer has no shell tween; preview has finite 130ms entry. Existing item/query/session and original hover-intent owner remain. |
| Downloads | `rust/apps/nxtools-desktop/src/downloads/task_window.rs:81`, `rust/apps/nxtools-desktop/src/download_taskbar_overlay.rs:469` | Phase content uses shared 120ms; overlay 90/120ms. Original lifecycle owns hide completion. Reduced visuals do not currently remove its 120ms hide timer. |
| Markdown outline | `rust/apps/nxtools-desktop/src/tools/markdown_reader/outline.rs:326` | 160ms reveal honors reduced motion; original hover/focus region remains. |
| Markdown heading | `rust/apps/nxtools-desktop/src/tools/markdown_reader/viewport.rs:215`, `:474` | Ordinary heading navigation starts a 1200ms alpha decay with frame requests but no reduced-motion branch. Concrete in-scope policy gap. |
| Extensions | `rust/apps/nxtools-desktop/src/main_window/extensions_page.rs:214` | Deferred official Sheet; original modal/candidate/focus owner. |
| Notifications | `vendor/gpui-component-080/src/notification.rs:430`, `:781` | Official 400/200ms transitions and one lifecycle clock. Nothing mounted/persistent at rest does not arm a timer. No second clock. |

Local source review is not installed behavior or a performance measurement.

## Native materials

1. [Microsoft Acrylic material](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic): Acrylic is GPU-intensive and may increase power use. Guidance favors opaque vertical sectioning surfaces and transient overlays. WinUI Acrylic adapts to transparency settings, Battery Saver, low-end hardware and High Contrast; background Acrylic also changes when inactive.
2. [Microsoft system backdrops](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/system-backdrop-controller): Mica is opaque, samples wallpaper once and is intended for a persistent foundation. Acrylic is semi-transparent frosted material intended for transient/light-dismiss surfaces. The article's WinUI 3/Windows App SDK controller setup is not automatically provided to a GPUI HWND.
3. [DWM_SYSTEMBACKDROP_TYPE](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type): documented minimum Windows 11 build22621; MAINWINDOW means Mica, TRANSIENTWINDOW means Desktop Acrylic, TABBEDWINDOW means Mica Alt on current Windows 11. Effects may evolve with Windows.

Current GPUI Blurred is an existing candidate for an isolated proof, not certified Windows 10 compatibility. A future documented DWM path would need C# ownership and explicit supported-version fallback. Do not introduce WinUI/AppSDK, a new backend or dependency upgrade merely to produce a first visual sample.

## Composition questions for the prototype

- Does real backdrop remain inside the intended rounded contour, including transparent gutter and shadow? DWM material covers the window bounds; GPUI alpha corners alone do not prove native clipping.
- Does source-over retain the alpha needed by material composition, while UnderBorder removes duplicate edge coverage for the actual semi-transparent fill? Opaque tests do not answer the translucent case.
- Does switching solid → material → solid clear all native effects? Current code uses different native branches and does not establish transition cleanup.
- Does an unavailable effect yield the correct opaque layer rather than a transparent or black rectangle?
- Can the passive preview retain NOACTIVATE and original focus, including inactive material behavior?
- Does the existing platform event/snapshot owner provide the system facts needed for transparency, High Contrast and power fallback? Add only facts required by the chosen supported path; no periodic preference polling.

## Resource comparison protocol

### Control

Use real isolated GPUI/C# windows before product integration. Same executable build/profile, geometry, DPI, independently selected color scheme, font, synthetic content and open windows. Compare the existing complete theme with the complete acrylic theme; compare motion enabled/reduced separately. The acrylic theme covers all surfaces, with shared backdrop and surface roles rather than blur per control. Stageshow/browser frame rates and memory are excluded.

Warm each case, then perform three matched runs in alternating order. Suggested observation windows: 30s static idle, 30s changing background, 20 open/close cycles, and 30s interaction. Record OS/build, GPU/driver, refresh rate, DPI, window area and active/inactive state alongside the results. Do not mix Debug numbers with Release acceptance.

### Scenarios

| Scenario | Concrete failure it detects | Decision if observed |
| --- | --- | --- |
| Hidden/closed + static idle | New application frame loop, timers or retained resources after close | Fix lifecycle and confirm the full theme returns to idle |
| Visible static desktop | Unexpected continuous application redraw or persistent compositor load | Locate app versus DWM owner; optimize shared backdrop area and themed surface composition |
| Animated content behind window | DWM/GPU cost hidden by low app CPU | Compare area and backdrop paths; do not claim free material |
| Search typing/results/IME | Input latency, replayed result animation, stale/focus errors | Correct existing identity/input boundary; reject effect that delays input |
| Clipboard scroll/hover/reopen | Preview activation, stale reveal, growing HWND/resource count | Fix original session/cancellation boundary |
| Main shell/download progress | Large-area cost or progress-driven repaint amplification | Optimize persistent-window composition and progress invalidation within the full theme |
| Main + settings + popup + passive preview | Multiplied blur work, inconsistent theme state, stale saved palette or increased memory across windows | Share semantic theme state, avoid redundant backdrop layers, and fix window lifetime/resource ownership |
| Theme switch with open/reused/new windows | Partial adoption, reset color/font choice or lost input/focus | Update all surfaces through one deferred theme owner and preserve independent settings |
| Repeated style switching | Native effect not cleared, leaks, black/transparent fallback | Repair one material owner/reset path before adoption |
| Reduced motion/system transparency off | Continued presentation frames or effect contrary to preference | Correct independent motion/material policy |
| Battery Saver/High Contrast/remote session | Incorrect fallback or illegible content | Use verified supported fallback, otherwise solid |
| Windows 10 and 100/125/150% mixed DPI | Unsupported call, clipped corners/hit targets, focus/placement loss | Retain solid on unsupported path; fix supported geometry before rollout |

### Metrics

- NxTools and PlatformHost: CPU time converted to one clearly stated utilization convention, private committed bytes, working set, relevant handle/HWND counts. Working set and private memory are different figures; do not sum overlapping metrics.
- DWM: CPU time and GPU engine usage, matched to the same session/load; shared desktop activity makes unrelated GPU totals insufficient attribution.
- GPU: relevant process/engine activity and dedicated/shared memory where available. Task Manager snapshots alone do not establish a stable delta.
- Frames/input: frame-time median/P95/P99 and missed refresh deadlines using a suitable Windows presentation trace; input-to-visible-result latency on matching synthetic work.
- Power: report measured energy/power only with a suitable counter and controlled run; otherwise report it as unmeasured. GPU usage is not a battery-life percentage.
- Lifecycle: settle to baseline/no new presentation work; bounded resource counts after repeated hide/reopen and style switches. DWM may legitimately recomposite a changing background without application repaint.

Avoid arbitrary universal MB/% budgets before baseline variance is known. First separate repeatable differences from measurement noise, then optimize composition based on refresh-budget misses, interaction latency, idle work and sustained system load. Whole-app coverage is fixed: persistent and simultaneous windows require their own results, not extrapolation from a small popup. Unsupported transparency uses the acrylic theme's own opaque fallback roles, not the existing theme.

## Evidence and limitations

The reference image, actual theme/native code, motion owners and Microsoft guidance were reviewed. Browser design evidence is in [the study](../../../docs/ui/theme-preview/preview.html); actual GPUI/Windows material, frame and input evidence is in [native verification](native-verification.md). The selected native policy follows the pinned framework's WCA Accent path. No full-product material integration or process/DWM/GPU/power benchmark has run; local Windows11 build22631 evidence does not certify Windows10.
