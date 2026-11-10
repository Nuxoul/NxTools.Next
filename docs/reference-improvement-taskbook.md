# Reference-driven improvement taskbook

Status: B1, B2 and B3 accepted at their recorded boundaries; B4–B7 paused by user.

This taskbook covers the20approved improvements selected from Mister Download Manager, Ghost Downloader and Bandizip. It is the cross-feature work map; each independently deliverable item retains its own requirements, design, execution plan and recorded acceptance level. Later batches remain pending.

## 1. Outcomes

- Correct the observed link-panel routing and browser Cookie-store gaps.
- Improve download admission, discovery, control and task visibility without replacing existing owners.
- Evaluate concrete connection/discovery optimizations against the current engine rather than assuming a reference is faster.
- Extend the documented split-file capabilities toward Bandizip, separating raw joining, archive reading and archive creation.

## 2. References and retained baseline

### Reference authority

- [Mister source index and comparison](reference-code/mister-download-manager.md): server observations, browser integration, automatic detection, limiter/completion interaction and connection algorithms.
- [Ghost source index](reference-code/ghost-downloader.md): task interaction, URL replacement, batch templates and range coverage.
- [Download behavior authority](reference-code/download.md): inspect indexed AB Download Manager sources first, then legacy NxTools. Existing product behavior remains authoritative unless this taskbook explicitly changes it.
- [Bandizip volume evidence](reference-code/archive-volumes.md): distinguish installed 7.32 evidence from current official documentation and distinguish read/create/join routes.
- [Existing volume contract](../.trellis/spec/nxtools-archive/backend/zip-volumes.md) and [archive parent requirements](../.trellis/tasks/10-16-archive-manager/prd.md).

Mister and Ghost are GPL references. Use independently described behavior and clean-room implementation; do not copy their code, comments, tests or assets. Bandizip observations are not reusable source code. Review the license and distribution requirements of any genuinely necessary new parser or SFX stub before adoption.

### Completed work to preserve

- The Add Download Link asset, original InputGroup/InputState, paste action, compact geometry and one-shot clipboard prefill are already implemented and verified.
- Basic progress-window Server information is already implemented. Preserve same-response host/peer/route/headers-time, media provenance, persistence and original window lifecycle; see [delivery](../.trellis/tasks/11-09-download-server-information/outcomes.md).
- Preserve existing native high-resolution download icons and the accepted bounded parallel ZIP extraction. Do not restart completed extraction tuning or change its acceptance evidence.
- Preserve persistent handoff identity, acceptance after durable task creation, browser cancellation-failure compensation, current-user pipe transport and bounded Blob intake.
- Preserve the existing live-updatable limiter, adaptive controller, stop/sync/checkpoint-before-rebalance ordering and artifact publication rules.

Archive additions remain under **Phase E — Format expansion and ZIP editing**. The delivery batches below do not rename or advance the archive parent's canonical phases. Update the relevant owning plan before starting an archive addition that expands its previously excluded scope.

## 3. Work map

| ID | Deliverable | Reference | Primary owner | Dependency |
| --- | --- | --- | --- | --- |
| REF-01 | Link-panel settings govern actual routing | Observed NxTools gap | Desktop clipboard/link panel | None |
| REF-02 | Browser Cookie-store identity propagation | Mister | Browser extension / BrowserHost | None |
| REF-03 | Explicit handoff and browser-fallback reasons | Mister | Extension / existing handoff contract | None |
| REF-04 | Explicit download action in the link panel | Mister | Desktop clipboard / downloads | REF-01 |
| REF-05 | Fine-grained browser capture rules | Mister | Extension / existing admission owners | REF-03 |
| REF-06 | Deliberate speed-limit edit submission | Mister interaction | Desktop / existing download limiter | None |
| REF-07 | Sleep, hibernate and exit completion actions | Mister | Existing completion coordinator / PlatformHost | None |
| REF-08 | Replace the URL of a paused task | Ghost | Download request/transfer/artifact owners | None |
| REF-09 | Previewed batch URL templates | Ghost | Existing add/batch intake owners | None |
| REF-10 | Actual range-coverage visualization | Ghost | Download projection / progress window | None |
| REF-11 | Queue-level completion intentions | Mister | Queue / existing completion coordinator | REF-07 |
| REF-12 | Event-first, on-demand media discovery | Mister | Extension discovery / Rust media probe | REF-03, REF-05 |
| REF-13 | Learned per-host connection ceiling | Mister | Existing adaptive/admission owners | Measured baseline |
| REF-14 | Reuse an eligible first GET response | Mister | Existing resolution/HTTP/transfer owners | Measured baseline |
| REF-15 | Retire persistently slow connections | Mister / Ghost | Existing adaptive/rebalance owners | REF-13 decision |
| REF-16 | Bounded multi-address connection selection | Mister | Network route / existing connector | Measured baseline |
| REF-17 | Split-size presets and explicit units | Bandizip | Archive create form / existing writer | None |
| REF-18 | Generic numbered-file joining | Bandizip | File operation / native source-output owners | None |
| REF-19 | ALZ/EGG split reading, Test and extraction | Bandizip | Archive readers / native volume owner | Reader/method research |
| REF-20 | ZIP-based split SFX creation | Bandizip | Archive creation / native publication | Prefix/stub research, REF-17 |

