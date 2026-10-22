# Design boundary

## State and source of truth
Existing history worker/connection remains sole storage owner. Add distinct summary projection APIs without changing existing full-body methods. A summary carries a ClipboardItem presentation snapshot plus full Unicode character count; its type is not a native-action payload. UI must hydrate active full content by ID before effects. Native formats are loaded in the same worker snapshot for action.

Thumbnails are disposable derivatives next to managed originals, never entries/native formats. Use an already-resolved image codec version if adding a direct dependency is necessary, PNG-only bounded decode, no enlargement, maximum640x288 to cover current capped list geometry at common DPI, transparent PNG output. One bounded worker/queue coalesces requests and invalidates old-root work. Cleanup must not delete originals; asset purge removes a derivative only after original is no longer referenced. No render-time filesystem checks or generation.

Runtime pause belongs to ClipboardApplication. Desired deadline/until-resumed, generation/operation ID, pending native command and applied state are distinct. Independent SetClipboardCapturePaused carries the runtime gate; ConfigureClipboard/hotkey failure cannot control this privacy boundary. C# sets the read guard on its event owner before acknowledging. Rust drops in-flight physical captures while paused/pending. Deadline uses one generation-guarded task; reconnect reapplies, restart defaults running without saving config. Command failure does not claim success. Existing permanent enabled and link/tool semantics stay unchanged.

## UI selection record
Source shape: existing compact history popup, not new workspace/split panel. Exact locked Kit0.7.1 controls: existing ListItem+img for rows; InputGroup+original InputState for search; Button::small().outline() for explicit pause/resume/retry and Popover::open/on_open_change/content for two pause choices; Label for durable runtime state. These controls are laid out directly, not Toolbar.child (which forces ghost+compact). Popover content creates no entities; cross-owner callbacks defer, closing returns original search focus. Default300x440/minimum280x280 remains. No new palette/window/preview owner.

## Files and exclusions
Expected: Clipboard crate lib/query/summary tests; Desktop clipboard_worker/thumbnail module, clipboard controller/capture_pause, ui/history/efficiency/capture_controls/summary_targets and panel tests; Protocol independent pause DTO, C# parser/EventWindow/pause helper+tests; independently scoped task/docs. Shared Cargo/Protocol/C# test registration writes preserve dirty baselines. Never touch Archive implementation or format whole workspace.

## Diagnostics
Derivative lifecycle: bounded batch/start+terminal with correlation, item counts, elapsed/fixed failure stage only; no file paths or image contents. Full-item action uses existing clipboard.action correlation, same pending-before-dispatch and terminal handling. Pause logs accepted/native-applied/failed/expired/reapplied with correlation/generation/deadline duration/elapsed; C# native failure codes remain native-owned. No frame/hover/key/byte-tick telemetry, no repeated cache-hit logs.

## Preserved interaction invariants
Reread legacy popup constructor/Open/Activated -> search focus; hide -> menu close -> preview hide -> native base Hide; paste beforeRestore callback. Keep Next existing failure-retry policy, one passive noactivate preview, generation/hover/cancel guards, final native client before reveal, and cross-entity next-turn updates. No change to ordering of native paste/hide in this slice. Thumbnail path never replaces original in ClipboardItem domain/IPC.
