# REF-12 — Evidence-driven candidate selection

## Change boundary

One candidate mechanism: select usable candidates from the existing bounded observations. Do not change7s media scanning,1s page context,1s overlay or5s YouTube observation. Preserve the original event/cache/content/page-hook/PerformanceObserver/Blob/batch owners and all REF-04/05/06 behavior. No native/Rust change or Cargo verification is planned.

The candidate filters MIME/range-identified media fragments and DOM-declared decorative audio at existing projections while retaining bounded raw observations. Original bucket/pageKey identity scopes asynchronous capture/scan/chooser publication; removed-tab cleanup also handles replacement. Source/deterministic verification is recorded in [candidate delivery](ref12-outcomes.md); browser equivalence and adoption are not established.

## Baseline and comparison

Frozen before REF-12 edits at `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b2-pre-ref12-baseline`,73files,2026-11-10T06:08:44+08:00. This includes completed REF-05/Popup changes. `b2-ref12-pre-status.txt` and `b2-ref12-pre.diff` record the local dirty boundary. The original `b2-extension-baseline` remains untouched. Candidate source is `D:\Code\NxTools.Next-main\browser_extension\nxtools-interceptor`.

The coordinator owns the separately authorized rerun of the same isolated DOM/workload through baseline and candidate on one supplied Page, three serial interleaved groups. Helpers must not create/claim/finish any TaskSpace. Record actual CDP browser TaskDuration, wall duration, scan/event/runtime work, retained observation/candidate counts and discovery latency; emit raw trials, median and slowest only from actual runs. Bounded named Popup/prepare/round stages and exact held-request ownership preserve partial raw output and the failed Page. No measured adoption is claimed by source/helper readiness.

## Selection semantics

- Keep manifests, Blob, ordinary/opaque resources and unknown evidence. No generic minimum size or suffix-only fragment filter.
- Identify response fragments from actual ISO segment MIME, or valid incomplete Content-Range plus a numeric range/byterange/bytestart URL parameter and actual audio/video MIME. Stable full-resource ranges, ordinary ZIP and unknown-MIME ranges remain usable.
- Explicit attachment and original DOM media source evidence preserve independently requested/played media. Small complete videos/audio and complete transport-stream files are not fragments merely because of size/suffix.
- Decorative audio requires actual DOM audio metadata: finite positive duration at most2seconds, autoplay, no controls, no loop and explicit presentation/none role. Unknown duration or ordinary small audio remains. Controlled/video media evidence wins over an incidental decorative occurrence of the same URL.
- Store normalized source facts in original observations; retain the original120-observation/TTL/alias/storage owners. Filter Popup and overlay projections before expensive choice enrichment. When capacity is reached, selected resources take precedence over excluded observations; no extra negative cache/store/runtime.
- Cross-resource enrichment must not transfer DOM/discovery evidence to another source identity. DOM fact changes must bypass the existing sent dedup so metadata arrival and controls changes are observable. Preserve stable handoff/resource IDs, Cookie context and original source ranking.

## Planned files

`capture-policy.js`: normalized source facts, source-aware merge and selection predicate.
`bg/{capture,request-context,resource-cache,popup-state,resource-choices,tab-bridge}.js`: record original response/DOM facts, apply selection to existing projections/capacity, guard old page owners at publication.
`content/media-info.js` and `content-script.js`: observe original DOM facts, retain original candidate/dedup/TTL owner and apply the same selector. `content/resource-choice-text.js` merges bounded local candidates through the original request and rejects retired pageKey replies.
`manifest.json` and `bg/tab-bridge.js`: load existing capture-policy in matching content order; no permission/CSP/dependency expansion.
`background.js`: original removed-tab cleanup for onReplaced.
Tests: real background events/runtime/cache/virtual time and production content listener fixtures, plus a supplied-Page browser comparison helper and deterministic workload. Existing harness API seams may gain missing supported WebExtension callbacks/events only.

## Lifecycle and diagnostics

Use the existing per-tab bucket identity to retire asynchronous captures/scans after navigation/history/closure/replacement, not a new lifecycle manager. Clear original scan/index/thumbnail ownership with the original tab. Preserve original3.5s explicit-demand throttle and deep fallbacks. Selection returns only fixed reasons and adds no per-event/byte/frame logging or native host demand. Existing handoff/correlation, durable acceptance/cancellation and payload-safe diagnostics remain; no new native message protocol.

## Required evidence

Each changed behavior first fails at its production event/runtime boundary, then passes. Cover redirect, known/unknown metadata, manifests, small complete/media/ordinary/opaque/Blob, fragment and decorative facts, explicit demand/manual/batch, capacity/TTL/alias/source ranking, navigation/history/visibility/close/replacement and late replies. Run all extension tests, retaining B1/REF-05 admission. Browser equivalence/performance and final adoption remain coordinator-owned and unperformed here.
