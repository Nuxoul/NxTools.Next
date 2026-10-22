# Installed feedback: Reveal, tool activation and hotkey alignment

## Approved scope

User reports continuing Explorer target_not_confirmed toast on Open containing folder, Tools tile body incorrectly opens settings instead of toggling enabled, and Clipboard summon hotkey stacks below title instead of right-aligned. Screenshots supplied. Fix these three only; preserve accumulated dirty changes, no automatic packaging/installation/service changes.

## Evidence and boundaries

Current protected PlatformHost matches packaged SHA256 5BCD8D283765169C106269256092484DA37216C95A6016880BDA6A68CB3BEDAE. Payload-free Oct14 shell lifecycle confirms accepted native selection then target_readback/one fallback/target_not_confirmed after853.7093ms, correlation5f7aec30-a882-4c96-afe5-8b3650f9eb70 PID15888. This excludes stale host, missing target and foreground-only failure, not all readback causes.

References: download.md latest Explorer/ABDM/legacy completed Window construction and action chain; link-detector.md says legacy has no link tool equivalent (user now explicitly supersedes whole-row settings behavior); clipboard.md original MainWindow.axaml1460–1495 keeps recorder to right of label/status separately. Official complete GPUI Kit0.7.1 index required; inspect ListItem/Button and Settings/SettingItem/Form public slots. No dependency changes/private source edits.

## Invariants / files

Rust main_window/tools_page.rs keeps180x160card grid, one enabled snapshot and original deferred section-save owner; body pointer/Enter/Space toggles, Configure pointer/keyboard opens one keyed settings window without toggling/parent bubbling. No native show/hide/settings policy change.

Rust main_window/clipboard_page.rs keeps original recorder/input focus, suspend/ack/register/error/cancel, one action/owner; title-adjacent optional help and control right-aligned same row at default/minimum; Windows registration state visible below. Use existing official SettingItem public render slot/aligned_setting_row, not second settings controller.

C# ExplorerShellActivation.cs retains actual active visible view/exact selected path, result/correlation/source-window close, detached STA, bounded retries/fallback. Add red-capable actual native/COM chain regression rather than decoded snapshot only. Passive existing Explorer reads allowed for diagnosis only; no navigation/activation/select/close. Never report accepted launch as confirmed target.

## Verification

Actual production GPUI red/green card pointer/keyboard/config reuse/durable save and hotkey bounds+existing capture/cancel tests; exact safe C# native reader fixtures; Desktop full/strict lint/fmt and affected source gates. No real Clipboard/download/browser/config mutation. Live user action acceptance remains explicit; no automatic new package this turn.
