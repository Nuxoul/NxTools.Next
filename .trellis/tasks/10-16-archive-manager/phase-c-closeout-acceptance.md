# Phase C — Archive browser and entry lifecycle: source closeout independent acceptance

2026-10-21. **Most requested closeout behavior independently verified; full Phase C NOT accepted.**
One concrete cache-sweep fairness defect remains. Current full Desktop922/922 and strict lint/fmt
PASS do not waive the failing desktop-shell source gate or unperformed live UI/native acceptance.
Only [checkpoint](phase-a-checkpoint.md) owns current status. Prior reports/results remain history.

## Findings and dispositions

### P2 — locally retired leases beyond the first eight can starve
`csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs:283–285` always takes the first8
locally released leases. If all8 remain obstructed, a ninth released lease is never retried even
when its obstruction disappears. Filesystem enumeration skips any ID still in the leases map
(line293), so it does not compensate. Slots can stay occupied until explicit-ID retry or restart.

Parent independently reproduced against the freshly built PlatformHost assembly using its normal
owned-test-cache seam:9tiny sealed outputs, each root obstructed by a probe-created unknown file;
all releases correctly incomplete. Remove only ninth obstruction, call normal Sweep20times:
ninth root remains; direct Release(ninthId) then retires it. First8unknown files remain untouched.
This is bounded-cleanup starvation, not permission to delete obstructing files. Restore bounded
round-robin/progress across locally retired leases and add this exact original-owner regression;
do not add unbounded loops, recursive deletion or a scanning service.

Evidence: parent scratch `phase-c-closeout-cache-probe/{Probe.csproj,Program.cs}` and
`phase-c-closeout-independent-20261021-175140/additional-cache-probe.json`.
Initial probe compilation lacked System.IO, executed no product code; corrected probe built with
0warnings/0errors and ran. Its exit0 records observations, not product behavior PASS.

### Reviewed root-rename concern — not retained as a confirmed finding
The single optional read-only cache review also suggested a held-root rename/missing-GUID case.
Parent attempted Directory.Move while the production lease held its handles; Windows refused with
IOException before release. No original-root rename or authority loss occurred in that probe.
Do not report this scenario as reproduced or turn it into a blocker without evidence.

### Prior confirmed retirement defect — bounded regression closed
The actual original-cache fixtures independently pass incomplete-release accounting, same-process
retry after removing obstruction, recreated-cache StartupCleanup recovery, replacement-root/ancestor
refusal, malformed/legacy/no-record preservation, nested obstruction, expiry/active/edited/locked
preservation. The former ignore-RetireEmpty/delete-manifest defect is no longer reproduced by those
cases. The fairness issue above is distinct; not a reason to repeat all cache research.

## Source review / implemented closeout

Reviewed closeout contract/outcomes, production UI store/browser capture/application restore,
private password projection/protocol/IPC write clearing/native ticket parser, original password
coordinator and relevant actual tests. One bounded optional review covered cache only.

- Window/column state is separate/path-free/<=16KiB with validation and corrupt-file preservation;
  one bounded wake/latest snapshot writer, no production close callback join or disk IO. Native
  close/F4 and keyed application restore are wired, not just a helper module. Observed logical
  bounds/maximized, columns/visibility/sort/panels are captured. TestWindow state/restore regressions
  passed; physical placement/maximize/mixed-DPI/focus still not established.
- Cache24h abandoned eligibility and actual constructor-started StartupCleanup are present; active
  manifests remain exclusively held. Version2 fixed-parent retirement records preserve original
  ancestry/root/output identity through obstruction/restart. Safe bounded cleanup is implemented
  except the locally-retired fairness defect above.
- Browser read opts into original finite password handling via browser-only command naming, keeping
  public ArchiveRead schema password-free. Original authenticated pipe/client/runner/Job/ticket/
  cancellation owners are reused. Official Dialog+masked Input response checks current operation,
  session/view generation and entry snapshot, and Escape/retired replies cannot authorize Shell.
- Private secret wrapper has redacted Debug and clearing on Drop; IPC encoded buffers clear after
  write. Native bounds/decodes/clears byte arrays before routing only matching tickets. This is not
  a claim that managed strings/GUI editing buffers provide perfect memory erasure.