## 4. Ordered delivery batches

| Batch | Tasks | Exit condition |
| --- | --- | --- |
| B1 — Existing correctness | REF-01, REF-02, REF-03 | Settings, browser context and handoff outcomes match actual behavior. |
| B2 — Intake and control | REF-04, REF-05, REF-06, REF-12 | Explicit download intake, predictable limits/rules and lower-work discovery retain existing coverage. |
| B3 — Split-file usability | REF-17, REF-18 | Precise split-size input and a separately identified raw join workflow pass real file tests. |
| B4 — Task operation and visibility | REF-08, REF-09, REF-10 | URL replacement, template admission and range display use original task owners. |
| B5 — Completion intentions | REF-07, REF-11 | Persistent task/queue intentions share one cancellation-aware completion coordinator. |
| B6 — Connection experiments | REF-13, REF-14, REF-15, REF-16 | Each candidate has a measured adoption/retain-current decision; changes are evaluated separately. |
| B7 — Format expansion | REF-19, REF-20 | Reader/stub provenance, exact capability matrix and independent readback/native lifecycle acceptance are recorded. |

Independent work within a batch may proceed separately. Dependencies are explicit in the work map. Do not block earlier deliverables on later protocol, codec or algorithm work.

## 5. Task requirements and acceptance

### REF-01 — Link-panel settings govern routing

**Deliver:** make the production row action consume the current `link_panel_copy_only` and `auto_open_browser` settings through the existing routing owner. Copy-only takes precedence. When automatic browser opening is disabled, a Web row does not dispatch a browser-opening command; copying and explicitly named actions remain available.

**Acceptance:** exercise the real row with pointer and keyboard for the settings combinations; assert copied text versus actual platform commands. Preserve local-file/directory behavior, settings persistence, focus, panel reuse and generation cancellation. A source-token assertion is not sufficient.

### REF-02 — Browser Cookie-store identity

**Deliver:** carry supported download/tab Cookie-store identity to the existing Cookie query, including Firefox containers and private context. Query the originating store rather than a URL-only default store. Retain the existing Header master gate and Cookie sub-gate; an unavailable originating store must not silently select another store.

**Acceptance:** deterministic extension fixtures use the same URL in different stores and observe the exact queried store and handed-off Cookie. Disabled gates perform no Cookie query. Test unsupported browser APIs, closed/replaced tabs, redirects and native-message roundtrip. Never persist or log Cookie values or create another credential owner.

### REF-03 — Handoff and fallback reasons

**Deliver:** expose stable, actionable reasons for rule exclusion, temporary pause, unavailable desktop, desktop refusal, timeout/disconnect and browser cancellation failure. Keep policy decisions distinct from operational failures and from accepted/persisted task outcomes. Do not use a generic success response for channel enqueue.

**Acceptance:** actual extension/host handoff fixtures verify each reason, browser ownership, identity-matched replies and cancellation-failure compensation. The popup can explain why the browser retained a download without showing URLs, headers or credentials in diagnostics.

### REF-04 — Explicit link-panel download action

**Deliver:** add a named `使用 NxTools 下载` action for admitted HTTP(S) links. Route it to the original Add Download confirmation owner. Opening this action does not itself probe, create a task or transfer bytes; the existing Continue/confirmation actions retain that authority.

