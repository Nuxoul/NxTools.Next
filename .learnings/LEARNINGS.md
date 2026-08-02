# Learnings

Corrections, insights, and knowledge gaps captured during development.

**Categories**: correction | insight | knowledge_gap | best_practice

---

## [LRN-20260802-MV3-NONBLOCKING] correction

**Logged**: 2026-08-02T21:25:22+08:00
**Priority**: critical
**Status**: resolved
**Area**: frontend

### Summary

An ordinary user-installed Manifest V3 extension must never register a blocking `webRequest`
listener; adding `webRequestBlocking` is not a valid Chrome/Edge fix.

### Details

The migrated browser extension retained an unconditional legacy `onHeadersReceived` listener with
`"blocking"`. Chrome permits that capability only for force-installed enterprise extensions, and
the callback did no useful cancellation work. AB Download Manager's MV3 adapter explicitly reports
that response blocking is unavailable and uses the Downloads API boundary instead. The first real
browser smoke checked the handoff result but did not fail on extension background errors, so it
missed both this registration error and CSP-blocked popup progress styles.

### Suggested Action

For MV3 browser integrations, observe request metadata non-blockingly, wait for durable native-host
acceptance, and only then cancel the browser-owned download through `chrome.downloads`. Gate releases
with both a source contract and real Service Worker/popup error capture.

### Metadata

- Source: user_feedback
- Related Files: `browser_extension/nxtools-interceptor/background.js`, `tests/runtime/browser-extension-smoke.mjs`
- Tags: chrome, edge, manifest-v3, webrequest, automation, csp
- Pattern-Key: browser.mv3_nonblocking_handoff
- Recurrence-Count: 1
- First-Seen: 2026-08-02
- Last-Seen: 2026-08-02

### Resolution

- **Resolved**: 2026-08-02T21:25:22+08:00
- **Notes**: Removed the blocking listener, replaced the popup inline progress style with a CSP-safe
  native progress element, added static contracts and real browser background-error diagnostics,
  then passed source and packaged Chrome handoff regressions.

---

## [LRN-20260802-CHROME-EXTENSION-AUTOMATION] Use the DevTools extension domain on current Chrome

**Logged**: 2026-08-02T00:00:00+08:00
**Area**: browser automation

### Learning

Google Chrome 137+ ignores `--load-extension`; a target URL alone also cannot wake an extension that
was never installed. Current automated extension tests should use Puppeteer's DevTools pipe with
`--enable-unsafe-extension-debugging`, start with `enableExtensions: true`, and explicitly await
`browser.installExtension(path)`.

Puppeteer 25.4.0's array-valued `enableExtensions` launch branch does not reliably await its nested
installation promises, so explicit installation gives deterministic errors and preserves the browser
for diagnosis. Keep the runner in `.mjs`; setting root package `type=module` changes the existing
extension `.js` contract tests. On Windows, Chrome for Testing may also require the documented
AppContainer read/execute ACL on its extracted directory.

### Metadata

- Source: debugging
- Related Files: `tests/runtime/browser-extension-smoke.mjs`, `package.json`
- Tags: chrome, puppeteer, extensions, native-messaging, windows
- Pattern-Key: browser.chrome_extension_devtools_install
- Recurrence-Count: 1
- First-Seen: 2026-08-02
- Last-Seen: 2026-08-02

---

## [LRN-20260802-GPUIAUTOMATION] best_practice

**Logged**: 2026-08-02T18:53:00+08:00
**Priority**: high
**Status**: resolved
**Area**: tests

### Summary

Reliable GPUI Windows acceptance automation needs explicit focus settling, throttled character
messages, window-relative selectors and native state assertions.

### Details

Zero-delay `WM_CHAR` delivery immediately after a posted click can lose the prefix while GPUI is
still applying focus. Wait briefly after the click and throttle characters. Coordinate scenarios
must also tolerate transient notification cards that cover a target button. `PrintWindow` captures
the composed GPUI HWND without requiring manual screenshots, while `WS_EX_TOPMOST` and visibility
must be asserted independently of pixels.

### Suggested Action

Keep UI acceptance steps data-driven, use unique artifact paths, provide a deterministic local HTTP
fallback, and record both PNG evidence and machine-readable native assertions.

### Metadata

- Source: debugging
- Related Files: `tests/runtime/windows-ui-automation.ps1`,
  `tests/runtime/download-ui-smoke.ps1`
- Tags: gpui, windows, automation, screenshots, focus
- Pattern-Key: windows.gpui_acceptance_harness
- Recurrence-Count: 1
- First-Seen: 2026-08-02
- Last-Seen: 2026-08-02

### Resolution

- **Resolved**: 2026-08-02T18:53:00+08:00
- **Notes**: Public 100 MB and local slow-fixture acceptance runs both passed unattended.

---

## [LRN-20260802-001] correction

**Logged**: 2026-08-02T00:00:00+08:00
**Priority**: critical
**Status**: resolved
**Area**: frontend

### Summary

NxTools.Next feature work must preserve the checked-out project's existing code and UI language
instead of introducing an assistant-selected visual direction or unrelated capability.

### Details

The user explicitly requires the remaining download phases to follow `docs/download-plan.md`, the
reference projects, and the current NxTools.Next GPUI implementation. New download UI must reuse
the existing theme tokens, density, typography, components and interaction architecture. Scope is
limited to the named phase items; visual novelty is not a product requirement.

### Suggested Action

Before each slice, freeze the reference behavior and current visual invariants, add a real-boundary
regression, and reject unrelated features or new design systems. Do not commit or push unless the
user explicitly requests it.

### Metadata

- Source: user_feedback
- Related Files: `docs/download-plan.md`, `docs/reference-code/download.md`, `rust/apps/nxtools-desktop/src/main_window.rs`
- Tags: scope, reference-driven, gpui, visual-consistency
- Pattern-Key: download.reference_scope_and_visual_parity
- Recurrence-Count: 1
- First-Seen: 2026-08-02
- Last-Seen: 2026-08-02

### Resolution

