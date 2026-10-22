# Phase C — Archive browser and entry lifecycle: closeout outcomes

2026-10-21. Requested cache defect and three source gaps are implemented and scoped regressions
pass. **Not full workspace green or independent Phase C acceptance.** The current checkpoint is
[phase-a-checkpoint.md](phase-a-checkpoint.md); [closeout contract](phase-c-closeout-contract.md)
records the rule and edit boundaries. Prior [implementation](phase-c-browser-outcomes.md) and
[partial independent acceptance](phase-c-browser-acceptance.md) remain byte-identical history,
including907/5,909/3,old908/908,format/lint/font failures and the probe's reproduced defect.
No spike/fallback reauthorization, D/E/F, dependency/vendor upgrade, product GUI/Shell launch,
package/install/signing/system configuration/source recycling/commit/push occurred.

## 1. Cache retirement and restart recovery

Original-cache regression first observed red: Release incorrectly reported full retirement after
output deletion. Native regression also observed red when a replacement `files` ancestor held the
same original file: output was deleted before directory identity rejection. One optional bounded
read-only cache review identified that ancestry ordering, nested-obstruction authority ordering and
malformed-record startup handling; parent reproduced/fixed/retested, no second review loop.

`ArchiveEntryCache` now distinguishes validated output removal and actual session retirement.
A version2 bounded manifest retains original root/directory identities, one output's full identity,
size/mtime/hash/attributes/MOTW, expiry and retirement facts. Before any directory deletion, move
the held manifest with native no-replace rename to `<lease-id>.retire.json` in the fixed sessions
parent. Unknown files continue to block empty-directory deletion; failure returns incomplete and
retains the recovery record/lease. Later legal retry and recreated-cache startup recover only the
original root identity. A replacement root, replaced output ancestor, edited/locked output, legacy/
malformed manifest or bare GUID directory is never cleanup authority. No recursive removal.

Every traversed sealed output ancestor is checked against recorded identity BEFORE opening the
file for deletion; retirement rechecks root/directory identity on each retry. The record is deleted
only after the root is actually gone. Failure during retirement does not discard the record merely
because the selected output is already absent. Unsupported old manifests remain retained.

Expiry policy is explicit24h from creation UTC for abandoned non-retired sessions. Active leases
retain their exclusive manifest lock, even past expiry; independent child-browser lease survives
parent close. Explicit release marks retirement eligibility immediately, still checking every safety
condition. Constructing the cache via the existing per-pipe ArchiveEntrySession starts one background
bounded batch; max8 filesystem entries plus max8 locally retired leases, then normal admissions
continue the enumerator. No resident scanning timer, unbounded retry or replacement worker.
Tests use controlled TimeProvider, real native locks, tiny synthetic files and actual StartupCleanup.

## 2. Window/column persistence is actually wired

`archive/browser::ui_state` is a separate versioned <=16KiB path-free presentation file under the
existing local data directory, not AppConfig.archive. It stores logical x/y/width/height/maximized,
10column widths/visibility, Tree/comment flags and sort. Numeric limits and name-column visibility
are validated. Missing state uses defaults; corrupt/invalid/oversized existing state is preserved
and write-disabled, including a reread before atomic sibling-temp/sync/rename publication.

One bounded wake slot and one writer thread coalesce the latest snapshot; no disk IO or thread join
occurs in the GPUI close callback. Native close captures the view snapshot then defers application
retirement; F4 captures the same final snapshot. The keyed application keeps the store independently
of individual windows; the next open uses its latest snapshot and restored logical bounds/maximized
state, with off-display origin fallback. Saved columns are applied before data installation and
survive the next immutable snapshot projection. Production application close/reopen test asserts
actual view restoration and the separate on-disk file contains no archive path. Store tests use a
test-only completion wait, not production GUI-blocking shutdown.

## 3. Browser encrypted read and selected temp-open

