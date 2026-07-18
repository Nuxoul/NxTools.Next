# Clipboard panel design QA

- Baseline/problem evidence: `C:\Users\Administrator\AppData\Local\NxTools\clipboard\images\81dba1d304a9da0d7161c1743d6ce02a029e9009bb54a81fba38bc20d22fdbaf.png`
- Product design source: the existing NxTools.Next GPUI/Zed palette, typography hierarchy and compact shell.
- Interaction/layout references (not pixel-copy targets): `C:\Users\Administrator\AppData\Local\NxTools\clipboard\images\ccb045a2747a53cf045e7ab58b0aa7ab72cd11f2acc25a3ee3380874267b95f0.png`, `C:\Users\Administrator\AppData\Local\NxTools\clipboard\images\7d4dc43ff13e3e2ca65e5cf77acf76d7c9636c8b24bcf8d6fe443411f29a1bd0.png`
- Implementation screenshot: unavailable; the user owns GUI execution and requested that Codex not launch the application.
- Viewport: 340 × 440 logical pixels.
- State: dark clipboard panel with mixed text, image and file history.

## Full-view comparison evidence

The supplied implementation screenshot showed redundant type chips, textual process-name chips, uniformly oversized rows, a tiny image thumbnail with unused space, strong card borders and an unnecessary paste badge. The revised code removes the duplicated type metadata, uses a quieter Zed-like row surface, renders source application icons with name tooltips, and uses a variable-height list for aspect-ratio-preserving image previews.

## Focused region comparison evidence

Code and reference measurements confirm 8px shell padding, 6px section rhythm, compact metadata, one-line text/file content and image preview height capped at 270px without upscaling. A rendered focused comparison is unavailable until the user captures the revised panel.

## Findings

- [P1] Rendered implementation evidence is missing.
  - Impact: typography, icon-font shaping, actual process-icon sharpness, image crop and final spacing cannot be truthfully approved from source code alone.
  - Fix: run the packaged build, capture the same mixed-content state at 340 × 440, then compare it beside the supplied screenshot and correct any remaining P1/P2 drift.

## Required fidelity surfaces

- Fonts and typography: code specifies compact 9–13px hierarchy; rendered glyph shaping remains unverified.
- Spacing and layout rhythm: variable-height layout and measured shell rhythm are implemented; rendered evidence remains unverified.
- Colors and visual tokens: original NxTools.Next dark palette is retained and softened with a new subtle border token.
- Image quality and asset fidelity: real clipboard PNGs and extracted process icons are used; rendered scaling remains unverified.
- Copy and content: redundant type/dimension/debug labels are removed; essential counts, time and content remain.

## Comparison history

1. Initial evidence: user screenshot identified P1 density, hierarchy and image-layout failures.
2. Fixes: variable-height GPUI list, adaptive image rows, simplified metadata, built-in GPUI component icons, real process-icon extraction and quieter Zed-style surfaces.
3. Post-fix visual evidence: blocked because the GUI was not launched by request.

## Implementation checklist

- Capture the revised panel in the same mixed-content state.
- Verify built-in SVG icons and source process icons.
- Check text/file row density and wide/tall image scaling.
- Fix any remaining P1/P2 differences before marking visual QA passed.

final result: blocked
