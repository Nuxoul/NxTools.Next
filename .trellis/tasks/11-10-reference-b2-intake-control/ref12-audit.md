# REF-12 — Discovery candidate and measurement status

## Current owners

| Boundary | Candidate behavior and original owner |
| --- | --- |
| Response/request | Original webRequest → request-context/capture → cache. Actual response MIME/Content-Range/disposition generate normalized selection facts before naming enrichment; request/Cookie provenance is unchanged. |
| DOM | Original content scan/MutationObserver/loadedmetadata observes audio/video parent facts. Fixed-field changes bypass sent dedup; identical facts coalesce. Explicit deep demand still scans CSS. |
| Page hook / Blob | Existing fetch/XHR/createObjectURL events and Blob capabilities/fallback remain. No credentialed manifest refetch. |
| PerformanceObserver | Original buffered observation and scan fallback remain; weak local observations cannot override known negative or strong DOM facts. |
| Selection | Shared capture-policy omits reliable fragments and explicitly decorative short audio, retaining manifests, attachments, Blob, ordinary/opaque/unknown and small complete sources. Popup/background chooser/local chooser use that same mechanism before expensive enrichment. |
| Explicit demand | Original3.5s throttle/injection/frame/deep scan remains; new pages retire old throttle/thumbnail state. Wrapped content response supplies thumbnail. |
| Lifecycle | Original bucket identity rejects late request/response/page/scan replies after navigation/history/context/close/replacement. onReplaced clears only removed tab. Content pageKey rejects delayed chooser results. |
| Cache |120distinct/20minute/600ms coalesced local persistence and aliases remain; retained candidates take capacity precedence over excluded raw observations. Source/frame/pageKey/discovery/DOM identity never moves through same-URL enrichment. |
| Periodic fallback |7s visible scan,1s context,1s overlay and5s YouTube observation remain different original jobs. |
| Manual/batch | Original identity/manual confirmation/Blob/32item batch authority remains separate from discovery selection and automatic admission. |

## Reference evidence

Indexed Mister `mediaKind/noteMedia/mediaList` and legacy NxTools DOM/CSS/Performance/buffered observation/explicit scan/navigation behavior were inspected before candidate changes. Mister's blanket small-file/extension thresholds, same-folder suppression and credentialed sniffing are not copied. ABDM preceded legacy download behavior; independent local ABDM extension source is unavailable. No reference implementation, tests or assets are copied.

## Verified candidate

`discovery.behavior.test.js`17/17 exercises complete production events/runtime/cache, delayed public APIs and virtual time, including Blob/source-url DOM fact isolation. `discovery-content.behavior.test.js`5/5 loads complete production content scripts with element/API doubles, not browser DOM acceptance. `discovery-workload.test.js`7/7 verifies order/metrics/statistics/source loading, bounded named jobs, exact24-round scheduling and failure-preserving partial output. Combined discovery29/29, full extension255/255 and71file syntax checks pass. Production Popup14/14 in the full suite includes exact held refresh/pause ownership, once-only release and later-request passthrough. RED/GREEN and source/lifecycle evidence are in [candidate delivery](ref12-outcomes.md).

## Frozen baselines

Original `b2-extension-baseline` is unchanged. The discovery-only `b2-pre-ref12-baseline`, including completed REF-05, was frozen before REF12 changes:73files at2026-11-10T06:08:44.9384511+08:00. Both are under session scratch `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`. Candidate is `D:\Code\NxTools.Next-main\browser_extension\nxtools-interceptor`.

## Browser verification and acceptance

`tests/discovery-browser.mjs::runDiscoveryComparison(page,{baselineDirectory,candidateDirectory,outputPath,onStage?})` accepts one supplied Page and absolute source directories. One ego nodejs invocation runs production Popup DOM checks and serial AB/BA/AB groups with identical APIs/settings and all original timers. Popup, iframe readiness, installation, each trial/round and idle are named observable stages. Each round is separately started and bounded by a supported Page wait;24rounds/80fragments/24duplicates and8.1s idle remain unchanged. Partial raw Popup checks/held IDs/current metrics/completed round observations/candidates/work are checkpointed before a completed trial. Failures retain stage, original error/late-effect facts and a safe read-only snapshot without navigating the page; pending failures stop subsequent actions. Actual TaskDuration and raw trial distributions remain distinct from Node doubles. No TaskSpace lifecycle action belongs to this helper.

The coordinator accepted this mechanism as source/owned-fixture and executed isolated-browser resource-list denoising; see [authoritative acceptance and all six raw trials](coordinator-acceptance.md). Real Popup DOM22 and comparison/conformance78 passed. Raw observations91remain; candidates91→10 with complete small media/manifests/Blob/hook/CSS preserved. Original7s/1s/1s/5s responsibilities remain. Active/idle task medians1995.342→1959.447ms and262.074→258.909ms, discovery latency1813.100→1817.950ms and the differing third candidate idle sample do not establish stable CPU/latency advantage. B2 is completed at this stated level; native/physical/IME/DPI/provider, human visual, installed and full-workspace acceptance remain separate.
