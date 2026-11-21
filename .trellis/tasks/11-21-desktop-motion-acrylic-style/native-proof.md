# Native GPUI material proof

## Boundary

Deliver actual GPUI windows backed by Windows DWM, using the existing isolated UnderBorder fixture. The browser study is design evidence only. The native proof does not start the business runtime, read clipboard/search history, persist settings, register hooks, package or replace installed binaries.

Extend the isolated Cargo example with an opt-in native-material entry and a separate module. Preserve the existing search/capture entry, tests, ordinary preview binary and private source-over/UnderBorder provenance. Add one named candidate directory to the existing private builder; do not edit production vendor sources or Cargo.lock.

Preview-only C# owns native material, corners and one startup fixed-frame alignment. The launcher targets only its freshly launched PID plus exact preview title. Client screen origin/dimensions are preserved; GPUI owns logical layout/input/activation. No repeated show/hide, resize, global-focus or window-region controller is added.

## Surfaces and released components

| Surface | Actual components | Shape and adaptations |
| --- | --- | --- |
| Workbench | Component0.8.0 TitleBar, Sidebar/SidebarMenu/SidebarMenuItem, ListItem, GroupBox, Button, InputGroup/Input | 900x600DIP,176DIP rail; actual public Styled background slots; ordinary component variants retained, no Toolbar.child action transformation |
| Clipboard panel | TitleBar, InputGroup/Input, ListItem and Button | 480x540DIP; synthetic records and local selection only; stable InputState |
| Search | Original Large InputGroup with Search addon and stable InputState | 620x60DIP/r8, no extra transparent gutter for a whole-HWND backdrop |

Current/Solid and Acrylic use the same palette, font and synthetic content. F2 changes light/dark through the existing deferred theme/root boundary; Escape closes the owning primary window. The complete theme scope is unchanged; these are representative native rendering fixtures, not production adoption.

## Material boundary

Use the pinned Windows backend's WCA19 Accent policy: state4/flags0/gradient0x01000000/animation0 for Acrylic, state0 for Solid. DWM system backdrop remains None1, with one blur mechanism. C# records BOOL/native errors, corner preference33/ROUND2 and frame facts. Setter acceptance is not visible-effect evidence; the real foreground desktop composition must pass the controlled blur/transmission check. The proof targets local Windows11 build22631, with build22621+ required by its native boundary.

GPUI paints an opaque background in Solid and a tinted translucent background in Acrylic. Child controls keep readable component-owned surfaces. Transparent Base Root refinement is deferred; callback-local state changes never synchronously reenter an application/entity owner. No whole-window opacity or browser renderer is used.

## Regression and diagnostics

Before treating the slice as ready:

- Native readback mismatch guards, actual Accent apply/reset, dark-policy readback in both directions and fixed-frame normalization run on owned HWNDs. Client geometry, foreground/Z order and style/SysMenu bits are preserved; mismatched identities and THICKFRAME are rejected before writes.
- Actual GPUI rendering asserts same InputState and content after local selection and deferred theme change, bounded control geometry, intentional tint alpha and opaque Solid.
- Reuse the native source-over blend regression and original standalone component tests.
- Inspect actual native GPUI windows; GPU-only render_to_image cannot capture the DWM backdrop.

Events carry one correlation ID with accepted/start, native applied/failure, window close/finish and elapsed/native codes. Only fixed surface/style identifiers and native scalars are recorded; input text, titles, paths and user payloads are excluded. No per-frame or pointer streams.

## Acceptance limits

Native material and fixed-client frame evidence is recorded in [native verification](native-verification.md), with [run instructions](../../../docs/ui/native-material-preview.md). Full application adoption, production resize/saved-bounds/passive lifecycle, Windows10/mixed DPI, IME and CPU/private-memory/DWM/GPU/power comparison remain separate gates.
