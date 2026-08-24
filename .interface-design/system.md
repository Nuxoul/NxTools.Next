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
- Form sections place a fixed label/description row directly above the control it explains. A
  dominant multiline editor stays full width and fixed height with internal scrolling; supporting
  single-line fields follow below it, while exceptional security policy lives in a separate row.
  Input content never grows the settings page or moves the fixed footer.
- The settings view owns one local draft. Navigation changes presentation only; native close and
  Cancel discard the draft, while Apply keeps the existing validated persistence lifecycle.
- Quick Launch settings inside the main workspace use a stable two-rail layout rather than one long
  page: a 160 px local navigation rail and one flexible page viewport. The five pages are General,
  Index, Context, Commands and Diagnostics (常规、索引、上下文、命令、诊断); only the selected page
  is mounted in the content viewport, and changing pages resets that viewport to its top without
  mutating any setting.
- The Quick Launch local rail uses 40 px rows, a neutral selected fill and a 2 px accent edge. Its
  page header is one 48 px border-separated strip with Title 14/20 and Label 11/16 copy. The rail,
  header and module header remain fixed while only the selected page scrolls. This information-
  architecture slice preserves the existing immediate setting actions; it does not invent a draft
  footer or pre-empt the later Hotkey, volume, exclusion, command-editor and diagnostics redesigns.

### Auxiliary dialogs

- Schedule, confirmation and transient safety windows use one `DialogShell`: the shared 36 px
  client titlebar, one bounded body and one fixed 48 px footer. Dialog content never paints a second
  titlebar/footer boundary or invents a local action height.
- Footer actions stay on the 28 px action tier. Apply is the sole filled accent action; Cancel is an
  outlined secondary. Destructive confirmation remains neutral at rest and uses the danger surface
  only on hover or press.
- Native close and Cancel discard local work and clear the owned window registration. Confirmation
  closes only after the underlying operation succeeds, and all cross-entity callbacks defer to the
  next application turn.

### Hotkey field

- A global shortcut is presented by one reusable `HotkeyField`: a fixed 260 px rail with a 32 px
  inset recording control and one 16 px status line. The chord uses Mono Body 12/18; status copy uses
  UI Label 11/16. Changing lifecycle state never changes the field width or moves its enclosing row.
- Applied, native-waiting, recording, applying and failed states share the same geometry. A 6 px
  semantic marker and restrained border carry state: success for applied, warning for waiting,
  accent for focus/recording/applying and danger for failure. No state becomes a card or a row-wide
  accent fill.
- Clicking an applied or failed field begins recording; waiting/applying fields do not accept a
  second request, while the recording field may cancel through the existing Escape/focus lifecycle.
  The pending chord remains visible while Windows registration is in flight. A failed correlated
  result keeps the last applied chord and exposes the boundary error without mutating persisted
  settings.

### Volume scope list

- Local index scope uses one full-width, border-contained checkbox list instead of wrapping chips.
  The first 48 px row is the mode owner: `自动包含全部 NTFS 卷`, with a Label explanation that newly
  discovered volumes join the scope. Discovered volume rows are 40 px and keep one aligned checkbox,
  Body 12/18 mount point and Label 11/16 file-system/item metadata rail.
- An empty persisted volume vector means automatic-all. In that mode the automatic row and every
  discovered volume are visibly checked. Clearing any volume materializes the remaining discovered
  mounts as an explicit selection; selecting every discovered mount again canonicalizes back to the
  empty automatic-all representation, even if persistence still contains a stale unavailable mount.
  Selecting the automatic row also restores that representation. Because an empty vector is reserved
  for automatic-all, the final currently selected volume cannot be cleared from the checkbox list.
- Selection is communicated by the standard checkbox only. Volume rows remain neutral at rest and
  may use the shared hover surface; they never become accent chips, badges or cards. An undiscovered
  state stays inside the same 40 px list row so discovery never changes the surrounding page rails.
