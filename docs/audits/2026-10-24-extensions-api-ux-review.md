# Extensions API and UX review — 2026-10-24

## Scope, result and evidence level

Reviewed the **Desktop Extensions system**, not the browser download-interceptor extension: declarative packages, one-shot scripts, host calls, Quick Launch providers, Clipboard/Download manual actions, committed events, download resolvers, import/trust/settings/runtime management and session activity.

**Conclusion:** a substantial, bounded v1 exists and covers simple commands, text transformations and download resolution. It is not yet a complete general-purpose extension platform. More urgent than adding many methods are inconsistent completion/error contracts, irreversible management actions and missing traversal/input-applicability contracts. Current UX is **6/10, provisional source-based heuristic assessment**, not measured user satisfaction or visual/native acceptance.

Review is read-only for product code. One documentation deliverable was added. No Cargo/C# build, product launch, live-window operation, installed extension execution, user configuration/credential/clipboard access, network operation, packaging, install, service operation, commit or delegation was performed. An existing change in `extensions/manual_actions.rs` was preserved. The separately retained abnormal main window was not touched.

References read before conclusions: [WinShun reference](../reference-code/winshun.md), [extension protocol](../extensions/PROTOCOL.md), Extensions owner specs, current UI/component-selection contracts, Kit0.8.0 release/API index. WinShun source was not independently re-opened in this review; reference notes are historical source evidence, not a new comparative execution. The API-review skill resolved only to an unavailable external document; that missing document was not treated as evidence or a blocker to direct source review. UX heuristics were loaded and applied.

## API capability matrix

| Area | Implemented, source-verified | Limit or gap |
| --- | --- | --- |
| Package lifecycle | Strict bounded manifests/ZIP, immutable content-addressed staging, explicit replace/import, enable/disable/remove, current content/runtime trust | No declared host-version/required-method compatibility requirements; no machine-readable public JSON Schema located in the inspected extension docs/tree |
| Script runtime | Python, Node, Windows PowerShell, pwsh and package-local EXE; version1 JSONL; current identity, queue/deadline/cancel/process-tree accounting | One-shot only. Resident W10 explicitly deferred, not an implementation defect |
| Capability discovery | `host.describe`, runtime-specific GUI/headless method groups, several effective limits | Method names only, inconsistent grouping of qualified/unqualified names; no per-method parameter/result schema or complete cap set, no contextual indication of resolver reentry restrictions |
| Downloads | `list`, `get`, `probe`, `submit`, `enqueue`, `pause`, `resume`, `cancel`; host-owned receipts and idempotency | List first32 only, no cursor/filter/has_more; reduced task snapshot, no explicit progress snapshot or extension methods for retry/remove/reveal. These are capability gaps, not proof every method should be exposed |
| Clipboard | `get`, `query`, `create_derived`, native-confirmed `copy`; original preservation and causal commit owner | Query first32, no cursor/has_more; text/files only for derived/copy. Images expose ID/dimensions and explicitly `image_read_available:false`; no bounded image/blob read/write API |
| Quick Launch | Local Finder catalog/custom commands/declarative package search; update existing open query; script providers/result batches and typed effects | Host `quick_launch.search` does not run indexed-file search or merge async script-provider results, unlike the actual launcher. No scope/completeness/truncation indication |
| Common native/UI | Open, structured non-admin program launch, copy, notify through original owners | GUI-only, correctly unavailable in background composition; no extension-facing file/folder chooser, prompt/form or arbitrary view injection |
| Settings/secrets | Text/bool/integer/choice/file/folder/password; explicit Save/Cancel; current package secret lookup, compensated credential transaction | File/folder fields are text-only. No required-field/validation schema beyond present types/ranges, no typed display-value/label split for choices |
| Events/automation | Download created/completed/failed, Clipboard committed; independent opt-in, causality/no-replay, repeated-failure pause | No bounded user-visible automatic-event history or last failure category; current recent activity is manual-only |
| Developer services | Protocol prose, runnable Python/JS/PowerShell transport examples and three complete bundled Python packages | No general host storage, proxy-aware HTTP, progress/status reporting, SDK/type/schema/test harness product entry. Add only when justified, not arbitrary raw DB/HWND/entity access |

