# Rehearsal: NxTools whole-app theme study

## Composition

Gold: stageshow `panel-annotation` and `overlap` for the workbench/supporting-surface hierarchy; `raycast` for the search interaction. Gold wide/compact stills were captured and inspected. NxTools uses Windows caption assets, actual module names, Sarasa UI SC, neutral action roles and synthetic download/clipboard content.

The leading object is a usable-scale workbench. Supporting surfaces occupy quiet margin rather than obscuring content or actions. Medium width keeps the workbench composition; compact width uses a closer crop with complete focal controls, not a miniature desktop. The background is static and exists to reveal material, not to add motion.

## Stories

| Story | Cause | Payoff |
| --- | --- | --- |
| Search | Pointer enters the search field and types a synthetic query | Three results arrive once; search geometry stays fixed |
| Clipboard | Pointer rests on the selected record for the original 800ms intent delay | One preview appears, with the primary panel retained |
| Theme | Select Acrylic, then Apply | Workbench and supporting surfaces adopt one theme; color scheme/font remain unchanged |

Playback is once, initiated by Play/Replay. Stop and reduced motion show the complete poster. Explicit replay is the reset cause; there is no automatic loop seam or animated wallpaper.

## Verified

Environment: Windows 11 build22631; Chrome157.0.8103.79 for browser checks. These are browser results, not GPUI/native or device-DPI acceptance.

- TypeScript strict typecheck: passed for preview and copied stageshow kit.
- Static build: passed; scripts/styles are bundled in `preview.html`, without a CDN.
- Browser regression: **45 assertions passed**, no captured browser errors.
- Theme/color/light-dark independence and retained window DOM identity: passed.
- Overview families, three finite stories, Replay/Stop, fixed search geometry and unobscured Apply hit target: passed.
- System/preview reduced motion, immediate complete poster, no CSS animation under reduction, no automatic restart after reduction: passed.
- Host overflow and focal action bounds at1100/763/390/358px: passed.
- Keyboard theme activation and visible focus: passed.
- Host layout scan at1100/390px: no issues detected; 19/14 controls and20/8 padded containers inspected respectively.
- Final image inspection: dark/light overview, search payoff, theme-switch payoff, medium clipboard and compact clipboard. User acceptance is pending.

The repeatable regression is [check.mjs](check.mjs). Build and run instructions are in [README.md](README.md).

## Film evidence

Full recordings for all three stories at1100/390px and state recordings for Clipboard/Theme were taken. The final relevant layout gates have zero never-intentional findings and print:

```text
rehearsal passed (necessary, not sufficient: references/verify.md and the fresh eye in review.md still stand)
```

| Scene | Widths | Layout gate | Motion sampling |
| --- | --- | --- | --- |
| Search | 1100 /390 | Clear | Inconclusive |
| Clipboard | 1100 /390 | Clear | Inconclusive |
| Theme | 1100 /390 | Clear | Inconclusive |

The final Theme1100 state film sampled2032 frames across12.23s, with78 gaps above9ms and a largest gap of171.8ms. The film process returns **2**, not success, because motion inside gaps was not judged. Zero layout findings and the printed gate line do not convert incomplete coverage into a complete pass.

Recordings/reports remain in session scratch: `theme-search-{1100,390}`, `theme-clipboard-{1100,390}-delivery`, `theme-theme-390-delivery`, and `theme-theme-1100-handoff` (`.png/.json/.log`).

## Saved review frames

- [Dark overview](../../../.trellis/tasks/11-21-desktop-motion-acrylic-style/preview-dark.png)
- [Light overview](../../../.trellis/tasks/11-21-desktop-motion-acrylic-style/preview-light.png)
- [Search payoff](../../../.trellis/tasks/11-21-desktop-motion-acrylic-style/preview-search.png)
- [Compact clipboard](../../../.trellis/tasks/11-21-desktop-motion-acrylic-style/preview-clipboard-compact.png)
- [Theme switch payoff](../../../.trellis/tasks/11-21-desktop-motion-acrylic-style/preview-switch.png)

## Limitations

- Complete frame-by-frame smoothness is not certified by the gapped recordings. User visual approval and an independent visual review are not recorded.
- Native Acrylic, translucent UnderBorder edges, HWND hit testing/shadows, focus/IME/passive activation, Windows10 and mixed DPI are not tested here.
- No NxTools/PlatformHost/DWM/GPU/private-memory/power measurements were taken. Browser blur/timing is not an estimate of product cost.
- This is an authored theme study, not a feature-complete clone or pixel-equivalent capture of every current GPUI surface. Scene controls are illustrations; host preview controls are interactive.
- No product source, framework patch, dependency lock, configuration, service, package or installation was changed.

## Palette revision

Acrylic roles follow the product's one-translucent-layer model: window shell 72 %, panes 43 % over the shell (84 % composed), popups 97.6 %. Dark and light overview stills were re-captured in Chrome after the change: the wallpaper hue still bleeds through shell and sidebar, and text/dividers stay legible. Browser evidence only; the native surface matrix is in the task's surface-matrix.md.
