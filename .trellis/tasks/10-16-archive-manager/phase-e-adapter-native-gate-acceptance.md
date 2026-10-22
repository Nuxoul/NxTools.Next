# Phase E — ZIP container/native same-source gate: independent partial acceptance

2026-10-22. **Partial container evidence reproduced; P202/P203 and Phase E NOT accepted.**
Actual same-source native replacement remains FAIL32. One additional concrete uppercase-volume
routing defect is confirmed. No edit-source lease handoff is approved or implemented by this review.
Only [checkpoint](phase-a-checkpoint.md) owns current state. [Implementation outcomes](phase-e-adapter-native-gate-outcomes.md)
and previous reports remain historical/unchanged.

## 1. P2 — uppercase numbered-volume path is misclassified as corrupt on Windows

`rust/crates/nxtools-archive/src/zip_container.rs:213`: discovery recognizes z/Z numbered extensions,
then checks caller membership using case-sensitive PathBuf equality against generated lowercase
.zNN paths. On Windows, the same existing file opened as created.Z01 is therefore rejected despite
created.z01 working. One bounded read-only source review found this; parent reproduced against the
freshly tested small six-volume set, linking an identified existing Archive rlib:
`lower=Ok(1)`, `upper=Err(CorruptArchive)`. Windows Test-Path confirmed both names resolve.

Fix exact selected-volume membership using the actual parsed volume/family identity and Windows
path semantics without broadening discovery; add a small original public-entry regression.
No production source/test was edited here. Probe itself exits0 to confirm the negative observation,
not because the product behavior is correct. Its rlib/hash are recorded; it is not a separate fresh
Cargo build. Case-sensitive-platform expectations must remain explicit.

## 2. Same-source native gate — independently reproduced, decision still required

Original chain is actually exercised by the new C# fixtures: held ArchiveSourceLease, protected
parents, captured target identity, complete candidate write/readback, original Commit/final recheck/
atomic path replacement. The tests still REQUIRE successful publication; neither native32 nor
missing late-cancel stage was relabeled as an expected success.

Each exact fixture separately LISTed then RUN:
- held source same path commit: selected1/run1/pass0/fail1/skip0/not_run0;
- same path cancellation truth: selected1/run1/pass0/fail1/skip0/not_run0;
- changed revision refusal: selected1/run1/pass1/fail0/skip0/not_run0.

Total unique LIST3=RUN3, **PASS1/FAIL2/SKIP0/NOT_RUN0**. Both failing cases stop at
`replace_file / native32 / FailedBeforeCommit`; late-cancel native_published is NOT reached.
Independent failure-proof JSON confirms before/after source SHA256
579FFEC4A8A21CD5CC2612BD52E382A35FB8B313804B54BBCF04D445981BF48D, held identity unchanged,
journal FailedBeforeCommit and replaced=false. This is an actual native blocker, not inference
from sharing flags or POSIX_SEMANTICS. It proves the existing held-source arrangement cannot
complete this tested same-path publication; it does not itself authorize releasing protection.

The proposed edit-only candidate-verified/worker-reader-closed/final-source-rechecked FILE-lease
handoff is still a user decision, not this acceptance's implementation recommendation approval.
Its new final-check-to-publication TOCTOU and exact ownership/cancel/failure behavior must be agreed
before code. Do not alter ordinary extraction/creation sharing, move/delete original, add backups
or blind retries to obtain green. Failed fixture roots remain scratch evidence.

## 3. Delivered container boundary and independent checks

Reviewed contract, engine public opt-in/candidates/revision, container disk mapping/raw-record copy,
small regression and native commit fixture. One optional bounded review covered container only;
parent checked native gate and executed tests. No second review loop or broad matrix.

- with_zip_volumes is explicit opt-in. Default/with_limits and production worker do not enable it;
  portable volume revisions are not all-volume native file-ID admission.
- Candidate API returns anonymous unpublished encoded files, no path publication authority. Small
  writer adjusts actual disk-relative records and preserves tested payload integrity. It is not
  complete record-preserving ZIP editing or general encrypted/ZIP64 production-volume acceptance.
