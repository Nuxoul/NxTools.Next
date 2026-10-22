# WinShun W1 full Chinese pinyin matching

## Goal and boundary
Continue the existing W1 task: add full spelling, initials, mixed syllable prefixes and polyphonic Chinese name matching. W2 homophone correction and W6–W10 plugins remain deferred. This is a Rust query/highlight change, not a UI redesign, index persistence change or service deployment.

## Requirements
- Keep literal/ASCII scoring, syntax, exclusions, drive/path/parent filters, stable ranking, history, generations and cancellation. New pinyin matches are a lower-priority fallback; retain the existing application GBK-initial channel and its scores.
- Each consecutive Han character may consume a nonempty prefix of one of its readings. ASCII letters/digits match themselves case-insensitively. Match full spelling (`baogao`), initials (`bg`), mixed prefixes (`baog`, `bgao`) and standard polyphonic readings (`yinhang`, `chongqing`). Accept `v`/`u` for ü as the reviewed reference does.
- New fallback applies to ordinary positive fuzzy name terms only. Explicit exact/word/prefix/suffix operators and negative/path/parent terms remain literal; no silent query-language expansion. Existing application legacy matching is retained separately.
- Share matched UTF-8 ranges between query matching and launcher highlighting. Keep existing literal/fuzzy highlight behavior for action filtering; do not introduce GPUI/native state ownership.
- Bound query preparation and matching without enumerating pronunciation combinations. No per-name transliteration allocation in indexed search. ASCII paths retain their fast path. Name-mask filtering must not discard Han names eligible for pinyin; preserve selective candidate safe supersets.
- Use reviewed MIT pinyin-data, include full license/provenance and deterministic regeneration. No new runtime dependency/network fetch or reference execution.
- Diagnostics reuse existing query generation/batch elapsed/count lifecycle; no per-row logging or raw query/name/path payload.

## Acceptance criteria
- [x] Red-capable tests through FileQuery and real catalog/external merge prove the current missing full-spelling behavior.
- [x] Full/initial/mixed/polyphonic/ASCII-digit/ü and UTF-8 highlight tests pass; malformed/long/noneligible terms and punctuation do not broaden matches.
- [x] Existing Finder suite and frozen legacy application/ASCII ordering remain green; name masks and candidate narrowing retain every admitted match.
- [x] Exact synthetic Indexer tests exercise grouped/row/delta/scoped/cancel paths without real scans or service changes.
- [x] Desktop highlight regression, full safe Desktop tests, strict affected Clippy, workspace test compilation, fmt and applicable source/spec gates pass.
- [x] Actual outputs/counts and bounded-cost measurements documented. Installed/native/Release service acceptance is not claimed.

## Reference
Read `docs/reference-code/winshun.md` and Quick Launch research before current source. Verified read-only WinShun Pinyin.h/.cpp, Query.cpp positive/negative admission and tst_core.cpp pinyinFind/pinyinSearch; data generator and independent MIT license. No WinShun tests or product execution performed.