- **Resolved**: 2026-08-02T00:00:00+08:00
- **Notes**: The Stage 3 completion reused the existing theme, density and keyed-window lifecycle,
  stayed within the two canonical download-plan items, added no unrelated media capability, and
  produced a validated package without committing or pushing Git changes.

---

## [LRN-20260727-001] correction

**Logged**: 2026-07-27T17:45:00+08:00
**Priority**: high
**Status**: resolved
**Area**: infra

### Summary

NxTools.Next 的本机索引服务升级应先关闭精确受保护路径中的旧进程，再覆盖同一发布目录。

### Details

此前把正在运行的受保护 Desktop/PlatformHost 文件锁问题扩大为版本目录部署方案。用户明确要求
本地单用户工具直接关闭旧进程。正确边界是只终止 `%ProgramData%\NxTools\NxTools.Next\Service`
下精确匹配的旧 Desktop/PlatformHost，排除当前安装器，等待退出后原子更新三件套并重启服务；
不得仅凭进程名终止其他安装或无关程序。

### Suggested Action

在提权安装器中增加有界的精确路径进程清理，并用真实子进程测试覆盖“运行中目标被关闭后可替换”。

### Metadata

- Source: user_feedback
- Related Files: csharp/NxTools.PlatformHost/Windows/IndexerServiceInstaller.cs
- Tags: indexer, installer, process-lifecycle, upgrade
- Pattern-Key: deployment.stop_exact_owned_process_before_replace
- Recurrence-Count: 1
- First-Seen: 2026-07-27
- Last-Seen: 2026-07-27

### Resolution

- **Resolved**: 2026-07-27T17:44:00+08:00
- **Notes**: The elevated installer now terminates only exact protected Desktop/PlatformHost paths,
  excludes itself, waits for exit, and the real-process regression proves the target can then be
  overwritten while an unrelated same-name process remains running.

---

## [LRN-20260725-001] correction

**Logged**: 2026-07-25T00:10:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

Environment-variable instant results must open existing filesystem targets instead of always
copying the expanded value.

### Details

The first NxTools.Next slice modeled every `%NAME%` expansion as `CopyText`, despite the reviewed
SwiftList provider selecting `Execute` for existing directories/files, splitting semicolon-delimited
variables into individual targets, and using copy only for non-path or missing values. The user
correctly identified that `%TEMP%` and similar path variables should open their resolved location.

### Suggested Action

Derive typed path/text targets in Rust: open existing directories/files, split bounded multi-path
values, and retain copy as a fallback only for values that cannot be opened. Protect primary action,
context-menu and file-index routing behavior with regression tests.

### Metadata

- Source: user_feedback
- Related Files: `rust/crates/nxtools-finder/src/environment_variable.rs`, `rust/apps/nxtools-desktop/src/quick_launch.rs`
- Tags: quick-launch, environment-variable, action-semantics

### Resolution

- **Resolved**: 2026-07-25T00:45:00+08:00
- **Notes**: Existing directory/file values now emit `OpenTarget`; bounded multi-path values split
  into typed results, and non-path or missing values retain a copy fallback. Finder and GPUI
  regression tests cover the corrected behavior.

---

## [LRN-20260724-008] correction

**Logged**: 2026-07-24T16:45:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

When primary and secondary references disagree, preserve the primary reference's useful behavior
before adopting the secondary implementation's narrower rule.

### Details

SwiftList's public `WebUrlInstantProvider` accepts only explicit HTTP(S) URLs, but the user has
repeatedly established Listary as the primary Quick Launch reference. Listary's actual `Class230`
also recognizes bare domains and IP addresses and normalizes them to HTTPS. The first conclusion
based on SwiftList alone was therefore too narrow. NxTools keeps Listary's bare-target convenience
while replacing its legacy permissive regex with structured URL and host validation.

### Suggested Action

For Quick Launch behavior, resolve Listary's managed call chain first, use SwiftList to clarify open
implementation details, and explicitly document intentional hardening where the references differ.

### Metadata

- Source: conversation
- Related Files: docs/reference-code/quick-launch-research.md,
  rust/crates/nxtools-finder/src/web_url.rs
- Tags: quick-launch, reference-priority, url, listary, swiftlist
- Pattern-Key: quick_launch.reference_priority
- Recurrence-Count: 1
- First-Seen: 2026-07-24
- Last-Seen: 2026-07-24

### Resolution

- **Resolved**: 2026-07-24T16:45:00+08:00
- **Notes**: Added structured bare-domain/IP HTTPS normalization and reference-backed tests.

---

## [LRN-20260724-007] correction

**Logged**: 2026-07-24T15:50:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

Reference screenshots can establish layout and behavior without making framework type names valid
product copy.

### Details

The calculator result subtitle copied `System.Double` from a reference screenshot. That is a .NET
implementation type and is meaningless in a Rust-owned calculator UI. Result labels must describe
the user-visible representation instead: `计算结果`, `十六进制`, `二进制`, `中文数字`, and `中文金额`.

### Suggested Action

When adapting a reference UI, classify each visible string as product language, sample data, or
implementation leakage before reproducing it.

### Metadata

- Source: user_feedback
- Related Files: `rust/crates/nxtools-finder/src/calculator.rs`
- Tags: quick-launch, calculator, product-copy

### Resolution

- **Resolved**: 2026-07-24T15:50:00+08:00
- **Notes**: Replaced framework/type labels with user-facing representation names and updated tests.

---

## [LRN-20260724-006] correction

**Logged**: 2026-07-24T15:00:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

Quick Launch calculator mode must be explicitly entered with a leading `=` instead of
opportunistically interpreting ordinary search text as mathematics.

### Details

The first implementation recognized complete expressions such as `12 * 3` directly. The intended
interaction is Listary-style explicit calculator input such as `=12 * 3`; without the leading
marker, the same text remains a normal unified search and may query the file index.

### Suggested Action

Keep the trigger rule in the shared Rust `calculator_answer` boundary so ranking, index suppression,
typed copy actions, and GPUI presentation cannot diverge.

