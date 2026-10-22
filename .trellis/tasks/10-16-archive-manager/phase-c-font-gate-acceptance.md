# Phase C — Archive browser and entry lifecycle: font gate independent acceptance

2026-10-21. **Font-detector false-positive remediation independently PASS; this static gate CLOSED.**
No overall Phase C/A–F or live/native/user acceptance is granted. Only
[checkpoint](phase-a-checkpoint.md) owns current state. [Implementation outcome](phase-c-font-gate-outcomes.md)
and historical failed observations remain unchanged.

## Source and evidence review

Compared the current desktop-shell-lifecycle.ps1 against its saved implementation opening copy:
only two explanatory comments and the font predicate differ. No other shell checks, file exclusions,
source collection/encoding, business Rust, fonts, theme or dependency were changed by that delta.

Old constant branch `UI_FONT_FAMILY[^=]*` could start at a legal variable argument and cross into a
later unrelated assignment; old quoted-string matching could cross lines. Implementation's retained
complete-match/encoding evidence identifies the preview.rs744–790 span1809characters, with default
CP936 read producing1match versus explicit UTF8 producing0. Current actual typography constants
are single families Microsoft YaHei UI and Consolas. This review does not treat historical offsets
as necessarily unchanged current source or claim the old failure was a real CSS font list.

The new predicate requires an actual const/static declaration for the constant branch, bounds its
scan and quoted value, and accepts whitespace before direct literals. Existing comma-list rejection
remains, without skip/whitelist/disabled rule or a parser framework. The16case regression reads the
exact predicate from the production gate, covering actual direct/constant/static lists, multiline
formatting, legal variables/single families, unrelated comma strings and damaged-decoding mode.
No new concrete in-scope defect was found in this bounded review; no delegate/review loop was used.
This is a bounded static detector, not an exhaustive Rust parser or font-rendering acceptance.

Implementation retained red evidence13passed/3failed. Parent checked the current regression SHA256
against final-red TestSha2561044DF917EC588D9D87C688601F6E63A5D19E57F4999DB2FDE57352630AFA599;
it matches. No test was weakened for green. Parent did not replace production with the old rule.

## Independently executed checks

| Check | Actual result |
| --- | --- |
| desktop-font-family-regression.ps1 | **16cases/16passed/0failed**,exit0 |
| Full desktop-shell-lifecycle.ps1 |PASS,exit0; source-only |
| PowerShell Parser on both scripts |PASS,0parse errors |
| spec-contracts |PASS106documents/14owners,document-only |
| task validate |PASS82implement/79check; two existing oversized-context document warnings retained |
| scoped diff --check |PASS |
| current baseline preservation before report/status edits |**336/336 unchanged**,including Desktop source/task/gate files |

Unique parent evidence: session ceebe351-d540-4ca3-a460-dadc71f3a7df,
font-gate-independent.ps1 and font-gate-independent-20261021-233142/{commands.jsonl,individual logs,
baseline.json,preservation.json}. Implementation's151historical count is not reasserted as a new
older-baseline comparison. Its unrelated concurrent Rust changes are not attributed to this fix.

## Disposition and stop

Accept the detector-only fix and close the last observed font static gate failure. Earlier cache
fairness acceptance stays closed; prior922/922,archive40,Protocol59,IPC31,native10 and strict lint/fmt
remain earlier evidence, not rerun checks. No additional source-closeout task or speculative blocker
is introduced by this result.

Actual default/minimum Light/Dark/CJK, picker/Shell/input/DPI/mixed-monitor/UIA/user acceptance
remains **NOT RUN** under binding self-app safety denial. Phase A/B unfinished obligations remain.
No Cargo/product/cache/native suite,GUI/Shell launch,system mutation,package/install,commit/push or
D/E/F work occurred. Only this additive report/current checkpoint status is edited in acceptance;
implementation and historical reports remain untouched. Stop without declaring full Phase C PASS.