- Volume mutations keep the existing Rust persistence/indexer synchronization boundary and defer
  cross-entity application updates to the next application turn. One pointer gesture emits at most
  one persistence/indexer update, whether it lands on the indicator or the remaining row content.

### Master-detail settings

- Repeated editable identities use a continuous master list beside one detail editor. Master rows
  are 42 px, square within one enclosing boundary, and use Body name plus Label resolved context.
- Selection uses a neutral fill and a non-shifting 2 px leading edge. Disabled rows remain legible
  through muted text and reduced emphasis; they do not become disabled-looking cards.
- The detail editor uses shared 32 px field shells. Related fields form equal-width rows with one
  8 px gutter and aligned 16 px label baselines; descriptions truncate before they move controls.
- Changing the master selection only changes the visible draft entity. It never persists, validates
  or starts native work before the settings shell's existing Apply boundary.

### Settings matrix

- Dense numeric and boolean policies may share one two-column matrix inside a single section. The
  matrix owns one outer boundary, one title row and one quiet 1 px center seam; cells never receive
  independent card backgrounds, radii or elevation.
- Each matrix row has two equal flexible cells, a shared minimum height and aligned trailing control
  rails. Pairing follows task meaning rather than source-field order, so capacity, adaptive splitting
  and transfer limits can be scanned horizontally without creating a dashboard of mini cards.
- The matrix only subdivides its parent's width. It never supplies a minimum width that can stretch
  the settings viewport, and changing a value still belongs to the enclosing settings draft and its
  existing Apply/Cancel boundary.
- Window policy and taskbar-overlay policy use separate matrices. The task-window matrix pairs
  progress/completion visibility with completion feedback; the taskbar matrix leads with enable/edit,
  then pairs placement, dimensions and information visibility on stable trailing rails. Segmented
  modes, numeric fields and toggles remain ordinary row controls rather than becoming nested cards.

### External tool setup

- An external tool follows one vertical acquisition sequence inside a single settings section:
  configured executable first, managed acquisition second and public manual sources last. Every
  step owns the same full-width row and left control rail; actions are never stranded at the far
  edge of a mostly empty row.
- The configured path is a 32 px inset field with one 28 px icon picker. Managed acquisition is the
  only filled action because it completes the workflow in-product; cancellation stays quiet beside
  it. Public source links are outline actions and read as a final fallback, not peer primary actions.
- Status copy stays in the row that owns it. Progress, cancellation and failure may change text or
  reveal a sibling action, but must not resize the settings shell or move its fixed footer.

### Work surfaces

- Lists and inspectors use continuous surfaces separated by low-contrast rules.
- Empty/error/loading states preserve the same content boundary instead of introducing centered cards.
- Numeric transfer columns use Mono; user-facing Chinese remains in the UI font.

### Taskbar transfer overlay

- The passive taskbar overlay is a transparent telemetry rail: progress lanes own the accent, while
  optional speed, ETA, task/connection counts and transferred bytes use Label 11/16 with Mono
  numeric rhythm. It does not introduce cards, pills or a competing filled surface.
- Normal and edit modes share the exact content frame, lane capacity and information coordinates.
  Edit mode adds only an absolute accent boundary and the existing resize handle; borders, handles
  and pointer affordances never add padding, change typography or displace progress.
- Transfer ticks repaint GPUI only. Move/resize and native-close mutations retain the typed Windows
  boundary, correlation identity and deferred cross-entity lifecycle.

### Quick Launch popup

- The primary launcher is one reused 620 x 410 graphite work surface with a 42 px search header. An
  empty ordinary query collapses the native window to 44 px and renders no result surface; a
  non-empty query expands it to the full height without changing width or creating another shell.
- The search header is the focal control: UI font, Body 12/18, semantic text/placeholder/icon roles,
  an 8 px horizontal rhythm and one quiet bottom rule. The popup consumes `Theme` roles only; it
  does not add a search card, decorative accent fill or private fractional Chinese type size.
