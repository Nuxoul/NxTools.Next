# Phase C — Archive browser and entry lifecycle: partial-delivery independent acceptance

2026-10-21. **Full Phase C NOT accepted. Bounded browser/read/selected-ZIP behaviors reproduced,
but an actual temp-cache retirement defect, declared source gaps and current-workspace gate failures
remain.** This is review of the partial delivery, not a statement that it reached all phase exits.
Only [checkpoint](phase-a-checkpoint.md) owns current status. [Implementation outcomes](phase-c-browser-outcomes.md)
and prior reports remain unchanged. No fallback reauthorization or new phase is required to fix C.

## 1. Concrete in-scope finding — P2 cache retirement loses retry authority

`csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs:154,159–174,202–206`:
After verified output deletion, Lease.Release calls RetireEmpty but ignores its false result and
returns true. If a viewer/user has added another file directly under the owned session root,
RetireEmpty removes the manifest first, then Windows refuses the nonempty root deletion. The cache
then drops/disposes the lease as successfully released. Later sweep cannot reopen the root without
its manifest. Unknown content is correctly retained; the defect is false full-retirement success
and losing the bounded ability to finish owned cleanup after the obstruction is gone.

One optional read-only native review identified this; parent independently reproduced it using the
current built PlatformHost assembly and a scratch-only test apphost through its existing owned-test
cache seam. No Shell/UI or archive contents were executed. Normal control release removed its root.
The obstructed case observed:
- release=true; selected output absent; added unknown file still present;
- manifest absent; session root still present; repeat release=false;
- after deleting only the probe-created extra file and triggering normal admission sweep, empty
  session root still remains.

Evidence: parent scratch `phase-c-cache-probe/{Probe.csproj,Program.cs}` and
`phase-c-browser-independent/cache-probe-result.json`. The probe built with one WindowsBase reference
resolution warning and0errors, then exited0 because it reproduced this failure shape; exit0 is NOT
product behavior PASS. No production source/test was patched during acceptance.

Fix the original release/retirement accounting and preserve narrow owned retry/manifest authority
until actual retirement, without deleting unknown/edited/locked files or adding recursive cleanup.
Add a red-capable original-cache regression for this exact sequence. Merely adding the already
missing startup/expiry trigger does not recover manifestless orphan roots safely.

## 2. Source and production-boundary review

Read approval/contract/reference index, current outcomes, model and production selection/rendering,
application read installation/entry/Shell gates, tests, original shared IPC seam and typed picker.
A bounded independent review covered native materialization/cache/selected-budget boundaries.

Confirmed bounded implemented paths:
- Formal settings Open -> typed bounded STA single-file picker -> original ArchiveReadClient and
  shared archive router, without stealing ordinary Desktop notifications.
- Finite v2 browser reads, retained batches before ACK; only succeeded+cleanup-complete terminal
  with validated/matching browser snapshot installs an executable view.
- Canonical keyed activating windows, deferred view/app/Tree work, immutable snapshot+ordinal
  selection and official ListItem virtual viewport; actual production selection/sort/filter/
  keyboard/right-click/horizontal/column callbacks have compiled regressions.
- Entry result checks outer success, typed operation/session/generation/current snapshot and cleanup
  before a single Shell submission or child browser. Cancelled/stale results release their own lease.
- Native selected ZIP extraction/source revision/cancel and owned edited/locked retention fixtures
  execute real small workers/native file boundaries, never actual Shell Open.

These observations do not certify the whole temp lifecycle, every format, real native picker input,
physical window/focus/DPI/UIA or actual user visual acceptance.

## 3. Independently executed verification

All direct Cargo used jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, serial/offline, with lane checks.
An unrelated height_feedback Desktop build was observed; no duplicate was started or process killed.
Owned C# test apphost built before native checks; matching isolated-codecs worker rebuilt after lint
feature-unification checks and before the final C# LIST/RUN and3exact pipe-chain tests.

| Check | Independent result |
| --- | --- |
| Desktop `archive::` | 34passed,0failed/ignored; first876filtered, repeated878filtered; no summed count |
| Desktop full attempt1 | **907passed/5failed**,0ignored,114.16s; NOT PASS |
| Desktop full attempt2 | **909passed/3failed**,0ignored,109.26s; unintended continuation rerun; NOT PASS |
| Protocol / IPC full libraries | 57 /31passed,0failed/ignored |
| Read consumer / cancellation / worker wire / protected extraction | 6 /10 /17 /1passed,0failed |
| Selected-entry budget library regression | 1passed,16filtered,0failed; synthetic metadata only |
| C# test project build |0warnings/errors,exit0 |
| C# exact `--fixtures --list "archive browser"` then identical RUN | LIST=RUN=PASS same5names,skip/fail/not_run0; worker selected ZIP among them |
| Three exact product_read_chain tests | each1passed/23filtered,0failed; typed pipe+actual worker, finite v2, unavailable-source cleanup/native error |
| Production strict Clippy | **FAIL**,ui/preview.rs:70 type_complexity |
| Test strict Clippy | **FAIL**,same preview type_complexity |
| Archive isolated-codecs all-targets strict Clippy |PASS |
| Full fmt --check | **FAIL**,ui/preview.rs:129 formatting |
| desktop-shell-lifecycle | **FAIL**,font_family/CSS-fallback-list guard; causal source not established |
| ui-preview-isolation / ui-consistency / ui-token-boundaries / platformhost-test-selection |all PASS,source-only |
| spec-contracts / component catalog / task validate / diff --check |PASS;106docs/14owners,79pages/78locked mappings,82implement/79check; existing context-size warnings remain |