- Browser-password read attempts retain original native control identity. Existing no-output extract
  preparation and once-only real extraction remain intact. Actual AES256 ZIP selected output bytes
  and RAR5 header-encrypted cancel/wrong->correct/read/selected lifecycle were rerun; no blanket
 7z/RAR4/full crypto-method matrix acceptance is implied.

## Independent execution

Serial/offline/locked Cargo with jobs1/incremental0/RUSTFLAGS=-C codegen-units=256. Other workspace
check was observed and allowed to finish; bounded lane waiting, no duplicate build or process kill.
No resume wrapper: unique evidence directory, append-only per-command JSONL and separate raw logs.

| Check | Actual independent result |
| --- | --- |
| Desktop archive:: |40passed/0failed/ignored,882filtered |
| Protocol / IPC libraries |59 /31passed,0failed/ignored |
| Read cancellation / consumer / worker wire / protected extraction |10 /6 /17 /1passed,0failed |
| Production strict Clippy (Desktop/Protocol/IPC/Archive bins) |PASS |
| Test strict Clippy same owners |PASS |
| Archive isolated-codecs all-targets strict Clippy |PASS |
| Full fmt --check |PASS |
| One full Desktop suite |**922passed,0failed/ignored/filtered**,113.21s,exit0 |
| C# test apphost build |0warnings/0errors,exit0 |
| Worker rebuilt after final feature checks |PASS; matching isolated-codecs |
| Exact C# LIST then same RUN, archive cache fixture + archive browser |**LIST=RUN=PASS same10names**,skip/fail/not_run0 |
| desktop-shell-lifecycle |**FAIL**,font-family guard at script50 |
| ui-preview-isolation / ui-consistency / ui-token-boundaries / platformhost-test-selection |all PASS,source-only |
| spec-contracts / catalog / task validate / diff --check |PASS;106docs/14owners,79pages/78locked entries,82implement/79check |

Focused40 overlap full922; do not add counts. Handoff921/1 and older907/5,909/3 remain immutable
historical failures; current922/922 is a distinct observed run, not a rewrite or proof of their causes.
No archive implementation/test was changed to obtain this result.

Font source gate still reports its original CSS-fallback-list rule. Handoff documents a cross-line
regex match in nonarchive preview.rs; this review does not claim the rule is fixed or infer archive
causality. Address that guard/source with its owner before claiming all source gates green. Existing
context warnings remain native-contracts35497/archive-reference37272bytes above32768, not failures.

## Acceptance boundary / next action

Accept the bounded window-state/password/previous-retirement-remediation evidence above. Do NOT
close all LIFE05 or full Phase C while local retired-cache starvation remains. Next source change
should be the small bounded fairness repair and red-capable9lease regression, not broad rework or
another review cycle. Font gate remains separately unresolved.

Real default/minimum Light/Dark/CJK screenshots, native picker/Shell/user input/DPI/mixed-monitor/
UIA/user visual acceptance remain **NOT RUN** under binding self-app denial. No GUI tools, launcher
bypass, actual Shell, product runtime, package/install/system mutation/source deletion/commit/push
or Phase D/E/F work occurred. Phase A/B unfinished obligations remain unchanged.

## Evidence and preservation

Parent scratch ceebe351-d540-4ca3-a460-dadc71f3a7df:
- phase-c-closeout-verify.ps1
- phase-c-closeout-independent-20261021-175140/{commands.jsonl,individual logs,baseline.json,
  preservation.json,additional-cache-probe.json}
- phase-c-closeout-cache-probe/ (scratch-only tiny native file probe; roots retained, no unknown cleanup)

Matching worker SHA2566A2BDB0D40CE587AF1F552D23DC4C9FA09EEF937D5CE732A14CC58C974FEDA8D;
PlatformHost.dll SHA25667D47ED5EF8BE58052AF2912189051658CA644785E40B327678153896BC0BEE1.
Before record edits **237/237 current acceptance-baseline task/archive source/manifest files matched**.
Implementation's146historical count is not reasserted as an independent earlier-baseline check.
This acceptance adds this report and updates current checkpoint only; implementation and historical
reports remain untouched. Stop at this Phase C boundary; no automatic D/E/F advance.