Reuse the original `ArchiveReadClient`, one dedicated archive receiver, metadata credit, authenticated
platform pipe, original runner/Job and worker `InteractionPort`/password secret channel. Public
`ArchiveReadCommand` schema stays password-free/read-only. Browser-specific `archive_browser_read`
uses the same typed read request and original transport/reservations, enabling finite private
password coordination only internally. ACK/cancel remain original public read controls.

Non-secret `ArchivePasswordPrompt` binds operation/session/view generation/prompt/ticket generation,
error and attempt. Rust projects it through official Dialog+masked Input; reply matches current
read or current source-bound snapshot/entry generation before dispatch. Cancel/Escape clears input,
replies typed cancel, and cannot launch Shell. Deferred prompt display rechecks current lifetime.
Close/refresh/disconnect invalidates tickets; stale/foreign native tickets are refused. Terminal
read/materialize clears pending prompt identity. Queue submission failure cancels original work.

Private reply uses a redacted-Debug secret byte wrapper cleared on Drop, over the original pipe;
IPC serialized write buffers are cleared after write. C# bounds/decodes/zeroes the secret bytes and
feeds only the matching original runner ticket. No password config/log/argv or extra public
ArchiveRead authority. The existing max3 attempts and exact PasswordRequired/WrongPassword versus
ambiguous AuthenticationOrDataError semantics remain; no text matching or guessed categories.

Actual encrypted RAR5 browser read originally failed protocol_violation. The native source-identity
call was using password preparation's attempt control rather than the original operation control.
The minimal fix keeps original control for browser read attempts while retaining distinct diagnostic
attempt IDs. RAR5 cancel, exact WrongPassword correction, metadata/source context and selected
materialization then pass. Existing extract no-output preparation/once-only extraction remains.

Native format evidence is deliberately small: independent AES256 ZIP selected payload verified byte
for byte, plus attributed RAR5 header-encrypted read/cancel/wrong->correct/selected-output lifecycle.
This is NOT all ZIP encryption/method combinations or blanket7z/RAR4 interoperability acceptance.
The worker/shared7z path compiles; no full codec/large/performance matrix was rerun.

## 4. Executed checks and counts

All Rust commands checked machine lane and used jobs1/incremental0/RUSTFLAGS=-C codegen-units=256,
locked/offline. Busy lanes were waited/rejected, not duplicated or killed. Checks have unique raw
logs; no nested-value continuation or overwrite of prior failures.

| Check | Result |
| --- | --- |
| Archive Desktop focused after wiring | **40passed**,0failed/ignored;40 also pass in full run |
| Protocol / IPC libraries | **59 /31passed**,0failed/ignored |
| Product read cancellation / consumer | **10 /6passed**,0failed |
| Worker wire / protected extraction | **17 /1passed**,0failed |
| Production strict Clippy Desktop+Protocol+IPC+Archive bins | PASS `-D warnings` |
| Test strict Clippy same owners | PASS `-D warnings` |
| Archive isolated-codecs all-targets strict Clippy | PASS `-D warnings` |
| Final `cargo fmt --all -- --check` | PASS,exit0 |
| One final full Desktop integrated run | **921passed /1failed**,0ignored;105.30s,exit101; NOT PASS |
| C# isolated test-host final build | **0warnings /0errors**,exit0 |
| Matching isolated-codecs worker rebuilt after final feature checks | PASS,exit0 |
| Final C# exact LIST then identical RUN `--fixtures "archive cache fixture" "archive browser"` | **selected=run=pass10**,skip=fail=not_run0 |
| Architecture source gates | ui-preview-isolation,ui-consistency,ui-token-boundaries,platformhost-test-selection PASS; desktop-shell-lifecycle FAIL font guard |
| Preservation / diff before documentation close | **146/146** historical task files unchanged; diff --check PASS |

