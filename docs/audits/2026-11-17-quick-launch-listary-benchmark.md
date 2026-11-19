# Installed Quick Launch / Listary baseline — 2026-11-17

## Findings

- Both products passed all 35 pinyin observations, including full spelling, fragments, initials, mixed ASCII/Han, polyphonic city/bank names and `v` spelling. Directory and same-name-file checks also passed in both products.
- NxTools admitted the tested noncontiguous fuzzy query and all five FZF-style query forms. Listary returned no ordinary visible results for those six strings. These results establish query-string compatibility, not whether equivalent Listary-specific expressions exist.
- Listary responded earlier on the same jointly successful controlled queries: p50 **166.1 ms** versus NxTools **216.1 ms**, and p95 **191.9 ms** versus **253.8 ms**. This measures input-to-accessible-result, including UI Automation overhead.
- Real-system queries expose a more important application-candidate gap than the pinyin tests: Listary shows entries named `记事本` and `设置`, while NxTools shows a Quicker item and ordinary files for those queries. Both products produce unhelpful leading file results for English `notepad` on this profile.
- NxTools' broader fuzzy admission has a visible precision cost. `nxqlb btmm` puts the intended fixture first but also admits build-cache and thumbnail names. Required-target success is not a claim that every returned row is useful.

## Run and evidence

| Item | Observation |
| --- | --- |
| Time | 2026-11-17 10:20:44–10:26:54, UTC+08:00 |
| NxTools installed FileVersion | 0.7.1; protected desktop and its matching PlatformHost |
| Listary installed FileVersion | 6.3.5.94 |
| Controlled corpus | 15/15 targets literally observable in each product |
| Formal measurements | 23 queries × 5 repetitions × 2 products = 230 |
| Unscored observations | 30 literal availability checks and 46 warmups |
| Sampling errors | 0; all 46 product/query groups contain exactly five measurements |
| Product order | AB/BA alternating, same rotated query order per repetition |
| Cleanup | Queries cleared; owned Listary instance exited; original NxTools desktop preserved |

[Raw samples](../../artifacts/performance/quick-launch-listary-baseline-20261117/raw.json), [structured summary](../../artifacts/performance/quick-launch-listary-baseline-20261117/summary.json), and [complete per-repetition report](../../artifacts/performance/quick-launch-listary-baseline-20261117/report.md) retain the measurements. [Method and reproduction commands](../quick-launch-listary-benchmark.md) describe the runner and its boundaries.

## Defined-query results

Success requires every designated target in the first five ordinary visible results, absence of designated forbidden targets from captured rows, and no ordinary rows for the expected-empty case.

| Category | Queries | NxTools successful observations | Listary successful observations |
| --- | ---: | ---: | ---: |
| ASCII/Chinese literal and fuzzy | 4 | 20/20 | 15/20 |
| Pinyin | 7 | 35/35 | 35/35 |
| FZF-style syntax compatibility | 5 | 25/25 | 0/25 |
| Directory / same-name files | 2 | 10/10 | 10/10 |
| Expected-empty query | 1 | 5/5 | 5/5 |
| All defined predicates | 19 | 95/95 | 65/95 |

The fuzzy difference is `nxqlb btmm` targeting `nxqlb-BetaMemo.txt`. The five syntax strings are `^nxqlb-Beta`, `nxqlb Memo.txt$`, `"nxqlb-alpha-report.txt"`, `nxqlb alpha !draft`, and `nxqlb-BetaMemo | nxqlb-年度报告`. Their targets were available through literal queries in both products. Each difference repeated in all five measurements.

Four real-system queries are evidence-only and are not included in this score. The aggregate predicate count includes syntax compatibility and must not be presented as an overall product-accuracy ranking.

## Paired response timing

This subset includes only controlled queries that passed the target predicate in every repetition in both products: ASCII exact, multiword, Chinese literal, seven pinyin forms, directory and same-name files. Empty-result checks, unsuccessful queries and real-system examples are excluded.

| Product | Identical controlled queries | Samples | p50 | p95 | Maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| NxTools | 12 | 60 | 216.1 ms | 253.8 ms | 284.7 ms |
| Listary | 12 | 60 | 166.1 ms | 191.9 ms | 238.7 ms |

Listary's median is about 50 ms earlier and its p95 about 62 ms earlier in this subset. The complete report separately retains all-result conditional distributions, with their different response/no-result populations; those distributions are not the paired speed comparison.

## Real-system result examples

The first measured repetition provides these visible examples; full paths and all repetitions remain in the raw artifact.

| Query | NxTools leading results | Listary leading results |
| --- | --- | --- |
| `记事本` | `记事本2.0` under Quicker; one visible result | `记事本`, then `记事本2.0` |
| `设置` | `设置.qk`, documentation and cache/configuration files | `设置`, followed by documentation/files |
| `chrome` | `Google Chrome` first | `Google Chrome.lnk`, then `Google Chrome` |
| `notepad` | Two Git `notepad` paths lead; WindowsApps `Notepad.exe` appears fifth | `notepad.svg`, an archive and icon images lead |

If the intended task is launching the built-in Windows Notepad or Settings application, the Chinese-query examples are a concrete gap to investigate in NxTools' application source/alias coverage. The English `notepad` examples show that neither product should be judged successful merely because matching filenames appear.

## Recommended priorities

1. Verify and complete system/packaged-application candidate coverage for Notepad and Settings, including localized names and useful aliases. Reuse the existing Windows enumeration → Rust catalog boundary rather than promoting arbitrary executables.
2. Profile NxTools' input/debounce/provider/publication path against the roughly 216 ms paired median. Separate useful first-result publication from observer overhead before changing scheduling.
3. Inspect weak-match precision using the observed build-cache/thumbnail candidates. Preserve the successful target rank without treating additional unrelated rows as improved recall.
4. Test Listary-native equivalents separately when comparing expressive query capabilities. The current evidence already establishes compatibility differences for the recorded strings.

## Verification and limitations

The final offline report suite passes 12 tests; driver normalization passes five assertions. The real installed run exits successfully with 230 complete formal samples and no input/focus/cleanup errors. The project documentation contract gate passes. No product code was changed, no result was executed, and no build, history reset, configuration change or service restart was requested.

This is one installed-machine baseline with existing history, index scope, matching preferences and background load. Shared literal coverage controls fixture membership, not all those settings. The result window is observed after a minimum 1000 ms and 250 ms without content changes, bounded at 3500 ms; it is not a provider-completion signal. Visible rows do not establish full-index recall or global precision. UI Automation value updates do not exercise physical hotkeys, IME, typing cadence or paint timing. FileVersion does not establish correspondence to every unbuilt working-tree change. Five repetitions diagnose this sample, not a release latency SLA. Open/reveal, preview, directory switching, mixed DPI and restart/idle-resource acceptance remain untested here. Listary's native engine source is unavailable.
