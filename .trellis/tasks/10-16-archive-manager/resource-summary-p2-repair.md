# Retained RESOURCE summary P2 — Python-only repair

Date: 2026-10-19. Canonical **Phase A — Engine contracts and safety foundation** remains in progress.
This closes only the malformed retained-output summary P2 in the immutable
[independent acceptance](resource-tool-repair-acceptance.md). Original safe-stop, required-sample
and dispatch-timing corrections passed that prior independent acceptance; its C#132/132 is prior
evidence, not this run. The historical early whole-process exit cause remains unproven.

## Root cause and smallest correction

`collect()` reparsed every `RESOURCE ` line using an unconditional `json.loads` list comprehension.
A valid attempt JSON with `stdout="RESOURCE not-json\n"`, already retained by the driver's
`driver_exception` path, therefore raised `JSONDecodeError` and prevented any summary/later attempt.

Changed only `summarize.py` and `test_summary.py` plus tool/handoff/context/current-checkpoint docs.
The existing loop now catches only per-line `JSONDecodeError`; valid events remain in order and all
attempts continue. Independent `summary_parse` has status `Complete` or `Incomplete`, total
`error_count`, `errors` (one-based stdout `line`, fixed `invalid_resource_json` category), and
`errors_truncated`. At most16 error details per attempt; no raw payload/exception-text copy.
Original schema2 stdout/failure/operation/measurement/stop/cleanup/resource_verdict are unchanged.
An original PASS alongside `summary_parse=Incomplete` is explicitly NOT complete summary evidence.
Legacy conversion/timing labels and dynamic saved-selection counts are unchanged. No general
recovery for invalid outer JSON/files, no driver/sampler/stop/product/codec/budget/protocol change.

## Actual red -> green and Python checks

All commands ran from `D:\Code\NxTools.Next-main` with
`$env:PYTHONDONTWRITEBYTECODE='1'`. Logs were retained under this session's `PI_SCRATCH_DIR`.
The first behavioral regression invoked actual `collect()` before the implementation changed.

| Command | Actual result / exit |
| --- | --- |
| `python -m unittest discover -s tests/performance/archive-resources -p test_summary.py -k malformed_resource -v` BEFORE fix | Ran1, errors1; `JSONDecodeError` at original summarize.py:21; exit1 |
| SAME command AFTER fix | Ran1, pass1; exit0 |
| `python -m unittest discover -s tests/performance/archive-resources -p test_summary.py -v` | Ran8, pass8, fail/error/skip0; exit0 |
| `python -m unittest discover -s tests/performance/archive-resources -p 'test_*.py' -v` | Ran28, pass28, fail/error/skip0; exit0 |
| `ast.parse(path.read_text(encoding='utf-8-sig'), filename=str(path))` for summarize.py/test_summary.py |2files PASS; exit0 |
| Equivalent two-attempt project regression plus actual `summarize.py ROOT OUTPUT` CLI | summary produced, both attempts retained; exit0 |

Four new project regressions cover mixed normal/bad/normal lines, the exact driver-retained
not-json input followed by a normal attempt, truncated JSON with original PASS, and bounded error
diagnostics. The limit test exercises16/17/4096 bad lines including a131072-character sentinel
payload: exactly16 location/category details, correct total/truncation flag, diagnostic JSON
<2048characters, no sentinel copy, and a valid event after all bad lines retained. Every new test
checks every input file's bytes and SHA256 before/after actual collect, plus original fact values.
Existing Legacy, forced/incomplete, interrupted-log and dynamic count tests also pass.

## Actual minimal output and immutable inputs

The equivalent probe reused project test fixtures (no dependency on historical scratch), invoked
actual collect and CLI, compared their complete JSON outputs and verified all13input files unchanged.
Raw probe report/output: `summary-p2-minimal-probe.json` / `summary-p2-minimal-probe-output.json` in
session scratch. Durable exact-input and preservation regressions are in `test_summary.py`.

| Attempt | Retained events / independent parse state | SHA256 before = after |
| --- | --- | --- |
| `01-broken.json`, stdout `RESOURCE not-json\n` | `[]`; Incomplete, error_count1, line1/invalid_resource_json; original Stopped/driver_exception/measurementIncomplete/cleanupConfirmed preserved | `701f8f04daf389119a7171b3cdbabe4586ca07dcb41868598050407f5ac8a7f8` |
| `02-healthy.json` | `[{"entries":5}]`; Complete, error_count0; original PASS and facts preserved | `5821cec492c34ffd550409709806c030fce4aacdec2ddbd0e8eaef07b48d0c1c` |

The mixed-lines regression separately retains ready/terminal events around bad stdout line3 and
all later-attempt facts. The truncated-PASS regression retains ready, marks bad line2 Incomplete,
and leaves original PASS/operationSucceeded/measurementComplete/failureNone unchanged. No failed
input is skipped or silently relabeled Legacy/PASS. Summary completeness is not resource validity.

## Final document and hygiene gates

| Gate | Actual result / exit |
| --- | --- |
| `python tests/architecture/spec-contracts.py` | PASS106documents/14owners; exit0; document/source-link gate only |
| `python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager` | PASS implement65/check62entries; exit0 |
| `git diff --check` and explicit whitespace/conflict/temporary-marker scan of all8touched files | PASS; exit0 |
| Task/tool SHA256 baseline comparison |87existing files:7authorized changes,80unchanged; only1new evidence file; all73historical task files unchanged |

Task validation retains the existing unrelated warning: platformhost/native-contracts.md is35497
bytes, above32768context injection limit. No unrelated spec was rewritten.
The original independent acceptance SHA256 remains
`f553329a7cba08ece6b8166b9a66464cea67337ca237de25585d9680a40434c6`;
all old reports/results and driver/control/sampling sources remain unchanged.

The initial whole-workspace-diff equality guard exited1 because unrelated parallel Clipboard edits
occurred during this task, not because summary files/history changed. Read-only comparison located
clipboard.rs, ui/polish_tests.rs, ui/preview.rs and a new ui/preview_rich.rs. They were left untouched;
the corrected scoped hygiene check passed/exit0 and records external deltas explicitly, rather than
claiming the whole dirty workspace stayed byte-identical. No reset/stage/cleanup of unrelated files.
Final task/tool baseline, diff and hygiene report: `summary-p2-final-hygiene.json` in session scratch.
No added dependency, debug instrumentation, speculative parser framework or new review round.

## Not run / stop boundary

No Cargo/Rust verification, C# build or132test execution, actual worker smoke,100k/1GiB/4GiBZIP64,
resource sampling, performance repeat, packaging/sign/install, commit/push, user-file operation or
real system/registry/service/configuration change. Python unittest's existing owned synthetic
process fixtures ran as part of the requested suite; these are not archive worker/sampling smoke.
No new broad review was started. Prior reports/results and resource-tool-repair-acceptance.md must
remain byte-identical; only the unique current checkpoint states current status. No Phase A,
ENG08/ENG09 or A–F completion, no Phase B or other feature continuation.
