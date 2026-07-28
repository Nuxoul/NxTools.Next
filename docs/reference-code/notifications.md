# Notification UI reference index

Last verified: 2026-07-20.

## Zed reference

Root: `D:\Code\zed-main`

| File | Verified responsibility |
| --- | --- |
| `crates\workspace\src\notifications.rs` | Replaces notifications by stable ID and schedules auto-dismiss. |
| `crates\workspace\src\toast_layer.rs` | Renders a dedicated overlay, isolates pointer input and owns dismiss timing. |
| `crates\notifications\src\status_toast.rs` | Compact surface, optional status icon/action and explicit dismiss affordance. |

Verified rendering details:

- status is communicated by a small leading icon instead of a fixed-width text badge;
- the dismiss affordance stays in the header action area and does not participate in message layout;
- an optional action is rendered after the message and owns its click handler;
- content remains compact, while the toast surface expands only when secondary information exists.
- Zed's toast layer applies a one-shot entrance animation and keeps dismiss timing in the owning
  lifecycle object; NxTools.Next extends that lifecycle with a short exit phase so removal never
  cuts the surface off abruptly.

## NxTools.Next contract

- transient operation state must not be appended to settings descriptions or change settings-row
  height;
- a reusable Rust `NotificationCenter` owns notification identity and lifecycle; feature modules
  publish typed progress, success, information or error notifications;
- GPUI renders a top-right overlay above every main-window module; C# has no notification UI role;
- publishing the same ID replaces the previous state so progress becomes success/error in place;
- persistent progress never schedules frames, completed notifications auto-dismiss, and every item
  has a manual close action;
- the visible stack is bounded to three newest items and stale dismiss timers may not remove a newer
  replacement with the same ID.
- completed file operations carry structured key details instead of burying paths in prose;
- database backup/compaction notifications expose the actual backup directory and an `打开目录`
  action through the existing Rust-to-C# shell command boundary;
- the close button is anchored to the notification's top-right corner and never shifts with title,
  message or detail length.
- appearance and disappearance are separate one-shot transitions keyed by notification generation;
  neither transition repeats or requests frames after its short duration ends.
