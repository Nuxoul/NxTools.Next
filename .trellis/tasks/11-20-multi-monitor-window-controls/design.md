# Technical design

## Boundary
Smallest gaps: mismatched monitor identities/work-area selection; missing native activation confirmation for job surfaces; missing post-Commit progress admission; changed defaults; missing install-originated Apply; and absent total-feature switches. Keep original controllers, task IDs, generations, storage owners and native adapters. No new scheduler, window manager, registration framework or dependency upgrade.

Source evidence and external reference chains are recorded in [boundary research](research/boundaries.md) and the [canonical Listary placement review](research/listary-placement.md). All product decisions are resolved; implementation starts after final plan approval.

## Monitor and coordinate contract (R1/R2)
C# samples cursor device coordinates, HMONITOR identity, device-pixel work area and effective DPI together at summon. Typed Rust/C# events carry that snapshot. Rust computes logical content sizes/placement using the selected monitor's scale and work area; final absolute x/y remain device-pixel coordinates at the native command boundary. Never compare a monitor enumeration ordinal with GPUI DisplayId or locate an owner display by overlapping mixed-DPI logical rectangles.

Quick Launch follows Listary's default placement: x = work_left + (work_width - actual_outer_width) / 2, y = work_top + work_height / 4, clamped when the complete window would leave the work area. Use the current cursor monitor, not the foreground application's monitor or a display enumeration ordinal. Listary's nonvisible launcher is reconstructed; Next retains its existing reusable HWND and recomputes location on every summon. Clipboard stays near the summon pointer and clamps within that same work area. The first visible frame must have the final location and size. A reused preview follows its parent panel monitor, retains one passive HWND and updates in place. Existing source-bound notification preparation remains unchanged. Link detection captures a current presentation monitor via the existing native bridge; it does not hardcode primary_display or restore a stale prior capture location.

Use current public GPUI display/visible-bounds APIs where adequate; C# owns all Windows sampling. Add only typed snapshot data needed for these supported paths, including synthetic assistant-originated summons. Native noactivate/final-client-readback/show ownership stays intact. Disconnection, retired generation and superseded content cannot reveal stale windows.

Defaults change only at Core's owning defaults/serde paths: LinkPanelPosition::TopRight, NotificationPosition::TopCenter. Explicit settings survive ordinary section merges/reloads. Do not infer an explicit old-default value was unset.

## Foreground presentation (R3)
Keep the existing archive job/batch window. Capture its real HWND and surface identity after construction; on an intentional show/reuse or visible terminal presentation, defer a correlated ActivateWindow request through C#. Pending identity is installed before dispatch and matched against current job/surface/attempt on result. Record accepted/start/terminal/elapsed/native code; failed activation is observable and cannot be called success because GPUI activated locally.

Do not make task windows permanently topmost, alter passive previews/notifications, or activate on every progress tick. Explicit user background/close remains authoritative: no resurrection on ordinary progress/terminal events, and background completion uses existing notification policy. Keep-open, real terminal/cleanup, cancellation prompts, open-result and one batch surface retain their original owners.

## Download presentation/default (R4/R5)
Preserve ordinary StartNow submission presentation. Extend the existing pending automatic-presentation ownership to controlled-browser StartNow acceptance: retain task ID and source anchor while Held, open only when the same task first reaches an eligible active state after browser Commit. Consume once. QueueOnly, Held, aborted/removed and automatic-progress-disabled cases must not open. Duplicate updates cannot create duplicate windows; close is presentation-only and suppresses later ordinary progress resurrection. Explicit resume/retry should reuse the original request owner when starting work; verify this against existing product policy and regression paths.

Reuse DownloadWindowLifecycle and task_windows_impl for task-keyed generation, hidden create/final bounds/show, foreground confirmation, same-window completion and background cap. Source placement/show failures must be reported by existing pending-result ownership rather than silent success.

Set new-task default_connection_limit=16 through Core's existing default function and serde missing-field path. Verify the actual new-task DownloadRequest receives 16. Preserve explicit settings, including 8, and existing task requests; leave default-queue max_concurrent_tasks and CLI semantics unchanged.

