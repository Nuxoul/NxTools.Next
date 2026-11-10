# B2 — Intake and control

## Owners and boundaries

B2 implements REF-04, REF-05, REF-06 and REF-12 in that order. DownloadApplication/NewDownloadView remain the intake and confirmation owners; ClipboardApplication retains link-panel identity and cancellation. Extension bg/settings, capture, request-context, resource-cache and native-transport retain their current responsibilities. Original engine task/global/runtime limiter and stores remain authoritative. No additional controller, transport, task queue, settings store or discovery runtime is introduced.

The pre-change extension is retained at `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b2-extension-baseline`; initial status and scoped diff are adjacent. Accepted B1 behavior and unrelated dirty changes are preserved.

## REF-04 — Explicit link-panel download action

The compact420DIP panel keeps28DIP rows. Compose released0.8.0 ListItem/Label with an additional named XSmall ghost Button, alongside independent Copy/Open; measure the remaining name lane and all action bounds. There is no Toolbar parent variant adaptation. Use the official Button's pointer/keyboard activation, not a separate key-command implementation.

The owning panel closes locally; deferred work checks its original window before reaching a weak binding to the existing DownloadApplication. The source must be one admitted HTTP(S) URL. A URL-only NewDownloadInitialSource feeds the original URL InputState, without browser-request auto-probe or one-shot clipboard reading. Continue and confirmation remain unchanged. Duplicate intake activates the original window. An eligible pristine blank draft may be filled; populated, edited-then-cleared, probing, submitting, browser/Blob and fixed-handoff drafts are preserved. Root's released public view() exposes the existing NewDownloadView; no extra retained production view owner is needed.

Keep placement, topmost/activation, original fixed confirmation, deferred input focus, IME/paste and close ownership. Carry the action correlation into native activation commands. Emit bounded started/finished state with fixed outcomes and elapsed time, never URL/path/text. Real production pointer/keyboard and close-observer fixtures cover selected URL, stable input/window, no clipboard/probe/task/network, local targets and replaced panels.

## REF-05 — Fine-grained capture rules

Extend the existing settings schema and actual automatic admission chain. Persistent enablement remains separate from session pause; session storage restores pause before automatic decisions after worker recreation. Order is pause/explicit exclusions, then type/known-positive length, then original handoff. Exact site matching reuses siteHostKey; address/type syntax is documented with its existing normalizers after the relevant production seam is traced. Unknown size is not fetched by the extension. Manual actions retain explicit authorization. B1 fixed reasons, original source Cookie store, strict confirmation pending, durable matching acceptance and cancellation/compensation remain unchanged.

Real Chromium/Firefox background events cover redirects, known/unknown length, precedence, reload, worker recreation, failure fallback and duplicate identities. Popup controls reuse current DOM/settings action owners.

## REF-06 — Deliberate speed-limit submission

Keep the original InputState and task/settings owners, distinguishing editing text from the last applied value. Change events update draft only. Valid Enter/blur commits once; Escape restores applied text. Invalid/empty keeps the applied rate and displays a local error. Positive enabled values and None/unlimited retain original explicit units, durable save ordering, minimum task/global/runtime ceilings and original waiter signalling. Inspect exact released Input/NumberInput events and real editor entry before editing each surface.
The KiB display rounds positive bytes upward only for presentation. Untouched Settings Apply preserves exact authoritative bytes; explicit edits keep checked KiB semantics. Original task-window refreshes use current full engine intent outside render and synchronize only a clean stable editor. External typed commands update applied authority while local dirty/pending/error drafts survive.

Use production GPUI typing/Enter/blur/Escape, reopen and failed persistence fixtures; retain real HTTP/HLS/DASH rate-waiter tests. No limiter or scheduler rewrite.

## REF-12 — Event-first media discovery

Network observations remain primary, publication is coalesced and candidates retain original navigation/tab/source ownership and bounded TTL/caps. Preserve DOM/page-hook/PerformanceObserver/Blob and batch fallbacks where response observations are insufficient. Explicit list demand may request existing deeper discovery; no credentialed manifest refetch is added to extension code.

Verify actual coverage before reducing periodic work, one mechanism at a time. Run at least three serial interleaved baseline/candidate groups with equivalent browser workloads, no Cargo or competing benchmark. Record raw task-time/idle-CPU, scans/events, retained candidates and discovery latency; report median and slowest run. Adopt a mechanism only with demonstrated benefit and preserved coverage; otherwise retain current behavior without a permanent experiment switch.
The supplied-Page helper exposes bounded Popup/prepare/trial/round stages and checkpoints partial raw output from initialization onward. Each async round is a named observable job waited with the supported Page API; pending failures stop further actions and preserve the page. Held Popup targets have exact once-only IDs, with subsequent requests passed through. All24rounds/80fragments/24duplicates,8.1s idle and original timers remain identical across AB/BA/AB.

## Verification and delivery

Each vertical slice observes an actual production-boundary RED before correction and GREEN afterward. Scoped formatting/lint, affected Node/Rust/HTTP/storage/IPC/GPUI and B1 regressions detect concrete integration failures; known independent baseline failures are attributed rather than suppressed. Spec/component/release/editor gates verify documentation and dependency boundaries, not native behavior.

Source/owned-fixture and executed isolated-browser B2 boundaries are accepted in [coordinator acceptance](coordinator-acceptance.md). The denoising mechanism is adopted with original timers retained; stable CPU/latency or overall-download speedup is not established. Native/physical/IME/DPI/provider, human visual, installed and full-workspace acceptance remain separate. Product/reference launch, packaging/install, registry/service/power, commit and push permissions are unchanged.