- Expansion motion runs only for the collapsed-to-expanded transition. Query-to-query result
  replacement keeps the expanded geometry and does not replay motion. The animation wrapper remains
  a definite flex container so the existing 40 px virtualized result rows cannot collapse.
- Ordinary application, file, command, calculator and URL results share one 40 px three-rail row:
  a transparent fixed 26 px icon slot, a flexible two-line copy rail and a fixed 48 px shortcut
  rail. No source may add a decorative icon tile, badge or alternate column geometry.
- Result titles use Body 12/18; subtitles and shortcuts use Label 11/16. The two copy lines use the
  2 px spacing token, while rails use the 8 px rhythm. Visible ordinary rows expose working
  Ctrl+1 through Ctrl+9 hints; rows outside that range retain the shortcut rail without fake copy.
- Ordinary results own one selected index and one neutral `element_selected` fill. Up/Down, guarded
  pointer movement and left-button press all commit that same index; a stationary pointer never
  steals keyboard selection, and a direct click never executes beneath a different visible row.
  Selection changes only semantic text/icon emphasis around a real Shell image; it never adds an
  icon tile, accent card, border or independent motion.
- Action mode is the same work surface and reuses the ordinary 40 px row shell: a transparent fixed
  26 px icon rail, flexible Body 12/18 title rail and fixed 48 px trailing rail. The only mode chrome
  is one 24 px Label 11/16 context line; it has no card fill, border, shadow or separate entrance
  motion. Shell submenu chevrons stay inside the shared trailing rail.
- Entering action mode freezes the parent result list, selected identity and scroll handle. Escape,
  root-level Left and empty-filter Backspace restore the saved query while action mode still owns
  input-change handling, then reveal that retained list at the exact prior offset on the first frame.
  Returning never clears the viewport to zero or launches a replacement search that later recenters it.
- Quick Launch preview is one reused, opaque, non-activating 400 x 410 window. Its fixed skeleton is a
  72 px identity header, an 8 px inset content rail and a 30 px path footer; content kind never resizes
  the native window. The root reuses the shared `PreviewShell`, while image, text and unavailable states
  reuse the shared neutral media surface rather than adding a card inside the preview.
- Preview identity uses Body 12/18 and Label 11/16; content and the path footer use the Mono family only
  where scan rhythm benefits. The media surface uses the shared graphite surface, subtle 4 px radius and
  contain-fit imagery. Quick Launch and Clipboard share these presentation primitives only—their layout,
  window ownership, sizing and pointer lifecycle remain independent.
- Changing an already-visible preview keeps the current shell present until the generation-matched payload
  is ready, then runs one 130 ms replacement entrance. It never plays an exit followed by another entrance
  for a target switch. Explicit pointer leave/dismiss may retain the 90 ms exit; stale load, stage or native
  callbacks remain guarded by the current entry/generation/window identity and never activate the preview.
- File-dialog search is a separate 500 px host-attached command rail, not a second launcher. Its 36 px
  search header remains anchored at the dialog bottom, while a 264 px result surface expands toward the
  host; the existing optional 36 px Quick Switch row remains available only for an empty query.
- Attached search uses Body 12/18 for input and result titles, Label 11/16 for paths and shortcuts, an
  8 px rail rhythm, transparent 26 px result icon slots and the same 40 px continuous result row used by
  Quick Launch. It adds no icon tiles, cards or unimplemented utility buttons.
- The attached surface owns one semantic window boundary; its client-decorated native HWND owns the
  restrained exterior shadow, which must not be faked inside the exact-size opaque GPUI root. Its visible
  width keeps a 16 px logical safety inset when the host is narrow, and its result overlap remains inside
  the owner at 96/120/144 DPI. The search header stays immediately below the owner bottom as in the
  reference. Host ownership, non-activation, first-show Topmost initialization and no-Z-order follow
  behavior remain unchanged.

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
- State toggles belong in the heading of the editor they govern. Ordering controls form one neutral
  group with edge-disabled states; destructive actions are spatially separated, remain neutral at
  rest and use the danger surface only on hover or press.
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
