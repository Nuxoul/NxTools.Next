# Mister Download Manager reference

Source: `D:\Code\misterdownloadmanager-main\misterdownloadmanager-main`.
Verified 2026-11-09. Download behavior authority remains [ABDM and legacy NxTools](download.md).
Mister is GPL-3.0-or-later (`Cargo.toml:5-10`); Ghost is GPLv3. Behavioral reference only: no code, comments, tests or assets are incorporated into NxTools.Next.

## Server information

| Source entry | Verified behavior |
| --- | --- |
| `crates/engine/src/server.rs:18-74` | `ServerSeen::answered` takes the successful response's `remote_addr`; shared per-download state retains its fastest new-connection sample. |
| `crates/engine/src/server.rs:130-167` | `TimedService::call` times connection creation, subtracts DNS time; HTTP divides by one and HTTPS by two to estimate RTT. It sends no ICMP or periodic Ping. |
| `crates/engine/src/server.rs:78-103` | Selected response-header patterns identify known CDNs, not arbitrary HTTP Server software. |
| `crates/app/src/whereis.rs:16-55,117-189,266-303` | IP, estimated RTT, proxy flag and CDN; local country data; medium is60..149ms, far starts150ms. Main line shows place/proxy and approximate RTT; tooltip shows IP. |
| `crates/engine/src/download.rs:1301-1407` | Proxy flag follows chosen System/Manual/PAC route; the peer can be the proxy rather than the origin. |
| `crates/app/src/model.rs:387-400,691-705` | Server observation is persisted with Item and refreshed from engine snapshots. |
| `crates/engine/src/dns.rs:567-739` | Separate multi-address TCP racing starts candidates250ms apart, total3s, caches winner5minutes. This adds TCP attempts; it is not the Server row's Ping. |

The estimated RTT assumes handshake round trips. TLS1.2, CONNECT and SOCKS violate the simple HTTPS divisor; fastest timing and latest peer need not identify one connection. VPN is not reliably classified. Do not call this a measured ICMP/TCP Ping.

### Progress lifecycle

`crates/app/src/app.rs:2263-2397,4636-4654` owns one `Progress` per task ID; reopen restores/raises it. `crates/app/src/progress.rs:245-387` creates hidden, applies theme, then shows; X/Escape remove presentation, while Cancel cancels the engine task and removes presentation. `app.rs:5095-5179,8088-8157` polls active work roughly every250ms and handles actions. Completion removes the progress window and creates another completion window. NxTools must retain its existing same-HWND completion and generation-scoped close/disconnect rules instead.

### NxTools implementation boundary

Use existing `HttpDownloadClient::probe_timed` selected HEAD/Range/fallback response and immutable network route snapshot. Expose final response host, peer IP when available, route policy and that response's headers latency. Label latency **探测响应**, not RTT/Ping or near/medium/far. It includes DNS/TCP/TLS/HTTP/server work. No extra request, DNS lookup, GeoIP service/database or polling controller. Proxy peer is not origin IP; System means policy, not confirmed use. Manifest observations must identify the observed endpoint rather than imply every fragment shares it. Old persisted probes remain unknown; no invented zero or rebuild.

## Speed limiter

`crates/engine/src/limiter.rs:1-99` is a shared per-download/global token bucket, capacity one second; receive chunks are limited before disk writes. UI KBytes/sec is multiplied by1024 (`settings.rs:790-794`). `progress.rs:564-677` commits numeric edits on blur/Enter/accessible SetValue. Remember persists `Item.own_limit` (`model.rs:361-364`, `app.rs:8131-8139`).

NxTools retains its limiter: `rate_limit.rs:54-123` resets state and wakes waiters immediately on changes; `http.rs:1577-1582` waits task/global limits concurrently. Mister only changes an atomic without waking existing sleeps and serializes the two waits. Its video/gallery runtime setters are no-ops and helper starts do not use the remembered task value. Reference the deliberate numeric commit interaction, not that algorithm.

## Completion options

`progress.rs:750-863` provides completion dialog, dial-up disconnect, Exit, shutdown/sleep/hibernate and force-close. Other options are effective only when the completion dialog is disabled. `app.rs:4618-4633,5101-5154` runs them only on success; power action wins over Exit. These are Progress-window memory, lost when that window closes. Simultaneous task actions are not batch-coordinated.

`queue.rs:14-64,151-188` persists queue completion actions, including opening a file. Queue complete includes exhausted failed tasks. `dialogs.rs:1990-2146` gives power actions a cancellable30s countdown; Enter cancels. NxTools keeps its existing400ms terminal coalescing, deduplicated folder actions, one30s power coordinator and cancellation when new work arrives (`completion_actions.rs:46-193`). Sleep/Hibernate/Exit and queue-level actions are separate potential additions, not part of the Server slice.

