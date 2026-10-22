# Phase C — Archive browser and entry lifecycle: cache fairness independent acceptance

2026-10-21. **Confirmed local-retirement starvation P2 independently CLOSED for this scoped fix.**
No full Phase C/A–F, real UI/native interaction or release acceptance is granted. The only current
status owner remains [checkpoint](phase-a-checkpoint.md). Prior [closeout acceptance](phase-c-closeout-acceptance.md)
and [implementation fix record](phase-c-cache-fairness-fix.md) remain unchanged historical evidence.

## Source and red-capable regression review

Compared current ArchiveEntryCache.cs directly with the implementation's saved opening copy.
The exact production delta has only three parts: one localRetirements ID queue, replacing fixed
first8dictionary enumeration with a fixed-budget queue batch, and enqueueing once when a live lease
is first marked released. The existing lock serializes the queue/lease map. A batch snapshots
min(8,queue.Count), dequeues each candidate once, discards absent/successful IDs and requeues only
incomplete leases at the tail. No filesystem-sweep, expiry, manifest, identity, content, metadata,
MOTW, active-lock or retirement deletion branch changed.

Read the actual original-cache regression:9tiny sealed leases are all retired with unknown-root
obstructions; only ninth obstruction is removed;2normal Sweeps must finish its root and record
without direct ninth-ID retry. First8unknown files and recovery records remain. After removal and
new admission, one Sweep cannot finish all9remaining leases; the next finite batches must finish
all. This observes original native files and owner APIs, not internal queue predicates.

Implementation's preserved red log actually records selected1/run1/pass0/fail1/skip0/not_run0:
`ninth locally retired lease starved behind eight obstructed leases`. Parent did not mutate
production back to red. It independently checked current SHA256 against all3recorded red-stage
sources (ArchiveCacheRetirementTests.cs, Program.cs, TestEffects.cs); all match, with no relaxed test.
No new concrete defect was found in this bounded fix; no additional review was delegated.

## Independently executed checks

Unique parent scratch evidence directory:
`cache-fairness-independent-20261021-221453` under session ceebe351-d540-4ca3-a460-dadc71f3a7df.
Commands in `cache-fairness-independent.ps1`; append-only commands.jsonl and per-command logs.

| Check | Actual result |
| --- | --- |
| C# isolated test-host build --no-restore -v:q | exit0,0warnings/0errors |
| `--fixtures --list "archive cache fixture"` | selected4,run0,not_run4; enumeration only |
| Same exact RUN filter | **selected4/run4/pass4/skip0/fail0/not_run0**,exit0 |
| LIST/RUN/PASS identities | exact same4names compared |
| platformhost-test-selection | exit0,source-only |
| spec-contracts | exit0,106documents/14owners; documentation only |
| task validate | exit0,82implement/79check; existing oversized-context warnings retained |
| scoped diff --check | exit0 |

Actual four identities:
1. archive cache fixture obstructed retirement restart
2. archive cache fixture expiry startup retained owners
3. archive cache fixture ancestry malformed recovery
4. archive cache fixture fair local retirement rotation

No Cargo, Desktop/Protocol/IPC/codec/full-native10 test, worker rebuild, GUI/Shell/system mutation,
package/install/signing/source deletion/commit/push or Phase D/E/F work occurred. Earlier922/922,
archive40,Protocol59,IPC31 and native10 remain prior independent evidence, not rerun counts here.
The previously unconfirmed root-rename concern was not reopened as a blocker.

## Preservation and remaining boundary

Before report/status edits, **155/155 current task/cache/test acceptance-baseline files unchanged**.
Red-stage test hashes independently match. Implementation's149historical count is not restated as
an independently checked older baseline. Only this additive acceptance report and current checkpoint
status are edited; production source, tests and historical reports are untouched by acceptance.

**Disposition:** accept this minimal bounded queue repair and close the confirmed fairness defect.
Do not repeat the cache design/spike or add a broader review/test matrix. Font source gate retains
its last observed FAIL and actual GUI/Shell/DPI/UIA/user acceptance retains NOT RUN; neither was
rerun or waived by this cache-only check. Phase A/B unfinished obligations and full Phase C native/
visual acceptance remain unchanged. Stop at the scoped acceptance boundary; no automatic D/E/F.
