# ZIP creation acceptance remediation

Date: 2026-10-18. Current phase: **Phase A — Engine contracts and safety foundation**.
This record repairs the independently reproduced creation-admission/parent-output findings and the source-confirmed namespace-binding gap. It does not replace or rewrite [independent rejection](zip-create-acceptance-review.md), earlier [creation evidence](zip-create-evidence.md), or the approved atomic-path overwrite contract. The unique current integration status lives in [phase-a-checkpoint.md](phase-a-checkpoint.md). Historical repaired integration was archive82/C#75 below; a later independent [directory-side-effect failure](zip-create-remediation-acceptance.md) is repaired in [output preparation evidence](zip-create-output-preparation.md). Phase A remains incomplete.

## Scope and source inspection

Read live Runner/Budget/OutputBridge/CreationSources/OutputSession and the actual test harness, applicable archive/PlatformHost/Protocol/IPC/shared contracts, design/implement/acceptance and creation contract/evidence/review. Rechecked legacy tracked archive terms: package/FFmpeg ZIP and image/video owners only; no general archive-window lifecycle found (not exhaustive generated/untracked search). No broad reference reinvestigation or writer/crypto/reader rewrite. Rust policy/codecs and C# handles/Job/native output ownership are unchanged. No UI, new format implementation, source deletion, dependency change, release/package/install, registry/service/drive mapping, user archive, commit or push.

## Admission ownership

`ArchiveWorkerAdmission.Capture` runs synchronously inside Start before detailed create validation, wire serialization, Run construction and every observable accepted event. It owns independent copies of SourcePaths, ZipOptions.ExcludePatterns and ConfirmedOverwrites, plus five immutable scalar `ArchiveAdmittedLimits` values. Other record fields are immutable strings/value types; the nested ZipOptions record is copied with its private pattern copy. Array.AsReadOnly/ReadOnlyDictionary wrap newly allocated private storage, never the caller's list/dictionary. Run saves only the captured request. Both approved Job/wire limits and asynchronous native source/output consumers use its fixed limits. The output bridge may copy the already-private authorization again; it never sees the caller's mutable dictionary in Runner execution.

Capture is bounded before retention: <=32 source paths, <=32 patterns, pattern <=1024 UTF8 bytes and <=8192 combined, <=4096 target confirmations, bounded path/identity/password strings and a 4MiB cumulative UTF8 admission reservation. Limits are read as at most five known unsigned scalar properties/dictionary/JSON entries, not an arbitrary unbounded serialization followed by a size check. Missing budget fields retain their old defaults and Approve keeps range/resource/memory-mismatch categories. Source first-path binding, create-name-only authorization and legacy omitted ZipOptions/default exclusions remain enforced.

Bounded enumerator copies are compared again against caller values/counts across the whole capture, and List version guards remain live through the capture. Observable inconsistent collection/budget reads reject before accepted (`request_changed`); capture does not claim a filesystem/source-set atomic snapshot or an atomic transaction over arbitrary caller-defined accessors. Once accepted there is no caller-owned mutable data reference used later. Tests include deterministic source List/array changes triggered while budget properties are read, not only mutation after acceptance.

## Output direction and namespace

Reject output root equal to or below any directory input; do not reject merely because an input directory is below the output parent and the final ZIP is its sibling. Final source-file equality, real full volume/128-bit identity aliases, no-reparse checks, multi-source overlap/name/case/prefix rules remain. Real worker independent .NET reader checks sibling output and confirmed overwrite exact names/bytes/empty directory. Separate fixture retains inside/equal/multi/same-file/hardlink/reparse rejection and source/owned-partial preservation.

Considered direct held-handle directory enumeration versus stable held-handle-derived namespace. Reuse existing GetFinalPathNameByHandleW(VOLUME_NAME_GUID) in `OutputNative.StableDirectoryPath`, originally present for native publication, to minimize changes. Before Directory.EnumerateFileSystemEntries, reopen the actual derived directory and VerifyRecord against its held SourceRecord. Original root/lineage handles deny delete sharing throughout enumeration. No original DOS path is reconstructed to enumerate. Root last component and child kind/attributes are discovered through actual no-reparse relative opens and held-handle basic info, not File.GetAttributes on a DOS name. Only ERROR_DIRECTORY267 from the directory-kind open permits regular-file retry, not denied/busy/reparse fallback. Failure resolving a usable stable local namespace refuses rather than enumerating a different tree. Existing one enumerator OR one active file/transient chain, depth/count/metadata limits and optimistic version checks remain; no full-directory buffer/member-handle retention or source CAS.

An internal per-run namespace seam substitutes the actual enumeration path with another GUID-owned real directory containing only a subset of common names. Relative opens and real directory identity checks remain production native operations. Root and nested substitution must return SourceChanged, zero commits, no final/partial. **No SUBST, DefineDosDevice, registry or real drive mapping modification was executed or claimed.** This is simulated namespace resolution with real identity enforcement, not a live remapping exploit or filesystem snapshot guarantee.

One bounded directed review additionally found collision checks still reopened outputRoot through a mutable DOS mapping after the output bridge had already held its actual namespace. Added a synthetic output-resolution seam with real output/source hardlink identity and approved confirmation: old code bypassed collision admission (selected1/run1/pass0/skip0/fail1,exit1); repaired code uses `ArchiveOutputSession.CheckCreationCollisions` over the actual held output drive/ancestor chain and relative final name, returning OutputCollision/zero commits with both source/target bytes retained. Standalone source fixtures may retain path-only admission because they cannot publish; Runner always passes the actual output session. No actual system mapping test or successful live remap exploit is claimed. This closes the output side of the same namespace binding, not a new overwrite contract.