**Acceptance:** exercise the real action and prove the selected URL reaches one original InputState/window. Duplicate activation reuses the window. Local paths do not enter HTTP intake; stale/closed panel callbacks do not modify another draft. Preserve manual one-shot prefill, paste, focus/IME and cancellation. Add no clipboard poller or competing popup queue.

### REF-05 — Fine-grained capture rules

**Deliver:** provide file-type, minimum-known-size, site/address exclusion and temporary-pause controls through the existing extension settings/admission chain. Apply global pause and explicit exclusions before type/size rules and the existing handoff acceptance. Reuse current matching/normalization where available rather than adding a second rule engine.

For minimum size, only a known length below the threshold is excluded; unknown size follows existing confirmation/probe behavior without an extra extension fetch. Temporary pause is session-scoped and distinct from persisted enablement. Explain decisions through REF-03.

**Acceptance:** real extension events cover known/unknown size, redirects, rule precedence, paused/resumed state, settings reload and browser fallback. Excluded candidates neither create desktop tasks nor cancel browser ownership. Duplicate events still produce one idempotent handoff.

### REF-06 — Speed-limit edit submission

**Deliver:** separate editable numeric draft from applied limit. Enter or focus loss commits a complete valid value once; Escape restores the applied value. An enabled limit requires a positive value, and disabling uses the existing unlimited representation. Keep units explicit and retain task/global/runtime ceiling semantics.

**Acceptance:** entering `100` does not apply `1`, `10`, then `100`; Enter followed by blur does not submit twice. Empty/invalid drafts retain the last applied limit and show a local error. Existing HTTP/HLS/DASH limiter waiters wake on changes. Reopen restores persisted task/global values without replacing the limiter.

### REF-07 — Completion action expansion

**Deliver:** extend the existing task completion intention with sleep, hibernate and application exit where supported. Intention belongs to the task/settings owner and survives progress-window close/reopen. Keep successful completion as the trigger; failure/cancellation does not execute the intention. Native power operations remain C# adaptations.

Use the existing single completion coordinator and cancellable countdown for power/exit intentions. New accepted work cancels the pending action; simultaneous task completions do not create competing countdowns. Unsupported native actions have an explicit unavailable/failure state. No force-termination option is included.

**Acceptance:** test durable reopen, presentation-only close, success versus failure/cancel, simultaneous completion, countdown cancel and new work. Inject native success/failure at the typed boundary; actual power actions require separate controlled acceptance and must not run in an ordinary test suite.

### REF-08 — Paused-task URL replacement

**Deliver:** allow an explicitly paused task to replace its source URL through the existing request, probe, transfer and artifact owners. Preserve the original task identity. Reuse downloaded ranges only when the existing resource-validator policy proves compatibility; equal length or Range support alone is insufficient.

**Acceptance:** actual HTTP/SQLite/writer fixtures cover compatible replacement, changed validator, same-size different resource, missing validators, unsupported Range, cancellation and restart. Refusal/failure preserves the original request and resumable artifact. Starting fresh requires an explicit existing redownload/restart path; never mix unverified bytes.

### REF-09 — Batch URL templates

**Deliver:** accept one ascending inclusive integer range per template, for example `file_{001..020}.zip`, preserving zero padding. Show the exact expanded URLs and count before confirmation. Keep independent start/end/step-expression languages out of the initial slice; step is one.

Use the existing batch admission limit. An expansion above that limit is rejected before task creation and asks for a smaller range, rather than creating hidden batches. Preview is local and performs no network work. Existing destination/name conflicts and per-item batch outcomes remain authoritative.

**Acceptance:** test padding, inclusive endpoints, invalid/descending ranges, admission limits, duplicate URLs and cancellation. Only selected confirmed entries enter the original batch command. Partial admission reports actual per-item results and never silently retries the entire batch.

### REF-10 — Range-coverage visualization

**Deliver:** show completed ranges, active written ranges and remaining gaps using the original transfer/segment projection. Reuse the progress window and existing theme; keep real actions and numeric metrics visible. Resolve many ranges into a viewport-bounded presentation rather than an element per byte.

