# Feature Requests

Capabilities requested by the user.

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
