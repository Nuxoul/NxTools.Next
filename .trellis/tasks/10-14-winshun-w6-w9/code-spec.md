# W6–W9 executable contracts

## 1. Scope / Trigger
Local version1 declarative/script package import, code-bound trust/settings, one-shot runtime and cross-feature effects. Extension domain owns state/validation; C# owns Windows Jobs/process/credentials; GPUI owns presentation; DownloadEngine, Finder and Clipboard worker retain original feature ownership. No W10/marketplace/native pointer or user-state test operations.

## 2. Signatures
- `ImportPreview::from_json/from_directory`, `stage_into`; `PackageRegistry::install/approve/revoke/set_enabled/set_event/set_provider/set_settings/set_resolver_chain/validate`.
- Core `AppConfig::try_update_section` and extension-specific wrappers retain lock/read/validate/sync/rename section atomicity.
- `ExtensionRuntime::invoke(InvokeRequest)->Result<InvokeOutcome,RuntimeError>` / `publish_registry` / `cancel_package` / `shutdown`; `FeatureAdapter::describe/call(HostContext,method,Value)->HostFuture`.
- `ExtensionService::new` GUI/headless composition; `publish_registry` installs fail-closed transition, publishes runtime registry then target resolver under one publication mutex.
- `DownloadEngine::{probe_with_resolution,submit_with_resolution,set_request_resolver}`; `DownloadResolutionControl::with_host_cancellation`; receipt `DownloadProbeResolution{receipt,expires_at_unix_ms,effective_request,provenance}`.
- `ClipboardCaptureWorkerHandle::subscribe_committed/capture_committed_async/extension_get_async/extension_query_async`; existing derived/native-copy Adapter; `EventGuard::admit/terminal/is_paused/retry`.

## 3. Contracts
Manifest strict unknown/duplicate-key parsing,1MiB/256packages/100handlers-settings/4096files/256MiB. Import preview executes nothing. Staged normalized declaration/listed-file SHA256 and exact selected absolute runtime path/hash bind trust. Replacement disabled/untrusted; approval never enables; enable checks actual approval and conflicts. Persisted enabled scripts require matching trust metadata; every launch/effect/result verifies real code/runtime/settings off event thread with <=4 blocking verification permits.

JSONL v1 run/correlation/generation/package/handler/content identity, invoke/result_batch/host_call/host_result/complete. Complete once and successful native exit/cleanup are distinct. No argv/env/config/diagnostic secrets. Input1MiB/frame256KiB/stdout4MiB/stderr64KiB; call dedup8active64total; results keyword200/ordinary3/provider12total. Runtime currently services host calls sequentially, within these admission caps. Queue32 is separate from global4/perpackage1; queue-inclusive default search3s/resolver10s/manual-event30s, caps10s/30s/120s, chain60s. Native job512MiB/8processes, cap1GiB/aggregate2GiB. Running permits transfer to broker pending cleanup, not consumer-future lifetime; replacement broker only after exact old process reap.

Download original resource/operation/session/approval/expiry bound opaque receipt is issued by host. Host-call `{request,idempotency_key?,resolution?:{operation_id?,mode:{kind:"resolve"}|{kind:"receipt",receipt:UUID}}}`; receipt requires explicit non-nil operation ID. Submit keeps original resource context, server uses stored effective request/provenance and merges confirmed presentation policy. Bypass is explicit original download UI/domain operation, not accepted in plugin host-call input. Durable idempotency checked before resolution. Any origin change drops ALL inherited headers/Cookie/Referrer and media; explicit new context replaces, never merges. No injected resolver call under scheduler/store lock. Every preparation, including no-op, carries generation into final persistence lock.

Default-off event/provider opt-ins independent of enablement. Successful feature commit only; original event drain stays independent, bounded notifications, no progress/capture/render script callback. Causality root/handlers/depth<=4, dedup4096/rate150ms/queue32; three failures/minute pause, UI explicit retry waits for new event/no replay. Protocol `error` is a failure even after valid completion. Quick Launch result IDs `extension-script:package/handler/item`, stable same-ID/selection/quota/current query/action checks; no plugin RunElevated/Shell.