### Metadata

- Source: user_feedback
- Related Files: `rust/crates/nxtools-finder/src/calculator.rs`, `rust/apps/nxtools-desktop/src/quick_launch.rs`
- Tags: quick-launch, calculator, query-trigger

### Resolution

- **Resolved**: 2026-07-24T15:00:00+08:00
- **Notes**: Added the required `=` prefix and regression coverage for prefixed and unprefixed queries.

---

## [LRN-20260724-005] correction

**Logged**: 2026-07-24T13:18:00+08:00
**Priority**: high
**Status**: promoted
**Area**: frontend

### Summary

Listary fixed-search window behavior cannot be inferred from `Class335` and
`DialogSearchWindow` alone; the `SearchWindowBase` constructor supplies the essential
one-time `Topmost=true` window policy.

### Details

The first NxTools correction verified only deferred GPUI creation and non-activating placement.
User runtime testing still showed the search bar remaining behind the initial Open/Save dialog
until a foreground switch. Re-reading the full inheritance chain showed that Listary combines
`Topmost=true` with `ShowActivated=false`, while position updates do not reorder the window.
Using `HWND_TOP` during every placement was a different behavior and caused foreground switching.

### Suggested Action

For reference-driven UI migration, trace constructors and base classes through the actual show
boundary before recording lifecycle invariants. Protect both the one-time native initialization
and the steady-state movement path with separate regression gates.

### Metadata

- Source: user_feedback
- Related Files: `.diagnostics/listary-managed/Listary/Main/Views/SearchWindowBase.cs`,
  `rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs`
- Tags: listary, window-lifecycle, topmost, reference-driven
- **Promoted**: `AGENTS.md`

---

## [LRN-20260724-004] best_practice

**Logged**: 2026-07-24T00:00:00+08:00
**Priority**: high
**Status**: pending
**Area**: frontend

### Summary

Do not cache a Windows platform command as applied when Rust has only enqueued it.

### Details

`PlatformClient::send_command` returning success means the correlated command entered the IPC
pipeline, not that C# and Windows accepted it. Caching owner, bounds, or visibility at that point
can suppress the retry after a native failure and leave an auxiliary GPUI window detached,
mispositioned, or hidden.

### Suggested Action

Either keep idempotent placement commands retryable, or update an applied-state cache only from the
correlated successful platform result. Never promote queued state directly to confirmed native state.

### Metadata

- Source: error
- Related Files: rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs
- Tags: ipc, correlated-result, window-lifecycle, retry
- Pattern-Key: ipc.queued_is_not_applied
- Recurrence-Count: 1
- First-Seen: 2026-07-24
- Last-Seen: 2026-07-24

---

## [LRN-20260723-PROTECTED-PACKAGE] correction

**Logged**: 2026-07-23T18:17:00+08:00
**Priority**: high
**Status**: promoted
**Area**: infra

### Summary

A packaged fix is not deployed when protected-copy redirection validates only the unchanged desktop
executable while a sibling service binary changed.

### Details

The indexer-only fix produced a new `NxTools.Indexer.exe` but an identical `NxTools.exe`. Startup
compared only the desktop executable, treated the older protected package as current, and redirected
to it before the new package could launch its PlatformHost and execute `ensure_indexer_service`.
Reporting the package as ready without validating the running protected binary repeated the user's
error. A later UI recovery was also insufficient acceptance evidence: the old service naturally
returned to `READY` after its rebuild, while all protected hashes still differed from the latest
package.

### Suggested Action

Treat the desktop, PlatformHost, and indexer as one atomic protected release. Both redirect and
post-package verification must compare all three binaries before claiming that the packaged fix is
active.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/main.rs, csharp/NxTools.PlatformHost/Windows/IndexerServiceInstaller.cs
- Tags: deployment, protected-copy, service, package-hash
- Pattern-Key: infra.protected_package_atomic_hash
- Recurrence-Count: 2
- First-Seen: 2026-07-23
- Last-Seen: 2026-07-23
- Promoted: AGENTS.md

---

## [LRN-20260723-INDEX-MAINTENANCE] best_practice

**Logged**: 2026-07-23T17:58:00+08:00
**Priority**: high
**Status**: promoted
**Area**: backend

### Summary

An indexer's bounded decision to rebuild from a last-good snapshot is maintenance, not a
user-facing failure.

### Details

The USN layer deliberately requests a background MFT rebuild when incremental subtree expansion
would exceed its work budget. Search remains available from the previous snapshot. Publishing that
reason through `last_error` made the UI claim that a volume had failed even though recovery was
working normally and completed successfully. The reason belongs in the maintenance event and log;
only poll, compaction, or snapshot-worker failures belong in the error channel.

### Suggested Action

Keep operational state, diagnostic reason, and actual failure as separate protocol concepts. When a
safe fallback is scheduled and usable data remains available, show an updating state and preserve
the reason for logs rather than elevating it to an error.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-indexer/src/lib.rs, rust/apps/nxtools-indexer/src/usn.rs
- Tags: indexing, state-machine, maintenance, error-semantics, last-good-snapshot
- Pattern-Key: backend.maintenance_not_error
- Recurrence-Count: 1
- First-Seen: 2026-07-23
- Last-Seen: 2026-07-23
- Promoted: AGENTS.md

---

## [LRN-20260722-QLFOCUS] best_practice

**Logged**: 2026-07-22T16:10:00+08:00
**Priority**: high
**Status**: promoted
**Area**: frontend

### Summary

Quick Launch keyboard and pointer selection must enter one preview state machine, and an interactive
preview must use a persistent native no-activate policy before it is shown.

### Details

Up/Down changed the selected row and viewport but never scheduled a preview, while pointer hover did.
Separately, GPUI `focus: false`, `SW_SHOWNA`, and `SWP_NOACTIVATE` prevented initial activation but
did not guarantee that later pointer interaction could never activate the preview HWND. The stable
fix schedules the selected keyboard result through the same generation-scoped delayed preview path
and applies `WS_EX_NOACTIVATE` to only the preview window before making it visible. A later real-window
retest showed that the first auxiliary GPUI window can still leave the Windows active HWND and GPUI
input focus out of sync, so preview creation must explicitly restore both layers to the search panel.

