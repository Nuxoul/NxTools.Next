# Feature Requests

Capabilities requested by the user.

---

## [FEAT-20260802-003] browser-native-messaging-takeover

**Logged**: 2026-08-02T00:00:00+08:00
**Priority**: high
**Status**: resolved
**Area**: downloads

### Requested Capability

Connect and optimize the existing NxTools browser extension instead of creating a new extension,
while preserving its project identity and UI style. Browser downloads, context-menu actions, batch
resources and Blob data must reach the NxTools.Next download engine without manual testing.

### Resolution

- Reused the original extension identity and surface.
- Replaced the unauthenticated loopback command socket with bounded Native Messaging.
- Added durable idempotency, metadata persistence, browser-safe fallback and Windows registration.
- Added repeatable Native Host and Chrome for Testing automation with popup screenshot capture.

### Metadata

- Frequency: first_time
- Related Features: download_takeover, native_messaging, browser_automation

---

## [FEAT-20260802-UIAUTO] windows_ui_acceptance_automation

**Logged**: 2026-08-02T18:30:00+08:00
**Priority**: high
**Status**: resolved
**Area**: tests

### Requested Capability

Run NxTools in a background-friendly Windows acceptance session, drive UI actions such as opening
the download page and entering a test URL, capture screenshots, and produce assertions without
requiring manual acceptance.

### User Context

Repeated download and taskbar-overlay checks currently require the user to click through the UI and
report visual failures manually.

### Complexity Estimate

complex

### Suggested Implementation

Provide a checked-in Windows harness with an explicit scenario file, deterministic local HTTP
fixture, window-relative input steps, HWND/style assertions, screenshot artifacts and a machine-
readable result. Keep production input and download paths unchanged.

### Metadata

- Frequency: recurring
- Related Features: download_ui, taskbar_overlay, release_acceptance

### Resolution

- **Resolved**: 2026-08-02T18:53:00+08:00
- **Notes**: Added a checked-in JSON-driven Win32 harness, deterministic slow HTTP fixture and
  download acceptance wrapper. It drives the real GPUI windows, captures PNGs, asserts native
  visible/topmost state across a foreground switch, and verifies the final artifact size.

---

## [FEAT-20260718-001] clipboard-native-paste-actions

**Logged**: 2026-07-18T10:05:00+08:00
**Priority**: high
**Status**: in_progress
**Area**: backend

### Requested Capability

Restore previous NxTools clipboard parity incrementally, beginning with native-format paste and original/copy/plain/no-line-break actions.

### User Context

NxTools.Next currently captures normalized text/image/file representations and exposes only click-to-paste, losing rich formats and accelerated paste behavior.

### Complexity Estimate

complex

### Suggested Implementation

Carry a bounded native-format bundle through the C# adapter and protocol, persist it in Rust, resolve paste intent in Rust, and expose the actions through GPUI.

### Metadata

- Frequency: recurring
- Related Features: clipboard_history, clipboard_paste

---

## [FEAT-20260722-002] visual-index-scope-management

**Logged**: 2026-07-22T09:40:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Requested Capability

Replace manual semicolon-delimited index-volume and excluded-directory text fields with visual
controls: discovered NTFS volume toggles plus native directory picking and removable path entries.

### User Context

Typing and maintaining drive roots and long Windows paths by hand is slow and error-prone in the
main Quick Launch settings surface.

### Complexity Estimate

medium

### Suggested Implementation

Keep the selected scope in Rust configuration, render it through GPUI, and add one generic native
folder-picker command at the C# Windows adapter boundary.

### Metadata

- Frequency: first_time
- Related Features: quick_launch_index_scope, index_exclusions

### Resolution

- **Resolved**: 2026-07-22T10:05:00+08:00
- **Notes**: Added GPUI volume toggles, removable exclusion rows and a C#-owned native folder
  picker; semicolon-delimited index settings inputs were removed.

---
