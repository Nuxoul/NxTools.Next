# Download server observation

Current framework source lookup follows [Kit0.10.0/defaultpre0.3.12](../../../../docs/reference-code/gpui-kit-0100.md). Earlier versioned component evidence remains dated; original DescriptionList/Progress composition, size/variants, task/native and deferred-owner contracts are unchanged.

## 1. Scope / Trigger

Changes to HTTP/media response metadata, persisted probe projection or the task-window Server row.
Evidence: `repo:rust/crates/nxtools-download/src/http.rs`, `repo:rust/crates/nxtools-download/src/message.rs`, `repo:rust/crates/nxtools-download/src/storage.rs`, `repo:rust/crates/nxtools-download/src/engine/projection.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/task_window.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/task_windows_impl.rs`.
Reference: [Mister/ABDM/legacy behavior](../../../../docs/reference-code/mister-download-manager.md); [original download authority](../../../../docs/reference-code/download.md).

## 2. Signatures

- `DownloadProbe.server_observation: Option<DownloadServerObservation>` is serde-defaulted and omitted when absent.
- `DownloadServerObservation { host: String, peer_ip: Option<IpAddr>, route_kind: DownloadRouteKind, response_ms: u64 }`.
- `DownloadRouteKind::{Direct, ManualProxy, SystemPolicy}` serializes as `direct`, `manual_proxy`, `system_policy`; `diagnostic_name()` returns those fixed categories.
- `HttpDownloadClient::probe_timed` retains its existing request/deadline/cancellation contract. Private `observe_response(&Response, response_ms)` creates the observation.
- `DownloadEngine::task_probe(task_id) -> Result<Option<DownloadProbe>, DownloadEngineError>` clones original in-memory transfer metadata; no queue/store/segment IO.
- `StoredResolvedRequest.probe` retains the original complete probe in existing `download_tasks.request_json`. Schema remains13; original scalar columns remain legacy fallback.

## 3. Contracts

Host, peer and duration belong to the same successful response: selected Range, otherwise plain-GET fallback, otherwise HEAD. Each response's duration is its request-start-to-headers elapsed time, not the complete probe/companion wait. Dropped companion work cannot overwrite the accepted result. Route is captured from the immutable operation's client; no mutable config reread. Current ManualHttp uses the network owner's explicit proxy for all requests; System denotes policy, not confirmed use.

HLS/DASH probe and transfer runners use their actual redirected manifest response, captured before body reading. Media presentation says 清单探测端点; it does not claim all fragments use that host. No new HTTP request, DNS/Ping, socket race, global timer, GeoIP source or rendering-owned network operation.

Persist the complete probe under the original task/transfer transaction before events. A task-only upsert preserves the existing stored probe. Legacy scalar-only probes have no observation; opening/reading them performs no rewrite. Full and selected detail projections retain the probe; compact list summaries exclude it. The original task window retrieves missing compact metadata from `task_probe`, without inventing another cache or state owner.

Progress uses released0.8.0 Small borderless horizontal `DescriptionList` public label/value slots, released `Progress` and the existing bounded official `Tooltip`. The identity area is36DIP at the default root: the filename uses the14/20 title tier above12/16 body metrics, remains one bounded line with a full-name tooltip, and the Server row remains one bounded line. The collapsed440×184 body does not scroll; progress/downloaded bytes, speed and ETA remain primary. Status is owned by the title and is not repeated. Resume capability, effective limit, active connections, segment count and the semantic segment table remain in the original440×280 expansion. Original footer actions, one task/entity/HWND and failed/completed geometry are unchanged.

The Server row renders the accepted host, a short route label (`直连`, `代理`, `系统策略`) and a separate `{N} ms` value.0–100ms uses the active semantic success color,101–300ms warning and301ms+ danger. Missing observation renders the existing truthful unavailable/pending/unrecorded state and no time rail; it never fabricates0ms. The observation is the original HTTP request-start-to-response-headers duration, not Ping or RTT, and the Tooltip explains that provenance. HLS/DASH retain the `清单探测端点` prefix. Peer IP is not read by the renderer and must not appear in visible text, Tooltip, accessibility text or diagnostics.

Extend original correlated probe-completion diagnostics only with fixed route/error categories and elapsed time. Preserve original accepted/start/terminal/error observations. Never log host/IP/URL/path/headers/cookies, names, payloads, raw exceptions or render/byte ticks.

## 4. Validation & Error Matrix

| Boundary | Result |
| --- | --- |
| HEAD fails or companion exceeds grace | Existing selected response/fallback owns observation; no late replacement. |
| All attempts fail or operation cancels | Existing typed failure/cancellation; no successful observation or task fabricated. |
| Manifest redirects | Host/peer/time all use actual manifest response. |
| Peer unavailable | Keep `None`; render without invented origin IP. |
| Legacy missing field | `None`; scalar metadata remains usable without rewrite. |
| SQLite write failure | Original transaction failure; no metadata-before-commit publication. |
| Closed/replaced task window | Original generation/weak-window cleanup; no resurrected presentation or new network work. |

## 5. Good/Base/Bad Cases

Good: proxied redirected HLS response persists its manifest host, proxy peer, route and corresponding headers time; reopen reads the same task observation.
Base: old paused task retains filename/range data and shows 未记录.
Bad: pair final manifest host with the initial probe IP, label request latency Ping, fetch IP/GeoIP on render, or lose observation on task-only save/compact projection.

## 6. Tests Required

`repo:rust/crates/nxtools-download/tests/server_info_contract.rs`: actual local direct/redirect/manual/System HTTP and original request counts; selected HEAD/Range/fallback/time pairing; overdue companion; actual HLS/DASH probe and runner redirects; SQLite reopen/task-only retention/full-selected/compact exclusion/legacy unchanged; original correlated diagnostic payload exclusion.

`repo:rust/apps/nxtools-desktop/src/downloads/server_information_tests.rs`: production bounds/semantic text and reachable actions for missing/direct/manual/System/long host/IPv6/media/Blob; original compact-projection retrieval; close/reopen/cancel. Existing task-window, lifecycle, engine/HTTP/storage/IPC contracts remain required. Source/TestWindow evidence does not certify installed/native/DPI/user visual acceptance.

## 7. Wrong vs Correct

Wrong: `server = final_host + first_response.peer + probe_total_ms`, followed by another Ping when opening the window.
Correct: capture `{response.url().host, response.remote_addr, that_request_headers_ms, immutable_client_route}` once at the original successful response; persist and project it through the original task owner.
