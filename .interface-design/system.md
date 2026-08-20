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

### Settings shell

- Product settings use one client-decorated shell: titlebar, stable navigation rail, one scrollable
  content column and a fixed 48 px footer. Changing pages never changes the window or footer geometry.
- Ordinary Chinese settings copy uses Body 12/18 for values and titles, and Label 11/16 for labels
  and explanations. Meta is reserved for bounded machine metadata, never used to shrink form copy.
- Text inputs are 32 px and ordinary actions are 28 px. Icon-only pickers are square actions with a
  tooltip; Apply is the only filled accent action and Cancel remains quiet.
- The settings view owns one local draft. Navigation changes presentation only; native close and
  Cancel discard the draft, while Apply keeps the existing validated persistence lifecycle.

### Work surfaces

- Lists and inspectors use continuous surfaces separated by low-contrast rules.
- Empty/error/loading states preserve the same content boundary instead of introducing centered cards.
- Numeric transfer columns use Mono; user-facing Chinese remains in the UI font.

### Work states

- Empty, progress and recoverable error states share one 72 px minimum row, 12 px inset, 18 px icon
  slot and 8 px icon-to-copy gap.
- Title/description always use Body 12/18 and Label 11/16 with a 2 px internal gap.
- Empty is neutral; progress uses accent only on the icon; error uses danger only on the icon/title
  and a 2 px leading edge. No state owns a card background, shadow or idle animation loop.
- Copy states what is absent or happening first, followed by one short next-step or scope sentence.

### Actions

- Icon-only actions require a tooltip and stable 28 px visible geometry.
- Destructive color appears only for destructive actions and their terminal states.
- Cross-entity GPUI callbacks defer to the next application turn.

### Notifications

- Main-window notifications form a bottom-right workbench layer with a 12 px client inset; they do
  not compete with the titlebar or the module's primary action.
- Each surface is 336 px wide, no taller than 112 px, uses the raised graphite surface, subtle border,
  small shadow and 4 px radius. A 3 px left marker carries progress/info/success/error color; the
  whole notification never becomes a semantic-color card.
- Title/message use Body/Label roles. Detail labels alone use Meta and detail values use Label Mono.
  Action and dismiss hit targets stay on the shared 28 px desktop action tier.
- Enter/exit motion remains limited to opacity and a 6/4 px vertical offset over 150/120 ms. At most
  three notifications are visible and every cross-entity dismissal is deferred with its rendered
  `(id, generation)` target frozen.

### Native tray

- The notification-area surface remains the Windows-native `NotifyIcon` and `ContextMenuStrip`; it
  is not recreated as a GPUI or owner-drawn menu.
- The desktop executable, taskbar and tray use the same NxTools brand asset. Release builds embed
  16/32/48/128 px variants; a missing product icon is a typed shell failure rather than permission
  to display the generic Windows application icon.
- Presentation is intentionally bounded: tooltip `NxTools`, then `打开 NxTools`, one separator and
  `退出`. Left click and Open restore the existing window; Exit keeps the existing explicit-exit
  lifecycle.

## Verification

- Real production GPUI windows must prove geometry, hit targets, state transitions and window ownership.
- Windows shell behavior remains covered at the typed PlatformHost/real-HWND boundary.
- Release screenshots are required at 100%, 125% and 150% DPI before a visual batch is accepted.
- Self-drawn GPUI surfaces are verified by production-window tests plus OS screenshots; UIA is supplemental and is not treated as an authoritative tree when controls are not exposed.