**Acceptance:** real projected layouts cover sparse/resumed/overlapping presentation inputs, parallel workers, unknown totals, long tasks and narrow windows. Rebalance/pause/reopen preserve the displayed facts. Unknown totals do not produce a fabricated percentage; rendering creates no network operation, sampling timer or second scheduler.

### REF-11 — Queue-level completion intentions

**Deliver:** persist completion intention on the queue owner, not the last visible task window. Support the applicable REF-07 actions and explicit open-directory feedback using the existing coordinator. Initial automatic execution requires the queue's admitted work to finish successfully; pending/scheduled work, failures or cancellations prevent automatic power/exit actions and remain visible in the outcome.

**Acceptance:** multiple queues, queued/scheduled work, insertion of new work, exhausted retries, cancellation and application restart cannot trigger early or duplicate actions. Directory actions deduplicate through the existing bounded feedback mechanism. One shared countdown owns power operations.

### REF-12 — Event-first media discovery

**Deliver:** prefer browser response events for ordinary media candidates; coalesce state publication and filter duplicate fragments or irrelevant small audio. Perform deeper manifest identification only on an explicit media-list request when observations remain insufficient. Rust remains the network/probe authority for actual downloads.

Preserve DOM/page-hook/PerformanceObserver/Blob and batch coverage where events alone cannot supply it. Replace redundant periodic scanning only after coverage is demonstrated; do not remove all fallback discovery or refetch authenticated manifests in the extension.

**Acceptance:** fixtures cover navigation, hidden/visible tabs, closed/replaced tabs, redirects, HLS/DASH, ordinary media, small complete files, fragments and Blob. Preserve existing bounded candidate/TTL ownership. Compare idle CPU, scan/event counts, retained candidates and discovery latency on the same pages; no missed supported source is traded for an idle-number improvement.

### REF-13 — Learned connection ceiling

**Deliver:** evaluate short-lived per-host learning when additional connections are demonstrably refused. Use the largest width that actually worked, scoped to the existing host/route identity, with an initial ten-minute expiry. Ordinary task failure, authentication errors and unrelated HTTP errors must not all become evidence of a connection ceiling.

Integrate any accepted result into existing admission/adaptation. A runtime observation is not a permanent global setting and does not bypass a lower user/task/host cap.

**Acceptance:** actual local servers permit a controlled width and reject extra connections; compare repeated refusal count, requests, throughput and recovery after expiry or route change. Pause/cancel and durable rebalance remain correct. Adopt only if the measured workload benefits without material regression.

### REF-14 — Eligible first-GET reuse

**Deliver:** determine whether an already accepted probe GET response can become the original transfer stream without a second request. HEAD and a one-byte Range response are not whole-file responses. Do not issue a new GET merely to manufacture a reusable response.

Any retained response belongs to the existing resolution generation and is released on cancellation, edit, replacement, window close, expiry or failed admission. Retain existing redirect, request-context, range-validator and bounded resource ownership. Do not inherit Mister's fifteen-minute retention period without workload evidence.

**Acceptance:** real single-use/slow-setup servers prove exact request counts and complete output. Exercise delayed confirmation, cancellation, route changes, stale resolution and body failure; no leaked stream, background transfer before confirmation or stale response reuse. If current probe shapes offer no useful reusable response, deliver that conclusion rather than adding extra traffic.

### REF-15 — Slow-connection retirement

**Deliver:** evaluate per-connection tail reduction through the existing adaptive/rebalance owner. Require stable samples, enough remaining work and no active manual/task/runtime speed ceiling. Reference thresholds such as two-second age and one-quarter of the best connection are experiment inputs, not proven product constants.

Stop/join/sync/checkpoint precedes reassignment. Keep existing real-error precedence and cancellation rules; do not add a second connection controller.

**Acceptance:** uneven-speed and recovering-server fixtures measure completion tail, retransmitted bytes and requests against current adaptive shrink. Stable/limited transfers must not churn connections. Pause/reopen retains exact output and original plan identity. Adopt only the independently demonstrated improvement.

### REF-16 — Multi-address connection selection

**Deliver:** inspect the locked HTTP stack's existing address-family/connection racing before introducing another mechanism. Evaluate bounded candidate selection and a short-lived winner cache only where current connection setup is measurably deficient. Prefer reuse or narrow tuning of existing behavior over separate probe sockets.

