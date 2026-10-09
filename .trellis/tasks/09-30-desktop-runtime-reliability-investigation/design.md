# Desktop Runtime Reliability Investigation Design

## Decision boundary

This task is an evidence-gathering and diagnosis effort, not an implementation task. The parent coordinates three symptoms while preserving their separate ownership boundaries:

- Quick Launch result visibility and animation: Rust/GPUI, with C#-owned native window visibility/activation and hotkey gesture delivery.
- PlatformHost process lifecycle/resource use: Rust DesktopRuntime supervisor owns child creation/restart/stop; C# owns the PlatformHost process's STA Windows message loop and native APIs.
- Clipboard capture reliability: C# owns Windows clipboard listener/read/staging/publication; Rust IPC receiver/worker/storage/UI owns downstream acceptance and history presentation.

No product behavior, queue size, retry count, process supervision, animation, or logging policy changes until the findings and proposed fix are presented for user approval.

## Investigation data flow

### Quick Launch visual lifecycle

```text
double-Control low-level keyboard gesture
  -> PlatformHost QuickLaunchGesture event
  -> Rust DesktopRuntime event dispatch / QuickLaunchApplication session generation
  -> GPUI panel show request and native window activation/visibility confirmation
  -> results_visible + results_appearance_generation
  -> 120ms generation-keyed result-surface entrance
  -> dismissal reason / hide acknowledgement / next session
```

Capture ordered timestamps for each boundary, window HWND/visibility/foreground state, session/generation IDs, and result-surface frame state. Never record query text, result labels, or paths. The visual oracle is a short screen recording/frame sequence synchronized to the lifecycle trace; a GPUI unit test can prove generation/transition invariants but cannot alone prove absence of a visible flash.

### PlatformHost process/resource topology

```text
launch route (shortcut/startup/manual/protected restart)
  -> NxTools.exe executable path/hash/session/parent
  -> DesktopRuntime::initialize
  -> PlatformHostSupervisor::launch_if_packaged
  -> selected PlatformHost path + --parent-pid
  -> supervisor restart/stop decisions
  -> PlatformHost process role, STA message loop, pipe connection
```

Inventory all NxTools PlatformHost processes by PID, canonical image path, hash, parent PID, command line, session ID, start time, and process role. Correlate each to a desktop instance and pipe endpoint. Sample CPU deltas, private bytes, working set, handle/thread counts, and child restart count at fixed intervals during idle, ordinary interaction, and controlled clipboard bursts. Do not infer a leak from working set alone. Do not terminate a process by name; any cleanup must match a verified full executable path and be approved/safe.

### Clipboard capture outcome ledger

```text
user copy -> Windows clipboard sequence / WM_CLIPBOARDUPDATE
  -> source context and exclusion decision
  -> native format/text/image/file read and capture limits
  -> optional retry/canonical rewrite
  -> publication-tail admission / staged payload creation
  -> bounded PlatformHost event channel
  -> PlatformPipeClient serialization/write
  -> Rust platform-event read/dispatch
  -> ClipboardCaptureWorker queue
  -> SQLite insert/dedup/commit
  -> GPUI query generation/refresh and visible history row
```

For each controlled copy attempt, create a correlation ID and record only safe metadata: sequence numbers, timestamps, format count/names only where non-sensitive, content-kind booleans, byte/pixel counts, decision/reason codes, queue depth/admission, IPC outcome, storage outcome/count, generation, and native error codes. Never record text, paths, window titles, or raw format bytes. The ledger must make the first missing transition observable.

## Hypothesis discipline

Maintain separate ranked, falsifiable hypotheses for each symptom. Examples to test, not assume:

- Quick Launch: result entrance restarts for a stale/new generation; native show/activation and first rendered frame are misordered; an immediate dismissal overlaps entry; panel status/results mount transition produces two visible frames.
- Multiple/high-resource PlatformHost: distinct legitimate desktop copies each supervise a child; a restart/parent-lifetime race leaves an orphan; startup/relaunch chain overlaps two desktop owners; clipboard/UIA/STA work or publication queue drives CPU/private-byte growth; unrelated processes were mistaken for PlatformHost.
- Clipboard misses: no `WM_CLIPBOARDUPDATE` observed; clipboard owner is temporarily unreadable and retries exhaust; content classification/capture limit excludes it; publication tail/channel/pipe rejects or disconnects; Rust worker/store dedup/rejects; visible query generation fails to refresh.

Each experiment changes one factor at a time and records a payload-free before/after artifact.

## Artifacts and ownership

- Parent: cross-symptom timeline, process topology table, per-attempt clipboard ledger, evidence-backed cause ranking, and user-reviewed proposed fix boundary.
- `09-30-quick-launch-result-panel-flicker`: visual reproduction and GPUI/native lifecycle findings.
- Existing `09-27-clipboard-performance-platformhost-crash`: PlatformHost CPU/private-memory/process topology and missed-capture investigation, appended without duplicating completed history pagination or image-format work.

All captured traces/screenshots and temporary harnesses live under the session scratch directory unless a reviewed, reusable test/tool is explicitly accepted into the repository. No temporary DEBUG logs remain in production code.

## Compatibility, privacy, and rollback

- Keep the Rust/GPUI versus C#/Win32 ownership boundary intact.
- Keep `CF_HDROP` and safe native clipboard format compatibility intact.
- Do not expose clipboard contents, credentials, file paths, window titles, or raw payloads in logs/artifacts.
- Investigation scripts must use exact executable paths and avoid destructive process/service actions.
- Since this phase makes no product changes, rollback is limited to deleting scratch artifacts and reverting task-document-only edits if the user rejects the plan; preserve all unrelated dirty work.
