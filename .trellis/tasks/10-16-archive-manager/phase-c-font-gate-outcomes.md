# Phase C — Archive browser and entry lifecycle: font source-gate outcome

2026-10-21. **Known source defects and this static gate are closed within the recorded scope;
not overall Phase C PASS.** Only [checkpoint](phase-a-checkpoint.md) owns current status.

## Reproduction and actual cause

Ran `desktop-shell-lifecycle.ps1` unchanged first: exit1, original font error at line50.
Extracted its actual predicate and complete match, including adjacent source. The only aggregate
match is also a per-file match: `rust/apps/nxtools-desktop/src/ui/preview.rs:744–790`, aggregate
index4615900, length1809. It begins at the legal `UI_FONT_FAMILY)` argument and ends at
`height: measured,`; it is not a cross-file match or an actual GPUI CSS font-list value.

The old constant alternative allowed `UI_FONT_FAMILY[^=]*` to cross the call, subsequent code and
an unrelated assignment at line776. This Windows PowerShell default code page936 decodes that
UTF-8 CJK line without its closing quote; the unrestricted string class then reaches a later
comma. Controlled default-read/UTF-8-read comparison gives1/0matches. The actual UTF-8 source
has its closing quote. Actual font constants remain single families: Microsoft YaHei UI/Consolas.

## Minimal change and red-to-green

Only the shell gate's font predicate/comments and a small PowerShell regression were changed;
no Rust, font, theme, layout, clipboard/history/preview behavior or dependency change. Require
an actual const/static declaration instead of a variable use, bound the type/value scan, and
allow whitespace before a direct literal. No file exclusion/whitelist, disabled rule or parser
framework. Original source collection/reading and all other shell checks remain unchanged.

`desktop-font-family-regression.ps1` reads the exact predicate from the real shell gate. Its final
16cases ran against the original rule:13pass/3fail, including the damaged-decoding cross-line
pattern and ordinary following comma text. The unchanged regression (identical SHA256) then
passed16/16. It rejects direct/UI/mono/static lists and multiline formatting; accepts single
families, variable/multiline calls, following strings and adjacent source fragments.

| This turn's check | Result |
| --- | --- |
| Final detector regression, unchanged red→green |13/3 →16/0; exit1→0 |
| Complete desktop-shell-lifecycle |PASS, exit0 |
| PowerShell syntax / only-font-predicate delta |PASS |
| spec-contracts |PASS,106documents/14owners; documentation only |
| Task context validate |PASS,82implement/79check; existing size warnings retained |
| Scoped diff/whitespace including new files |PASS |

Before report/status edits,151historical task files were unchanged. Of180Desktop Rust files,
178matched the opening baseline; concurrent changes were observed in extensions/manual_actions.rs
and ui/efficiency_tests.rs, left untouched and recorded separately. Of the unrelated dirty-file
baseline,1508matched and those same2changed; no claim of a globally unchanged workspace is made.

## Evidence and stop

Unique session scratch directory: `phase-c-font-gate-20261021-222332-fbb8386f`.
Original failure, full match/context, decoding comparison, red/green logs and hashes, scoped checks,
concurrent-change hashes and document checks are preserved there. Historical reports are unchanged.

Prior independent Desktop922/922,archive40,Protocol59,IPC31,native10,strictClippy/fmt and accepted
cache fairness remain **prior evidence, not rerun checks**. No Cargo, broad suite, native/cache test,
GUI/Shell launch, package/install/system mutation, commit/push or D/E/F occurred this turn.
Real default/minimum Light/Dark/CJK, picker/Shell/input/DPI/mixed-monitor/UIA/user acceptance stays
**NOT RUN** under the binding self-app denial. Phase A/B unfinished obligations remain unchanged.
No new source-closeout task is opened; stop at this scoped result, not overall Phase C acceptance.
