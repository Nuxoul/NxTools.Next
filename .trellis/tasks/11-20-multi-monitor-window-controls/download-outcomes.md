# R2/R4/R5 — Defaults and download presentation

## Download lifecycle

Browser confirmation preserves durable Held acceptance as distinct from start. Enabled StartNow retains one task-keyed presentation intent and captures the original confirmation anchor. The source closes on matching capture completion or bounded failure. Its stash key is distinct from explicit reopen placement. The first eligible post-Commit active snapshot consumes the intent once.

QueueOnly, disabled automatic progress, cancellation/removal and retired anchors do not present. Delayed work retains the existing passive cap. Duplicate progress cannot create another window or resurrect a closed one. Native capture, anchored placement and foreground confirmation use the original task/window/generation owners and handoff correlation.

Explicit Resume/Retry presents the existing keyed task window. Single-task list actions use foreground; bulk operations use the existing passive cap. Automatic retries and ordinary progress do not create windows. Existing scheduler, request snapshots, completion transformation and CLI behavior remain unchanged.

## Defaults

New/missing DownloadSettings.default_connection_limit is 16 per task. Existing explicit 8/custom values and already accepted requests remain authoritative. Queue max_concurrent_tasks remains 3.

Link panels default to TopRight, including the no-owner Settings fallback. Notifications default to TopCenter. Explicit positions survive section updates and reload; position draft/Cancel and failed save preserve the current live value.

## Verification

- Core106/106, Protocol120/120 and IPC73/73 pass, including omitted fields, explicit-value preservation and section reload.
- Controlled progress regressions: 2/2 pass through production confirmation, Held/Commit, localhost transfer, source ACK, keyed windows and no-resurrection boundaries. Actual new requests use16; retained requests use8/24 and queue concurrency remains3.
- Existing controlled handoff, download-window lifecycle and native-close filters pass.
- Actual Tools position selection, default notification preparation and global position Cancel/Save/failure tests pass.
- Download lifecycle, locked release/editor and spec source gates pass.

## Acceptance limits

Real Windows progress-window placement/foreground and installed browser acceptance were not run. The complete Desktop suite has unresolved failures recorded in the task's final verification report; these focused results do not certify the whole suite.
