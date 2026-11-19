# A — PlatformHost OneCommander native UIA memory lifecycle

## Goal

Repair the affected managed UIA native-memory ownership path in ordinary PlatformHost and elevated-directory helper without changing OneCommander pane/focus behavior. Source implementation is delivered separately from user-owned validation and independent acceptance. B6 timing remains unstarted.

## Incident evidence

Both reported roles approached20GiB private memory with a small managed heap; OOM stacks passed through managed UIA ListView proxies/COM marshalling. Both original startup roles called unconditional WarmUp. The complete native allocation/CCW share is inferred, not established by captured native allocation stacks.

## Repair requirements

- A-01: cover both startup roles, actual requests, target/window retirement, pipe disconnect, helper EOF and host disposal. No target/no actual demand must not create resident desktop-wide UIA observation.
- A-02: migrate all affected OneCommander managed System.Windows.Automation events, ordinary queries, anchors, raw-tree traversal, patterns and comparisons to native COM IUIAutomation/CUIAutomation8.
- A-03: reuse original HWND/process observations and native queries. Native focus registration is honestly global while enabled, lifetime-gated, with cached PID/HWND rejection before traversal; no invented per-process focus API.
- A-04: retain original bounded/apartment/event owners and physical slots after logical timeout/cancellation. Explicitly unregister, drain and release COM/client/handler/cache/pane/walker/pattern/native buffers and thread resources; no unbounded queue/replacement threads or wrapper process.
- A-05: preserve active/split panes, strong HWND-less WPF focus, capture before overlay, nearest CurrentPathGet/CurrentPathSet, read/navigation/inline-search and native activation/focus restoration in both roles.
- A-06: update the superseded startup-global managed subscription contract in system-shell/native specs; preserve unrelated Shell/clipboard/icon/STA and B1–B5 owners.
- A-07: low-noise correlation/fixed state/reason/count/elapsed/native-code diagnostics without path/focus text/payload secrets or event/byte spam.

## Acceptance ownership

The user owns verification. Source implementation does not certify native registration/removal/drain, actual COM/handle/thread/process retirement or sustained memory behavior. No build/tests/native fixtures/memory sampling were executed for this change, and no fixed long-measurement schedule is an agent execution prerequisite.

Acceptance remains not_accepted until the user/independent verifier confirms ordinary/helper no-target behavior, native lifetime, active/split/WPF pane behavior, cancellation, target/window/session/pipe retirement and memory stability. If growth remains, native allocation ownership requires further investigation; migration alone is not a repaired-memory result.

Do not launch real product/reference GUI, UAC or physical elevation as implicit validation. No real power/current Desktop exit, package/install, service/registry mutation, dependency/vendor upgrade, commit/push or B7.

## References

Read docs/reference-code/platformhost-uia-memory-repair.md and system-shell.md, then indexed actual SwiftList entries. The legacy NxTools checkout has no OneCommander source; Listary native engine was not available. Rust owns state/layout, C# Windows APIs, GPUI rendering. Existing capture120ms/navigation1500ms budgets and named physical ownership remain.
