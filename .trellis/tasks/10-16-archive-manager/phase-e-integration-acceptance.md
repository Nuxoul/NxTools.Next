# Phase E workspace integration convergence: independent acceptance

2026-10-22. **Recorded workspace integration blockers independently CLOSED at source/compiled-test
level. Not overall Phase E or first-release PASS.** Only [checkpoint](phase-a-checkpoint.md) owns
current status. [Convergence outcomes](phase-e-integration-convergence.md) and historical
[997/2 independent delivery](phase-e-delivery-acceptance.md) remain unchanged.

## Scope and source review

Reviewed convergence outcomes, actual working diff, current editor reference/patch authorization,
provenance gate, Clipboard source-gate mutations and original QuickLaunch/theme assertions.
No actionable new in-scope defect was found; no delegate or additional review loop was used.

- Current QuickLaunch still asserts content/rails/containment and approved external row-style gap;
  theme checks approved Graphite roles through real Kit projection. Those product/test files were
  not changed in this convergence delta; today's green is not proof of exact earlier failure cause.
- Editor source changes are one authorized serializer Boolean simplification and two precise
  whitespace fragments. Original57file hashes remain. The gate reverses the complete serializer
  fragment to original SHA256, checks all16Boolean combinations, and reverses element whitespace
  to the recorded approved passive baseline. No allow, dependency/backend upgrade or broad vendor
  formatting. Existing lowbit/passive exceptions remain independently bounded.
- Clipboard UI gate retains its prior checks and adds scoped original begin/cancel/save, actual
  source/rich body mounting, deferred pending/draft identity/focus and passive-document checks.
  Three tests copy only required source inputs into scratch; detached mount/item guard with token
  preserved in comments is rejected. This is source evidence, not a general Rust parser or UIA proof.

## Independent execution

All Cargo serial/locked/offline with CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256, shared lane checked before each command. Unique raw logs plus
append-only command JSONL. One full Desktop run, no failed whole-suite retry or source edits.

| Check | Actual result |
| --- | --- |
| QuickLaunch polish / Graphite projection complete-name --exact |each1PASS/0FAIL,1001filtered |
| Clipboard save-focus / stale-save completion / exact source-save+failure draft --exact |each1PASS/0FAIL,1001filtered |
| Editor list_continuation_indented / every_block_kind_round_trips / passive focus --exact |each1PASS/0FAIL,233filtered |
| Clipboard source-gate positive/negative mutation tests |3/3PASS |
| Markdown editor provenance and16Boolean equivalence cases |PASS |
| Desktop FULL-dependency strict Clippy --tests -Dwarnings |PASS; no --no-deps substitute |
| Global fmt --all --check |PASS |
| Workspace check --tests |PASS,compilation only |
| clipboard-lifecycle / clipboard-ui-lifecycle / quick-launch-lifecycle / ui-state-lifecycle / desktop-shell-lifecycle |all PASS,source-only |
| GPUI release / component catalog / spec / task-context / diff |PASS;109docs/14owners,79historical pages/78historical mappings,current0.8.0 authority;82implement/79check |
| ONE full Desktop suite |**1002PASS/0FAIL/0ignored/0filtered**,115.10s,exit0 |

All8exact selected test names and nonzero actual counts were checked. Five Desktop targeted cases
are included in full1002 and are not added to that total; the three editor cases are separate.
No separate Archive28/Protocol65/IPC38/native11+6 tests, codec worker build or performance matrix.
Earlier994/1,997/2 and implementation1002 results remain distinct observations, not rewritten by
this successful independent run. Existing reference/native-contract injection-size warnings remain.

## Evidence, preservation and disposition

Parent scratch ceebe351-d540-4ca3-a460-dadc71f3a7df:
integration-convergence-independent.ps1 and integration-convergence-independent-20261022-195349/
{commands.jsonl,exact-1..8.log,gate-mutations.log,editor-provenance.log,clippy.log,fmt.log,
workspace.log,desktop-full.log,other gates,baseline.json,preservation.json}.
Before record edits **487/487current task/Desktop/archive/editor/manifest/gate baseline files
unchanged**. Implementation's older2876/1314file baseline claims were not reasserted as a new
older-baseline check. Only this additive report/current checkpoint are edited in acceptance;
product source/tests/vendor/history remain untouched.

**Accept this integration convergence and close the known workspace blockers.** No new archive
cleanup task or repeat source/native review is required by this result. Real GUI/Shell/physical
input/DPI/mixed-monitor/UIA/user acceptance remains NOT RUN under binding self-app denial; earlier
A/B,JOB07/SAFE08 and other outstanding requirements remain. No package/install/system mutation,
source recycle,GUI launcher workaround,commit/push or Phase F occurred. Stop without declaring
whole Phase E/A–F or release acceptance.
