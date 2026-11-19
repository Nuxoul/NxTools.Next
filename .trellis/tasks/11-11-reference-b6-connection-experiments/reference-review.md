# B6 source review

Scope: source evidence for independent B6 execution. No UIA acceptance or packaging prerequisite applies. GPL references supply behavior, not copied implementation/comments/tests/assets.

## Reference order and verified chains

### AB Download Manager

Root: `D:\Code\ab-download-manager-master\downloader\core\src\commonMain\kotlin\ir\amirab\downloader\`.

- `downloaditem\http\HttpDownloadJob.kt:130,149` resumes by validating metadata and creating destination/parts/downloaders. Width/parts are derived at312; getPartDownloader353 fills inactive parts before safe dynamic splits under the shared split lock. Reducing width stops and joins before idle.
- `HttpDownloadJob.kt:631` tests resume/range/length/ETag then saves;695 cancels retry/scope and stops/joins every part before pause publication;747 flushes destination before checkpoint persistence.
- `connection\HttpDownloaderClient.kt:59` tests Range0–255 then no-range fallback. `OkHttpHttpDownloaderClient.kt:135` extracts metadata from a GET and closes its response;167 opens a separate transfer GET. Its commented head call is not an actual HEAD request. No retained complete probe response crosses confirmation.
- `part\HttpPartDownloader.kt:45,96` validates response before writing and observes mutable part boundary; `PartSplitSupport.kt:51,75` protects the moving safe zone and old/new inclusive boundaries. `PartDownloader.kt:63` excludes cancellation from retries and preserves critical failure priority.
- `OkHttpHttpDownloaderClient.kt:20,76` builds request context/Range and original Direct/System/PAC/Manual routing. No Mister-style learned host ceiling or separate all-address racing controller was found in this inspected chain.

### Legacy NxTools

`D:\Code\NxTools\Services\DownloadManagerService.cs`:

-1022/1073: probe uses a disposed response, with HEAD+Range, GET+Range, HEAD, full GET fallback; no retained stream.
-1743: five aggregate samples, roughly15percent stability, up to four splits and five-second benefit check. Queued segment growth does not override the active SegmentConnectionLimiter cap.
-2058/2357: original segment retry and recoverable status/IO rules.2393/2433 labels reset/EOF/TLS strings as pressure, enters permanent task-local reduced width after one error. That broad/non-expiring policy is not REF-13's allowed refusal evidence and is not adopted.
-4325/4376: legacy HTTP/1.1 connector/proxy policy is compatibility evidence, not the Next HTTP stack owner.

### Mister Download Manager

Root: `D:\Code\misterdownloadmanager-main\misterdownloadmanager-main\crates\engine\src\`.

- `hosts.rs:1,29`: learns maximum receiving width after two extra-connection refusals, permits a later higher successful width and expires after10minutes. Its host-only key must not replace required host+route identity.
- `download.rs:725,1450`: keeps an existing plain GET/POST response beginning at byte zero and consumes it for the same resource; reference retention15minutes is not an approved product value.
- `download.rs:2543,2718`: effective learned admission, stop/cancel then worker joins, and refusal evidence only while peer workers are receiving.
- `download.rs:43,2933,3085`: per-response age/speed sampling; age2seconds, quarter-best ratio and more than2seconds useful remaining are reference experiment inputs. Task/global limits disable retirement.
- `dns.rs:567,617,703`: all-address TCP probes start250ms apart within3seconds; winner preference lasts5minutes. These are extra sockets, not Server Ping or the existing Next connector.
- Ghost features/http_pack/task.py:184–261 was read after ABDM/Mister: remaining-tail splitting and five aggregate stable samples/five-second benefit window do not supply native refusal evidence or safe replacement for the original checkpoint owner.

## Next original owners

| Entry | Verified fact |
| --- | --- |
| `rust/crates/nxtools-download/src/http.rs:223` | Concurrent HEAD and exact one-byte Range;350ms companion grace; full fallback GET291 only when both fail. All responses are dropped because probe_timed returns metadata/timings only. |
| `tests/support/mod.rs:1225`, `tests/http_client_contract.rs:813` | Existing get-only fixture rejects HEAD/Range and accepts full200 GET: a real fallback-only reuse opportunity. HLS/DASH manifests383 are consumed as manifests, not ordinary file bodies. |
| `src/engine/segmented_runner.rs:267`, `src/http.rs:1051` | Accepted worker probes again, then opens a fresh transfer GET. Retaining a response without carrying accepted probe facts cannot remove that duplicate probe. |
| `src/engine/resolution.rs:19,94,399` | Original generation/active operations/receipt owner. Receipts store request/provenance only; at most256,10minute expiry. Resolver replacement invalidates active results and receipts. |
| `src/engine/resolution.rs:225,399` | Default no-op resolver returns without token/expiry; publish_resolution_receipt returns None. REF-14 must support the ordinary operation ID/generation without inventing extension provenance or assuming every probe has a receipt. |
| `src/message.rs:56`, `src/resolution.rs:16` | ProbeResolved/SubmitResolved carry operation options; no response-retirement command exists. Existing maximum concurrent resolutions is4. |
| `rust/apps/nxtools-desktop/src/downloads/new_download_view.rs:803,845,1150` | Original probe generation/stale result and submit/close owners; current receipt-based submit reuses provenance operation ID. Default flow needs the same original probe operation identity carried through confirmation if REF-14 is adopted. |
| `src/lib.rs:181`, `src/engine/segmented_runner.rs:1494,2021` | Requested active width1–64; adaptive maximum up to256queued segments; actual workers stay under the requested cap. No host/route learned cache exists. |
| `src/http.rs:2622`, `src/engine.rs:3287` | Broad retryability includes transport and408/425/429/5xx; default three attempts/100ms exponential base. No distinct additional-connect-refusal category exists. Ordinary HTTP/auth/body/validator failures are not ceiling evidence. |
| `src/engine.rs:187`, `src/engine/segmented_runner.rs:1588` | Original adaptive tick uses one-second aggregate samples, five-sample stability and benefit checks; segment retry disables adaptation. |
| `src/http.rs:847,929`, `src/engine/segmented_runner.rs:1719,1821,2287` | Rebalance signal stops and syncs writer; only joined Rebalanced can checkpoint Waiting and requeue/split/merge. Actual failure/completion/pause/cancel retain priority. |
| `src/rate_limit.rs:30`, `src/engine.rs:877` | Existing task/manual/runtime/global effective limit state is available; no second limiter/controller is needed. |

Paths abbreviated `src/` and `tests/` in this table are under `rust/crates/nxtools-download/`.

## Locked connector and route

`Cargo.toml:41` and Cargo.lock pin reqwest0.13.4, hyper1.10.1, hyper-util0.1.20, hyper-rustls0.27.9, rustls0.23.42 and tokio-rustls0.26.4. Enabled reqwest features are rustls/system-proxy without HTTP2/HTTP3; no upgrade is proposed.

Inspected registry source under `C:\Users\Administrator\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\`:

- `hyper-util-0.1.20\src\client\legacy\connect\dns.rs:211`: OS/getaddrinfo order; first address family is preferred, order within each family is preserved.
- `hyper-util-0.1.20\src\client\legacy\connect\http.rs:226,747,951`: Happy Eyeballs fallback family after300ms; parallel preferred/fallback families, but serial addresses within one family. Per-family timeout is divided among addresses. There is no same-family race or winner cache.
- `reqwest-0.13.4\src\connect.rs:89,783,904`: original connector timeout includes proxy/TLS. HTTPS proxy uses CONNECT then origin hostname/SNI TLS, never a direct-origin probe. Rustls retains original verification/certificate policy.
- Hyper-util `proxy\matcher.rs:99,227,662`: System proxy snapshots environment then supported Windows proxy settings at client build. Do not inspect or mutate real proxy secrets/settings in fixtures.

`rust/crates/nxtools-network/src/lib.rs:46,62,99,107` owns immutable operation route: Direct disables proxies, Manual installs one explicit HTTP proxy, System uses captured policy. System route_key is only system, not its actual endpoint. `nxtools-download/src/engine.rs:2962` reuses clients for30seconds by policy generation/route/certificate policy; original network pools retain16idle connections/90seconds.

REF-16 measured the actual same-family pending-connect gap through the original HTTP client: four1MiB transfers take8.134798seconds with four real SYN_SENT connects. The narrow Direct hostname preference is rejected after three port-swapped AB/BA/AB pairs regress median98.9693% and pending attempts2→4. Existing300ms cross-family fallback is also measured. See [REF-16 evidence and limits](ref16-outcomes.md); no winner cache or extra probe sockets are shipped.
The public Resolve(Name) interface sees a DNS hostname only, not an authority port. Any narrow DNS preference therefore stays hostname-scoped within the original client/policy/TLS lifetime; port-change controls must expose its cost rather than invent a request-scoped connector API.

## Retained authority

Read Download and Network owner specs, Server same-response observations, Desktop deferred callbacks/window lifecycle and current Kit selection/release/index before changes. No visual/variant/size/layout changes are proposed. Keep current official controls, stable InputState/IME/focus, B4/B5 authority and exact origin/request-context identities. Native physical and installed/Release acceptance are not implied by local HTTP or TestPlatform evidence.
