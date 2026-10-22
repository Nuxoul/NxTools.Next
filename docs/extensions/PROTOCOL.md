# NxTools extension protocol v1

Status: W6–W9 source implementation and isolated joint acceptance passed. Scripts can be independently enabled after explicit whole-user trust. No installed/native user-data or Release acceptance, malicious-code isolation or rollback of external effects is implied.

## Package, import and trust

Ordinary Desktop import accepts one ZIP package with `extension.json` at root or inside its only top-level folder. Every package-local script/dependency/asset must be listed in top-level `files`. Validation reads/decompresses bounded bytes only; explicit confirmation stages an immutable content-addressed copy into the application-managed extensions directory, including declarative manifests. The source ZIP may then be removed. New imports/replacements remain disabled and untrusted. Root/wrapper ZIP format, safety limits and unsupported variants: [ZIP import guide](../../.trellis/tasks/10-15-extension-zip-import/usage.md). Legacy lower-level declarative JSON and directory APIs remain compatible; standalone declarative schema is shown in `examples/commands.json`.

Script runtime `program`: `python`, `powershell`, `pwsh`, `node`, or `exe`. Opening environment/trust automatically probes PATH for an unconfigured interpreter using fixed version arguments and bounded ordinary-user Jobs. Existing defaults/overrides take priority; explicit invalid selections never silently fall back. Discovery only updates a draft/informational candidate, not trust or enablement. Explicit trust re-inspects the exact executable/hash and checks current selection/content inside the original locked transaction; an unconfigured detected candidate becomes its package override only upon approval. Run Environment still supports manual/virtualenv paths and explicit Save/Cancel. The host never downloads interpreters/dependencies or changes system PATH. Effective selection changes revoke affected approval and disable; EXE stays staged. Python runs unbuffered; PowerShell uses `-NoLogo -NoProfile -NonInteractive -File`, no automatic execution-policy bypass. JSONL UTF-8, structured arguments, no shell text. [Trust/PATH guide](../../.trellis/tasks/10-15-extension-trust-path/usage.md).

Whole-plugin trust grants all current ordinary-user permissions. Declared permissions and purposes are informational, not a filesystem/network sandbox. No host-assisted administrator/UAC/SYSTEM execution. Runtime/code/declaration changes require new trust. Time/output/process/memory/concurrency/cancel limits remain after trust. Same-user external dependencies cannot be exhaustively monitored; OS/network side effects cannot be rolled back by host.

Settings: text, bool, integer number with min/max, choice, file, folder, password. Save/Cancel explicit. Password defaults and raw config values are rejected; secrets reach the selected trusted handler only through `credentials.get` and the private transport, never argv/environment/log. Generic Windows credential storage accepts at most2,560 bytes; protocol32KiB is a transport limit, not a larger native storage promise. Blank secret draft means unchanged; explicit clear deletes.

## Handler registrations

`feature/entry`: `quick_launch/command`, `quick_launch/provider`, `tools/action`, `downloads/resolve`, `downloads/action`, `downloads/created|completed|failed`, `clipboard/action|committed`. Script action: `{"type":"script","entry_file":"entry.py"}`. Ordinary providers and automatic events are individually default-off. Events come only after feature commit, never progress/pointer/capture callbacks.

Automatic handler failures three times within one minute pause that handler. The manager shows the fixed reason and offers explicit retry; retry waits for a new committed event and does not replay old events. Download/Clipboard owner transactions stay committed even when a handler fails.

Resolver matching: exact host or explicitly included subdomains, bounded path `*`/`?`, declared priority then specificity. Default chooses one handler. A top tie is an explicit conflict, not arbitrary choice. User ordered chains have at most8 distinct stages; previous validated result feeds the next, failure stops, unchanged/not_handled passes through. Repeated changed URL is a cycle. No rediscovery or broadcast after rewrite. No-match behaves as original download path.

## Transport

One UTF-8 JSON object per LF-delimited stdin/stdout line. stdout is protocol only. stderr is bounded but never copied into production logs. Max frame256KiB/input1MiB/stdout4MiB/stderr64KiB. Exit before valid completion fails. Host calls8active/64total; search items keyword200, ordinary3/provider12total. Per-plugin1/global4; queue32, finite native process trees. Deadlines include queue/identity/start/output/effects; timeout cancels owned ordinary process trees.

When `runtime.timeout_ms` is omitted, entry defaults are search3s, download resolver10s, manual/event30s. Explicit values are clamped to search10s/resolver30s/manual-event120s; the chain also has60s total cap. Native jobs currently use512MiB and8processes; broker enforces2GiB aggregate. These are resource limits, not access restrictions.

