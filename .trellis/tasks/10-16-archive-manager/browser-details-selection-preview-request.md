# Browser correction request — folders, details, marquee and internal preview

Status: approved scope implemented and compiled/isolated verified in browser-completion-outcomes.md. Previous partial outcomes remain historical. Native/physical/user visual acceptance and actual Windows integration apply are NOT RUN; no package/install delivery claim.

## User outcome (2026-10-27)

User rejects current screenshot: folders appear as tiny empty outline icons, metadata columns are missing, mouse rectangle selection is unavailable, and an internal previewer is requested. This later request supersedes the prior default-name-list preference; historical name-view reports remain unchanged.

## Confirmed current behavior

- `rust/apps/nxtools-desktop/src/archive/browser_icons.rs::key` excludes directory rows; browser row and Tree render16DIP Material folder outlines rather than shared native folder icons. The screenshot contains outlines, not completely absent elements. Desired recognizable folder appearance remains user acceptance.
- `archive/browser.rs::new` and `archive/ui_state.rs::UiState::default` set details_view=false. `shown_column` hides every metadata column in name mode. Detailed column data/render/sort and persisted visibility/widths already exist. Do not invent packed/original values for directories or unknown metadata.
- `archive/browser_drag.rs` handles row-down -> held4DIP move -> native virtual drag. BrowserModel is the only selection/focus/anchor owner. No marquee controller/rectangle implementation was found. Empty-body drag must be distinct from file-row native drag, scrollbar/header/splitter gestures and parent-row navigation.
- `archive/entry.rs` Materialize uses original source/snapshot/generation/worker/cache lease; current non-nested result sends lease-only Open to Shell. It is not an internal preview. Stale results release leases. Preserve password/consent/cancellation and exact cleanup boundaries.
- Existing `ui/preview_image.rs`, `ui/preview_text.rs`, `ui/preview_rich.rs` provide image geometry and bounded safe text/Markdown rendering, but ClipboardPreview itself is coupled to ClipboardApplication. Reuse safe underlying components, not a second clipboard/runtime or raw external-program launch.
- Existing archive PRD treated bounded image preview as optional; user's new request promotes an internal preview requirement, not automatic approval of arbitrary format decoders/dependencies.

## Clear requested corrections / proposed invariants

1. Recognizable folder icons in left Tree and right list, shared native icon cache where applicable; no archive extraction or member filesystem probe for icons; existing asset/official icon fallback remains visible while pending/failure.
2. Default detailed right pane: Name, Packed size, Original size, Type, Modified date; readable name width, aligned header/row geometry, original sortable/resizable controls and horizontal scrolling. Keep optional name-list choice; preserve existing user column widths/preferences, define explicit migration rather than resetting all UI state.
3. Empty-body rectangle selection with visible rectangle; ordinary replaces selection, Ctrl adds/toggles under a consistent baseline; retain existing single selection owner, keyboard Ctrl/Shift and native row drag. Account for scroll position/virtual rows; cancel stale gesture on folder/snapshot/refresh/close/Escape/focus loss. No second OLE loop.
4. Proposed internal preview: optional resizable right pane, single selection updates generation-scoped preview; double-click retains original open behavior. Original controlled materialization/lease is reused, never Shell Open for preview; background bounded decode/read, stale completion release, close/change selection cleanup, no network/script execution or silent external fallback. Missing/unsupported/too-large/error/loading are visible and distinct.

## Confirmed preview scope (2026-10-27)

User explicitly approved images and text/code/Markdown. PDF, Office, audio/video, executable launch and edit/write-back are excluded. This supersedes the prior pending-scope question; historical research and failed outcomes remain unchanged.

## Component/lifecycle review before code

Locked Kit/Base/Component/Assets0.8.0/defaultpre0.3.9 unchanged. Keep current official ListItem/public virtual-list approved multiselect fallback, Tree, resizable panels, PopupMenu, Root and original toolbar. Preview controls use official Small/Medium Buttons, existing image primitives/read-only document surface, Alert/Spinner/Empty; no fabricated Card/Grid or private vendor modifications. Exact relevant released sources and real regression boundary must be checked before implementation.

## Acceptance to lock after scope decision

- Default actual rendered pane includes all requested headers/cells with honest unknown metadata at minimum/default widths and Light/Dark; existing selection/sort/width persistence survives.
- Tree/list folder icons are visibly recognizable; pending/failure fallback and stale icon results tested, no member extraction for icons.
- Actual press/move/release marquee selects intersecting visible/projected items with scroll/modifiers, never dispatches native drag or opens a file; original row drag regression remains green.
- Internal preview selected-format content stays in app, has bounded resources, supports latest-selection wins and cancellation/close/lease release; no external launch or untrusted active content execution.
- Source/compiled/synthetic/native/user visual acceptance reported separately. No package/install/service/Registry/COM changes authorized by this request.
