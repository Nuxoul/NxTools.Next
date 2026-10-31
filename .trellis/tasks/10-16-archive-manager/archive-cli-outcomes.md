# Archive CLI — source and isolated command delivery

2026-10-24, same **Phase F — Explorer integration and genuine native drag**. User explicitly
approved switching from manual testing progression to list/test/extract CLI + structured results.
[Approved contract](archive-cli-contract.md), [plan](implement.md),
[usage](../../../rust/apps/nxtools-cli/README.md). Current status belongs only to
[the checkpoint](phase-a-checkpoint.md); [sequence26 live evidence](manual-package26-live-acceptance.md)
and original source acceptances remain separate.

## Delivered boundary

- `NxTools.Cli.exe archive list|test <source> [--json] [--codepage N]` and
  `archive extract <source> --dest <directory> [--conflict fail|skip|overwrite|rename]
  [--create-destination] [--json] [--codepage N]`.
- Original ArchiveReadClient with original bounded v1 metadata/credits/ACK and no-output TEST;
  extraction uses original ArchiveJob typed commands/events and reserved cancellation. No new
  codec, worker protocol, native Job/source/output transaction or GUI/extraction substitute for drag.
- Early archive-only C# mode before ordinary adapter/image-pinning/DPI/extension initialization.
  One private canonical GUID current-user pipe, exact actual sibling CLI parent, same ordinary
  user/session and live held process, connected pipe server PID match; no reconnect or GUI state.
  Original ArchiveReadSession/ArchiveJobSession plus original approved helper/runner own execution.
- Explicit destination, default fail on conflict, original full-path Here hierarchy/MOTW/budgets.
  Explicit consent only for the matching create_destination ticket; lowspace/risk/output-budget
  requests refuse. No automatic overwrite, source deletion, hidden elevation or output cleanup.
- Noninteractive: encrypted TEST/extract returns password_required; no password argv/env/config,
  password UI/retry or P209. Existing download syntax/output tests retained. ConnectOnly refuses
  archive at both parser and public runner before host lookup/start.
- Single schema1 JSON result including failures/syntax, original operation request ID, authoritative
  terminal/commits/skips/journal/native/cleanup. Accepted/receipt/cancel ACK is not completion.
  Child cleanup failure cannot become success. Ctrl+C is wired to original cancel; the child is
  created without a console so the console signal cannot terminate it ahead of protocol cleanup.
  Physical control-signal delivery itself was not exercised and remains unverified.
- New production diagnostics define safe archive.cli_host start/connected/stopped/failure session
  events; original operation events retain original correlation. No payload/secret/byte-tick logging.
  Runtime capture of the new host events was not established in this round; do not claim that the
  existing shared daily sink recorded them. No unrelated logging-runtime repair was attempted.

## Reference and actual verification

