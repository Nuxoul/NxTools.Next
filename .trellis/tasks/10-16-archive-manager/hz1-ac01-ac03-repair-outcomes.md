# HZ1-AC01 / AC02 / AC03 scoped repair

2026-10-28. Only the three independently confirmed findings in icon-hz1-independent-acceptance.md. No redesign, HZ2–5, source reclaim, budget/concurrency/dependency/vendor/WAC/icon/installer changes; no package/sign/install/product launch/Registry/service/ACL/UserChoice/commit/push. Round-start copies and protected hashes are in session scratch/hz1-ac-repair-baseline; cumulative HEAD diff is not round ownership. Previous reports/logs remain historical.

## Minimal boundary and agreed regression seams

- AC01: original ArchiveApplication submit success / retire_form -> one private form-scoped advisory retirement helper. Only UI records/deadlines are dropped; native max2 retained slots, running Win32 calls and cancellation policy remain untouched. Failed submit retains valid state; other forms and late receipts are isolated.
- AC02: original ArchiveOutputSession.Prepare.pendingRootParts -> ObserveSpace -> typed optional missing_destination_directories -> checked Rust allowance -> existing form. It is a held-native fact, not parsed opaque identity/path guessing. Preserve outer/archive version1 with additive optional result field; old result missing the count while destination_missing=true becomes unknown, never certain0. Count0..64 must agree with missing flag; no guessed allocation unit. Existing Form/Field + Small unbordered DescriptionList + Small outlined refresh and Medium fixed footer remain unchanged; no new UI/layout/appearance owner.
- AC03: borrowed Path equality only; no allow/suppression or unrelated lint fix.

Required RED seams from user: original close failure retained; actual Start/submit plus cross-form and late receipt; actual retained zero-member browser -> missing target form -> original native result boundary; native GUID-temp no-create observation; strict DTO bound/current request. No compile/selection error counts as behavior RED. Tests are source/compiled/isolated-native evidence, not user/physical/DPI/true-peak acceptance.

## Evidence

Before any production fix, successful Desktop compilation then original lifecycle filter:1PASS/3FAIL/0ignored/1429filtered, exit101 (hz1-ac-red.log). Exact failures: accepted Start leaves2UIrecords expected1; missing-target0available/zero-member still yields definite0demand; original closed-form pending assertion still fails. Existing disconnect testPASS. New optional count test is added at actual native observer before native production changes. Every Cargo checks idle lane and same-command jobs1/incremental0/codegen-units256; all commands serial. Append final commands/counts/results after actual execution.

## Final verdict — AC01 CLOSED / AC02 CLOSED / AC03 CLOSED (source / automated boundaries)

All three requested repairs are complete, with no remaining in-scope failure. This does not
close the archive A–F phase, physical/user acceptance or release/install acceptance. Previous
PARTIAL reports and independent acceptance are retained unchanged as pre-repair history.

### AC01 — exact advisory-record retirement

Root cause: `jobs.rs::retire_form` and accepted `submit_form_admitted` removed/closed the form
without retiring that session's `jobs.spaces`. The record survived until receipt or7sdeadline.
One private `ArchiveApplication::retire_form_space(form)` now filters only matching session IDs,
records a fixed `archive.space.finished`/`form_retired` outcome under the original operation ID,
and drops their UI metadata/deadline. It is called on form retirement and only the successful
submission branch. Repeated retirement is idempotent; rejected submission does not invoke it.
No native cancellation command, native slot release, timeout change or replacement worker exists.

Evidence: retained actual close assertion and actual official Start pointer test were RED
before this repair (closed record present; accepted first form left2records instead of1).
They are now GREEN. Accepted-submit regression keeps the second form's pending record, rejects
the first late receipt, then completes the second through its original result owner. Rejected
submit regression preserves both valid observations and the original retryable form, consumes
the first current observation, then closes each form independently. Original native max2-held
slot/cancellation/late-publication fixtures remain PASS in the12exact native selection.

### AC02 — held-native missing-target fact and checked allowance

Root cause: original `space.rs::estimate` counted archive-internal directories and Name/Smart
wrapper only. Native `Prepare.pendingRootParts` knew the target components that execution would
create, but that fact was omitted from the observation. Zero-byte Here therefore returned
allocation0/shortfall0 even for a missing two-level destination.

Minimal native delta: `ArchiveOutputSession::ObserveSpace` returns
`checked((uint)pendingRootParts.Length)` from the original held no-reparse namespace traversal;
`ArchiveSpacePreflight.Observe` forwards it. No extra stat/scan/create/probe/partial/source write.
Existing full target identity/suffix hash and before/after handle validation remain unchanged.
The number is never inferred from the opaque identity, full path strings or a guessed cluster.

