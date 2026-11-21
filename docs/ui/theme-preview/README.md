# NxTools theme preview

Open [preview.html](preview.html) in the project browser. It bundles React, stageshow, scene code and styles; no CDN or product runtime is used. The bundled Sarasa font, app icon and Windows caption artwork are loaded from their existing repository paths, so keep the preview inside this checkout.

## Controls

- Theme: existing visual style or frosted Acrylic style.
- Color scheme: independent Catppuccin or Graphite study, with independent light/dark mode.
- Overview: Downloads, Tools, Settings, Markdown, and menu/detail surfaces. Auxiliary panels and notifications share the selected theme.
- Motion: search results, clipboard hover preview, and application-wide theme switching. Play/Replay starts a finite story; Stop restores its complete poster.
- Reduced motion: the complete payoff without playback. The OS preference is honored independently of the optional preview checkbox.

The settings and content inside a scene are authored illustrations, not live product commands. Only the surrounding preview controls operate this sample. All task, search and clipboard content is synthetic.

## Source and build

`src/scenes.tsx` composes stageshow Stage/Window/Row/Field/Button/Segmented controls. `src/main.tsx` owns host-only selections and explicit playback. `src/styles.css` contains the theme roles and wide/medium/compact compositions. The copied `kit/` retains the stageshow license.

With Node 22 or another supported current Node release:

```powershell
Set-Location docs\ui\theme-preview
npm install --no-audit --no-fund
npm run typecheck
npm run build
```

Build creates `preview.html`. For isolated tooling, set `NXTOOLS_PREVIEW_TOOLS` to an installed package directory containing the same dependencies; no product package or Cargo configuration changes are needed.

## Browser interaction check

Serve the repository root on loopback, then run:

```powershell
$env:CHROME_PATH = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
python -m http.server 4782 --bind 127.0.0.1
# In a second shell, from this preview directory:
node check.mjs http://127.0.0.1:4782/docs/ui/theme-preview/preview.html $env:PI_SCRATCH_DIR
```

The check exercises independent appearance choices, retained scene identities, each overview family, complete playback/replay/stop, system/preview motion reduction, responsive action bounds and keyboard activation. Captures and its JSON report go to the supplied output directory.

## Design boundaries

This is a visual theme study, not a pixel-equivalent export of every current GPUI screen. Stageshow parts express public component roles; they are not a replacement component library for the product. Windows caption assets replace the kit's macOS traffic lights within the same Window frame. Search sizing follows the approved larger search-surface study.

Real Acrylic composition, UnderBorder behavior with translucent fills, Windows 10, mixed DPI, native focus/passive previews, and CPU/private memory/DWM/GPU/power costs require separate GPUI/native validation. Browser blur and browser frame timing do not establish those results.
