# IPC and browser handoff lifecycle correctness

## Ownership and boundaries

- `nxtools-ipc::local_pipe` owns native pipe waits, deadlines, EOF, current-user/security descriptors and transient admission recovery. Existing DownloadHost and Indexer admission stay the owners of sessions and commands.
- The extension projects request context once at the native wire boundary. `includeRequestHeaders=false` sends no captured headers or cookies; `includeCookies=false` excludes cookies even when captured. The enabled cookie-API fallback stays unchanged.
- DownloadEngine owns staged Blob confirmation, import, cancellation and startup orphan reclamation. BrowserHost owns only upload capabilities and unclaimed upload staging.
- A trusted popup runtime port retains the existing demand-only transport for the popup lifetime. Closing the port releases it; pending requests/full Blob operations retain their own leases. Browser startup and diagnostics remain passive. Disconnection never starts a reconnect loop.
- PlatformHost's broker control reader has an explicit unlimited cumulative mode. Per-run input/stdout budgets and the 256 KiB line bound are unchanged.

## M7 / L5 native transport

Use `FILE_FLAG_OVERLAPPED` for both ends and explicit event-backed `ReadFile`/`WriteFile` completion. Independent operation events preserve duplex writes while the reader blocks. Waits use the absolute initial-frame deadline and a signalled retirement event; cancel the specific pending operation and observe its completion before releasing buffers/OVERLAPPED memory. Read newline disarms only the initial deadline. BrokenPipe/NoData/NotConnected remain EOF. Writers must not toggle shared PIPE_NOWAIT mode after this change. Service writes retain one absolute 500 ms frame deadline and immediate retirement cancellation.

Replace the atomic-only retirement notification with the smallest shared signal needed by existing Indexer session ownership. Keep lease/generation, queue bounds, identity admission and join-before-slot-release unchanged. No monitor thread or idle polling is introduced.

When WaitNamedPipe returns NotFound, sleep at most 25–50 ms and at most the remaining connection deadline. Documented transient accept failures close that instance and recreate it with bounded backoff and coalesced fixed diagnostics. CreateNamedPipe and other unrecoverable errors retain current failure behavior. Apply the same transient classification to the Tokio platform accept boundary.

Measure ten seconds of the old idle reader before production changes, then the same fixture after them. Instrumentation remains test-only. Add behavioral tests for absent-pipe attempts, initial/trickle deadlines, EOF, duplex operation, idle retirement, bounded nonreading writes, frame bounds and connect-close-before-accept followed by a healthy client.

## L6 wire projection

Replace the unused header-text projection with `{headers,cookie}`. Headers contain no Cookie key; Cookie is carried only by the dedicated field. Apply both preferences before wire serialization and avoid cookie API reads when disabled. Single and batch submissions share this projection. Keep native protocol 2 and the 256 KiB bound. Verify old extensions remain readable by the new host and reduced new-extension messages remain readable by the old host; no schema/version change is needed if existing optional/unknown-field behavior permits it.

## L7 staged Blob and popup lifetime

Move complete staging into an engine-only `pending` subdirectory before it can be acknowledged as a confirmation source. Import and discard resolve that claimed location through the original engine commands; direct imports retain the unclaimed upload path. BrowserHost startup expiry scans only its original top-level upload files, not claimed pending files. A fresh engine startup removes pending orphan files because confirmation windows do not survive that owner process. Failed admission restores retryable consumer-not-ready input or cleans terminal unowned input without changing browser fallback. Existing destination compensation and no-replace publication remain intact.

Use the existing runtime sender trust policy for a named popup port. A lease is acquired once per open popup, released on disconnect, and never reconnects autonomously. Periodic passive snapshots use the retained host; after host loss the popup remains disconnected until an explicit demand/reopen, instead of relaunching every 1.5 seconds. Keep existing popup layout/actions and secret-free state projection.

## L9 cumulative budgets

Represent the budget explicitly (nullable finite limit or a named factory), never by interpreting `int.MaxValue`. Unlimited readers do not accumulate an integer lifetime counter. Finite callers retain overflow-safe remaining-budget checks, strict UTF-8, truncated-line errors and frame limits. Stream-generate over 2^31 bytes of valid frames with bounded buffers and exercise the broker's actual chosen reader mode.

## Diagnostics

Existing correlation IDs pass through extension, BrowserHost, IPC and engine. Log fixed stage/reason, terminal result, elapsed time and native codes only. Transient admission diagnostics are coalesced; idle waits, pointer movement, bytes, header/cookie/URL/path payloads are not logged. Ownership transfer/import/discard/orphan cleanup get bounded lifecycle events on their original owners.

## Verification boundaries

Use only synthetic requests, temporary files, temporary pipes/loopback, fake hosts and TestApp. No product launch, package/install/service/native registration/real profiles or network. Rust has one executor with the workstation environment and offline mode. C# fixtures are selected after effects review. Extension Node tests load the real modules; real DOM gates use an isolated temporary Chrome profile and block network. Native installed Indexer release acceptance remains separately authorized.
