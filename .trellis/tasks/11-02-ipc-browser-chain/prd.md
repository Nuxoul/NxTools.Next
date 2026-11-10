# IPC and browser handoff lifecycle correctness

## Goal

Fix M7, L5, L6, L7 and L9 at their existing transport, browser adapter, Blob owner and extension-broker boundaries. Preserve all unrelated uncommitted work and the completed indexer/download/input/extension tasks.

## Requirements

### M7 — efficient bounded local pipe I/O

Measure ten seconds of idle reader wakeups or thread CPU before and after the change using isolated temporary pipes. Remove idle polling while retaining the absolute initial-frame deadline, EOF classification, cross-thread reader/writer ownership, existing writer behavior and frame bounds. Missing-pipe connection attempts must use bounded backoff within the remaining total deadline rather than spin. State the chosen read/cancellation model and its effect on both pipe handles and writes.

### L5 — transient accepts do not end the host

A client that connects then closes before ConnectNamedPipe must not permanently end download/indexer/platform admission. Recreate the affected pipe instance and accept the next client for documented transient outcomes. Keep unrecoverable creation failures actionable; bound consecutive retry delay and diagnostic rate without payload logs.

### L6 — truthful, compact request context

Verify README/spec meaning of includeRequestHeaders and includeCookies. Disabled request headers must not send authorization, cookie or headerText. Remove redundant Native Messaging fields unused by the host and carry cookies once. Verify old-extension/new-host and new-extension/old-host behavior under the existing compatibility contract. A32-item synthetic batch with4KiBCookie per item stays below the request bound; host validation/filtering remains correct.

### L7 — Blob ownership and popup transport lease

A fully staged Blob handed to a desktop confirmation remains available beyond the one-hour staging cleanup threshold. The original engine/desktop owner governs confirmation, final import and cancellation/close cleanup; orphan cleanup follows that owner at startup. A popup retains one native transport lease while open and releases it on close through the existing one-second idle policy. Ten seconds of fake-host popup activity starts the host once and presents stable connected/not-running state. Browser startup and diagnostics never start Native Messaging, passive snapshots never start Desktop, and no autonomous reconnect is introduced.

### L9 — long-lived bounded-line control stream

The broker control reader explicitly omits a cumulative stream budget, without using int.MaxValue as a sentinel. Other reader callers retain finite cumulative limits; the256KiBline limit is unchanged. A streaming synthetic source supplies more than2^31bytes of valid frames without retaining them in memory, and the reader continues successfully.

## Acceptance

- Each confirmed repair has a compiled/executed behavior-red regression before its production fix and a passing final result.
- Use existing real caller/transport/confirmation/background/fake-host boundaries with synthetic data and temporary isolated resources.
- Safely scoped IPC/download-IPC/BrowserHost/Indexer tests, CLI test check, affected Desktop handoff/index-bridge tests and strict Clippy; changed-file rustfmt; isolated C# fixtures; all Node tests and network-blocked temporary-profile DOM/popup gates.
- Report root causes, changed symbols, M7/L7 design decisions, compatibility, before/after measurement, red/green evidence, exact commands/results, deferred release commands and existing unrelated failures in outcomes.md.

## Constraints

Entry baseline138dirty status entries and20source copies are saved in session scratch/ipc-browser-chain-baseline. No reset, unrelated formatting, commit/push/archive, dependencies, extension UI redesign, page-hook trust changes or drop-target product changes. No packaging/install/product/service restart, real network/browser profiles/user data or Native Messaging registration. Indexer real-volume/process and packaged LocalSystem gates remain unauthorized; record exact release commands without executing them.

One Cargo executor; same-process jobs1/incremental0/codegen-units256 and offline mode, long operation timeout and residual-process inspection after timeout. Desktop test debug0 and one test thread. C# fixture selection follows reviewed TestEffects/Program options; DOM fixtures use separate temporary Chrome profiles and block network. Preserve the unrelated date-dependent downloads calendar failure and Archive formatting differences without repairing or suppressing them.