## Browser integration

ABDM baseline: `desktop/app/.../nativemessaging/host/DefinedHandlers.kt:8-23` → single-instance IPC → `IntegrationHandlerImp.kt:29-45,93-116` → ordinary HTTP/HLS add owner. `NativeMessagingHost.kt:36-100` owns stdio/EOF. Local ABDM checkout does not contain its independent browser-extension repository.

Mister chain:

- `extension/manifest.json:9-55` → MV3 background/content-script.
- `extension/background.js:58-78,211-257`: one-shot `sendNativeMessage`; rule cache1minute, host rechecks admission.
- Chromium `onDeterminingFilename` / Firefox blocking `onHeadersReceived` and pause/resume `onCreated` → candidate rules → offer → nmhost → loopback IPC → application → File Info/direct start.
- `background.js:392-457,1209-1371`: browser/type/site/address/size/permission filters, POST/redirect context and browser fallback.
- `crates/nmhost/src/main.rs:216-320,434-520,564-685`: settings authority,1MiB framing, application startup, IPC forwarding, EOF exit.
- `crates/ipc/src/lib.rs:329-523`: random loopback port and token, bounded concurrency/size/deadline. Acknowledgement occurs after channel enqueue, before application persistence.
- `background.js:924-970,1128-1226`: exact Cookie store/container/private context; media cookie jar; optional browser proxy/PAC. Only bounded text/URL-encoded POST≤512KiB, not arbitrary Authorization headers or uploads.
- `crates/app/src/app.rs:2029-2202,6674-6741` → exact-URL duplicate policy → one File Info window plus FIFO queue. `dialogs.rs:737-872,1010-1118` uses supplied filename/size to avoid another probe and focuses Save As.

NxTools keeps `bg/native-transport.js:191-327`, `bg/handoff.js:454-510`, `handoff-policy.js:10-82` and Rust BrowserHost/current-user pipe/engine receipt. SQLite-accepted identity before browser cancellation, stable handoff ID and cancellation-failure rollback are stronger than Mister's channel-enqueue acknowledgement and ignored cancellation failures. Do not replace transport with loopback TCP or one-shot native messages. Do not adopt implicit POST replay or downloads starting behind a confirmation window.

### Media and automatic link popup

`background.js:464-681,838-920` uses response events, filters fragments/small audio, caps60items/tab and50tabs, stores session-only state with200ms coalescing. A user-requested uncertain manifest is sniffed up to4096bytes, bounded2MiB. NxTools has broader DOM/page-hook/PerformanceObserver/Blob/batch coverage; event-first discovery and on-demand deeper work are useful optimization directions, not measured speed claims.

`app.rs:2857-2892,8035-8093` checks clipboard once/second, ignores initial content and accepts only one whole URL with a supported protocol and conservative downloadable suffix. It enters the same File Info queue. `app.rs:1336-1366,8354-8407` can start downloads behind that confirmation by default and only compensates unseen browser tasks on orderly exit.

NxTools already has one-shot manual prefill (`downloads/clipboard_prefill.rs:10-137`) and an independent generation-scoped reusable automatic link panel (`clipboard.rs:1025-1231`, `clipboard/link_detection.rs:94-221`). An explicit **使用 NxTools 下载** action can route HTTP(S) to the existing confirmation owner; avoid another clipboard poller/window queue and preserve no automatic task/probe on manual prefill.

## Engine and Ghost comparison

| Candidate | Source-backed value | Decision |
| --- | --- | --- |
| Mister host ceiling learning | `engine/src/hosts.rs:1-88`: repeated connection refusal learns successful host width,10min decay | Consider after actual host-refusal evidence; not a scheduler replacement. |
| Mister slow connection retirement | `download.rs:43-50,3096-3110`:≥2s connection, below quarter of best, sufficient remaining work; disabled when limited | Useful bounded heuristic; benchmark against Nx adaptive shrink first. |
| Mister retained first GET | `download.rs:732-831,1450-1489`: retained probe response≤15min reused as transfer | Useful for single-use URLs; requires lifecycle/resource study. |
| Mister multi-IP racing | `dns.rs:672-739`: bounded TCP race/cache | Extra connections and routing semantics are a tradeoff; not part of Server UI. |
| Nx adaptive scheduling | `engine.rs:180-334`, `segmented_runner.rs:2050-2092` | Keep stable sampling/growth rollback/shrink and durable stop-sync-checkpoint-before-split. |
| Ghost HTTP adaptation | `features/http_pack/task.py:184-261`: slowest-range splitting, five stable samples, five-second benefit check | Behavioral reference; Nx already has comparable adaptation. |
| Ghost task UX | paused URL replacement, URL templates, range coverage graph and feature packs | Useful independent product additions; no automatic protocol expansion. |

