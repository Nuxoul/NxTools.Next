# Extensions quality

Evidence: `repo:rust/crates/nxtools-extensions/tests/package_import.rs`, `repo:rust/crates/nxtools-finder/tests/extension_commands.rs`, `repo:rust/crates/nxtools-core/src/extensions.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/extensions_page.rs`.

## Scenario: Declarative package import and current execution
### 1. Scope / Trigger
W6 preview/import/enable/replace/remove/alias and actual command/tool activation.
### 2. Signatures
ImportPreview::from_json, PackageRegistry::install/set_enabled/set_aliases/validate_manual_keywords; Core import_extension_json/set_extension_enabled/set_extension_aliases/remove_extension; Finder search_with_packages(_normal)/prepare_package_action.
### 3. Contracts
Version1 bounded1MiB manifest/256packages/100handlers/200items; stable IDs, valid HTTP(S) websites, no embedded URL credentials/elevation/unknown feature shortcut. Shared Windows template validator preserves quote/backslash semantics. New import/replace disabled, no execution during preview. Package content hash excludes user aliases; effective keywords checked in both save directions, failure no write. Actual execution rechecks current target/action; pending-before-dispatch and success-only native completion. W6 declarative QuickLaunch/Tools actions compose with W7–W9 Downloads/Clipboard scripts, never silently reinterpret declarative unsupported entries.
### 4. Validation & Error Matrix
Invalid/newer/oversized/duplicate/keyword/template -> reject preview/commit; conflict -> explicit UI, old state retained; cancel -> no import/write; disconnect/late ID -> no preview/action application; disabled/updated row -> no native dispatch.
### 5. Good/Base/Bad Cases
Good: JSON preview then confirm disabled, change alias to resolve conflict then enable; actual temp-file reopen verifies persistence. Base: old config empty extensions produces original Finder results. Bad: flatten manual commands, silently overwrite collision, or hide privilege button but allow dispatch.
### 6. Tests Required
Extension package tests; Core config bytes/reopen/rollback; Finder URL/argv/fixed list/alias; GPUI preview/import/toggle/alias/remove/current action and native queue; isolated picker5. Historical W6 checkpoint Desktop692/Core46/Finder85/Protocol45/extensions8. Joint W7–W9 source gates and exact newer counts are in the owning parent outcomes, not inherited installed/DPI/Release acceptance.
### 7. Wrong vs Correct
Wrong: preview executes, conflict mutates registry, disabled retained result launches, path URL template uses plus for spaces. Correct: validated immutable preview -> explicit atomic commit -> enabled typed action with current identity; percent-encoded words valid for URL components; native correlated terminal.

## Scenario: Trusted runtime and feature boundary lifecycle
### 1. Scope / Trigger
W7–W9 package/trust/settings/runtime, feature host calls, Download receipts, committed events and GPUI publishing. Evidence: `repo:rust/crates/nxtools-extensions/src/runtime.rs`, `repo:rust/crates/nxtools-extensions/src/broker.rs`, `repo:rust/apps/nxtools-desktop/src/extensions/service.rs`, `repo:rust/apps/nxtools-desktop/src/extensions/feature_adapters.rs`.
### 2. Signatures
ImportPreview::from_directory/stage_into; PackageRegistry::approve/revoke/set_enabled/set_settings/set_event/set_provider; ExtensionRuntime::invoke/publish_registry/cancel_package/shutdown; FeatureAdapter::call(HostContext,method,Value); DownloadEngine::probe_with_resolution/submit_with_resolution; DownloadResolutionControl::with_host_cancellation; EventGuard::is_paused/retry. Full payloads/contracts: `.trellis/tasks/10-14-winshun-w6-w9/code-spec.md`, `docs/extensions/PROTOCOL.md`.
### 3. Contracts
Trust binds actual staged declaration/listed code and selected interpreter path/hash; approval and enable separate. Defaults script import disabled/untrusted, events/providers default off. Runtime/effect/result verifies current code/settings/runtime with4bounded blocking permits. Separate queue32/global4/perpackage1, queue-inclusive entry deadlines3s/10s/30s capped10s/30s/120s. Broker cleanup owns running permits until terminal/reap; one detached consumer cannot terminate unrelated runs. Broker replacement only after exact reap, not after explicit shutdown. Publication fail-closed transition, never temporary no-op; every prepared download including no-op binds generation into final acceptance lock. Host cancellation/lease/future drop reaches actual engine. Host receipt keeps original request and probe operation ID; invalid/expired/revoked returns resolution_required/no parser rerun. Event protocol error counts failure; three failures/minute pause, explicit manager retry/no old replay. No secret payload diagnostics; native2560bytes is not transport32KiB.
### 4. Validation & Error Matrix
Untrusted/disabled/tampered -> no effect/new trust; stale batch/action -> no publish/dispatch; queue/deadline/drop -> cancel, retain cleanup slot; broker not reaped -> unavailable/no replacement; publishing -> IdentityChanged/no task; bad receipt -> resolution_required/no silent network; headless UI/unknown method -> ui_unavailable/unsupported; attempted secret failure -> reverse compensation/CompensationFailed; failed event -> owner commit retained and circuit state exposed.
### 5. Good/Base/Bad Cases
Good: independent approve/enable/temp config reopen/revoke, counted localhost Probe receipt one resolver call, real commit->script->host task->causal no-loop. Base: empty registry/no broker/original feature behavior. Bad:35queued on one package, consumer drop stops all, no-op publish gap, unbound cached result, frame completion assumed native cleanup, declared permissions misrepresented as sandbox.
### 6. Tests Required
Actual broker two runs cancel-one then third success; queue1active32pending plus3otheractive, queued deadline/reap recovery/shutdown; five installed runtime examples; staged code/runtime/declaration tamper; fake credential compensation. Desktop paused resolver cancel/drop zero probes/tasks, localhost receipt reuse count1 and wrong/expired/revoked zero rerun, publication barrier/no-op generation rejection, real event->script->host causal loop, GPUI current provider/action/manager minimum pointer+keyboard/SaveCancel/trust-enable-revoke/chain/retry and Clipboard commit owner. Full source/compiled checks as parent outcomes; not installed native acceptance.
### 7. Wrong vs Correct
Wrong: retire consumer, free process permits immediately, stop broker on closed output receiver. Correct: cancel one run, retain native cleanup accounting, replace broker only after confirmed reap. Wrong: temporary no-op then new config, or silent receipt resolution fallback. Correct: fail-closed publication + final generation lock; host-issued original operation receipt or explicit resolution-required retry.