Native final10names: obstructed retirement restart; expiry startup retained owners; ancestry malformed
recovery; encrypted selected ZIP private lifecycle; encrypted RAR header cancel selected lifecycle;
original session stale missing helper; empty context actual budget; selected revision lifecycle;
strict wire backward defaults; owned lease edited retention. All Fixture classification was explicit.

First10native attempt: selected10/run5/pass4/fail1/not_run5,skip0; RAR failure stopped the selection.
After focused diagnosis/control-ID fix, exact RAR1 passed. Final same10LIST/RUN passed with matching
worker. None of the intermediate failures was relabeled or its raw log overwritten.

Final worker SHA256:
`6A2BDB0D40CE587AF1F552D23DC4C9FA09EEF937D5CE732A14CC58C974FEDA8D`.
Final raw logs in session scratch `163f435b-1ee5-4c07-8102-31a2c7441b9b`:
- `phase-c-closeout-final-{clippy-bins,clippy-tests,clippy-isolated,desktop-full}-20261021-172852.log`
- `phase-c-closeout-final-{fmt,worker,native-host,native-list,native-run}-20261021-173445.log`
- `phase-c-closeout-focused-20261021-170845.log`, `phase-c-closeout-wire-20261021-170845.log`
- `phase-c-closeout-worker-regression-20261021-172741.log`
- cache red/retry/identity/review-red2/review-green unique logs; RAR diagnose/stage/id unique logs.

## 5. Remaining integration/native acceptance, not silent PASS

Full Desktop's one failure:
`ui::efficiency_tests::style_feedback_fade_and_card_keep_theme_selection_states`
at `rust/apps/nxtools-desktop/src/ui/efficiency_tests.rs:1252`, Dark selected-vs-idle painted background
equality. This is the concurrent history-style task's source/test; no causal attribution to archive
was established and no unrelated assertion/module was changed. Other task later starts its own
style_feedback_ run; that cannot erase this integrated921/1 observation. No second full-suite run.

`tests/architecture/desktop-shell-lifecycle.ps1:50` still fails font-family guard. Read-only per-file
matching located a cross-line regex match beginning `UI_FONT_FAMILY)` in nonarchive
`rust/apps/nxtools-desktop/src/ui/preview.rs:744`; this is not proof of a CSS font list or archive
causality. No source gate was relaxed and no preview repair made here. Earlier temporary history
feedback test API compile errors were later resolved by the other task before final run; all
original logs remain.

Phase mapping:
- UI07/LIFE03 source gap: independent bounded presentation persistence and real close/reopen wiring
  delivered/tested; physical maximize/minimize/mixed-monitor acceptance NOT RUN.
- LIFE05: truthful recoverable retirement,24h eligibility, actual startup bounded sweep,
  active/edited/locked/unknown/replacement preservation delivered/tested; actual external viewer
  physical lifecycle NOT RUN.
- LIFE04/LIFE02: encrypted read + selected open uses original finite private password lifecycle;
  Escape/close/old ticket cannot authorize Shell; native selected AES ZIP/RAR5 evidence only.
- Existing UI02-UI06/LIFE01/LIFE06 browser, selection and deferred generation gates remain covered;
  actual Light/Dark/CJK captures, pointer/keyboard/DPI/UIA/user acceptance remain NOT RUN.

Stop at Phase C independent review boundary for this source closeout. **Do not grant full Phase C,
A/B, integrated workspace green or release acceptance** while the unrelated integrated failures and
binding real-GUI/Shell/DPI/UIA/user rows remain. No automatic D/E/F continuation.

Final documentation/preservation close: spec-contracts PASS106documents/14owners; component catalog
PASS79pages/78locked mappings; task context validate PASS82implement/79check with retained size
warnings for native-contracts/archive reference. Final146/146historical hashes match and diff --check
exit0. Logs: phase-c-closeout-final-{spec,catalog,task}-20261021-174023.log. No Cargo/native rerun
was started for document changes; other tasks' active build processes were left untouched.
