# NxTools.Next interface system

## Direction

- Audience: high-frequency Windows users managing downloads, search and clipboard history.
- Core task: recognize state, execute the next action quickly, and keep context while switching modules.
- Feel: a cold, precise graphite workbench inspired by Zed—compact, continuous and quiet.
- Signature: one shell grammar across modules; each domain keeps its own information topology.
- Reject: card walls, decorative pills, gradients, broad accent fills, arbitrary fractional Chinese sizes, and controls whose size changes with their page.

## Foundation

- UI/CJK font: `Microsoft YaHei UI`; monospace metadata: `Consolas`.
- Typography: Title 14/20 semibold, Body 12/18 regular, Label 11/16 regular, Meta 10/14 regular.
- Spacing: 2, 4, 6, 8, 12, 16 and 24 px only.
- Controls: compact 18 px, default 22 px, actions 28 px, inputs 32 px.
- Radius: 4 px controls, 6 px floating surfaces; continuous work surfaces stay square.
- Shell geometry: 36 px titlebar, 48 px module header, 208 px sidebar, 42 px sidebar row.
- Color: consume semantic roles from `theme.rs`; accent is reserved for focus, selection and the primary action.
- Motion: short state transitions only; no idle animation loop.

## Shared patterns

### Main titlebar

- One 36 px chrome row with the official 18 px NxTools asset, Body product title and Label module metadata.
- The drag region owns all remaining width; utilities never overlap it at the 920 px minimum window.
- Global settings is one shared 28 px subtle action, separated from a contiguous 36 px Windows-control group by 4 px.
- Minimize, maximize, restore and close use four distinct embedded 12 px SVG assets and visible Chinese tooltips; caption glyphs never depend on a font fallback.
- Minimize, maximize/restore and close keep typed PlatformHost behavior; styling never replaces the native lifecycle seam.

### Module header

- One 48 px continuous row with the shared 12 px horizontal inset.
- Title uses Title; bounded status/count metadata uses Label and may use Mono.
- Trailing controls form one 4 px action group; only the primary action is filled.
- Search belongs in the trailing group when it filters the current module.

### Sidebar navigation

- Fixed 208 px rail and 42 px rows.
- Body title and Label description share a stable two-line baseline.
- Selection uses neutral fill plus a narrow accent edge; no filled accent card.

### Work surfaces

- Lists and inspectors use continuous surfaces separated by low-contrast rules.
- Empty/error/loading states preserve the same content boundary instead of introducing centered cards.
- Numeric transfer columns use Mono; user-facing Chinese remains in the UI font.

### Actions

- Icon-only actions require a tooltip and stable 28 px visible geometry.
- Destructive color appears only for destructive actions and their terminal states.
- Cross-entity GPUI callbacks defer to the next application turn.

## Verification

- Real production GPUI windows must prove geometry, hit targets, state transitions and window ownership.
- Windows shell behavior remains covered at the typed PlatformHost/real-HWND boundary.
- Release screenshots are required at 100%, 125% and 150% DPI before a visual batch is accepted.
- Self-drawn GPUI surfaces are verified by production-window tests plus OS screenshots; UIA is supplemental and is not treated as an authoritative tree when controls are not exposed.
