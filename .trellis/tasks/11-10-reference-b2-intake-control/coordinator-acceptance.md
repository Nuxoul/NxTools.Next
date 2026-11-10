# B2 — Coordinator acceptance

Date: 2026-11-10.

## Decision

**Accepted for source/owned fixtures and the executed isolated-browser boundaries.** REF-04, REF-05, REF-06 and REF-12 satisfy this batch's requirements. No B2 product finding remains open from the directed review and subsequent focused verification. Whole-workspace, native/physical/IME/DPI/provider, human visual and packaged/installed acceptance are separate.

- REF-04 retains original link-panel and Add Download owners, stable URL input/window, dirty-draft protection and no automatic clipboard read/probe/task/network work on URL-only intake.
- REF-05 retains original admission/settings owners, explicit rule precedence, unknown-length behavior, session pause and B1 handoff/Cookie ownership. Production Popup fields, autosave/error drafts, keyboard pause/resume, delayed refresh and retry behavior pass real DOM checks with isolated APIs.
- REF-06 retains exact applied bytes for untouched global settings, including1/1023/1537B/s, and current authoritative task limits across compact progress refreshes, external typed commands and reopen. Applied state, invalid/dirty drafts, original persistence, limiter wakeup and deferred callback ownership remain distinct.
- REF-12's evidence-based candidate selection is accepted as resource-list denoising. Existing fallback/timer cadence remains unchanged. The measured result does not justify a stable CPU or discovery-latency speedup claim, and no timer-reduction optimization is adopted.

## Independently executed checks

Named sets overlap and are not an additive test total. Rust invocations used a single lane, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, scratch TEMP/TMP, `--locked --offline`, and serial test threads. They completed before/after, not during, browser measurement.

| Check | Result | Retained evidence |
| --- | --- | --- |
| Recursive extension Node suite |255passed/0failed |[Node log](evidence/coordinator-node.log) |
| Desktop `ref06_` |11passed/0failed |[REF-06 log](evidence/coordinator-ref06.log) |
| Desktop `ref04_` |6passed/0failed |[REF-04 log](evidence/coordinator-ref04.log) |
| Desktop `download_intake_` |11passed/0failed |[Intake log](evidence/coordinator-intake.log) |
| BrowserHost full library |48passed/0failed |[Host log](evidence/coordinator-browser-host.log) |
| Desktop `ref01_` |4passed/0failed |[B1 regression](evidence/coordinator-ref01.log) |
| Desktop `tools::polish_tests::` |11passed/0failed |[UI regression](evidence/coordinator-polish.log) |
| Production Popup DOM |22checks/0failures |[Raw browser run](evidence/ref12-coordinator-browser-trials.json) |
| Comparison/conformance assertions |78checks/0failures |[Coordinator assertions](evidence/ref12-coordinator-browser-assertions.json) |

## Browser comparison

One coordinator-owned isolated TaskSpace and supplied Page completed the whole run and was finished. User pages were not claimed. HTTP/HTTPS/WebSocket traffic was blocked; browser APIs and media duration were controlled fixtures. No extension installation, product/native host or reference program was launched.

Frozen baseline: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b2-pre-ref12-baseline`.
Candidate: `D:\Code\NxTools.Next-main\browser_extension\nxtools-interceptor`.
Entry: [runDiscoveryComparison](../../../browser_extension/nxtools-interceptor/tests/discovery-browser.mjs).

All six trials used24response/chooser rounds,80distinct fragments and24repeated fragments per round,2664response events, and an8.1second idle interval. Both variants remained visible. Each retained91raw observations; complete small media, manifests, ordinary/opaque resources, Blob, page-hook and deep-CSS sources were preserved. The accepted candidate excludes the80fragment candidates and one decorative-audio candidate, retaining the10useful candidates in all three trials.

Metric: CDP `Performance.TaskDuration` with `threadTicks`, renderer thread including the same-origin background fixture iframe and identical instrumentation. This is not OS installed-extension CPU. Raw stages, per-round observations/latency/work counters, metric endpoints and results are retained in the linked JSON; the original fallback timers ran normally.

| Execution order | Variant | Active task ms | Idle task ms | Candidates / observations |
| --- | --- | ---: | ---: | ---: |
|1|Baseline|1957.720|265.007|91 /91|
|2|Candidate|1963.812|265.299|10 /91|
|3|Candidate|1959.447|258.909|10 /91|
|4|Baseline|2005.576|239.162|91 /91|
|5|Baseline|1995.342|262.074|91 /91|
|6|Candidate|1705.635|45.020|10 /91|

| Summary | Baseline | Candidate |
| --- | ---: | ---: |
| Active task median / slowest ms |1995.342 /2005.576|1959.447 /1963.812|
| Idle task median / slowest ms |262.074 /265.007|258.909 /265.299|
| Median of trial-median discovery latency ms |1813.100|1817.950|
| Slowest observed discovery latency ms |1830.400|1839.300|

The concrete benefit is cleaner candidate presentation without dropping supported complete sources or deleting underlying observations. Task-time medians are close, discovery latency is slightly higher, and the third candidate idle sample differs substantially from the other two. Three controlled trials do not establish a stable CPU/overall-latency advantage. The1second timer callbacks remained16per idle interval;7second scan alignment produced one or two callbacks. Keep the existing7s scan,1s context,1s overlay and5s YouTube responsibilities rather than attributing timer savings to this change.

## Limits

- The API/DOM fixture is not an installed MV3 service worker or actual media decoder/provider authentication. Human visual, native HWND/Shell/clipboard, physical keyboard/IME/DPI/accessibility-provider and installed/release acceptance were not performed.
- Existing message8/9, strict Desktop lint and Clipboard source-token baselines remain separately attributed in [batch outcomes](outcomes.md). This scoped acceptance is not a whole-workspace green claim.
- No packaging/install/deployed replacement, registry/service/power action, commit or push is authorized by this acceptance.

B2 may be closed at this stated level and B3 — Split-file usability may begin under the approved taskbook. Archive work remains in the canonical Phase E — Format expansion and ZIP editing.