Additive observation field `missing_destination_directories: Option<u32>` / nullable uint:
outer protocol1 and archive-space version1 unchanged; optional omission preserves legacy result
decoding, unknown/unmapped fields stay strict. Valid supplied count is0..64 and must agree with
`destination_missing` (0 iff false). Wrong type/negative/>UInt32/contradictory/>64/current-ID mismatch
are rejected. Rust validates IDs/allocation/identity plus count before plan application; C#
`ValidFor` regression checks the same new-field bound/flag/current-request contract. Nullable
count absent from an old missing=true observation means unknown, never0; old existing=false
observation still has no missing target cost. New C# always publishes the exact count.

Rust's narrow `estimate_observed` reuses the original member estimate, then adds
`checked_mul(count, actual_allocation_unit)` and `checked_add(member_allocation, extra)` once.
Unknown member/target count or overflow keeps allocationNone with an explicit limitation.
Name/Smart archive wrapper and target missing components are separate allowances, not duplicated;
original complete empty archive behavior remains no destination creation. Reserve stays0B,
source/overwrite deletion credit0, original output/memory/bomb/readmission/FIFO concurrency and
execution-time missing/low-space continue/cancel confirmation remain unchanged.

Evidence: actual original browser snapshot containing one0B file -> Here extract form targeting
missing\\child -> original correlated native observation result with0available -> actual form
draw was RED at definite0demand before fix. Same retained assertion now passes, strengthened to
exact8192B with count2/unit4096. This is two advisory directory allowances, NOT measured disk peak.
Checked Plan.shortfall tests at0capacity agree. Existing/single/two missing native observations,
target appearance and actual replacement are exercised; source bytes/mtime and no-created-target
assertions remain. Here/Name/Smart (including single-file-folder), unknown, inconsistent counts,
unknown member, unavailable metadata and near-u64-max arithmetic are covered. Original app/form
late-epoch/revision/error/loading and minimum-footer regressions also pass in the archive set.

### AC03 — borrowed path comparison

Only `slot.path == PathBuf::from(path)` changed to
`slot.path == std::path::Path::new(path)`. It compares the same Path components without creating
an owned allocation; case/canonicalization/matching behavior is not widened. The retained browser
all/selected request-identity regressions pass. The independent pre-repair strict failure is
documented in icon-hz1-independent-acceptance.md; this round's required strict command now exits0.
No lint allow, warning suppression or unrelated lint cleanup was added.

## Actual verification / raw evidence

All logs below are under current session scratch
`C:\Users\Administrator\.pi-desktop\scratch\9d8169e5-656b-47ae-9113-af5789f5091c`.
No Cargo/build timeout; no duplicate build, kill or delegation/reviewer loop. Before EACH Cargo,
the PowerShell command checked cargo/rustc idle, and inherited CARGO_BUILD_JOBS=1,
CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. Commands ran serially with full-operation budgets.