### Suggested Action

Treat selection source as input to one preview controller. For any auxiliary interactive Windows
surface that must preserve parent keyboard input, create it hidden, apply its persistent extended
window style, and only then show it.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs, csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs, tests/architecture/quick-launch-lifecycle.ps1
- Tags: gpui, keyboard-navigation, preview, focus, ws-ex-noactivate
- Pattern-Key: frontend.preview_selection_and_noactivate
- Recurrence-Count: 2
- First-Seen: 2026-07-22
- Last-Seen: 2026-07-22

### Promotion

- **Promoted**: `AGENTS.md`
- **Rule**: Share one preview state machine across selection inputs and apply persistent no-activate styles before showing auxiliary preview HWNDs.

---

## [LRN-20260722-QLPLACEMENT] correction

**Logged**: 2026-07-22T17:20:00+08:00
**Priority**: high
**Status**: promoted
**Area**: frontend

### Summary

A hidden GPUI Windows window's requested placement is not necessarily applied to its native HWND;
showing that HWND externally can expose the `CW_USEDEFAULT` creation size.

### Details

GPUI Windows stores the requested placement when `WindowOptions.show` is false and applies it through
its own activation path. Quick Launch bypassed that path with C# `ShowWindow(SW_SHOWNA)`, so the first
preview briefly appeared at the large Win32 creation bounds even though GPUI later reported the
correct logical `400x410` size. Rust now computes the intended bounds and C# atomically applies those
physical-pixel bounds with `SWP_NOACTIVATE` before visibility.

### Suggested Action

Whenever Rust creates a hidden GPUI HWND that C# will reveal, make the pre-show native contract include
the complete final bounds, no-activate style, stacking policy, and visibility order. Do not infer native
placement from GPUI's logical `WindowOptions` alone.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs, csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs, tests/architecture/quick-launch-lifecycle.ps1
- Tags: gpui, windows, placement, cw-usedefault, first-frame
- Pattern-Key: frontend.hidden_gpui_native_placement
- Recurrence-Count: 1
- First-Seen: 2026-07-22
- Last-Seen: 2026-07-22

### Promotion

- **Promoted**: `AGENTS.md`
- **Rule**: Apply complete native bounds before C# reveals a hidden GPUI HWND.

---

## [LRN-20260722-QLMETA] correction

**Logged**: 2026-07-22T16:45:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

A visible search result must always have a metadata preview; content-read failure is a degradation
case, not a reason to suppress the preview window.

### Details

The quick-launch loader returned `None` for stale index rows, virtual application targets,
temporarily unavailable paths, and text files that disappeared between metadata and content reads.
The hover lifecycle then hid the reusable preview window, making previews appear intermittent even
though every Windows positioning and visibility command succeeded.

### Suggested Action

Build the metadata payload first for every visible result, then opportunistically upgrade it to a
bounded image or text body. Keep unavailable-target and read-race regression tests at the loader
boundary.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs
- Tags: quick-launch, preview, graceful-degradation, stale-index
- Pattern-Key: frontend.preview_metadata_fallback
- Recurrence-Count: 1
- First-Seen: 2026-07-22
- Last-Seen: 2026-07-22

### Resolution

- **Resolved**: 2026-07-22T16:49:00+08:00
- **Notes**: Added an always-available metadata fallback, protected text read races, and passed the
  full Release package gate with dedicated unavailable-target and preview-motion regressions.

---

## [LRN-20260721-SMOKE-EVENT] correction

**Logged**: 2026-07-21T15:12:00+08:00
**Priority**: high
**Status**: resolved
**Area**: testing

### Summary

A duplex service smoke test must demultiplex unsolicited status events before asserting a correlated
search response. PowerShell's `@($null)` can report a count of one, so blindly reading the next frame
created a false-positive result with an empty path and elapsed time.

### Suggested Action

- Read until `indexer_event/search_batch` with the expected generation and `complete=true`.
- Treat EOF and deadline expiry as failures, and assert a non-empty real result path.
- Apply the same event filtering to every long-lived named-pipe probe.

### Metadata

- Source: release_gate
- Related Files: tests/performance/indexer-service-smoke.ps1
- Tags: named-pipe, smoke-test, event-demultiplexing, powershell

---

## [LRN-20260721-IDXCPU] correction

**Logged**: 2026-07-21T13:12:00+08:00
**Priority**: critical
**Status**: resolved
**Area**: backend

### Summary

An indexer's `Ready` state and functional search result do not prove that its background lifecycle is idle or resource-acceptable.

### Details

The release gate verified status stability and search latency but omitted startup/maintenance CPU and memory. The packaged indexer can consume roughly one logical core during full MFT reconstruction and retain a very large working set; comparing only protocol correctness to Listary concealed the operational regression.

### Suggested Action

Require an agent-runnable service CPU/memory probe across restart, initial build, ready idle and journal maintenance. Treat snapshot persistence and bounded resident memory as first-class indexer requirements, and cite exact reference-source boundaries instead of implying unavailable native Listary internals were inspected.

### Metadata

- Source: user_feedback
- Related Files: tests/performance/indexer-idle-cpu.ps1, rust/apps/nxtools-indexer/src/lib.rs, rust/apps/nxtools-indexer/src/mft.rs
- Tags: indexer, cpu, memory, persistence, release-gate
- See Also: LRN-20260721-LSR
- Pattern-Key: performance.release_gate_requires_resource_probe
- Recurrence-Count: 1
- First-Seen: 2026-07-21
- Last-Seen: 2026-07-21

### Resolution

- Added a real LocalSystem service CPU/memory regression probe selected by PID through Windows performance counters.
- Replaced per-session full MFT reconstruction with versioned immutable snapshots, memory-mapped restore and persisted USN deltas.
- Final packaged-service restart measured 0.03% average CPU, 0.13% peak CPU and 5.4 MB peak private working set over eight seconds; the pre-fix reproduction averaged 8.10% and exceeded 1.1 GB.
- **Resolved**: 2026-07-21T13:45:00+08:00

