# REF-12 — Candidate delivery

## Outcome

**Accepted evidence-based candidate-list denoising**, at the source/owned-fixture and executed isolated-browser boundaries in [coordinator acceptance](coordinator-acceptance.md). B2 is completed at that level. All91raw observations remain; candidates91→10 retain complete small media/manifests/Blob/hook/CSS. Original timer cadence remains. No stable CPU/discovery-latency or overall-download speedup is claimed.

## Selection and original owners

One evidence-driven selection mechanism derives candidates from the original bounded observations:

- Actual ISO-segment response MIME identifies a fragment. An incomplete valid Content-Range plus a numeric `range/byterange/bytestart` parameter identifies a fragment only with actual audio/video response MIME. Ordinary ZIP/unknown-MIME range responses and stable full-resource Range URLs remain candidates.
- Manifests, explicit attachments, Blob and real DOM audio/video source evidence retain their original authority. No generic size threshold, suffix-only fragment rejection, same-folder guess or manifest refetch is used. Small complete audio/video, complete transport streams, ordinary files and opaque sources remain.
- Decorative audio requires all actual DOM facts: audio, finite positive duration at most2s, autoplay, no controls, no loop and explicit presentation/none role. Missing facts retain the candidate. Video or independently controlled use of the same URL survives incidental decorative observations; equally strong newer metadata can update.
- Raw excluded observations remain in the original120-observation/20minute cache, aliases and600ms coalesced local persistence. Capacity prefers retained candidates before excluded observations, then original capture time. Popup and chooser filter before presentation enrichment. No separate negative cache, setting, experiment switch or discovery runtime exists.
- Source-ranked MIME/size enrichment does not transfer discovery/DOM facts, frame/pageKey/source, resource/handoff identity or cross-tab credential context. MIME-derived selection facts originate only at actual response/content observation boundaries. A Blob's DOM playability does not transfer to a different associated source-url mirror; its actual response facts remain authoritative.
- The existing content candidate Map and sent dedup retain normalized facts. loadedmetadata and controls/autoplay/loop/role/type/srcset changes can republish the same URL despite unchanged size/geometry; identical facts remain coalesced. Local chooser candidates use the original runtime request, bounded to80, with `excludedUrls` derived from the same tab's original URL index. Weak local fallbacks cannot reintroduce a known fragment. Unavailable background retains original local fallback; changed pageKey retires a delayed chooser response.
- Original bucket identity scopes async network/request/page captures and explicit scans. Navigation, history/context change, close and replacement retire old publication; replacement clears only the removed tab. Same-page cache updates preserve bucket identity. Original scan/index/thumbnail state is cleared with the page; new-page explicit demand does not inherit the old3.5s throttle. Current thumbnails use the original wrapped content response.

The7s visible-page scan,1s context,1s overlay and5s YouTube observation remain separate original fallbacks. DOM, explicit-deep CSS, PerformanceObserver, page-hook, Blob, manual and batch paths remain. Selection returns fixed `fragment/decorative_audio` reasons and adds no native message, native-host demand, payload log or per-event diagnostic stream.

## Source boundary

Product files are confined to `capture-policy.js`, `background.js`, `manifest.json`, `bg/{request-context,capture,resource-cache,popup-state,resource-choices,tab-bridge,router}.js`, `content-script.js` and `content/{media-info,resource-choice-text}.js`.

`capture-policy.js` is loaded before content helpers in both manifest and original re-injection order. Permissions, CSP and dependencies are unchanged. REF-04/05, accepted B1 and unrelated dirty changes remain. The targeted REF-06 precision/current-task repair and its necessary Cargo verification are recorded in [REF-06](ref06-outcomes.md). Packaging, installation, product/reference launch, registry/service/power operations, commit and push remain outside this delivery.

Behavior references: indexed Mister response/media selection and legacy NxTools DOM/CSS/Performance/lifecycle paths were inspected before candidate changes. ABDM download behavior was inspected before legacy intake; independent local ABDM extension source is unavailable. No reference implementation, tests or assets are copied.

## Deterministic verification

