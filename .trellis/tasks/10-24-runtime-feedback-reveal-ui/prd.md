# Runtime feedback: floating visibility, Reveal confirmation and UI boundaries

User2026-10-24 reports: floating assistant loses highest/topmost visibility; two PlatformHost paths; currency target Select right edge clipped; unattractive selected/focus outer outlines in attached images; Quick Launch and Downloads Reveal opens Explorer but returns timeout/error. Goal repair proven boundaries, explain process roles and preserve real failure detection.

## Scope / acceptance
- Inspect user-provided screenshot copies and current native logs/process/service command lines read-only. Log summaries never contain payload text, paths, URLs/titles or credentials. Existing services/windows/data remain untouched.
- Floating assistant retains passive HWND/input layer, no activation, drag/drop/edge/fullscreen behavior. Verify initial topmost and recover lost native topmost using bounded existing observer; never hide on Shell desktop or nonfullscreen foreground. Respect intentionally configured fullscreen hide. Add production-boundary owned HWND regression before changes.
- Explain Updater\\Bootstrap --trusted-update-service versus Service platform adapter --parent-pid; do not kill/uninstall/merge processes solely by name.
- Currency Select stays actual official32DIPSelect inside460x320 feature shell; all label/caret/frame visible within parent/native bounds. Fix layout allocation, not expand native budget or repaint fake Select. Preserve input/selection/save/search/generation.
- Remove screenshot-reported redundant outer focus halo via existing released API where possible, keep standard control boundary and selected semantics. Preserve keyboard navigation/focus accessibility with a nonredundant indication. No blanket theme/vendor upgrade or all-focus deletion from guesses; unavailable image attachment/visual acceptance must be disclosed.
- Reveal: repair shared C# selection/active-view observation seam based on actual log diagnostics and a red-capable call-boundary regression. Parent/target/selectedfile/currenttab confirmation stays strict; do not treat S_OK/processlaunch/folderopen alone as success or simply extend timeout. Meaningful phase/count/HRESULT/attempt/elapsed diagnostics with one correlation; no payload logging. No actual Explorer navigation without separate authorization.
- Verification: reviewed safe synthetic C# fixtures only, Rust focused/fullDesktop/strict/workspace/fmt as relevant and serial workstation memory policy. No product/package/install/service/realclipboard mutation, commit or push in this turn. Preserve extensive unrelated dirty work and previous notification/floating fixes.

## Evidence constraints
Provided four source PNGs copied to scratch only. Browser panel unavailable; Paint observation returned no image attachment. Local Windows OCR reads synthetic copies only; it identifies currency/CNY and clipboard search, not aesthetic certification. Do not claim visual inspection or a numerical UI quality score without usable images.

## Native authorization amendment
User explicitly selected “允许，做完整原生验证” for one owned scratch Reveal target, exact native directory/selection readback, and closing only a newly created verified test-target file-manager window. Actual host is DOpus13.22; same owned target positive and unselected synthetic-file negative are authorized. This does not authorize normal product/package/install/service/realclipboard/userdata changes or unrelated file-manager navigation/window closure.