Preserve immutable DNS/proxy/TLS route semantics, cancellation and original error reporting. Explicit proxy paths must not perform unintended direct-origin probes.

**Acceptance:** controlled multi-address cases cover one slow/unreachable address, address-family fallback, route change, cache expiry and cancellation. Record setup time and extra connection cost. If existing connector behavior is equivalent or better, retain it and close the experiment with evidence.

### REF-17 — Split-size presets and units

**Deliver:** provide familiar size presets plus a positive integer and explicit KiB/MiB/GiB unit through the existing create form. Normalize once to exact checked bytes. Keep the writer's actual size/count/format restrictions authoritative; roundtripping a preset and reopening a draft must not silently change its bytes.

ZIP disk-coordinate volumes and continuous 7z splitting remain different format operations. Unsupported combinations show a reason before submission. Preserve fresh-target/no-overwrite and original per-volume publication facts.

**Acceptance:** production input/format-switch tests and real worker/native output verify unit boundaries, overflow, existing family refusal, maximum family count, cancellation and partial publication. Independent reader extraction must reproduce exact source bytes. Readable formats do not become split-writable merely because a size control exists.

### REF-18 — Generic numbered-file joining

**Deliver:** add an explicitly named raw join operation for a consecutive `.001/.002/...` family. Review the ordered family and output destination before writing; missing or ambiguous parts fail before publication. Keep raw joining distinct from signature-bound ZIP/7z reading and do not silently override those archive-open routes.

Use the existing native source/output ownership patterns and bounded streaming. Create a fresh output without overwriting a prior file. Cancellation/failure cleans only the owned incomplete output and preserves every source part. No per-part content-hash pass or whole-family memory buffering.

**Acceptance:** actual file fixtures cover byte-exact joining, selecting a later part, missing middle part, numeric ordering, source replacement under the existing source contract, output conflict, write failure and cancellation. Test data larger than the streaming buffer and assert bounded retained memory.

### REF-19 — ALZ/EGG split reading

**Deliver:** research and implement separately declared ALZ and EGG reader capabilities, including their real volume transitions and supported compression/encryption methods. Integrate with original Browse/Test/extract and native protected-volume ownership; do not add creation/editing as a side effect.

Publish a capability matrix grounded in actual samples. Unsupported methods/encryption have typed outcomes, not an apparent successful empty archive. Choose a compatible implementation strategy before changing dependencies; no proprietary binary reuse or general codec framework is implied.

**Acceptance:** independent oracle fixtures cover every declared method, ordinary/split archives, cross-volume members, missing/damaged parts, encrypted cases where supported, cancellation, output names and real cleanup. Verify byte-exact output outside performance measurements. Do not label the whole family supported from extension recognition or one sample.

### REF-20 — ZIP-based split SFX creation

**Deliver:** add explicitly selected ZIP-based SFX creation with and without splitting. Establish stub source/license, architecture, executable prefix layout, split entry conventions and verified size constraints first. Retain original candidate verification, fresh-family preflight, native publication journal and actual partial-family outcomes.

The reference's prefix/4GiB constraints require direct verification for the chosen stub and output form. Do not generalize ordinary ZIP offsets or byte chopping into an SFX writer. This item does not add RAR/ZIPX creation, split-set editing or silent file association changes.

**Acceptance:** real creation/native fixtures prove valid prefix/central/end coordinates, small forced-boundary cases, existing-family refusal, cancellation and partial publication. Independently read/extract all supported output forms. Controlled execution of the project's own stub is a separate native acceptance step; ordinary codec tests do not execute archive members or reference applications.

## 6. Shared implementation requirements

### Ownership and interaction

Rust owns domain state, routing decisions and layout; GPUI renders existing entities/windows; C# owns Windows APIs. Extension code owns browser observation and presentation, not durable engine acceptance. Reuse original task/plan/segment/artifact/queue identity and completion coordination. Do not introduce replacement managers, transports, schedulers or settings stores for these additions.

Follow [architecture](../.trellis/spec/shared/architecture.md), [engineering contracts](../.trellis/spec/shared/engineering.md) and relevant package owners. Before changing a migrated interaction, read its verified note and reference source chain, including construction/base/native show-hide boundaries, record invariants and add a red-capable regression at the real boundary.

