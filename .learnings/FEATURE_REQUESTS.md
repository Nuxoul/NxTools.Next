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
