# Browser handoff lifecycle and external intake resolution

## Goal
One owner transfers a browser download at a time. Externally supplied Add Download requests resolve immediately without requiring Continue; the manual Download-panel Add entry keeps its current interaction.

## Requirements
- Control the actual browser item before desktop automatic intake. If control is not reliable, leave it in the browser and do not open a competing desktop download.
- Preserve the same handoff ID through pending confirmation, durable held acceptance, browser cancellation and desktop activation. Pending confirmation is not durable acceptance.
- User cancellation, desktop rejection/failure or expiry ends the pending desktop operation and restores the browser original. Late confirmation cannot start or recreate a retired handoff.
- Persist accepted desktop tasks with a handoff hold; no task/batch/queue/scheduled/restart path may start them before browser cancellation has succeeded. Commit is identity-bound and idempotent. Cancellation failure aborts the held task before restoring browser ownership.
- Recover extension worker operations using only secret-free session metadata; retain original browser/native/Cookie-store/POST/Blob/manual/batch protections. Never erase a completed browser file to hide duplicate ownership.
- External URL/link-recognition/overlay/browser-prefilled intake automatically begins the original cancellable resolution flow after the original window is established. Manual blank/clipboard-prefill Add remains explicit. Preserve edited drafts, window reuse, focus/placement and generation rejection.

## Acceptance
- Public Chrome/Firefox event tests demonstrate pause/control before prepare, confirmation no transfer, durable acceptance before browser cancel, activation only after cancel; cancel/refuse/timeout/disconnect/control failure and worker recovery never allow two download owners.
- Engine real SQLite and localhost HTTP tests prove held task cannot send transfer bytes before commit, including normal resume/queue/reopen; abort/late confirmation/duplicate/commit retry preserve identities.
- BrowserHost framed real pipe fixtures exercise new identity-bound commands and fixed diagnostics. Existing manual/Blob/batch/Cookie suites remain green.
- Production desktop window entry fixtures distinguish external auto-resolution from manual Add and verify cancellation/stale/deferred entity boundaries.
- Serial Cargo with prescribed memory variables; no packaging, installed registration, real website/credentials or unrelated worktree changes.