The same review identified premature generic invalid_request for long create name/password and a count-race category. New ErrorCompatibility red failed expected invalid_component/actual invalid_request (selected1/run1/pass0/skip0/fail1,exit1); restored invalid_component/invalid_create_request and split count mutation into request_changed. Deterministic custom-count/list/array cases pass. Directed new13/run13/pass13/skip0/fail0,exit0; no second review.

## Actual red to green

All mutations below are at the real Start accepted callback before async bridge/source acquisition; synthetic files/helper are GUID-owned, existing fixed helper refused.

| Regression | Before production fix | Repaired directed evidence |
| --- | --- | --- |
| Replace source list element | selected1/run1/pass0/skip0/fail1/not_run0, exit1; independent ZIP names changed | fixed original approved source names/bytes |
| Add source list element | same failing counts/exit1; extra unapproved source | original source only |
| Clear source list | same failing counts/exit1; worker_failure instead of accepted source operation | original source only |
| Change ExcludePatterns after accepted | selected1/run1/pass1/skip0/fail0/not_run0, exit0; already serialized before accepted | retained defense-in-depth coverage with private owned snapshot |
| Change native-consumed source/output budgets | selected1/run1/pass0/skip0/fail1/not_run0, exit1; resource_limit after changed caller budget | source and output consume approved immutable values |
| Replace target + update original confirmation | same failing counts/exit1; success overwrote new unconfirmed target | target_changed/FailedBeforeCommit/commits0, replacement bytes retained; add/remove authorization scenarios retained |
| Old extract/Gzip/XZ shared confirmation | same failing counts/exit1 at extract/success | all three paths refuse changed target using fixed accepted identity |
| Safe parent output | selected1/run1/pass0/skip0/fail1/not_run0, exit1; worker_failure | success incl confirmed overwrite, independent exact manifest/bytes |
| Namespace substitution | selected1/run1/pass0/skip0/fail1/not_run0, exit1; omitted source members published | real identity check rejects root/nested mismatch; no output |

Before parent repair: admission8/8 green, self-inclusion guard1/1 green, parent1/1 red. After namespace repair: original7 small creation+new11 selected18/run18/pass18/skip0/fail0/not_run0, exit0. Additional capture List/array-change checks then new11/run11/pass11/skip0/fail0, exit0. A new assertion originally expected null state for no-authority existing-target failure; actual established state is FailedBeforeCommit, so only that new test expectation was corrected. No original test was removed/relaxed.

## Verification ledger

Every Rust command inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; one Cargo channel. Initial background wrapper disappeared without END/exit status; no Cargo/rustc/worker survived inspection. Its partial log is **incomplete, not PASS**. Re-ran the entire batch synchronously with1800s timeout, no duplicate/surviving build. Current actual logs `remediation-verified-*` in session scratch:

| Command | This-turn result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast |82passed/0failed/0ignored, exit0; all result lines summed |
| cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --offline |exit0 |
| cargo clippy -p nxtools-archive --features isolated-codecs --tests --offline -- -D warnings |exit0 |
| cargo fmt -p nxtools-archive -- --check / cargo fmt --all -- --check |both exit0 |
| cargo check --workspace --tests --offline |exit0; NOT workspace tests |
| cargo test -p nxtools-extensions --test archive_import --offline |7pass/0fail/0ignored,exit0 |
| cargo test -p nxtools-clipboard --lib archive_ --offline |5pass/0fail/0ignored,39filtered,exit0 |
| C# test project build --no-restore |0warnings/0errors,exit0 after code changes |
| Exact C# original62+new13 |final list selected75/run0;execution selected75/run75/pass75/skip0/fail0/not_run0,exit0 |
| Actual4GiB+17 ZIP64 fixture |included and PASS in final75:FastDeflate,real Job/worker/native publish,independent Python streaming exact size/SHA256;not inherited |
| Documentation/context/selector/hygiene |post-document spec-contracts101docs/14owners,catalog79pages/79rows/78locked,context30implement/27check and fixture-selector gates all exit0;scoped git diff--check exit0,existing Program.cs CRLF advisory only |

Independent tool: CPython3.13.2 zipfile and existing7-Zip **26.00**, `D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`, SHA256 `4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`. Small AES create oracle actually passed at four semantic levels through existing script, password stdin only, not argv. This is not the historical24.09 binary/provenance. No download/install/PATH mutation. Expected payloads originate from synthetic literals, not output metadata.

Final logs: `remediation-final-native-{list,tests}.log`; prior pre-review73/73 integration remains actual evidence,not the final source result. The final75 includes both review regressions and original62 unchanged. Final inspection found0Cargo/rustc/worker/test-host/7z processes,0owned admission/source-binding/ZIP-create roots,fixed helper absent. Python executable SHA256 `7d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1`. No owned partial/spool remained in cleaned fixture roots;anonymous spools close with worker cleanup.

## Remaining scope

The integrated green remediation checkpoint is saved before any new creation tracer. Next approved Phase A scope remains7z/TAR/TGZ/TXZ creation, then remaining native attributes/MOTW/source-delete eligibility, product IPC/package mapping/backpressure and broad resource acceptance. No new-format implementation was mixed into this repair. ZIP64large-count/RSS/performance/long-path/UNC/storage/crash/full workspace tests/default C#suite/UI/native-user/release are not established here. Phase A remains incomplete; no Phase B advancement.

Final scope/resource checks: only the six archive native production files, two new regression files plus harness registration/effect classification, and task/owner evidence were edited this turn. No original62test body changed. No new DEBUG diagnostics or embedded edit commands;context files remain below32768bytes. Source/static gates certify only their documented scope,not broader native/product/Phase A acceptance.