- Native all-volume leases, new-target conflict/race/publication journal/UI,7z/RAR volume families
  and all P203 edit actions remain unimplemented as disclosed. P204/P205 not implemented by scope.

All Cargo serial/offline/locked with jobs1/incremental0/RUSTFLAGS=-C codegen-units=256 and lane check.

| Check | Independent result |
| --- | --- |
| Exact candidate library test |1PASS,0fail/ignored,12filtered |
| Engine small tests |14PASS,0fail,1explicitly filtered100k |
| ZIP boundary / volume contracts |2 /4PASS,0fail/ignored |
| Archive isolated strict Clippy lib/bins/tests |PASS,-Dwarnings |
| Workspace check --tests |PASS under current Kit0.8.0/pre0.3.9; compilation only |
| Full fmt --check |PASS |
| C# test host build |0warnings/errors,exit0 |
| Three exact native boundary tests |1PASS/2FAIL as above,not overall green |
| Hash-verified7Zip26.00 l -slt/t/x |all3commands exit0; Multivolume+,Volumes6,member disk0/offset4 |
| Extracted payload |600bytes,SHA256 DB4F2AC25D140369324DBED60D7B8E314FDF1252C171F8513FB7DBF5CC92E88D |
| spec/catalog/current-GPUI-release/task/test-selection/diff gates |all PASS;108docs/14owners,current0.8.0 authority,two approved patches; existing context warnings retained |

21small compiled tests above are distinct candidate1+engine14+boundary2+volume4, not a full archive
or Desktop suite. 100k was explicitly skipped and did NOT run in this independent turn. The earlier
implementation's accidental100k selection remains a disclosed scope violation, not new authorized
performance evidence; no repeat. No full Desktop, large/performance/format cross-product or GUI run.

Native fixtures call source/output boundary directly and do not launch the codec worker. This
acceptance deliberately did not rebuild a worker merely to imply worker-job evidence. No worker
or product runtime was launched by the native tests. Oracle is a scratch-only test reader, not a
product backend. All volume output/oracle extraction is under the unique owned scratch directory.

## 4. GPUI upgrade impact / combined follow-up boundary

Current product is exact Kit/Base/Component/Assets0.8.0/defaultpre0.3.9, not historical0.7.1.
The current GPUI source/lock/provenance gate and workspace --tests compilation passed here. This
is not independent rerun of upgrade979/979 or real native input/render acceptance. No UI was changed.
Future Phase E UI must use current released APIs; do not copy stale0.7.1pin instructions. Existing
approved ListItem/virtual-list owner is not automatically replaced by DataTable; no new component
spike or dependency upgrade is required by this partial acceptance.

## Evidence, preservation and disposition

Parent scratch ceebe351-d540-4ca3-a460-dadc71f3a7df:
- phase-e-native-gate-independent.ps1
- phase-e-native-gate-independent-20261022-124613/{commands.jsonl,per-command logs,volumes/,extracted/,
  baseline.json,preservation.json,uppercase-probe.log,uppercase-probe-identity.json}
- phase-e-uppercase-probe.rs/exe (scratch-only), independently retained failed native GUID roots under
  phase-e-implementation-20261022-102842/native-same-source/fixtures.

Before records **221/221current acceptance-baseline task/archive/native source/manifest files
unchanged**. The implementation's481opening baseline is not reasserted as a fresh comparison.
Acceptance only adds this report and updates current checkpoint; no product/dependency/vendor/
sharing/rights/transaction/history changes. No source recycle/system mutation/GUI safety bypass/
package/install/signing/commit/push or Phase F.

**Disposition:** bounded ZIP container progress and actual native gate evidence are substantiated,
not Phase E/P202/P203 PASS. Preserve default-off production state. Carry uppercase-volume repair,
edit-only lease decision and current0.8.0 UI authority into the next combined task when requested;
this review does not issue that task or approve the lease amendment. A–D pending requirements and
real GUI/Shell/DPI/UIA/user/release acceptance remain unchanged/NOT RUN.