Primary API evidence: `rust/crates/nxtools-extensions/src/lib.rs:50`, `script.rs:6`, `invocation.rs:29`, `runtime.rs:21`; Desktop `extensions/feature_adapters.rs:48,195,294,350,366`, `extensions/ui_host.rs:58`, `quick_launch.rs:264`, `clipboard/extension_adapter.rs:23,96,171`.

## Prioritized findings

### F1 — P1 / UX severity3: plugin error explanation is discarded before the UI

**Actual path:** `runtime.rs:556` accepts completion including its value, then `runtime.rs:567` converts every `status:"error"` into `Err(RuntimeError::HandlerFailed)` before returning `InvokeOutcome`. `extensions/manual_runs.rs:273` maps that error to failed state with **no message**. Its plain-text bounded message handling at line260 is therefore unreachable for an ordinary error completion.

The bundled text plugin intentionally returns a safe fixed explanation on invalid input (`examples/clipboard-text-tools/protocol.py:129`). A synthetic invalid-JSON invocation confirmed `status:error` with nonempty `value.text`, no stderr and no host effects. Source tracing confirms the host subsequently drops it. A user cannot distinguish invalid JSON/settings from a missing dependency/protocol problem by the feedback alone.

**Recommendation:** retain a bounded typed failure detail through the runtime/UI contract: stable error code, optional safe user-facing plain text, retryability/remediation where owned. Do not copy arbitrary stderr/stack traces/payload into logs. Preserve native cleanup/timeout/cancellation identity. Test actual script -> runtime -> manual UI for a failing action, not only plugin fake-host tests.

### F2 — P1 / UX severity3: removal immediately loses settings and whole dependent resolver chains

`extensions_settings.rs:484` binds “移除扩展” directly to `app.remove`. `extensions.rs:854` cancels its current runs and commits removal without a confirmation/undo layer. `nxtools-extensions/src/registry.rs:495` removes the complete installed package (including aliases/settings/approval/selection) and **every resolver chain containing it**, not just its stage. `nxtools-core/src/extensions.rs:216` persists the result.

This is not merely hiding a card. An accidental click may remove a configured multi-stage chain involving other extensions. Reimporting the ZIP does not reconstruct those settings/chains. Built-in examples are intentionally not automatically resurrected after removal.

**Recommendation:** explicit destructive confirmation describing affected package configuration/running calls and number of dependent chains, or a correctly scoped recoverable undo. Do not silently execute the remaining chain stages after removal. Keep exact expected package/config identity at deferred commit. Credential/staged-byte cleanup policy should be disclosed separately; this review did not establish a credential leak or a secure deletion guarantee.

### F3 — P2 / UX severity3: valid `handled` completion is rendered as manual failure

The public protocol lists `success`, `handled`, `not_handled`, `error`. `Invocation::accept` (`invocation.rs:242`) admits all four without entry-specific validation; runtime returns `handled` normally. `manual_runs.rs:254` recognizes only `success` and `not_handled`; all others become `Failed(HandlerFailed)`. A tools/Clipboard/Download manual plugin reporting `handled` can therefore complete successfully at protocol level yet show failure.

**Recommendation:** define a single per-entry status matrix. Either treat `handled` as manual success, or explicitly constrain it to resolvers in protocol/schema and reject misuse at the protocol boundary. Do not silently call an admitted completion a runtime failure. Add real runtime-to-UI status matrix tests. The current built-in manual examples use `success`; this is especially a third-party compatibility defect.

### F4 — P2 / API completeness: bounded list reads have no continuation or completeness contract

`feature_adapters.rs:195` takes the first `limit` tasks after `engine.tasks()`, caps1..32 and returns a plain array. Its strict `List` DTO at line394 has only `limit`; no offset, cursor, status filter or continuation. Repeating a call cannot discover later IDs, and `downloads.get` is only useful for a known ID. Clipboard query also returns a bounded plain array (`clipboard/extension_adapter.rs:179`).

