# File-dialog inline search design QA

- Source visual truth path:
  `C:\Users\Administrator\AppData\Local\NxTools\clipboard\images\2eeac9e1f93f04696ba26a607451c229e7964c8b46ef7acda867e647db5e18e7.png`.
- Source behavior: Listary 6.3.5.94's fixed file-dialog search and Quick Switch behavior documented
  in `docs/reference-code/listary-file-dialog-quick-switch.md`.
- Supporting open-source visual reference:
  `D:\Code\SwiftList-main\App\Views\InlineSearchWindow\InlineSearchWindow.xaml`.
- Problem implementation screenshot:
  `C:\Users\Administrator\AppData\Local\NxTools\clipboard\images\d812568c1608ce765dfd29604aaf15977a769cd4d71386f00500f5f7213ca60a.png`.
- Revised implementation screenshot path: unavailable; the user owns native GUI execution and
  requested that Codex not launch the application.
- Viewport: source image is 1181 × 682 pixels; native display density is not encoded in the
  screenshot.
- Source fixed bar dimensions measured from the screenshot: about 500 px wide, 36 px search header,
  and 35–36 px default directory row.
- CSS size and density normalization: not applicable to this native GPUI surface.
- State: dark Windows Open dialog, empty query, fixed search bar attached below the dialog.

## Full-view comparison evidence

The source screenshot shows a two-row empty-query state centered on the dialog's visible bottom
edge: a 36 px search header and a 36 px selected directory source row. The reported implementation
capture shows the lower search surface horizontally offset from the upper dialog and three
nonfunctional action icons. The revised GPUI surface is 500 logical pixels wide, uses the two
reference row heights and contains no unimplemented header actions. Placement now reads both the
live owner and search HWND through `DWMWA_EXTENDED_FRAME_BOUNDS`, compensates the target HWND's
non-client insets, and applies one bounded visible-frame correction after resize.

Rendered post-fix evidence is unavailable, so full-view fidelity cannot yet be approved.

## Focused region comparison evidence

Code inspection verifies a 36 px search row, 11 px horizontal padding, one 14 px Material search
icon, 13 px input text, and a 36 px default directory row containing a folder icon, source name,
truncated path and `Ctrl+G`. The three nonfunctional star/history/manage-search icons were removed.
The precise edge attachment, typographic shaping, and native-DPI result cannot be judged without
the user's post-fix capture.

## Findings

- [P1] Revised native rendering is not yet captured.
  - Location: file-dialog bottom edge and GPUI search surface.
  - Evidence: only the pre-fix screenshot and revised source are available.
  - Impact: the DWM edge correction and compact density cannot be visually certified.
  - Fix: test the packaged build at the same dialog size and DPI, then capture the empty-query
    state and one populated-results state.
- [P1] The supplied implementation capture is horizontally misaligned.
  - Location: center line between the file dialog and the lower fixed-search HWND.
  - Evidence: user-annotated screenshot listed above.
  - Impact: the fixed search reads as a detached arbitrary popup instead of a dialog extension.
  - Fix: native placement now centers the final DWM-visible search bounds against the live
    DWM-visible owner bounds; requires post-package screenshot verification.

## Required fidelity surfaces

- Fonts and typography: compact 13px input and 10–12px result hierarchy are implemented; native
  shaping remains unverified.
- Spacing and layout rhythm: width, height, padding, duplicate border, and visible-frame anchoring
  were corrected; rendered evidence remains unverified.
- Colors and visual tokens: the existing Zed-derived NxTools theme is retained, with the search row
  using the semantic surface token and results using canvas.
- Image quality and asset fidelity: the surface uses the existing Material Symbols search icon; no
  raster imagery is present.
- Copy and content: the placeholder is “搜索文件、文件夹或应用”; empty-query default content names
  the captured directory and its source file manager.

## Comparison history

1. Initial evidence: the user reported late appearance, incorrect spacing/layout, and failed
   Explorer-to-dialog switching.
2. First fixes: child create/show recognition wake-up, bounded polling fallback, generic Explorer/
   Desktop/third-party source state, global search with preferred-directory ranking, and the
   two-row Listary-style empty state.
3. User correction: the lower surface remained horizontally misaligned and exposed three
   nonfunctional icons.
4. Current fixes: live owner/target DWM frame compensation, bounded post-resize correction,
   bounded third-party active-pane recognition retry, source retention independent of Auto Quick
   Switch, and removal of the dead header actions.
5. Lifecycle/performance correction: a transient foreground HWND during dialog construction now
   consumes only one bounded retry tick instead of cancelling the pending host. Root-window moves
   are coalesced through one immediately posted window message; child moves are ignored, and the
   already-bound GPUI HWND follows locally in the Windows adapter without re-running dialog
   structure/path discovery or round-tripping owner/visibility commands through Rust.
6. User retest still reported drag stutter. The captured 11:20:05–11:20:07 PlatformHost trace
   showed 14 redundant full recognitions of the same visible dialog, peaking at 11 scans/s and
   66.27 ms for one scan. The current correction reuses the already-recognized visible HWND before
   publishing another foreground event or traversing the dialog controls.
7. Second user retest reported that first display again required a foreground switch and that
   following still visibly trailed. The 11:26:56 trace proves recognition, owner assignment,
   placement, and native visibility all succeeded before the switch, so discovery was ruled out.
   The attempted Z-order workaround was invalid: the user confirmed that it caused Windows to
   switch foreground windows by itself.
8. Current correction restores `SWP_NOZORDER | SWP_NOACTIVATE`, defers the first native placement
   and show until GPUI has completed the hidden window's creation turn, and makes the C# host
   visibility event the single fixed-bar dismissal authority. Initial placement calibrates the
   target HWND frame once; subsequent drag events reuse those insets, read the owner once and
   execute one non-activating `SetWindowPos`.
9. User retest confirmed drag following is now smooth, but initial display still remained hidden
   until a foreground switch. The previously reviewed Listary leaf classes were incomplete
   lifecycle evidence: `SearchWindowBase.cs:26-33,333-349` additionally sets `Topmost=true` once
   and uses `ShowActivated=false`. NxTools now reproduces that one-time initialization while
   keeping all steady-state movement `NOZORDER + NOACTIVATE`; native transition logs record owner,
   topmost, cloaked, visible, foreground and final bounds for the next runtime trace.
10. User retest confirmed immediate display and smooth following. The next capture showed selected
    result fill stopping at the content width instead of spanning the 500 px list surface. The row
    renderer now explicitly uses the full available width, so selection and hover share one
    edge-to-edge surface. The user requested no standalone package for this one-line correction.
11. Post-fix visual evidence for the full-width selection: blocked until the next packaged build.

## Implementation checklist

- Verify immediate appearance on first dialog activation.
- Verify the bar touches the visible bottom edge at 100%, 125%, and 150% DPI.
- Verify `dialog → Explorer/Directory Opus/Total Commander/OneCommander folder → same dialog`
  performs one directory switch.
- Verify empty query shows the latest source row and both click and `Ctrl+G` navigate the original
  file dialog.
- Verify a typed query returns global results while direct children of the dialog's current
  directory receive a bounded ranking preference.
- Capture empty and populated states for the final comparison.

final result: blocked
