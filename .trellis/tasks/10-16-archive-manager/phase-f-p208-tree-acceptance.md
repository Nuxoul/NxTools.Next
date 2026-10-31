# Phase F P208 directory / unknown-size — independent acceptance

Date: 2026-10-23. **PASS for the finite production source / controlled-consumer slice after one
local fixture correction.** Prior concurrent Desktop compilation blocker no longer reproduces.
Not actual Explorer/foreign COM/physical acceptance, full P208, Phase F or first-release completion.
Current state: [sole checkpoint](phase-a-checkpoint.md). [Implementation outcomes](phase-f-p208-tree-outcomes.md)
and historical reports/failure logs remain unchanged; [production contract](phase-f-p208-production-contract.md)
is the finite capability authority. F-P208-AC01 remains CLOSED.

## Review and accepted behavior

Parent inspected original BrowserModel/model_drag, formal row DTO construction, protocol validation,
source-origin admission retention and original C# worker/source/output/cache wiring. One bounded
read-only C# review inspected descriptor/index/actual budget/cleanup; no second review loop and no
new confirmed product defect. Current approved scope:

- At most32 selected roots,128 actual descriptors INCLUDING parents,32 files,256 contents requests,
  256KiB serialized command and512MiB known estimate / actual output total. Original cache32leases
  and native owners are not enlarged.
- Immutable snapshot traversal includes filtered/hidden descendants, explicit/implicit/empty/multilevel
  directories. Parent/child overlap visits original entries once; deterministic relative hierarchy
  preserves original file ordinals/context/source, not current displayed indices. Paths are validated
  before rebasing, not sanitized. Unsafe/link/encrypted/unknown-encryption/conflicting/out-of-budget
  selections reject wholly; no partial truncation or silent hidden-member exclusion.
- Rust/C# receivers independently require every actual same-case parent directory and cap distinct
  top-level roots. Nullable size is unknown, not zero; implicit directory ordinal omission is descriptor
  metadata only and cannot authorize materialization. Files retain real original ordinal.
- SDK592byte FILEDESCRIPTORW uses directory attributes and omits FD_FILESIZE for directories and
  unknown-size files. Full-array lindex includes directory slots; directory FileContents refuses before
  request counting/content invocation. Descriptor request does no materialization. Native IStream Stat
  returns actual sealed size; repeats open independent cursors and held streams survive retirement.
- Distinct contents serialize through original materialization plus verified actual-length accounting.
  Remaining allowance narrows original EntrySession/worker output BEFORE writes. Repeated index is
  charged once. Retained/unverified/incomplete failure cannot finance extra distinct output.
  BrowserEntry encoded source-family cap independently stays512MiB; shrinking output does not shrink
  source permission. Existing cancellation/actual-cleanup/oneSTA/oneOLE and AC01 connection binding stay.
- When remaining reaches0, new file requests reject even zero-byte output (original positive-budget
  contract); zero-byte files work with positive allowance. Explicit current limit, not unknown=zero.

## Fixture defect discovered and minimally corrected

Fresh final browser collection initially ran **27PASS/1FAIL**,exit101 (`04-browser.log`). Failure:
`production_row_drag_tree_directory_pointer_freezes_or_rejects_hidden_member_wholly`,
`browser_drag_tests.rs:483`, `actual directory row`. It had not reached its drag/assertion boundary.

Cause: the fixture installs a snapshot containing only `tree/deep/child.txt`; original approved
`ArchiveBrowser.install` auto-enters exactly one safe root directory (`browser.rs:346-350`). The test
then asks for the root-level `archive-folder-tree` row which is no longer visible. This is a fixture
setup error, not a reason to change approved product navigation.

Only `browser_drag_tests.rs:477-479` changed: add one unselected root-level ordinary file so the root
has two rows and does not auto-enter. The original directory pointer events, expected full hierarchy,
unknown child ordinal and hidden encrypted descendant WHOLE rejection assertions are unchanged.
No product/layout/controller or warning assertion was weakened. Exact failed case passes in
`04b-directory-fixture.log` (1PASS), then all28 browser tests pass inside the one complete Desktop
run. Original failed log retained; no repeated whole browser collection/green-hunting full suite.

## Independent execution

Evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\p208-tree-independent-20261023-193133`.
Runner: session scratch `p208-tree-independent-verify.ps1`; raw command logs, commands.jsonl,
independent-results.json, csharp-meta.json and full-meta.json retain exits/counts/identity/drift.

| Check | Independent actual result |
| --- | --- |
| Fresh isolated PlatformHost THEN Tests builds | PASS,0warnings/errors;01,02 logs |
| Production model through actual Desktop target | 10PASS,0FAIL;03-model.log (not supplemental model crate) |
| Initial final browser collection | 27PASS/1FAIL;04-browser.log; local fixture correction described above |
| Exact corrected directory-pointer case | 1PASS,0FAIL;04b-directory-fixture.log |
| Application lifecycle including both AC01 gaps | 12PASS,0FAIL;05-application.log |
| Protocol library | 74PASS,0FAIL/ignored/filtered;06-protocol.log |
| IPC library | 40PASS,0FAIL/ignored/filtered;07-ipc.log |
| Desktop/Protocol/IPC strict Clippy --tests -- -D warnings | PASS,exit0;08-strict.log |
| Workspace --tests | Compilation PASS,exit0;09-workspace.log; not execution of every workspace test |
| Desktop/isolated-codecs feature checks then matching worker | PASS,10-12 logs; no feature/dependency edit |
| Exact C# controlled set | LIST11=RUN11=PASS11,selected11/run11/pass11/skip0/fail0/not_run0;13,14 logs; names/order equal |
| Worker SHA before/after C# | `0F6DA80E4C26B74CB90BAABD0D6144E9E107920994AC57673BBC743B069467D4`,identical |
| ONE full Desktop after convergence | 1071PASS,0FAIL/ignored/filtered,139.97s test time,exit0;15-desktop-full-once.log |
| Formal collections within that full run | model10/browser28/application12,including12 formal drag names; overlapping, not additional disjoint runs |
| Scoped rustfmt | PASS,8 affected Rust entry/modules,skip_children=true; no global reformat |
| Source/doc gates | PASS: Phase F tree/origin/reservedcancel/STA/content, Shell, GPUI catalog/release/editor provenance, specs110documents/14owners, task and diff |

Exact C# names verified equal in LIST/RUN/PASS:
- archive Phase F virtual tree wire shapes conflicts bounds
- archive Phase F virtual tree SDK descriptors directory refusal
- archive Phase F virtual tree actual ZIP unknown streams identity
- archive Phase F virtual tree actual remaining budget cleanup
- archive Phase F virtual tree actual unknown Zstd native stream
- archive Phase F virtual tree source versus output budget
- archive Phase F virtual formal wire alias source peer rejection
- archive Phase F virtual descriptor indexed streams lifetime
- archive Phase F virtual STA gesture cancellation responsiveness
- archive Phase F virtual actual worker delayed indexed contents
- archive Phase F virtual slow worker cancel source close disconnect

Actual tiny ZIP verifies full directory-array indices map original file ordinals, zero descriptor
leases, independent repeated streams and sealed-identity refusal. Independent tiny no-content-size
Zstd read yields size=null/encrypted=false, descriptor without FD_FILESIZE, real worker payload `one`
and actual native Stat3 with retained stream lifetime. Small lowered-output fixtures verify actual
remaining budget, concurrent/repeat behavior, known estimate mismatch, zero remaining and encoded
source/output separation. Old slow-worker cancel/Escape/source-close/disconnect still waits original
cleanup. These are controlled consumers, not a foreign Explorer target or actual DoDragDrop loop.

Inspected preserved historical reds: formal-tree1FAIL, protocol missing-parent/root3PASS+2FAIL,
source-versus-output budget selected1/run1/fail1. They remain implementation red evidence, not new
parent runs against reverted product source. Parent executed final code only and retained its own
fixture failure above. No old Ole32 pre-cancel timeout fixture or codec/password/conflict/100k/large/
performance matrix run. Each Cargo checked/waited for the shared lane and inherited jobs1/incremental0/
RUSTFLAGS=-C codegen-units=256. Host preceded checks; matching worker rebuilt after feature/compile
checks and before exact native fixture run. No competing build/process termination.

## Integration / preservation / remaining

The old floating-assistant compile errors are no longer blockers: actual Desktop target tests,
strict Clippy and workspace test compilation pass without editing that feature. Full run monitored710
source/project files; five unrelated concurrent files changed:
- rust/apps/nxtools-desktop/src/tools/floating_assistant/view.rs
- rust/apps/nxtools-desktop/src/tools/floating_assistant/application/tests.rs
- csharp/NxTools.PlatformHost.Tests/FloatingAssistantCoordinatorTests.cs
- csharp/NxTools.PlatformHost.Tests/Program.cs
- csharp/NxTools.PlatformHost.Tests/TestEffects.cs

Thus1071PASS is an observed integration run, NOT frozen-entire-workspace certification. No archive
source changed during the full run, no full-suite rerun or unrelated fix. Initial3124-file baseline
before report/checkpoint writes had15changed:1ownedfixture correction +14concurrentassistant files,
missing0. Current product/IPC/protocol/native archive implementation, dependencies/vendor, installer/
package scripts and historical archive reports were not edited by acceptance. Only additive acceptance
report, sole checkpoint and the one local test fixture correction are owned workspace edits.
**84/84 delivery files unchanged**; setup SHA remains
`1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
Concurrent source/docs preserved, no reset/commit/push. Task validation retains existing context-size
warnings, not a reason for unrelated documentation restructuring.

Password interaction/encrypted or unknown-encryption entries and unsupported kinds remain excluded.
Actual GPUI HWND/capture/physical-button continuity, real external OLE/foreign-process COM/Explorer,
DPI/UIA/user visual acceptance remain NOT RUN/unaccepted. Binding GUI security denial not bypassed.
No Registry/COM registration, service/bootstrap/recycle/product startup/package/sign/install. Sequence25
does not contain this source. Installer duplicate payload remains deferred until next separately
approved packaging. Prior A-E/JOB07/SAFE08/native/user debts remain. Stop: **finite directory/unknown-size
source and controlled chain accepted; no new confirmed product blocker**, not fullP208/PhaseF/firstrelease.