Logs are in `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

| Boundary | RED evidence | Final GREEN |
| --- | --- | --- |
| ISO fragments remain observations, not candidates | `b2-ref12-fragment-red.log`:0/1 | Initial1/1; included in final25/25 |
| Range, decoration, strong-use priority, capacity/restore | `b2-ref12-selection-red.log`:1/4 |4/4, `b2-ref12-selection-green.log` |
| Navigation/history/context/close/replacement and explicit demand | `b2-ref12-lifecycle-red-final.log`:5/11; each retired capture recreated three old observations; new page inherited throttle |11/11, `b2-ref12-lifecycle-green.log` |
| Production content metadata/source facts | `b2-ref12-content-red.log`: duration arrival stayed at one message; source facts absent | Final content5/5; element/API doubles, not browser DOM |
| Background/local chooser merge and delayed page reply | `b2-ref12-merge-red.log`:14/16 | Included in final25/25 |
| Cross-source manifest/segment priority | `b2-ref12-precedence-red.log`:16/17 | Included in final25/25 |
| Cross-tab MIME cannot create selection evidence | `b2-ref12-source-red.log`:17/18 | Included in final25/25 |
| Ordinary ZIP Range remains usable | `b2-ref12-ordinary-range-red.log`:14/15 | Included in final background17/17 |
| frame/pageKey/source identity | `b2-ref12-frame-red.log`:15/16; original frame0 became3 | Included in final background17/17 |
| Blob DOM facts stay with the played URL | `b2-ref12-blob-source-red.log`:16/17; associated fragment was restored | Included in final background17/17 |
| Current REF12 background/content/workload checks | — |29/29, `b2-acceptance-discovery-final.log`:17background +5content +7workload/stage utility checks |
| Current recursive extension including B1/REF-05 and helper prechecks | — |255/255, `b2-acceptance-extension-final.log` |
| Current extension JS/CJS/MJS syntax | — |71files, `b2-acceptance-syntax-final.log` |
| Current spec/component/release/editor contracts | — |Four passed;128documents/14owners; `b2-acceptance-static-final.log` |
| Earlier scoped whitespace/diff gate | — |Passed; `b2-ref12-diff-check.log` |
| Failure-page preservation helper RED | `b2-helper-failure-red.log`: failed Page was navigated blank a second time | Included in full255/255 |
| Helper/production Popup prechecks | — |7workload/stage utilities and14Popup production-script tests included in full255/255; not real browser acceptance |

Counts overlap. Existing cache tests retain canonical/alias cleanup,120distinct capacity, TTL cutoff, restore, source priority and late acceptance. Actual new event/runtime tests cover redirect, manifest/no-suffix MIME, ordinary/opaque/small complete media, range/attachment precedence, duplicate fragments, Blob, manual confirmation and `openDialog:false` batch authority. B1 Cookie/pending/cancel/compensation and REF-05 rules/session pause remain in the full recursive run. The workload utility tests validate ordering, missing-CPU rejection, statistics and real source-bundle loading; their test numbers are synthetic arithmetic inputs, not measurements.

## Browser handoff

Frozen discovery-only baseline, including completed REF-05, is unchanged:

`D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b2-pre-ref12-baseline`

73files, frozen2026-11-10T06:08:44.9384511+08:00. Adjacent `b2-ref12-pre-status.txt` and `b2-ref12-pre.diff` record its boundary. Original `b2-extension-baseline` is also unchanged.

Candidate:

`D:\Code\NxTools.Next-main\browser_extension\nxtools-interceptor`

Entry: `tests/discovery-browser.mjs::runDiscoveryComparison(page,{baselineDirectory,candidateDirectory,outputPath,onStage?})`. The coordinator supplies its already-authorized Page in the same ego nodejs invocation; optional `onStage({stage})` receives low-frequency progress:

```js
const { runDiscoveryComparison } = await import("file:///D:/Code/NxTools.Next-main/browser_extension/nxtools-interceptor/tests/discovery-browser.mjs");
const result = await runDiscoveryComparison(page, {
  baselineDirectory: "D:/.pi-desktop/data/scratch/0ffe6ffa-9e52-420b-af61-a29fdd0f37b4/b2-pre-ref12-baseline",
  candidateDirectory: "D:/Code/NxTools.Next-main/browser_extension/nxtools-interceptor",
  outputPath: "D:/.pi-desktop/data/scratch/0ffe6ffa-9e52-420b-af61-a29fdd0f37b4/b2-ref12-browser-trials.json"
});
console.log(JSON.stringify(result));
```

The helper accepts Page directly and never creates, claims or finishes a TaskSpace. It runs production Popup DOM checks, then three serial interleaved groups: baseline/candidate, candidate/baseline, baseline/candidate. Each trial uses the same isolated API implementation/settings,24response/chooser rounds,80distinct fragments,24repeated fragments per round, complete/manifest/ordinary/opaque/Blob/DOM/hook/CSS sources and an8.1s idle interval. All original timers run normally.

Raw results contain CDP TaskDuration start/end and active/idle deltas, wall time, per-round discovery latency, event/runtime/scan/timer work, retained observations/candidates and DOM/selection checks. Summary median and slowest are computed only from the three actual trials of each variant. Missing TaskDuration is an error, never replaced by counts. The metric is renderer thread task time including the same-origin background fixture iframe, not installed-extension OS CPU. Media metadata is deterministic; real DOM events/properties and production scripts are used, not actual media-decoder/provider acceptance.

Partial JSON is written before Popup starts and before every named stage. `popup.load/settings/autosave/save/pause/resume/held-*/session/release` identify controlled asynchronous phases. Prepare separates document, iframe creation/readiness, background/content installation and readiness. Each `trial.<group>.<variant>.active.round.<1..24>` is one named asynchronous Page job, started by a synchronous evaluate and observed with supported bounded `waitForFunction`; normal waits are10s, the unchanged8.1s idle has a12s observation bound. No unsupported evaluate timeout option or accelerated clock is used. Pending timeout is a failure with possible late effects, never success or permission to continue.

Current trial metrics, completed rounds, per-round observations/candidates/work and Popup checks/held-request IDs/types/states persist without needing a completed trial. Only each intended first refresh/pause request is held; exact IDs release once and later requests pass through. On failure the output retains stage, original error name/message, executionStopped/mayHaveLateEffects/pending-job facts and a safe read-only snapshot (or snapshot error); the failed Page is not navigated away. Original errors are rethrown. Metric disabling cannot replace the original failure. Both variants use identical staged instrumentation/checkpoints; measured thread task time includes that instrumentation.

## Coordinator acceptance and limits

The coordinator completed real isolated Popup DOM22checks and78comparison/conformance assertions, retaining all six AB/BA/AB raw trials, assertions and logs in [the acceptance evidence](coordinator-acceptance.md). Both variants retain91observations; the candidate retains10useful candidates. Active task median1995.342→1959.447ms, idle262.074→258.909ms and discovery latency1813.100→1817.950ms are controlled fixture results; the third candidate idle sample differs substantially. The accepted benefit is denoising, not stable CPU/latency superiority. Original7s/1s/1s/5s duties remain.

The coordinator-owned TaskSpace was finished without claiming user pages. This implementation session performed no browser operation. Installed MV3, actual media decoding/provider authentication, native HWND/physical keyboard/IME/DPI, human visual, packaged/installed and full-workspace acceptance remain separate. B2 status is completed at the explicitly accepted boundaries; see the authoritative coordinator record.