### Known issues and limits

- Mister forces HTTP/1.1 (`download.rs:1404-1422`); its IDM-only benchmark (`docs/bench/run5/REPORT.md`) does not compare NxTools. No reference program or benchmark was executed for this study; no relative throughput or memory claim is established.
- Ghost `app/services/speed_meter.py:40-57` uses a one-second counter and100ms sleeps rather than a token bucket. `features/http_pack/task.py:38-91,145-165,263-294,409-416,491-500` has weaker validator/range/durable-checkpoint behavior; fixed5s unbounded network retries. Preserve Nx's existing resumable writer identity and bounded retries.
- Ghost completion checks zero running tasks, including failures, and invokes power actions without countdown (`task_service.py:660-678`, `services/plan.py:34-53`). Do not adopt.
- Country lookup requires separately reviewed CC BY4.0 DB-IP data attribution; no such dependency is added.

## NxTools server implementation and lifecycle entries

Reference lifecycle verification follows ABDM first: `ShowDownloadDialogs.kt:52-105,163-221` → `DesktopSingleDownloadPageComponent.kt:23-47` → shared `BaseSingleDownloadComponent.kt:107-134,208-227` → app-utils `window/custom/CustomWindow.kt:343-399` → Compose Window. Keyed presentation close is distinct from engine cancellation. Legacy `Views/DownloadProgressWindow.axaml.cs:26-59,84-103` → `Services/WindowAnimationService.cs:13-44`, with `Views/MainWindow.axaml.cs:2145-2181,2193-2225,2844-2879`, owns constructor/attachment, actual Show/Close, keyed reuse/activation/placement and same-window completion. No claim is made about unavailable framework-native internals.

Next implements `DownloadProbe.server_observation` in `rust/crates/nxtools-download/src/message.rs`; `http.rs::observe_response/probe_timed` and original HLS/DASH manifest methods capture one successful response. Media runners preserve their actual manifest observation. `storage.rs::StoredResolvedRequest` stores the complete probe in the existing request_json envelope/transaction at schema13, preserving it on task-only saves; legacy scalar probes remain supported without read-time rewrite. `engine/projection.rs::task_probe` retrieves original in-memory metadata for compact-projected windows without queue/SQLite IO.

`downloads/task_window.rs::server_information_value` uses exact released `description_list.rs:143-189` Small/borderless/horizontal label/value slots and existing bounded official Tooltip (`tooltip.rs:58-87`). It occupies the existing32DIP identity area's second line; progress filename is one truncated line with original full-name tooltip, failed/completed names remain two lines. Original440×184/440×280 clients, metrics/footer/actions and window lifecycle remain unchanged. [Executable contract](../../.trellis/spec/nxtools-download/backend/server-information.md); [delivery](../../.trellis/tasks/11-09-download-server-information/outcomes.md).

## B1 existing correctness

`tools/link_panel.rs` closes the production row's owning panel before deferred `ClipboardApplication::dispatch_link_panel_route`. The owner reads current `link_panel_copy_only` / `link_panel_auto_open_browser`: copy-only wins; ordinary Web rows copy when automatic opening is disabled; named Copy/Open remain independent. Local-file reveal and directory open use the original typed Shell commands. Production pointer/Enter/Space fixtures inspect TestPlatform clipboard bytes and outgoing platform commands; native Shell/focus/DPI and human visual acceptance remain separate.

`bg/request-context.js` / `bg/handoff.js` preserve originating download/request/tab store identity and query `cookies.getAll({url,storeId})` without URL-only or alternate-store fallback. Privacy filters candidates but does not identify an ordinary tab/container. A unique referrer tab is fed back into snapshot/Cookie resolution; ambiguous sources do not borrow credentials. Same-URL container/private, redirect, closed/replaced-tab, unsupported-API and disabled-gate fixtures observe exact Cookie queries and Native Messaging payloads. No extra Rust store field, permission or credential store is introduced. [B1 extension results](../../.trellis/tasks/11-09-reference-b1-existing-correctness/extension-outcomes.md).

BrowserHost accepts only an identity-matched durable engine receipt. A created confirmation window returns `accepted=false` with `awaiting_confirmation/confirmation_pending`; QueueOnly persistence remains distinct from channel enqueue and download completion. The popup explains fixed policy/failure reasons without request payload. The cross-language extension/host fixture checks named-store Cookies, confirmation ownership, actual pipe/engine cancellation after browser-cancel failure, stable replay and refusal; the stored result remains one Cancelled task. [B1 host results](../../.trellis/tasks/11-09-reference-b1-existing-correctness/host-outcomes.md).