For GPUI callbacks, update local state in the current lease and defer cross-entity work, including immediate/cache-hit paths. Preserve focus/IME, cancellation, generation/disconnect and presentation-only close. Durable changes commit before publication; configuration updates merge only the owning fields under the existing transaction.

### UI composition

Follow [component-first selection](../.trellis/spec/shared/ui-component-selection.md), [current Kit release](reference-code/gpui-kit-080.md), [component table](reference-code/gpui-kit-component-table.md) and [exact API index](reference-code/gpui-kit-component-index.md). Inspect the released compound source before implementation and record shape, public API/slot, variant/size and parent adaptation.

Reuse official Settings/Form/Field/InputGroup/NumberInput/Menu/Dialog/DescriptionList where appropriate. Keep recognizable official action boundaries; Toolbar child adaptation must not erase intended outlined actions. Range visualization may arrange actual state as custom content, but ordinary controls are not hand-painted substitutes. Preserve current dependency graph, theme and native dimensions unless the individual task explicitly justifies a changed layout.

### Diagnostics

Define accepted/start, meaningful transition, terminal outcome and elapsed/native-code events at the original owning boundaries. Carry one correlation ID through Rust/IPC/C# or browser handoff. Use fixed reasons, counts and state categories; never log URLs, clipboard contents, paths, Cookie/Header/credential data or response bodies. Coalesce high-frequency streams; no per-pointer/frame/byte/sample logging.

### Delivery discipline

Each item records changed owners, tests, results and remaining limits. Experiments use temporary harnesses or existing test seams, not permanent feature flags or duplicate runtime paths. No unnecessary hash/fingerprint pass, compatibility framework or unrelated refactor. Preserve existing user changes.

This taskbook does not start implementation or authorize packaging, installation, product/reference-app launch, service/registry changes, commit or push. Future package requests follow [PACKAGING.md](../PACKAGING.md) and the existing release gates.

## 7. Validation and performance decisions

### Functional acceptance