---

## [LRN-20260721-LSR] correction

**Logged**: 2026-07-21T12:55:00+08:00
**Priority**: critical
**Status**: resolved
**Area**: frontend

### Summary

Do not implement Listary-compatible search behavior from the research summary alone; reopen and
trace the exact decompiled result-limit, incremental-publication and selection-input call chains.

### Details

Quick Launch hard-coded an eight-result provider/UI limit and let row hover immediately overwrite
keyboard selection. The index maintenance policy also surfaced an internal bounded-work rebuild as
a user-facing partial failure. These choices were not verified against Listary's concrete search
session, result collection and input-controller code even though the reference source was locally
available. Existing documentation was treated as a substitute for the required source audit.

### Suggested Action

Before changing Quick Launch behavior, cite the exact Listary decompiled files and methods, record
the observed result-window, incremental update, scrolling and mouse-versus-keyboard selection
invariants in `docs/reference-code/quick-launch-research.md`, then add boundary-level RED tests.

### Metadata

- Source: user_feedback
- Related Files: docs/reference-code/quick-launch-research.md, rust/apps/nxtools-desktop/src/quick_launch.rs, rust/apps/nxtools-indexer/src/usn.rs
- Tags: listary, reference-source, result-limit, selection, usn
- See Also: LRN-20260720-QAM, LRN-20260718-001
- Pattern-Key: migration.reference_source_before_behavior
- Recurrence-Count: 1
- First-Seen: 2026-07-21
- Last-Seen: 2026-07-21

### Resolution

- **Resolved**: 2026-07-21T13:05:00+08:00
- **Notes**: Reopened the exact Listary launcher and `SearchItemList` chains, documented the 50-result and real-pointer-movement invariants, added RED/GREEN boundary tests, and corrected result scrolling, selection arbitration, and USN maintenance availability.

---

## [LRN-20260721-QRY] correction

**Logged**: 2026-07-21T11:25:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

An asynchronous search provider becoming ready must replay the query visible in the popup, even if
disconnect handling invalidated the provider's internal query session.

### Details

During index-service reconnect or MFT rebuild, the application catalog could answer a query while
the file provider was unavailable. The later `Ready` status only refreshed diagnostics, leaving the
visible query stranded until another keystroke. Clearing the internal generation/query on disconnect
made the failure persistent even though the service already held a searchable multi-million-item
snapshot.

### Suggested Action

Treat provider readiness as part of the active search lifecycle. On a non-ready to `Ready`
transition, read the current popup input, start a fresh generation and resubmit it. Keep a regression
test covering delayed readiness and disconnect invalidation.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs, docs/reference-code/quick-launch-research.md
- Tags: quick-launch, reconnect, async-search, generation, lifecycle
- Pattern-Key: frontend.async_provider_ready_replays_visible_query
- Recurrence-Count: 1
- First-Seen: 2026-07-21
- Last-Seen: 2026-07-21

### Resolution

- **Resolved**: 2026-07-21T11:25:00+08:00
- **Notes**: Ready transitions now resubmit the visible popup query with a fresh generation.

---

## [LRN-20260720-QAM] correction

**Logged**: 2026-07-20T16:10:00+08:00
**Priority**: high
**Status**: promoted
**Area**: frontend

### Summary

Do not replace a reference product's in-window action-search mode with a generic floating context
menu just because both expose actions for a selected result.

### Details

Listary routes result right-click and bare Right-at-query-end through the same nested `SearchMode`.
The action list stays inside the launcher, remains searchable and keyboard navigable, can push Shell
submenus, and pops back to the preserved parent query/selection. NxTools.Next instead attached
GPUI Component's generic `ContextMenu` directly to each row, changing focus, lifecycle, navigation
and state restoration semantics.

### Suggested Action

Before implementing a reference interaction, trace the event through state transition, rendering
surface, keyboard routing, focus ownership and exit/restoration—not only the final commands shown.
For quick-launch actions, model an explicit Rust-owned action-mode stack and render it in the GPUI
launcher; reserve a native/floating context menu only for behavior the reference actually presents
that way.

### Metadata

- Source: user_feedback
- Related Files: docs/reference-code/quick-launch-research.md, rust/apps/nxtools-desktop/src/quick_launch.rs
- Tags: listary, quick-launch, actions, context-menu, interaction-model
- Pattern-Key: migration.reference_interaction_state_machine
- Recurrence-Count: 1
- First-Seen: 2026-07-20
- Last-Seen: 2026-07-20
- Promoted: docs/reference-code/quick-launch-research.md

---

## [LRN-20260720-QLP] correction

**Logged**: 2026-07-20T15:16:00+08:00
**Priority**: high
**Status**: promoted
**Area**: docs

### Summary

Do not rename an unplanned stabilization slice as the next numbered phase when a canonical delivery
plan already defines that phase.

### Details

`docs/quick-launch-plan.md` defines phase 2 as the MFT/USN-backed local file and folder index. The
assistant instead called application-search polishing “phase 2” and implemented some phase-3
actions, making reported progress diverge from the approved roadmap even though the code was useful.

### Suggested Action

Before answering “what is next” or starting a numbered phase, reread the plan headings, name the
canonical phase verbatim, and classify extra stabilization work as a phase-1 follow-up. Any reordered
roadmap requires explicit user approval and a corresponding plan-file update.

### Metadata

- Source: user_feedback
- Related Files: docs/quick-launch-plan.md, rust/apps/nxtools-indexer/src/main.rs
- Tags: roadmap, phase-gating, progress-reporting, scope
- Pattern-Key: workflow.canonical_phase_gating
- Recurrence-Count: 1
- First-Seen: 2026-07-20
- Last-Seen: 2026-07-20
- Promoted: AGENTS.md

---

## [LRN-20260720-NOT] correction

