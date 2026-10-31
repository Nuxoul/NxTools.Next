# Shared executable engineering contracts

## Scope and status
Required rules below are grounded in current storage, transport and native boundaries. They do not claim every old call site complies. [Known gaps](foundation-backlog.md) and package topics distinguish observed implementation from target. Evidence: `repo:rust/crates/nxtools-core/src/lib.rs#AppConfig`, `repo:rust/crates/nxtools-ipc/src/lib.rs#PlatformBridge`, `repo:rust/crates/nxtools-download/src/message.rs#DownloadFault`, `repo:rust/apps/nxtools-desktop/src/network_settings.rs`.

## Scenario: Correlated asynchronous operation

### 1. Scope / Trigger
UI/worker/IPC/native operation with delayed result, cancellation, reconnection or cached completion.
### 2. Signatures
`PlatformBridge::send_command_with_id(command: PlatformCommand, command_id: Uuid) -> Result<Uuid, PlatformBridgeError>`; `Envelope { protocol_version, id, correlation_id, message }`; notification CommandResult routes by command ID. Operation/session generation is owned by the caller/router, not inferred from payload text.
### 3. Contracts
Install pending state before observable dispatch; queue admission is not terminal success. Match command plus current generation/HWND/session as appropriate. On every terminal path clear all pending fields, elapsed clock and busy state independently; `take()` calls in short-circuit boolean expressions can leave state uncleared. Capture owned/immutable snapshots before async work; no lock/GPUI lease across reentrant cross-owner effects. Runtime queues are bounded, and retry/timeout policy remains owned by the feature.
### 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Disconnected/full queue/duplicate pending ID | Typed reject; no replacement/leaked pending entry |
| Matching result | Complete once, preserve accepted-vs-applied semantics |
| Old generation, closed HWND, stale query | Discard without changing newer state |
| Timeout/cancel/disconnect | Retire ownership, clear pending, no stale dispatch/close |
| Native process/API error | Stage/native code at native owner, retryable UI where contract permits |
### 5. Good/Base/Bad Cases
Good: pending installed before immediate native result. Base: generated-ID ordinary command. Bad: send then register, trust a stale result, or interpret timeout as pixels-ready.
### 6. Tests Required
Real immediate-result/queue-full/disconnect/generation seam; cancellation before completion; cache-hit callbacks; actual UI pending/close/focus assertions. Existing IPC caller-ID and desktop clipboard terminal tests are examples.
### 7. Wrong vs Correct
```rust
// Wrong: a fast result can outrun consumer state.
let id = bridge.send_command(command)?;
panel.pending = Some(id);
// Correct: register before the same ID becomes observable.
let id = Uuid::new_v4();
panel.pending = Some(id);
bridge.send_command_with_id(command, id)?;
```

## Scenario: Durable section-owned configuration

### 1. Scope / Trigger
Persisted settings edited by long-lived modules or worker migration.
### 2. Signatures
`AppConfig::update_section(path: &Path, mutate: impl FnOnce(&mut Self)) -> anyhow::Result<AppConfig>`; `save_to(&self, path: &Path) -> Result<()>`. Typed wrappers update downloads/network/system/Quick Launch/clipboard/blacklist.
### 3. Contracts
Lock sibling config lock, reread/parse latest disk, change only owned fields, normalize, write sibling temporary JSON, sync, close, rename, return merged cache. Ordinary clipboard save preserves latest storage root; migration worker alone owns changing it. Saved desired settings and applied Windows settings are separate.
### 4. Validation & Error Matrix
Missing/corrupt update target -> error, no resurrection/default overwrite. Commit failure -> old live storage owner retained. Native apply failure -> persisted intent may remain, live applied status must not claim success. First-run `load_or_create` is a separate observed initialization path, not the locked update guarantee.
### 5. Good/Base/Bad Cases
Good: another module's section survives stale cache save. Base: first startup creates defaults. Bad: whole cached AppConfig overwrite or migration publishes destination before durable commit.
### 6. Tests Required
Core stale-section/corrupt/missing tests and desktop delayed-migration settings-save race; real temporary files/SQLite, not helper-only assertions.
### 7. Wrong vs Correct
```text
Wrong: cached config -> whole-file save -> publish worker path.
Correct: locked reread -> mutate owned section -> sync/rename -> refresh cache -> native apply/result.
```

## Errors, logging and privacy
- Domain/library owners use their existing typed errors (`DownloadFault { code, message, retryable }`, ClipboardHistoryError, ProtocolError); composition uses anyhow context. Expected cancellation/supersession is not an empty result or an error toast. Maintain current code/exit schema, don't invent universal numeric codes.
- Never blanket swallow a failure that determines feature state; best-effort diagnostics cannot change feature result. Preserve source/native codes internally; user message must be actionable and payload-safe.
- New user-facing/cross-process operations define correlation/start/state/terminal/elapsed events. Native codes belong to the boundary that owns them. No clipboard text, URLs/query, path/file/window title, headers/cookies/credentials, child output or capability tokens in diagnostics. Length/kind/count/ID are enough; route_key is internal cache identity, not log-safe proxy metadata.
- No frame/pointer/byte-tick logs; coalesce or rate-limit. Rust init_file_logging uses daily/nonblocking sink and retained guard; C# uses bounded logging (see native topic). Error Display/Debug is not automatically safe: core anyhow path context and raw network errors need sanitization at diagnostic call sites.
- CLI output intentionally requested by a user may include task paths; it is not a diagnostic sink. Do not remove useful output on a false privacy premise; do not copy that output into logs.
- Root lints deny unsafe/dbg/todo and warn unwrap_used; verification uses -D warnings. Narrow unsafe in native/mmap owners needs documented lifetime/bounds. Existing narrow allows are not permission for new broad suppression; tests use contextual expect, not meaningless success assertions.