| Exact executed command / log | Exit / actual count |
| --- | --- |
| `cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline hz1_lifecycle_tests -- --test-threads=1`; `hz1-ac-red.log` | Compiled3m20, exit101;1PASS/3FAIL/0ignored/1429filtered. AC01close+actualsubmit and AC02form are the three behavior failures; disconnectPASS. No compile/selection error. |
| `dotnet build csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -nr:false -p:UseSharedCompilation=false`; `hz1-ac-native-red-build.log` | Exit0/0warnings/0errors. Native production unchanged at this point. |
| Same project `dotnet run --no-build --no-restore -- --fixtures --list 'archive space fixture held read-only observation'`, then identical filter without `--list`; `hz1-ac-native-red-list.log`,`hz1-ac-native-red.log` | LIST1/RUN0 then RUN1/PASS0/FAIL1/SKIP0,exit1: actual held observation lacked exact two missing components. |
| `cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline hz1 -- --test-threads=1`; `hz1-ac-green.log` | Exit0;18PASS/0FAIL/0ignored/1418filtered,1.76s. Includes retainedclose,actualsubmit+isolation+late receipt,rejectedsubmit,missing-target0Bform and allowance edge cases. |
| `cargo test -p nxtools-protocol --lib --locked --offline`; `hz1-ac-protocol.log` | Exit0;86PASS/0FAIL/0ignored,0.02s. Optional new-field legacy/strict/bounds/current-request coverage included. |
| C# same build command after final changes; `hz1-ac-native-build.log` | Exit0,0warnings/0errors,10.93s. Current incremental MSBuild, NOT an isolated clean build. |
| `dotnet run --project csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list 'archive space fixture' 'archive destination fixture missing low space confirmation'`, then exactly same filters without `--list`; `hz1-ac-native-list.log`,`hz1-ac-native-run.log` | Both exit0; ordered LIST=RUN=PASS12,SKIP0/FAIL0/not_run0.11original observation fixture methods (extended relevant assertions) +1original execution confirmation; no default/full/native suite. |
| `cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline archive -- --test-threads=1`; `hz1-ac-archive.log` | Exit0;279PASS/0FAIL/0ignored/1157filtered,102.24s. Original icon8/WAC12 allPASS, exact names checked. No weakened/edited icon/WAC assertions. |
| `cargo clippy -p nxtools-desktop -p nxtools-protocol --tests --locked --offline -- -D warnings`; `hz1-ac-clippy.log` | Exit0,48.73s; no warning suppression. |
| `cargo check --workspace --tests --locked --offline`; `hz1-ac-workspace.log` | Exit0,58.52s. |
| `rustfmt --check --edition 2024 --config skip_children=true` on jobs/space_requests/space/forms_tests/hz1_lifecycle_tests/archive_space | PASS. No global format or unrelated file rewrite. |
| Scoped `git diff --check` plus explicit new/untracked-file trailing-whitespace checks | PASS; round-start-relative diff saved `hz1-ac-round-only.diff` and inspected. |
| `python tests/architecture/spec-contracts.py` | PASS114documents/14owners; document/source navigation only. |
| `tests/architecture/platformhost-test-selection.ps1` | PASS reviewed fixture classification, source-only. |

One bookkeeping script first assumed UTF8 for PowerShell Tee-Object logs; it exited1 on their
UTF16BOM AFTER successful fmt/diff/spec/selection gates (`hz1-ac-static-check.log`). Corrected
scratch-only BOM-aware reader then verified ordered native LIST=RUN=PASS12, archive icon8/WAC12
and exact10file code delta (`hz1-ac-scope-accounting.json`). This is not product/test failure or
a behavioral RED; no tests rerun to manufacture a count.

## Round scope, preservation and remaining limits

Exactly10code/test files differ from round start:
Desktop archive/jobs.rs(two retirement calls),space_requests.rs(helper/borrowedPath/observed
estimate route+DTO fixture fields),space.rs(checked observation wrapper+tests),forms_tests.rs
(one DTO fixture field only),hz1_lifecycle_tests.rs(retained and new application boundaries);
Protocol archive_space.rs(additive field/validator/stricttests); C# ArchiveSpaceContract.cs,
ArchiveOutputSession.cs,ArchiveSpacePreflight.cs,ArchiveSpacePreflightTests.cs. Two project docs
are this new result report and the sole checkpoint prepend. Previous acceptance/outcomes reports
are not rewritten; no spec/reference/presentation redesign was necessary for these local repairs.

WAC/password, browser_host/browser/icon tests, titlebar/assets, installer helper, Cargo.toml/lock
are opening-hash identical. All prior user dirty work remains; code diff attribution is against
the saved round start, NOT HEAD. Original checkpoint151180bytes must remain exact suffix (final
document gate records this); prior failures/logs are preserved, not relabeled as historicalPASS.

No physical window/DPI/IME/UIA/user visual or actual-capacity-peak acceptance; no live mounts,
restricted-user ACL/quotas/disk exhaustion/huge archive/production named-pipe end-to-end exercise.
Native GUID-temp handle/reparse/identity/cancellation fixtures are isolated evidence only. Existing
target/debug worker is reused by original execution fixture, not rebuilt/packaged/release-approved
here. Reserve0 is a rule, not reserved disk. Directory allowances are estimates, not exact NTFS
allocation/peak promises. No package/sign/install/productlaunch/system changes/commit/push,
dependencies/vendor/decoder/queue/source-recycle change or HZ2–HZ5 work. Existing unrelated
production-only lint/globalfmt/installation/native acceptance limits remain their historical scope;
this round's requested strict--tests gate and related regressions have no remaining blocker.

Final document gate observed exit0 (`hz1-ac-final-document-check.log`): spec114documents/14owners,
scopedfmt/diff/whitespacePASS; original checkpoint151180bytes verified exact suffix, independent
acceptance byte-identical and9protected source/dependency hashes unchanged. Machine-readable
result: `hz1-ac-final-accounting.json`. Other-session cargo/rustc appeared after this task's
verification completed; they were not stopped, joined or duplicated, and no global-idle claim is made.