**Logged**: 2026-07-20T12:41:37+08:00
**Priority**: medium
**Status**: resolved
**Area**: frontend

### Summary

A transient notification is not interaction-complete when it appears and disappears without the
requested lifecycle motion.

### Details

The first notification pass corrected hierarchy, paths and actions but still inserted and removed
cards immediately. The user had previously requested low-cost appearance motion, so both entrance
and exit states belonged in the initial lifecycle contract.

### Suggested Action

Specify notification lifecycle states together with layout: use short generation-scoped one-shot
GPUI animations, retain an exiting item until its animation ends, and never use repeating animation.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/notifications.rs, rust/apps/nxtools-desktop/src/main_window.rs
- Tags: gpui, notification, animation, lifecycle, performance
- See Also: LRN-20260718-001
- Pattern-Key: frontend.notification_lifecycle_completeness
- Recurrence-Count: 1
- First-Seen: 2026-07-20
- Last-Seen: 2026-07-20

### Resolution

- **Resolved**: 2026-07-20T12:41:37+08:00
- **Notes**: Added bounded enter/exit states and regression coverage.

---

## [LRN-20260718-003] best_practice

**Logged**: 2026-07-18T13:30:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

GPUI window-activation loss is not an authoritative signal for closing a multi-window clipboard surface.

### Details

Opening the owned hover-preview window deactivated the clipboard panel. The panel treated every deactivation as an external click, hid itself, destroyed the preview before its topmost command completed, and allowed the windowless GPUI process to exit. After removing that path, delayed crashes still occurred because hover transitions destroyed and recreated preview windows. The next recurrence was proven from a Windows crash dump: a foreground notification already held the `ClipboardApplication` entity lease, then `ClipboardPanel::close` synchronously updated that same entity and triggered GPUI's fatal `double_lease_panic`. Windows foreground events from PlatformHost carry the native HWND needed to distinguish the panel, its owned preview, and external applications.

### Suggested Action

Use the C# foreground-window event as the single close authority, pass all owned native window handles into the Rust policy, and regression-test internal versus external foreground transitions. Defer cross-entity close side effects until the current GPUI update finishes. Keep the owned preview window alive and reuse/hide/reposition it across hover and panel sessions; only let process shutdown destroy it. Never close a multi-window surface from an undifferentiated GPUI activation callback.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/clipboard.rs, rust/apps/nxtools-desktop/src/ui/mod.rs
- Tags: gpui, popup, window-lifecycle, crash
- Pattern-Key: frontend.authoritative_owned_window_lifecycle
- Recurrence-Count: 3
- First-Seen: 2026-07-18
- Last-Seen: 2026-07-18

---

## [LRN-20260718-002] correction

**Logged**: 2026-07-18T12:10:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

NxTools.Next clipboard controls must use Google Material Symbols, and every GPUI icon asset must be embedded through an application `AssetSource`.

### Details

The UI used `gpui-component::IconName` without configuring `Application::with_assets`. The resulting SVG paths resolved against GPUI's empty default asset source, leaving category creation and management controls visually blank. The user also explicitly selected the Google Fonts Material Symbols library as the product icon source.

### Suggested Action

Use a typed Material Symbol map, embed selected official SVG files in the executable, and keep a test that loads every clipboard icon path through the real application asset source.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/assets.rs, rust/apps/nxtools-desktop/src/icons.rs, rust/apps/nxtools-desktop/src/ui
- Tags: gpui, icons, material-symbols, assets
- Pattern-Key: frontend.verify_runtime_asset_source
- Recurrence-Count: 2
- First-Seen: 2026-07-18
- Last-Seen: 2026-07-18

---

## [LRN-20260718-001] correction

**Logged**: 2026-07-18T09:35:00+08:00
**Priority**: high
**Status**: in_progress
**Area**: frontend

### Summary

Clipboard UI should use supplied projects for proven interaction rules and measurements while retaining NxTools.Next's own GPUI/Zed visual direction.

### Details

The first GPUI implementation exposed metadata as stacked chips, forced image content into the text-row geometry, and lacked a deliberate density/alignment system. The user rejected that result, then clarified that the old project is a reference rather than a pixel-copy target. Reuse behavior and layout logic, not the old Avalonia appearance.

### Suggested Action

Before editing GPUI layout, inspect the exact old NxTools view and supplied screenshots, extract interaction/layout invariants, then reinterpret them through the new GPUI/Zed design system and require screenshot-based design QA.

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/ui/history.rs, rust/apps/nxtools-desktop/src/ui/mod.rs
- Tags: gpui, clipboard, visual-fidelity
- Pattern-Key: frontend.reference_fidelity_before_implementation
- Recurrence-Count: 3
- First-Seen: 2026-07-18
- Last-Seen: 2026-07-18

### Promotion

- **Promoted**: `AGENTS.md`
- **Rule**: Inspect the old/reference implementation and capture lifecycle invariants before implementing migrated interactions.

The recurrences were missing behavior parity for QQ single-image promotion, hover preview, open-target actions, native resize, and finally the preview window lifecycle itself. Feature slices must include an explicit old-NxTools interaction inventory before implementation, not only before declaring the slice complete.

---
## [LRN-20260720-HWN] correction

**Logged**: 2026-07-20T09:49:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary
Do not introduce native HWND shaping when a square outer GPUI shell removes the transparent-corner artifact.

### Details
The initial fix moved rounded-window policy into the C# Windows adapter. The user clarified that the
outer frame does not need to be rounded. A rectangular GPUI root that fills the client bounds removes
the artifact with less code and keeps all visual policy in Rust/GPUI.

### Suggested Action
Keep popup outer shells rectangular, allow rounded inner cards, and regression-test that C# contains
no window-shape service.

### Metadata
- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/ui/mod.rs, rust/apps/nxtools-desktop/src/ui/preview.rs
- Tags: gpui, hwnd, transparent-window, architecture-boundary

### Resolution
- **Resolved**: 2026-07-20T09:49:00+08:00
- **Notes**: Removed native region shaping and made panel/preview outer shells rectangular.