## Automatic installed archive integration (R6)
Add one install-originated Desktop entry using Rust section persistence and the existing ArchiveApplication -> typed IPC -> ArchiveIntegrationService Apply. Run after runtime admission, trusted preparation and successful file finalization. Execute as original installing user, not LocalSystem or an alternate elevation account. The entry is required even for silent install and when normal launch is unchecked.

Fresh installation uses the existing recommended integration policy (cascade/Open With/default claim/supported extensions). Upgrade reapplication preserves previously configured extensions, menu commands and explicit opt-outs. Do not reset established settings to recommended values on every launch. Preserve original Apply/Revert controls as configuration and retry routes.

Wait for original platform connection readiness and a validated native terminal result. Preserve that acknowledgement across protected Desktop redirect and existing-instance forwarding. Use one install correlation ID through Rust/IPC/C#; expose genuine failure and user_choice_locked warning. Integration failure after package commit is not file rollback; package state and registration state remain distinct.

Reuse original complete capture/attempt-before-write/reverse compensation, retained approvals and applied ledger. Repair restoration at the same owner: failed default restoration retains retry evidence and returns incomplete state. Installer uninstall delegates recorded default choices to the existing native owner before retiring owned Classes values/ledger. Preserve later user-modified associations and exact ownership. No second raw UserChoice writer or whole key-tree deletion.

## Feature controls (R7)
Requested shape: one enable row at the top of each main Quick Launch and Clipboard page, outside the native titlebar, before existing Settings content. Exact component: Kit0.8.0 component::switch::Switch::new().checked().label().on_change(), Medium/default official visual size, with a semantic name. Plain parent layout only; no Toolbar.child adaptation, fake painted control or popup header action.

Callbacks accept the requested bool once, defer cross-entity work and persist only the owning section. Saving failure preserves prior durable/live state and exposes the existing error channel. Desired disabled state immediately prevents new local work; native pending/success/failure is represented truthfully and re-applied on connection recovery.

Quick Launch disablement gates queued/native/assistant summons, auto quick switch and file-dialog search without erasing their individual settings. Retire current panels/previews, query/preview/reveal generations and pending activation; re-enable restores existing preferences. Do not clear index data or change the shared service lifecycle.

Clipboard adds enabled to the original configuration/parser/native event owner: unregister its hotkey and skip capture admission when disabled, without disconnecting unrelated platform consumers. Native capture availability composes persistent enabled with existing temporary pause, preserving its intent/deadline. Local panel/capture/detection/currency/load/reveal completions check current enabled/generation. Hide all feature-owned transient surfaces and retain history/assets/settings. Main-page toggle remains usable while off.

## Expected files
- Core lib.rs plus defaults/config tests: R2/R5 and explicit-value preservation.
- Protocol lib.rs, C# parser/events/serializer and Windows NativeMethods/EventWindow: coherent monitor snapshot and Clipboard enable configuration.
- Desktop ui/mod.rs, quick_launch/panel.rs, quick_launch.rs, clipboard.rs and their existing preview/activation modules: selection, bounds, lifecycle and disabled admission.
- Desktop notifications tests only unless source-bound behavior reveals a concrete R1 defect; do not replace its native preparation.
- archive/jobs.rs and original job tests: job/batch foreground presentation and retirement.
- downloads/new_download_view.rs, browser_handoff_impl.rs, downloads.rs, task_windows_impl.rs and existing tests: pending post-Commit trigger/anchor and presentation-result correctness.
- main_window/quick_launch_page.rs, clipboard_page.rs and existing UI tests: official total switches.
- Desktop install entry/config/runtime IPC handling, archive/shell_integration.rs, installer/nxtools-next.iss and original integration/uninstall owners: mandatory install request and observed result. A new module is justified only to isolate this entry's real lifecycle, not a generic wrapper.
- Existing reference notes/spec owner topics and relevant regression gates: new verified chains/invariants.

## Compatibility and operational limits
Preserve current pending installer/runtime-dependencies and Program.cs changes, Kit0.8.0/pre0.3.9 and editor provenance. All final changes receive scoped diffs; no implicit commit, package, install, service stop or actual Registry operation. Safe synthetic tests are distinct from physical 100/125/150% mixed-monitor, original-user/elevated install/silent install and Windows default-app/Shell acceptance. Missing native acceptance is reported separately from compiled results.