Initial host frame includes `protocol_version:1`, `op:"invoke"`, UUID `run_id`, `correlation_id`, generation, package/handler/content identity, feature, typed `context`, nonsecret `settings` and host-owned `causality`. Replies must use exactly the same run ID; unknown/duplicate fields or keys reject. Scripts do not provide trusted generation/trust identity.

```json
{"protocol_version":1,"op":"host_call","run_id":"UUID","call_id":"describe","method":"host.describe","params":{}}
{"protocol_version":1,"op":"host_result","run_id":"UUID","call_id":"describe","success":true,"value":{"version":1}}
{"protocol_version":1,"op":"complete","run_id":"UUID","status":"success","value":{"text":"done"}}
```

Completion status: success, handled, not_handled, error. Streaming provider:

```json
{"protocol_version":1,"op":"result_batch","run_id":"UUID","sequence":1,"replace":false,"items":[{"id":"stable-id","title":"Result","subtitle":"Detail","action":{"type":"copy","text":"value"}}]}
```

Stable IDs are package/handler/item namespaced. Sequence starts1 and increases by1. Same ID updates in place; late new rows never displace committed selection. Action types: open(target), copy(text), program(executable,arguments[],working_directory), query(text), invoke(action_id). No elevation fields. Current query/content/trust and selected action are rechecked before effect.

## Host operations

`host.describe` advertises versions, actual headless availability and effective caps. Unknown method returns `unsupported`, not reflection. `credentials.get {key}` returns only own declared password setting. First-release methods through existing feature owners:

- `downloads.list {limit<=32}`, `downloads.get {task_id}`; probe/submit/enqueue `{request,idempotency_key?,resolution?}`; pause/resume/cancel `{task_id}`. Resolver may not recursively probe/submit/enqueue through itself; fixed reentry error avoids same-plugin slot deadlock.
- `clipboard.get {item_id}`, query `{query,limit<=32}`, create_derived `{source_item_id,text}` or paths. Original remains. Image snapshot gives ID/dimensions; image-read is not advertised yet. `clipboard.copy` uses existing native suppression and completion.
- GUI: `common.open {target}`, program `{executable,arguments[],working_directory?}` without elevation, copy `{text}` or paths, notify `{title,message}`. `quick_launch.search {query}` uses current available Finder sources; query updates an already open original input, never implicitly opens a popup. GUI-only methods return `ui_unavailable` in background host.

## Download result and receipts

Resolver invoke context: `{original_url,request,stage}`. `request` is the validated previous effective request, private context goes only to selected handler.

```json
{"protocol_version":1,"op":"complete","run_id":"UUID","status":"handled","value":{"url":"https://download.example/file","file_name":"suggestion.bin","valid_for_ms":60000}}
```

Optional explicit context: `{headers:[{name,value}],cookie:null,referrer:null}` replaces rather than merges. Any origin change drops automatically inherited ALL headers/Cookie/Referrer and old media; transformed resource clears old expected length/media. Existing proxy/TLS/rate/category/schedule/filename/resume validation remains host-owned. Plugin-owned HTTP is not automatically forced through product proxy.

Probe issues a host-owned opaque receipt bounded256/10minutes max, clamped by shorter plugin validity. Submit reuses original resource identity+operation ID, host-stored effective context/content/settings/session approval. Save filename/directory/category/schedule/rate/media choice may be confirmed without rerunning parser; URL/credentials/TLS changes require explicit retry. Forged/expired/evicted/revoked receipt fails resolution-required, not silent rerun. Durable accepted idempotent replay occurs before resolution. Old task resume uses persisted effective request and original validators without requiring a plugin.

Host-call receipt reuse keeps the original request and adds `resolution:{operation_id:"<probe.resolution.provenance.operation_id>",mode:{kind:"receipt",receipt:"<probe.resolution.receipt>"}}` to submit/enqueue (or repeat probe). The following host-call correlation ID may differ. Omitted resolution or `{}` means explicit new Resolve using the host correlation ID. Nil/missing receipt operation IDs, unknown fields, caller-provided effective context and bypass modes reject. Expired/changed/invalid receipt returns `resolution_required`; no implicit resolution. Host cancellation/lease closure and dropped host-call futures cancel the actual engine operation before probe publication or task commit.

## Examples and verification

`protocol.py`, `protocol.js`, `protocol.ps1` are actual inputs to the owned five-runtime test (PowerShell example runs under both installed versions). Node explicitly releases stdin after final flush. `download-resolver/` demonstrates not_handled without network effects; a site parser must implement its own verified logic, not fabricate links. EXE uses identical JSONL and must flush/exit; synthetic test apphost demonstrates the same protocol through staged package-local EXE. No SDK or resident process is required; W10 remains out of scope.