---

## [LRN-20260721-IDX] correction

**Logged**: 2026-07-21T09:20:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

An indexer handshake is not proof that a duplex named-pipe protocol works, and an NTFS enumeration
must seed the volume root independently of whether record 5 appears in the returned batch.

### Details

The initial service reached `READY` with zero paths because path reconstruction required an
enumerated self-parent root record. Listary's inspected index model instead receives and seeds an
explicit root FRN. After that fix, post-handshake commands still stalled because a synchronous read
on one duplicated pipe handle blocked writes through the other handle that shares the same Windows
file object.

### Suggested Action

- For NTFS, model root record number 5 before resolving enumerated children.
- Require a real two-round duplex pipe test where the reader is already waiting for request two
  while the writer sends response one.
- Keep a packaged-service smoke test that asserts `READY`, nonzero real items, file results,
  directory results and bounded wall time.

### Metadata

- Source: production_debugging
- Related Files: rust/apps/nxtools-indexer/src/mft.rs, rust/crates/nxtools-ipc/src/local_pipe.rs, tests/performance/indexer-service-smoke.ps1
- Tags: ntfs, mft, named-pipe, duplex, windows-service

---

## [LRN-20260722-QLP] best_practice

**Logged**: 2026-07-22T15:08:00+08:00
**Priority**: critical
**Status**: promoted
**Area**: frontend

### Summary

Every GPUI entity callback must defer an application update when that application can synchronously
write back into the currently leased entity, including cache-hit paths that appear read-only.

### Details

A quick-launch result-row `MouseMove` synchronously called
`QuickLaunchApplication::request_icons`. On an icon cache hit, that method immediately called
`popup.panel.update`, re-entered the panel lease and triggered GPUI `double_lease_panic`; Rust then
terminated through `FAST_FAIL_FATAL_APP_EXIT`. The icon-miss path did not crash, which initially
obscured the cache-dependent condition. The Windows crash dump and matching Release PDB exposed the
exact `pointer_moved_result -> request_icons -> panel.update` stack.

### Suggested Action

Update local view state during the current callback, but schedule cross-entity/application work for
the next GPUI application turn. Add a lifecycle gate at each callback boundary, not merely a unit
test for the downstream cache.

### Metadata

- Source: production_debugging
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs, tests/architecture/quick-launch-lifecycle.ps1
- Tags: gpui, entity-lease, mouse-move, cache-hit, crash
- Pattern-Key: frontend.defer_cross_entity_reentry
- Recurrence-Count: 1
- First-Seen: 2026-07-22
- Last-Seen: 2026-07-22

### Promotion

- **Promoted**: `AGENTS.md`
- **Rule**: Defer any GPUI callback's application update when it can synchronously write back into the currently leased entity.

---

## [LRN-20260723-001] best_practice

**Logged**: 2026-07-23T11:35:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

Every Windows `SendInput` declaration must model the full native `INPUT` union, even when the
current call only sends keyboard input.

### Details

A keyboard-only explicit union produced a 32-byte x64 structure instead of Windows' required
40-byte `INPUT`. `SendInput` therefore rejected an otherwise correct `Ctrl+L` sequence. Adding
mouse and hardware variants restored the native union size and made real file-dialog address
navigation succeed.

### Suggested Action

Expose the native structure size to tests and compare it with 40 bytes on x64 or 28 bytes on x86
whenever a new Windows input sender is introduced.

### Metadata

- Source: error
- Related Files: csharp/NxTools.PlatformHost/Windows/FileDialogWindowAdapter.cs, csharp/NxTools.PlatformHost.Tests/Program.cs
- Tags: win32, sendinput, abi, interop
- Pattern-Key: windows.sendinput_full_union
- Recurrence-Count: 1
- First-Seen: 2026-07-23
- Last-Seen: 2026-07-23

### Resolution

- **Resolved**: 2026-07-23T11:35:00+08:00
- **Notes**: The file-dialog adapter now uses a complete `INPUT` union and the shared ABI-size regression test covers both input senders.

---

## [LRN-20260723-002] correction

**Logged**: 2026-07-23T11:45:00+08:00
**Priority**: high
**Status**: pending
**Area**: frontend

### Summary

“切换到此目录”在可导航宿主中不应只由结果的文件夹类型决定。

### Details

用户指出：从打开/另存为对话框呼出速启后，选中普通文件、程序或其他具备本地文件系统目标的
非文件夹结果时，也必须显示“切换到此目录”。目录结果导航到自身；非目录文件系统结果导航到
其父目录。该动作只改变原文件对话框的当前目录，不启动结果、不打开新资源管理器，也不自动
确认打开/保存。

### Suggested Action

审查 Rust `directory_navigation_target` 对应用、快捷方式和其他本地路径来源的覆盖范围，以“能否
解析出安全的绝对目录目标”为动作可用性依据，而不是只匹配索引文件/索引目录两个来源枚举。

### Metadata

- Source: user_feedback
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs
- Tags: quick-launch, file-dialog, action-availability, local-path

---

## [LRN-20260723-003] best_practice

**Logged**: 2026-07-23T16:48:00+08:00
**Priority**: high
**Status**: promoted
**Area**: frontend

### Summary

GPUI entity callbacks must defer any update that can synchronously lease the entity currently
executing the callback.

### Details

`QuickLaunchPanel::run_search` owned the panel lease while synchronously updating
`QuickLaunchApplication`. Its cached-icon path immediately called `popup.panel.update`, which
re-leased the same panel and triggered GPUI's fatal `double_lease_panic`. The process aborted through
`panic_cannot_unwind`, so ordinary application logs did not contain the panic payload; the saved
native dump and Release PDB exposed the exact nested entity stack.

### Suggested Action

When a panel callback invokes another entity that may call back into the panel, use `cx.defer` (or
the shared deferred-update helper) at the return boundary. Keep architecture gates around known
entity cycles because ordinary unit tests do not exercise GPUI's runtime lease map.

### Metadata

- Source: error
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs,
  tests/architecture/quick-launch-lifecycle.ps1