Full attempt1 failures (no causal attribution to archive code established):
- clipboard::asset_tests::first_clipboard_preview_is_hidden_and_uses_confirmed_passive_presentation
  (clipboard.rs:3380, expected confirmed native chain);
- main_window::extensions_page::tests::w6_manager_pointer_preview_confirm_and_toggle_commit_real_config
  (extensions_page.rs:445,index out of bounds,len0);
- ui::history::kit_migration_tests::queue08_history_actions_modifiers_and_variable_content_keep_one_owner
  (history_kit_migration_tests.rs:122,72px vs64px);
- ui::polish_tests::history_row_keyup_dispatches_once_with_fixed_hit_bounds
  (polish_tests.rs:399,1080px vs64px);
- ui::preview::tests::rich_preview_resources_never_reach_http_and_long_markdown_scrolls
  (preview.rs:1060,last block layout absent).
Attempt2 retained failures1/3/4 above. Another task changed nonarchive sources during this session;
counts differ from historical908 and first focused run910. No unrelated source or assertion was fixed
here. The historical908/908 handoff is not an independently green current full suite.

### Verification script limitation, disclosed
The continuation wrapper parsed prior result arrays as nested PowerShell `value` wrappers and did
not skip completed names as intended. It unintentionally reran C#build/archive34/fullDesktop and
overwrote parent scratch's first full-suite raw log/results path. First counts/failure details were
inspected and remain in tool output; an explicitly labeled transcript-derived observed-attempts.json
records them, not a reconstructed raw log. Attempt2 raw log was copied to desktop-full-attempt2.log.
Original implementation evidence files were never overwritten. A fresh fixed remaining-only script
ran no further full suite. No successful rerun erased either failed observation.

## 4. Declared gaps confirmed; full Phase C remains incomplete

- UI07/LIFE03: window position/size/maximized and column presentation persistence not wired.
- LIFE04/password exit reconciliation: encrypted entry temp-open explicitly unsupported; browser
  password interaction not delivered. Prior password engine acceptance does not provide this UI.
- LIFE05: no expiry policy/startup sweep; current sweep only on new admission. The defect in§1 is
  additional, not merely this trigger gap. No arbitrary/recursive cleanup is authorized.
- Native7z/RAR selected-entry interoperability not proved by current ZIP fixture+synthetic budget test.
- Live default/minimum Light/Dark/CJK, actual picker/Shell/input/DPI/UIA/user acceptance remain NOT RUN
  under binding self-app safety denial. No bypass or formal product launch occurred.

The prior EDIT_LINES_UNSEEN stop is a previous-turn editing limit, not a permanent architectural
blocker; continue the same Phase C in a later implementation turn without repeating spike/fallback.

## Evidence, preservation and next boundary

Parent scratch ceebe351-d540-4ca3-a460-dadc71f3a7df:
`phase-c-browser-independent.ps1`, `phase-c-browser-final-checks.ps1`, `phase-c-browser-independent/`
logs/results, observed-attempts.json, cache-probe-result.json and preservation.json. Some original
results have nested value wrappers as explained; per-command logs/final-results/remaining-results
and actual selected/run/pass identities were checked directly.
Worker SHA256 FC2C9A8C6D3F1370285E220ABED661F674D5F082F1B33CF8585D24E33AACB402;
PlatformHost.dll SHA256 2989BAC9D73AE090BD2CB93A7BC1BE71DE758D9FBA00B68DDC4DDDA9A5683E56.

Before report edits, **228/228 tracked acceptance-baseline task/archive source/manifest files match**.
This review did not independently obtain the implementation's142-opening-file hash list, so does not
reassert that historical count as a fresh check. Only this additive report/current checkpoint change
is made by acceptance; no implementation/history/dependency/vendor edit. No100k/large/performance
matrix, GUI/Shell/system mutation, packaging/install/signing/commit/push or D/E/F occurred.

**Disposition:** retain reproduced bounded capabilities as partial evidence, do NOT grant complete
Phase C or complete temp-entry lifecycle PASS. Next implementation stays in Phase C: fix§1, finish
window/UI persistence, expiry/startup cleanup and browser password behavior; resolve unrelated
workspace gate failures with their owners before claiming a final integrated green suite.