Credential attempted mutation records before write, previous values read first, reverse compensation on every attempted mutation before failed config transaction returns. Generic native blob<=2560bytes;32KiB only transport; masked password default absent and blank unchanged/explicit clear. Managed strings not guaranteed zeroized. Test-support broker binding only dev/test feature; production broker exact sibling PlatformHost and ordinary parent checks unchanged.

## 4. Validation & Error Matrix
| Condition | Observable boundary |
| --- | --- |
| invalid manifest/path/reparse/code/hash/conflict | reject before registry commit, retain old config/package |
| untrusted/disabled/changed code-runtime-settings/stale query | no new launch/effect/publish; renewed trust or new query required |
| queue full/deadline/cancel/drop/disconnect | typed terminal, cancel ordinary owned tree; occupied cleanup slot not reused |
| detached one-run consumer | cancel only that run; unrelated run and later invocation survive |
| unconfirmed broker cleanup | unavailable, no replacement; explicit shutdown never restarts |
| registry publishing | fail-closed IdentityChanged; no no-op parse gap or mixed settings acceptance |
| forged/expired/evicted/revoked receipt | resolution_required, no hidden parser/network/task effect |
| resolver conflict/cycle/failed stage/same-plugin reentry | failure stop/no task; explicit retry or bypass only |
| host lease/future dropped during resolution | actual engine cancel/no probe publication/task commit |
| UI-only headless/unknown host method | ui_unavailable/unsupported, never reflection or implicit popup |
| secret mutation/config failure | reverse compensation, explicit CompensationFailed if incomplete; Cancel0writes |
| automatic three failures | paused state and reachable retry; owner commit remains successful |

## 5. Good/Base/Bad Cases
Good: preview->stage->approve->independently enable->invoke->native cleanup; localhost probe->matching host receipt->enqueue once; committed task->real script host enqueue->causal own-handler rejection; explicit event retry changes neither saved subscriptions nor trust.
Base: empty extension registry launches no broker and preserves original feature behavior; old tasks resume without plugin. Declarative manual elevated commands retain their original behavior but plugins never inherit it.
Bad: trust alone enables; cancelled result applies; dropped run stops whole broker;35pending behind one package; publish temporary no-op; parser reruns after receipt rejection; cross-origin credentials merge; event protocol error counted success; native spawn/admission interpreted as completed UI effect.

## 6. Tests Required
Actual import/temp config reopen/byte-preserving rejection; staged declaration/code/runtime tamper. `broker::tests::dropped_run_does_not_stop_other_package_or_next_invocation`; runtime queue32+global4, queue deadline, confirmed-reap recovery/shutdown; actual five runtimes and duplex fake credential fixture.
Desktop real shared-script resolver/publication barrier/revoke-retained task; adapter paused resolver explicit cancel/future drop zero HTTP/store effects; localhost probe receipt count1 and invalid/expired/revoked no rerun; GPUI `confirmed_extension_receipt_rejection_does_not_reparse_or_create_task`; actual committed event->script->host enqueue->own-handler no-loop; Clipboard worker13 and Adapter2; actual current Quick Launch batch/selection/action; min manager SaveCancel/trust-independent-enable-revoke/chain/opt-in and circuit retry. Fake secret mutation-then-throw/config failure/compensation failure/Cancel0writes.
Full Desktop, Extension/Core/Finder/Protocol/DownloadIPC, selected Download safe contracts, C# audited process/credential/picker; strict six owners/workspace tests/fmt/source/spec gates with serialized required Cargo environment. Installed physical/DPI/reader/live credential/Release checks are not inferred.

## 7. Wrong vs Correct
```text
Wrong: cancel future -> drop consumer -> channel send error -> shut all runs -> reuse dead broker.
Correct: detach consumer -> idempotent cancel own run -> retain pending permits -> native cleanup -> next invocation.
Wrong: publish no-op -> update config -> real resolver; old no-op commits unbound.
Correct: serialized fail-closed resolver -> runtime publication -> target; every generation checked under acceptance lock.
Wrong: probe -> submit original using Resolve again, or silently retry expired receipt.
Correct: host operation ID+opaque receipt -> validate stored resource/approval -> consume once; explicit failure requires user retry.
Wrong: credential write throws -> rollback only previous successful writes.
Correct: mark attempted before mutation -> compensate every attempted write in reverse -> report incomplete rollback truthfully.
```