**Recommendation:** maintain bounded reads, add stable opaque cursor/snapshot semantics, `has_more`/next cursor and a small typed filter/sort set. Specify consistency under task/history mutation and total payload budgets. Preserve old method compatibility or introduce an explicitly negotiated new response. Do not fix this with an unbounded limit.

### F5 — P2 / UX severity2: irrelevant actions appear on incompatible selections

`Handler` (`nxtools-extensions/src/lib.rs:81`) has no accepted content kinds, selection cardinality or input-size declaration. `manual_actions::available` at line24 filters only enabled/trusted package and feature/action; `ui/mod.rs:3126` supplies the same actions to all Clipboard rows. The built-in text tools explicitly reject images/files only **after invocation** (`examples/clipboard-text-tools/main.py:57`).

**Recommendation:** additive handler input-applicability declarations, backward-compatible defaults, host validation, and consistent menu hiding/disabled reasons. Show “仅支持文本”/selection limits before launching an interpreter. This improves UX and avoids unnecessary code startup; it is not a security sandbox or replacement for runtime input validation.

### F6 — P2 / UX severity3: automatic-event failure recovery lacks actionable cause

`committed_events.rs:254` receives the full runtime result but reduces it to `success:bool` at line266. `EventControl` stores pause state and only exposes `is_paused/retry`; the manager at `extensions_settings.rs:813` shows “一分钟内连续失败三次” and “重试自动事件”. `activity.rs:30,64` intentionally shows only current-session manual calls. No automatic-event error category/time/remediation survives for the user.

**Recommendation:** bounded per-handler last terminal metadata and small session history with fixed failure category, timestamp, correlation ID and relevant “检查运行环境/检查设置” entry. “恢复自动触发” is more truthful than “重试” when it only waits for the next event. Retain no replay of old events and no clipboard/URL/credential content in diagnostics.

### F7 — P2 / API semantics: `quick_launch.search` is not the user's full Quick Launch search

`quick_launch.rs:264` calls the local catalog with an empty external list and immediately returns up to32 rows. The real launcher at line2115 joins indexed items and script result rows, while line2167 starts the indexer/provider workflow. The host response has only `id/title/target`, no searched-source or partial/complete/truncated metadata. A plugin can interpret an empty array as “no files exist” despite the real launcher finding indexed files.

**Recommendation:** explicitly name/document local-catalog scope and include capability/response source metadata; if needed, add a bounded asynchronous indexed search using the original search owner with independent request/generation/cancel. Do not run providers recursively or borrow the visible launcher's query session.

### F8 — P2 / UX severity2: configuration navigation and path fields add avoidable effort

- Usage guide's “打开设置” (`catalog.rs:280`) calls `open_package`, which resets `detail_initial_page=0` (`extensions_page.rs:109`). It opens “用途与启用”, not page2 “插件设置”. Users then need to navigate again and click “编辑设置”. If it is meant as general details, the button should say so; if settings, deep-link to settings.
- File/folder settings at `extensions_settings.rs:627,639` are ordinary text inputs; the UI explicitly says there is no native picker. Path selection forces users to find/copy/type a path and can overwrite the author's description with that generic note.
- Chain editor at `extensions_settings.rs:846,960` starts from an empty draft and requires users to remember an exact hostname and click “读取此域名”. Existing configured hostnames are not shown as selectable entries. This is maintainable for a developer, less convenient for ordinary users.

**Recommendation:** direct settings-page entry (and immediate form when appropriate), existing C#-owned typed file/folder chooser with current draft identity, and a saved-host list/search/selection before chain editing. Preserve explicit Save/Cancel and existing routing/trust owners.

### F9 — P2 / developer UX: v1 discovery and error/compatibility contract need a public authoring layer

`host.describe` (`feature_adapters.rs:295`) advertises method names and only four limits; it does not describe request/result schemas, task/clipboard byte limits, status semantics or per-entry restrictions. Manifest schema/version1 alone cannot express required host methods or host compatibility (`lib.rs:50`). `HostReply::error` normally has `value.code`, but native open/program failures in `extensions.rs:278` only return `native_error_code`; download failures collapse many actionable classes to `download_failed` (`feature_adapters.rs:321`).

