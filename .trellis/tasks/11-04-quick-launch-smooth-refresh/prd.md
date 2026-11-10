# Quick Launch smooth refresh and first-result selection

## Scope

User requests Listary-informed visual logic, smoother query refresh without a locked/disabled-list appearance, and default selection of the first result.

Keep the original QuickLaunchPanel, InputState, query generation, 65ms coalescer, current-result early publication, action/preview owners and native window geometry. No indexer/ranking changes, new provider, release packaging or dependency changes.

## Acceptance

- A new ordinary query resets selection intent to default-first. Default selection follows rank zero as current batches arrive.
- Explicit keyboard/pointer selection is retained by result identity within the same query, including an explicit choice of the current first row. It does not carry across input changes.
- Retained old-query rows keep readable text and a stable default selection presentation while remaining ineligible for Enter, shortcuts, pointer execution, actions and preview.
- Pending search does not remove/repaint the whole result list as disabled. Existing empty/loading/error distinctions remain.
- Applications without an explicit provider subtitle use one centered title. Ordinary file/directory results use their parent path; explicit command/extension subtitles remain intact. Full action payload and accessible labels remain unchanged.
- Preserve current 40 DIP row body, optional 4/0 DIP external gap, window dimensions, theme tokens and icon/action ownership. Raise title hierarchy within the existing row rather than adopting all Listary dimensions in this slice.

## Implementation boundary

- `quick_launch/panel.rs`: selection intent and retained-row rendering; local title/subtitle composition.
- `quick_launch/retained_text.rs`: immutable display-only subtitle projection.
- `quick_launch/refresh_tests.rs` and existing affected tests: actual panel Input/replace/selection/paint and immutable-display regression.
- Reference/spec documentation: record the user-approved refresh/selection amendment and verification limits.

Component review: locked Component0.8.0 `ListItem` exposes children, selected and disabled but does not own the retained query/selection lifecycle; disabled also changes foreground and suppresses selected fill. Keep the existing focusable-row public Base Button boundary and existing row content shell for this scoped fix, preserving once-only keyboard/pointer dispatch and existing preview handlers. No Command/ListState replacement, Toolbar adaptation or vendor patch. InputGroup remains unchanged. Existing 40/26/48 rails are retained; title becomes 14 DIP with optional secondary 12 DIP text. Ordinary-state color comes from existing theme tokens.

## Reference and invariants

`docs/reference-code/quick-launch-listary-results.md` records inspected Listary6.3.5.94 managed sources and BAML templates. SearchMode preserves committed results during query work and defaults new results to the first selectable item; AppPathSearchItemViewModel omits subtitle; PathSearchItemViewModel uses parent path. Keep primary focus, hidden/show/hide ownership, passive generation-scoped preview, current-query action guards, failure status and deferred cross-entity callbacks. Existing correlated search/publication diagnostics remain; no additional per-key, frame or pointer logs.

## Verification

Run new regression before implementation, then targeted Quick Launch tests and relevant lifecycle/release/spec gates. Rust verification inherits the required single-job/no-incremental/codegen256 environment. Compiled TestWindow paint and synthetic interaction are separate from physical keyboard/IME, installed-product or user visual acceptance.
