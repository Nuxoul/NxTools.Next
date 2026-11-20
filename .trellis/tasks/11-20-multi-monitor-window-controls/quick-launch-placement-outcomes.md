# R1 — Multi-monitor presentation

## Placement and ownership

The Listary 6.3.5.94 managed launcher chain recorded in `research/listary-placement.md` is the placement reference: cursor monitor, actual outer HWND width for horizontal centering, top edge at one quarter of work-area height, and whole-frame clamping. Next retains its original popup HWND and session rather than reconstructing the launcher.

C# samples cursor coordinates, HMONITOR, work area and effective DPI together. Rust consumes the snapshot without treating GPUI DisplayId as an ordinal. Legacy events retain an ordinal fallback. Clipboard remains near the cursor; passive previews follow the actual parent display. Floating-assistant search carries the same sampled monitor facts.

## Confirmed presentation

`quick_launch/monitor_presentation.rs` owns one pending presentation under the original QuickLaunchApplication. Both first creation and reuse hide, stage offscreen, submit a frame, hide again, move to the target monitor, apply target-DPI client dimensions, read actual outer/client geometry, apply and verify final hidden bounds, then show passively before requesting the original native foreground/input-focus transition. Phase receipts have distinct command IDs under one operation correlation.

The native geometry query checks Desktop ownership and returns device-pixel facts; C# does not compute product layout. Session/HWND/connection checks, disablement, hide, disconnect and timeout retire delayed work. Previews cannot start from a primary panel still being presented.

The original QuickLaunchPanel desired-size policy retains confirmed work-area and outer/client frame allowances. Expanded results fit below the retained origin without overlapping the taskbar. Reopening clears these limits until the new monitor presentation is confirmed.

Link panels query the current cursor monitor, retain one passive HWND and use confirmed hidden target-DPI bounds before reveal. Clipboard previews remain passive and reusable. Archive job records retain the submitting owner's display/work area for new and reopened windows; operation forms and drag progress follow their existing parent owner.

## Verification

- Core/Protocol/IPC tests pass: 106/120/73.
- Multi-monitor snapshot/work-area/first-show/reuse/retirement: 6/6 pass at96/120/144DPI, including negative origins and expanded-panel clamping.
- `r1_popup_presentation_`: 3/3 pass, including actual application/GPUI hide versus delayed native receipt and expanded desired-size constraints.
- Assistant search prefill/reuse/session/live-edit tests: 2/2 pass with an explicit synthetic native HWND.
- Actual geometry serialization and Clipboard native enable/pause fixtures pass without live platform effects.
- Locked release/editor provenance, spec and relevant installer/archive source gates pass.

## Acceptance limits

Compiled TestWindow and native-fact fixtures do not certify first visible frame, real HWND DPI transitions, foreground/input focus or passive preview behavior on physical displays. Installed 100/125/150% multi-monitor acceptance remains separate. No package, installation, live Registry or service operation was performed.
