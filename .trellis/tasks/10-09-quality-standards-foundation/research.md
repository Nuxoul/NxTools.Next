# Standards evidence inventory — 2026-10-09

## Method and boundary
Single owner read root manifests, AGENTS, existing specs/tasks/reference notes and representative first-party source/test entries. One bounded read-only helper collects C#/extension evidence; no product edits/builds/launch/service/data inspection. Existing 9 scenario-bearing docs were saved to scratch for retention assertions;503 product/source/asset hashes saved before writing. Direct source inspection is authoritative; no GitNexus/ABCoder graph was used or claimed.

## Rust owner -> evidence -> convention
| Owner | Verified source/symbols | Spec interpretation / assertions |
| --- | --- | --- |
| Core | lib.rs AppConfig load_from/load_or_create/save_to/update_section1020–1149; ProcessBlacklist73–98; init_file_logging1366 | lock/reread section ownership, sibling sync/rename; distinguish first-run direct write; case normalize before.exe; daily nonblocking logging guard |
| Network | lib.rs NetworkPolicy/update/snapshot19–53; route_key99; build_client107 | immutable operation route; generation invalidation; route_key not log safe; direct.no_proxy/system/manual semantics; snapshot and local proxy tests |
| Protocol | lib.rs Envelope9–58, Message66, PlatformCommand | version-first bounded line framing, typed wire; platform result correlation, deserialize defaults compatibility |
| IPC | lib.rs platform_bridge83, CommandRouter, send_command_with_id; local_pipe.rs initial_frame_deadline | bounded queues, pending-before-send, generation replace/timeout; absolute Hello deadline disarms newline, actual PID/session identity separate |
| Download IPC | lib.rs frame/readiness24–68, DownloadIpcClient213, run_download_host275 | current-user pipe; Hello/version + command request routing, events/readiness not state store |
| Download | storage.rs open106/version12/Mutex, message.rs Fault475; http.rs joined probes; engine/segmented_runner; media | durable writer->checkpoint->publish, typed faults, operation route, no weakening fixture errors; real103 engine closeout |
| Clipboard | lib.rs history/open471, query cursor, cleanup1613/errors2015; archive/storage | SQL owner serialized by desktop worker, additive columns; bounded query/full cursor, owned assets/partial late import, migration commit |
| Finder | lib.rs FinderCatalog249, SQLite usage tables258, refresh341/search366/activation540; custom_command132 | ranking/prepared commands not native execution; local usage cache after successful durable write, last-good refresh; quote parity |
| Indexer | lib.rs runtime/search bounds32–38; mft/cache.rs magic/version3/maps; settings.rs backup persistence; service host auth | mmap base+delta notSQL; last-goodReady vs first failure; unique USN budget; protected PID/path/hash admission |
| Desktop | runtime.rs DesktopRuntime101; clipboard_worker; ui/mod schedule_entity_update74; task/quick-launch lifecycle tests | composition only, GPUI local update then deferred cross-owner, worker-ownedSQL, native pixels separate from target |
| BrowserHost | lib.rs NativeMessaging protocol2/max256KiB26–38, allowlisted events40, BlobReceiving | validate browser payload before typed submit, ack identity; noSQL; capability/body bounds and cleanup |
| CLI | lib.rs parse_invocation121, CliHostPolicy29, exit19–26 | parse then typed IPC, --connect-only no autostart; requested output not diagnostic logs |

## UI evidence
Later approved staged design fixes palette/control24/28/32/4-grid/radius3/6/8; current theme/metrics tests still old values. Type roles centralized16/14/12/11/10 with explicit glyph exception. Static motion120/160 and no common reduced-motion match. GPUI reference explicitly defers equivalent role/label provider capability. state::interactive_state Div has pointer on_click, so keyboard parity cannot be inferred from visuals. Gap ledger labels unresolved, never product edits in this child.

## Baseline preservation
Template prefixes/ends may be removed; each existing Scenario text remains unchanged except deleting literal template residue. Source-tagged paths retain indices and JSONL references. Shared architecture/engineering/verification/UI docs are new; native/extension navigation does not alter task registry/runtime. Pure docs require no Cargo.

## Native/web evidence convergence
- C# Program.cs and csproj: net8.0-windows/win-x64, PerMonitorV2 and adapter/pipe/guard lifecycle. ProtocolEnvelope/BoundedFrameReader: v1/GUID/1MiB/cancellable read. PlatformPipeClient queue256; dispatcher detaches blocking native operations and drains observed tasks. EventWindow owns STA/hooks/timers; bounded native helpers never imply interruptible Windows/COM.
- CenteredResizeCoordinator.TrySchedule defaults2 workers/64 slots, per-HWND coalescing; immutable native generation/CAS and readback from existing source/tests. BrowserRegistrationTransaction capture-before-mutate/attempt-before-write compensation is actual store API; no SQL/registry operation executed.
- PlatformLog.Write uses4096 TryWrite/dropped count and recursive field redaction; Dispose synchronously drains/joins, so bounded admission != bounded shutdown. Exception diagnostics excludes Message. Parent directly reread log/dispatcher/coordinator/test selection source to confirm helper facts.
- Extension manifest.json owns MV3 worker/document_start/all_frames/page hook/CSP; no package.json. Handoff policies bind accepted identity before cancel/erase; cancel error keeps browser item. Background bounds messages/pending; capture policy keeps privileged headers/referrer out of popup/storage. Popup tabs have ARIA+roving index/keyboard; filters only class selected; popup reduced-motion doesn't prove shadow overlay.
- One SOL read-only helper collected source/symbol/path evidence only, no recursive delegation or implementation/review. Parent validated key excerpts and writes all specs/checker/integration.

## Final writing decisions
- Kept72 original package-topic paths to avoid JSONL breakage; removed template prefixes/residue with source-backed local content. Preserved15 original scenario bodies across9 docs; one historical invariant-list native scenario gained seven-section structure without changing its list.
- Shared architecture/engineering/verification/UI/backlog and first-party entry; standalone C#/extension navigation without registry/runtime edits. Current user-approved visual target wins over old palette/density tests, explicit measured exceptions only.13 gaps recorded, no product edits.
- Small Python standard-library checker (3.11+),10 synthetic scratch tests; red empty-section/fence and valid-subheading false-positive regression proved checks and fix. It checks existence/navigation/structure, not symbol meaning/semantic correctness, UI or native/release acceptance.