**Recommendation:** one source-backed method catalog with request/result/error schema, qualified method names, capability version/context availability and stable bounded error codes. Publish JSON Schema + typed Python/JS wrappers + a disconnected validate/run-fixture CLI or test harness. Keep native code optional in a consistent error object; expose safe categories, not raw exception/payload strings. Required capability declarations should fail import/enable early with a clear message. Scope any host-version evolution separately and remain backward-compatible with existing ZIPs.

## What is already good and should not be rebuilt

- Catalog uses actual released ListItem/Icon/Label/Tag/outlined Buttons, responsive columns, search/empty state and state-driven next action (`catalog.rs`, `workspace.rs`). Old documentation's rejected prose gallery is not the current render.
- Toolbar uses public `content` composition with outlined controls rather than `child` ghost adaptation. Source tests cover real button paint and keyboard roving, but were **inspected, not rerun** in this review.
- Import and trust are actual modal Dialogs with current candidate/package identities, explicit approval, runtime detection/retry, fixed permission warning, disabled/loading/error states and separate enablement.
- Enabling does not secretly run manual actions; ordinary providers/automatic events require independent opt-in. Whole-user trust is accurately disclosed as **not a sandbox**. Do not remove this separation merely to reduce clicks.
- Native copy/open/process actions retain original owners and acknowledgement; manual cancellation is run-scoped and closing details does not cancel. Already committed effects are honestly not promised rollback.
- Secrets are masked/never preloaded, blank preserves and clearing is explicit. No unrestricted host filesystem/DB/HWND/entity reflection API should be added in the name of completeness.
- Causal event guards, receipt reuse, immutable packages/current runtime/code verification, default-off automatic behavior, process budgets and fail-closed publication are substantial foundations.

## Heuristic score and route to 10/10

Source-based score **6/10**: start10; error understanding/recovery diagnostic fails at severity3 (minus2, F1/F6); user control/reversibility fails at severity3 (minus2, F2). F3 reinforces trust/status ambiguity; F5/F8 add efficiency/recognition issues but are not double-counted into the coarse score. Good navigation, searchable catalog, clear action labels and separated trust/enable prevent a lower score. This is a triage estimate, not usability-test data.

To earn a defensible9–10: fix the major defects, make status and next actions consistent, filter incompatible input, deep-link settings/select paths, then actually exercise pointer/Tab/ShiftTab/Enter/Space/Escape, focus return, default/minimum viewport, long/error/empty/busy states, Light/Dark, real DPI and representative user tasks. If screenshots or user testing reveal additional failures, reassess rather than asserting the score from source/tests.

## Verification performed in this review

1. Read current protocol, specs, references, manifest/runtime/adapters and actual management/feedback/usage/settings/event code; compared local host search against real launcher composition.
2. `python -B docs/extensions/examples/test_examples.py -v`: **5 aggregate tests passed**, including the three packages' isolated subprocess suites, fake-host flushed sequential calls, strict transport checks, manifest completeness and deterministic in-memory ZIP contents. No ZIP build or user-state/native host calls.
3. One additional synthetic invalid-JSON plugin invocation: error completion contains nonempty user text, stderr empty, no host effects; source assertions confirm runtime drops that payload and UI's error branch has no message. This is not a compiled full-runtime/UI reproduction.
4. Relevant source files were re-grounded with read tags/hashes. Product code was not changed. No Rust/native/fullDesktop tests or interactive screenshot/reader/mixed-DPI acceptance were performed; prior task passes are not current audit execution evidence.

## Recommended sequence (proposal only)

1. Existing contract/lifecycle defects: F1, F2, F3 and F6; red-capable tests at actual boundaries.
2. Bounded API/DX completion: F4, F7, F9; explicitly version compatibility and error semantics.
3. Ordinary-user convenience: F5 and F8, then real interaction/visual acceptance.
4. Demand-driven new capabilities: leased/bounded image blobs, proxy-aware HTTP, namespaced storage or progress only for demonstrated use cases. Keep resident W10 deferred until separately approved. No arbitrary plugin WebView/UI injection, blanket network/filesystem API or weakened trust merely to make a larger API count.

This report is an audit and recommendation, **not implementation or deployment authorization**.