- Tags: gpui, reentrancy, double-lease, crash-dump
- Pattern-Key: gpui.defer_cross_entity_cycle
- Recurrence-Count: 1
- First-Seen: 2026-07-23
- Last-Seen: 2026-07-23
- Promoted: tests/architecture/quick-launch-lifecycle.ps1

### Resolution

- **Resolved**: 2026-07-23T16:47:00+08:00
- **Notes**: Cached icon application is deferred past the active panel lease; the new architecture
  regression check failed before the fix and passes after it.

---

## [LRN-20260725-001] correction

**Logged**: 2026-07-25T00:00:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

Quick Launch custom-command prefixes must not make command results exclusive.

### Details

The original phase-5 invariant suppressed application and MFT providers whenever a query remained a
possible custom-command prefix. User acceptance showed the resulting behavior was wrong: defining
`Test` and typing `t` returned only that command. Commands should be high-ranked instant results in
the same generation-scoped batch while ordinary applications, files and folders remain searchable.

### Suggested Action

Keep custom commands in the unified Finder result vector and never use command-prefix detection to
disable the file-index request. Preserve the existing late-generation rejection so asynchronous
file results cannot overwrite the command row.

### Metadata

- Source: user_feedback
- Related Files: rust/crates/nxtools-finder/src/lib.rs, docs/reference-code/quick-launch-research.md
- Tags: quick-launch, custom-command, provider-merge, ranking
- Pattern-Key: quick_launch.providers_are_additive
- Recurrence-Count: 1
- First-Seen: 2026-07-25
- Last-Seen: 2026-07-25

### Resolution

- **Resolved**: 2026-07-25T00:00:00+08:00
- **Notes**: Finder command results now merge with application and indexed-file providers; focused
  integration tests cover a one-letter command prefix.

---
## [LRN-20260728-DOWNLOADTRUNCATE] correction

**Logged**: 2026-07-28T17:31:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary

A wide GPUI flex column can still render only `...` when a nested percentage-width child uses
`truncate()`; outer bounds alone do not prove that text receives a usable layout width.

### Details

The first download-table correction added `w_full()`, `min_w_0()` and `truncate()` to the nested
file-name and path nodes. Production bounds tests passed because the outer name column was wide,
but the packaged UI still rendered only ellipses. GPUI's `truncate()` combines hidden overflow,
no-wrap and text ellipsis, and the nested percentage-width text node resolved to the ellipsis-only
case. The correct compact-row boundary is a flexible outer column with hidden overflow and
natural-width, no-wrap text children.

### Suggested Action

For nested GPUI table text, test the actual text-node bounds and avoid percentage-width
`truncate()` unless an explicit pixel width is available. Prefer parent clipping plus
`whitespace_nowrap()` for the first implementation.

### Metadata

- Source: user_feedback
- Related Files: `rust/apps/nxtools-desktop/src/main_window.rs`
- Tags: gpui, flex, truncate, download-ui
- Pattern-Key: gpui.nested_percent_truncate
- Recurrence-Count: 1
- First-Seen: 2026-07-28
- Last-Seen: 2026-07-28

### Resolution

- **Resolved**: 2026-07-28T17:31:00+08:00
- **Notes**: Replaced nested percentage-width truncation with natural-width no-wrap text inside
  the existing clipped flexible column and added a source regression gate.

---
## [LRN-20260728-DIRECTGITPUSH] correction

**Logged**: 2026-07-28T17:48:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

Do not require GitHub CLI when the user only asks to push an existing local repository to a known
GitHub remote.

### Details

The GitHub publish workflow was applied too broadly and introduced `gh` as a prerequisite even
though the requested operation was only remote configuration, commit and `git push`. GitHub CLI is
needed for PR and repository-management operations, not for ordinary Git transport.

### Suggested Action

Distinguish direct Git push from full GitHub publication/PR flow before selecting tooling. For a
direct push, use Git plus the configured credential helper and only request `gh` when a GitHub API
operation is actually required.

### Metadata

- Source: user_feedback
- Related Files: `.git/config`
- Tags: git, github, push, tooling
- Pattern-Key: git.direct_push_without_gh
- Recurrence-Count: 1
- First-Seen: 2026-07-28
- Last-Seen: 2026-07-28

### Resolution

- **Resolved**: 2026-07-28T17:48:00+08:00
- **Notes**: Switched the active publication flow to plain Git.

---
## [LRN-20260802-BROWSER-DIALOG] correction

**Logged**: 2026-08-02T22:12:00+08:00
**Priority**: high
**Status**: resolved
**Area**: browser download integration

### Summary

`openDialog=true` is a visible application lifecycle request, not a queue-only download mode.

### Details

The first browser implementation treated the extension's confirmation setting as
`DownloadSubmitMode::QueueOnly`. This produced a durable `Created` task with a URL-path filename but
never started the full desktop or showed the download confirmation window. Result-only automation
mistook task persistence for a correct ABDM/NxTools front-end handoff.

### Suggested Action

For migrated integrations, test the native show/focus boundary and pre-confirmation task count.
Route browser confirmation requests through a dedicated authenticated UI channel, keep request
credentials in the hydrated view, make probe metadata authoritative over untouched filename hints,
and compensate browser cancellation failure by dismissing that correlated dialog.

### Metadata

- Source: user_feedback
- Related Files: `rust/apps/nxtools-cli/src/browser_native_host.rs`, `rust/apps/nxtools-desktop/src/downloads.rs`, `rust/crates/nxtools-download-ipc/src/lib.rs`
- Tags: download, browser, native-messaging, gpui, lifecycle
- Pattern-Key: browser.open_dialog_is_ui_lifecycle
- Recurrence-Count: 1
- First-Seen: 2026-08-02
- Last-Seen: 2026-08-02

### Resolution

- **Resolved**: 2026-08-02T22:12:00+08:00
- **Notes**: Real Chrome automation now requires the visible confirmation window, clicks it, and
  verifies the final file; the package/protected four-binary hashes match.

---