Reference was read first, then exact SupaZip nested main/output/table/core and tracked legacy entry
points inspected. Verified commands/call chains/pitfalls were added to docs/reference-code/archive.md
before implementation. SupaZip direct truncation/plaintext argv/no cancellation/incomplete structured
errors were not copied. Legacy general archive CLI was not found in bounded tracked-source inspection.

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\archive-cli-20261024-083518`.
All inputs/outputs are self-owned tiny scratch fixtures or checked-in known small AES data. User's
D:\Temp\temp was not modified/read for content. Original failed logs remain.

| Actually executed | Result / evidence |
| --- | --- |
| Public original parser red | 0PASS/1FAIL exit101: unknown download command archive;01-parser-red.log |
| Initial CLI tests compile | Failed6 test-only await-on-synchronous-injection errors;02-cli-tests.log; fixed API calls, not product behavior |
| CLI first green | Library15 +entry3 PASS;03-cli-tests.log, existing download output matrix runs inside library |
| Initial strict check | Failed large JobStep enum;04-cli-strict.log; boxed control, no suppression |
| Strict CLI and original isolated-codecs worker build | PASS;06,07 logs; no dependency version upgrade |
| First manifest preparation | Refused missing Debug BrowserHost before writes; no unrelated build; used original generator with existing release identity inputs and current worker in scratch |
| Isolated C# host then tests | PASS,0warnings/errors;08,09 and19 logs; original generated worker approval, no edited checked-in hashes |
| C# exact host fixture set | LIST4=RUN4=PASS4,skip0/fail0/not_run0, same order;20/21 logs |
| First real list lifecycle | Native read terminal succeeded/cleanuptrue but hostexit1 -> CLI7;12-first-list-failed.log retained |
| EOF fix and exact list retest | PASS exit0;13/14 logs; stop token now precedes normal channel completion |
| Initial actual command smoke | 11commands PASS;15-cli-smoke.log +smoke/results.json; exact bytes/tree/conflicts/errors |
| Checked-in real command gate | 15commands PASS;16-checked-in-cli-gate.log +archive-cli-6f70ea80-663e-4a8c-bcc8-5997dd7c1c2a/passed.json |
| Known small AES TEST without password | Correct password_required/exit6/cleanuptrue;17-encrypted-no-password.log |
| Known small AES extraction prompt | Correct password_required/exit6/committed0/cleanuptrue/targetempty/stderrempty;24-encrypted-extract.json |
| Public runner ConnectOnly red -> green | Red exit101 observed wrong host lookup result3 instead of syntax2;22 log. Minimal boundary refusal then1PASS;23 log |
| Child console boundary source red -> green |30-no-console-red.log exit1 ->16assertionsPASS after original CREATE_NO_WINDOW pattern; source gate only |
| Final scoped strict +CLI tests/build | PASS;31/32/33 logs, library15/entry4,0FAIL/ignored; includes original downloads and actual-router cleanup/identity tests |
| Final-built actual list | PASS exit0/3entries/2batches/cleanuptrue;34-no-console-list.log |
| Actual list/extract after runner change | PASS;28/29 logs; extract2files11bytes/Committed/originaljournal/cleanuptrue |
| Source/doc/whitespace gates | archive-cli.py16assertions, spec navigation/contracts and scoped diff check PASS; not native acceptance |
| Final no-console extraction | PASS exit0/2files11bytes/exact literal bytes+empty/implicit tree;35-final-no-console-extract.log |

The checked-in integration gate independently generates ZIP Store literals and CRC/unsafe mutations.
It verifies exact metadata, actual TEST counts, exact regular bytes/full empty+implicit tree, default
conflict preserves original, explicit skip/rename/overwrite, missing destination refusal/explicit
creation, unsafe path no output, source missing native2/3, unsupported type and one JSON/no stderr
including rejected password argv. Negative expected command exits are passing rejection assertions,
not accidental failed test processes. Gate does not claim broad format or performance acceptance.

One optional read-only review was started but exceeded the bounded pass and was stopped without a
final report. It is **not review PASS**, no findings were invented, and no second review was started.
Parent fixes above are based on reproduced command/test failures, not speculative review objections.
No full Desktop/workspace/codec×password×conflict/100k/large/performance matrix was run.

## Preservation and delivery

All Rust commands used one inspected lane, CARGO_BUILD_JOBS=1/CARGO_INCREMENTAL=0/
RUSTFLAGS=-C codegen-units=256. No build timeout, duplicate Cargo or unrelated process termination.
Original manifest generator produced scratch approval inputs; existing release Desktop/Indexer/
BrowserHost hashes were identity inputs only, none of those products was launched or changed.
C# builds use isolated output directories; source-default fail-closed manifests stay unchanged.
Cargo.lock changes only nxtools-cli's existing internal/workspace dependency edges, no versions.
Concurrent existing work was preserved; cumulative git diff is not attributed to this round.

Installer SHA308F02D08B22FF66018CAD963A66605C9E3E0F9B2BA8334E1963C55645DA2DD6,
sequence26 manifestSHA23DE45C0...7FB4D515 and6protected binaries were rechecked unchanged.
No scratch CLI/host/worker children remained at preservation observation. No installation, release
sequence/signing/packaging, service/bootstrap/Registry/COM, GUI bypass, user-data deletion, commit/push.

New executable is an **isolated Debug build**, not an installed/released upgrade:
`...\archive-cli-20261024-083518\bin\NxTools.Cli.exe` with its matching host/runtime/worker siblings.
Use the whole matching directory, never copy only CLI into installed sequence26. Usage is checked in.

Remaining boundaries: no password-input capability by design; physical Ctrl+C/kill-during-output and
runtime diagnostics capture unverified. Real Explorer tree/Escape/encrypted/fresh-gesture/refresh-close,
foreign COM/HWND capture/button continuity/DPI/UIA/user visual remain separate. First two-file drag
has only its recorded partial evidence, content/sample confirmation remains pending. P206/P207 live
registration stays NOTRUN/ProductApplyAuthorized=false; priorA–E/JOB07/SAFE08 obligations remain.
Not fullP208/PhaseF/first-release completion. Stop at this finite noninteractive CLI slice.