- Add the regression at the actual extension, HTTP, SQLite, writer, IPC, GPUI or native boundary before changing its behavior.
- Check production pointer/keyboard actions, focus, pending/error states and real bounds for UI changes; distinguish these from screenshots, native/DPI and human acceptance.
- Exercise local deterministic network/file fixtures and cancellation/restart where task or artifact state changes. Do not substitute source-token checks for lifecycle or exact output.
- Select affected package tests, strict lint/check/format and source gates from the owning spec. A failing relevant gate is repaired in scope or explicitly recorded; it is not relabelled passed.
- Every Rust verification uses `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, with TEMP/TMP in session scratch in the same command. One Cargo lane; full-operation timeouts; inspect surviving processes before any retry.

### Algorithm experiment acceptance

REF-12 through REF-16 require baseline evidence appropriate to their claimed benefit. Discovery work measures idle CPU, event/scan work and supported-source coverage; connection work measures setup/completion time, requests/retries, extra sockets, retransmitted bytes and peak memory.

For a timed candidate, change one mechanism at a time and run at least three serial baseline/candidate trials on the same workload, interleaving their order. No build or competing benchmark runs during measurement. Record raw trials, median, slowest observed run and resource/request costs. Compare complete byte-correct output outside the timed window. Distinguish controlled fixtures from external-network measurements.

An experiment is complete with either a measured adoption or a supported retain-current decision. Record the targeted benefit and any tradeoff plainly; do not claim overall faster downloads from lower idle CPU, a synthetic delay case or a comparison between non-equivalent outputs. Preserve previously accepted extraction measurements without rerunning them for this download work.

## 8. Completion criteria

The total work is complete when all 20 IDs have an independently recorded outcome, direct feature tasks satisfy their acceptance, experiment tasks have an adoption decision, and the integrated original workflows remain correct. A capability matrix must distinguish archive read/Test/extract/create/join/edit support.

Source/fixture completion, physical/native/DPI acceptance and packaged/installed acceptance are separate statuses. A subset passing does not mean the full workspace or release passed.

## 9. Known issues and limits

- B1 acceptance corrections pass extension170/170, BrowserHost47/47 and production REF-01 GPUI4/4 with polish11/11/link-panel8/8. Message contracts retain14passed/1failed (expected8/product9). Desktop strict Clippy reports initial-dirty-file nonminimal_bool and unwrap_used; the old Clipboard export-call source gate still fails. Supplementary popup/content DOM remains incomplete. None of these limits is relabelled passed. [B1 outcomes](../.trellis/tasks/11-09-reference-b1-existing-correctness/outcomes.md).
- No comparative Mister/Ghost throughput benchmark has been executed. The connection items are candidates, not established speed advantages.
- ZIPX split creation and untraced legacy volume variants remain unverified reference capabilities, not promised deliverables here.
- FTP/FTPS, Torrent, debrid, gallery/video helper packs, browser proxy/PAC import, remote control, POST replay, forced process termination, GeoIP/CDN expansion and automatic downloads behind confirmation are outside these 20 tasks.
- Retain current limiter wakeup, durable checkpoint/publication and handoff compensation. Do not adopt reference shortcuts that weaken those boundaries.

## 10. B1 delivery record (2026-11-09)

**B1 — Existing correctness** implements REF-01 current-setting routing, REF-02 originating Cookie identity and REF-03 explicit ownership/outcomes. The three acceptance corrections retain closed Chromium source context, preserve valid manual confirmation as normal pending without task acceptance, and close stale explicit Copy diagnostics exactly once. Current extension170/170, BrowserHost47/47 (including9B1cases), host strict Clippy and production REF-01 GPUI4/4 pass. The extended seven-frame/five-scenario fixture verifies exact container/private Cookies, real manual pending, browser ownership, original pipe/engine cancellation compensation, stable replay and refusal.

**Source/fixture slice complete:** current REF-01 GPUI4/4, link-panel8/8 and polish11/11 pass. Real ordinary-row and explicit Copy close observers establish the replacement before deferred work; captured Copy tracing proves normal/stale/ownerless exact-once terminals. Scoped Rust formatting and four spec/component/editor gates pass. Message14/1, Desktop Clippy and the Clipboard source gate remain non-green; prior full Desktop results are not current-workspace certification. Supplementary popup/content DOM is incomplete. Installed/native/physical/IME/DPI/human-visual acceptance was not performed. [Results and acceptance levels](../.trellis/tasks/11-09-reference-b1-existing-correctness/outcomes.md).

## 11. B2 delivery record (2026-11-10)

**B2 — Intake and control** is completed and **accepted for source/owned fixtures and the executed isolated-browser boundaries**. [Coordinator acceptance](../.trellis/tasks/11-10-reference-b2-intake-control/coordinator-acceptance.md) records independent Node255, Host48, Desktop REF0611/REF046/intake11/REF014/polish11, real Popup DOM22 and comparison/conformance assertions78. Its six AB/BA/AB raw trials/assertions/logs remain in the task evidence directory. No B2 test rerun accompanies this status update.

REF12 is adopted as evidence-based resource-list denoising:91observations remain while candidates decrease91→10; complete small media/manifests/Blob/hook/CSS remain. Original7s/1s/1s/5s responsibilities are retained. Fixture active/idle task medians1995.342→1959.447ms and262.074→258.909ms, discovery latency1813.100→1817.950ms and the differing third candidate idle sample do not establish stable CPU/latency or overall-download speedup. Native/physical/IME/DPI/provider, human visual, installed and full-workspace acceptance remain separate. [B2 results](../.trellis/tasks/11-10-reference-b2-intake-control/outcomes.md).

## 12. B3 execution record (2026-11-10)

**B3 — Split-file usability** (REF-17/REF-18) is **accepted at source, production TestPlatform and owned-native fixture boundaries**. [Coordinator acceptance](../.trellis/tasks/11-10-reference-b3-split-file-usability/coordinator-acceptance.md) closes B3-01/B3-02 and E1/E2: real validation reasons, first keyboard Confirm, actual preset/+/- controls and Submit-to-immutable-Job/wire. Independent repair verification passed REF17 8/REF18 11/forms48/draft12/Job129/quick3/tools11, Core REF17 2/Protocol REF18 1/IPC62, scoped format and four source gates. Prior coordinator native split3/raw4 evidence remains applicable; those native selections were not rerun in the repair round. Strict Desktop lint retains only two unrelated download baselines; physical/Release/full-workspace acceptance remains separate. B1/B2 and canonical **Phase E — Format expansion and ZIP editing / in_progress** remain unchanged. B4–B7 are paused; no further implementation is dispatched.
