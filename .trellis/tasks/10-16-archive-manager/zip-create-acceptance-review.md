# Independent acceptance — bounded general traversal / ZIP creation

## Verdict

**NOT ACCEPTED as a closed creation slice yet.** Real bounded ZIP Store/Deflate/AES256 creation, read-back validation and the real4GiB+17 ZIP64 test pass; however three concrete admission/compatibility defects were independently reproduced. An additional namespace-binding risk was source-confirmed but not live-exercised. Phase A/A–F remain incomplete. No product code changed by this review.

## Verification independently executed

No initial Cargo/rustc contention. Every Cargo inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; commands serial,full-operation timeout,no timeout/duplicate build. Disk inspected before large test:C34.39GiB/D63.24GiB free.

| Check | Actual review result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast |82passed/0failed/0ignored,exit0;complete log independently summed |
| Same-feature worker build |exit0 |
| Strict archive Clippy --tests |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0,not workspace tests |
| Extensions archive_import / clipboard --lib archive_ |7/7 and5/5PASS,39clipboard filtered |
| C# build --no-restore |0warnings/0errors,exit0 |
| Exact prior54+new8 harness cases |list selected62/run0;execution62/run62/pass62/skip0/fail0/not_run0,exit0 |
| Actual ZIP64 large fixture |Included and PASS:4GiB+17bytes,FastDeflate,actual Job/worker/native publish and independent Python streaming size/SHA256 oracle |
| AES independent create oracle |PASS with existing local7-Zip26.00,not the report's24.09 (see below) |
| spec-contracts/catalog/task-context/fixture-selector |PASS101docs/14owners;79pages/79rows/78locked;28implement/25check,no truncation;selector source gatePASS |

Independent oracle environment configured explicitly. The exact reported24.09 binary was unavailable in this review session; used existing `D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`,version26.00,SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f,with its existing7z.dll. No download/install/PATH change. Passing oracle therefore establishes independent26.00 reading,not reproduction of the report's exact tool build or its official provenance.

No full/default C#suite,full workspace tests/UI/Release/install/system-state mutation performed. After tests and probes: no Cargo/rustc/archive-worker/test-host/7z left,fixed test helper absent,0archive-zip-create roots and0review roots. Creation count limits<=32roots/65536records and lack of ZIP64large-count/RSS/performance/long-path/UNC validation are honestly documented limitations,not surprising review defects.

## Concrete findings — actual worker probes

Scratch-only C# project `zip-create-review-probe/` references the built PlatformHost using its existing internal test friend name. It operates GUID temporary roots,actual built Rust worker,actual native output and .NET independent ZIP reads. Accepted callback is the deterministic admission→asynchronous-execution interleaving; no product hooks/registration/source changes. Probe records outcomes (exit0 is probe completion,not semantic acceptance).

### P1 — SourcePaths collection is not snapshotted at admission

`csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs:100,140,189-204,398` validates the caller's IReadOnlyList then retains the original request. IReadOnlyList does not make the underlying List/array immutable. Native acquisition rereads it asynchronously.

Probe: request.Path=A,SourcePaths List[A]; in accepted callback change element0 to B,then allow worker task to start. Observed:

```text
Category=success,FilesCommitted=1,CommitState=Committed,CleanupComplete=true
archive entry=replacement.txt
decoded content=unconfirmed-source (B),not approved-source(A)
```

Thus accepted root identity/path binding can change after validation; `SourcePaths[0]==request.Path` held at check but is false at actual acquisition. This is an internal caller lifetime/race defect,not a claim that untrusted code can already call the internal interface. Required: create one bounded immutable request/admission snapshot BEFORE observable acceptance and scheduling; validate/serialize/store/acquire that same snapshot. Include nested collections/options and mutable limit representations consumed later as appropriate. Add mutation-after-accept/before-acquire real-worker regression,including changed list count/first-root binding. Do not add another late equality check as the sole lifetime solution.

### P1 — ConfirmedOverwrites can be changed after acceptance

`ArchiveWorkerRunner.cs:105,140,238-240`; `ArchiveWorkerOutputBridge.cs:38-39` validates the dictionary before scheduling but copies it only on the asynchronous worker task. This leaves an interval in which the accepted overwrite confirmation is mutable.

Probe: capture old target identity into Dictionary,Start accepts; accepted callback replaces target with new-unconfirmed-target and updates original dictionary to its new identity. Observed success/Committed1 and replacement of that new target with created ZIP. The new object was not confirmed in the accepted request. This is **not** the expressly accepted target-recheck-close→publish TOCTOU: authorization itself changed before it was frozen.

Required: immutable confirmed identity snapshot before acceptance; same snapshot throughout output bridge construction and commit. Add acceptance→bridge barrier regression: mutating caller dictionary cannot update confirmation, and replacement target must be refused/preserved. Preserve legitimate approved atomic-path semantics and current ownership cleanup; no need to reopen native overwrite primitive.

### P2 — Safe source-child / sibling-output layout is rejected

`ArchiveCreationSources.cs:632-634` rejects BOTH output-at/below-source and source-at/below-output. Contract§18 only calls for rejecting output inside an input tree/self inclusion.

Probe: source=<owned>/input containing ordinary files; outputRoot=<owned>;createName=created.zip. Final archive is outside input,so no self inclusion. Observed worker_failure/OutputCollision,FilesCommitted0,no archive. This common product action (compress a directory next to itself) is broken.

Required: reject the dangerous direction only,plus actual identity/reparse/self-file guards. Keep output-inside-source/equal-source root/reparse/alias tests; add directory source below output parent with sibling final as a successful real-worker/independent-reader case. Do not simply remove all collision checks.

## Additional source-confirmed namespace risk — no live drive remap test

`ArchiveCreationSources.cs:435-445,535-546,319-327`: held root/lineage handles anchor child opens and identity checks,but enumeration and initial child attributes use reconstructed `\\?\<drive-letter>\...` paths. DOS/SUBST mapping is a separate mutable namespace,not necessarily pinned by those handles. If mapping changes to another tree with a subset of same child names,enumeration could omit original members while relative child reads/final checks still refer to unchanged original handles. Final SourceCheck verifies retained object/version,not equivalence of the enumerated namespace to the held directory.

One bounded read-only reviewer reported this; parent checked those exact paths and control flow. **No SUBST/DefineDosDevice/system mapping was modified or runtime exploit demonstrated.** Fix namespace binding by deriving a stable path from the held directory (with real handle identity verification) or enumerating the held handle; if intentionally unsupported drive mappings,detect/reject them consistently before reads. Add a synthetic mapping seam or separately authorized owned-drive fixture; don't claim live remap coverage from static inspection. This does not demand an atomic filesystem/source-set snapshot beyond the documented optimistic consistency policy.

## Confirmed strengths / continuation

Rust SourcePlan semantics,bounded one-file source reads,public sequential ZIP writer,source hashes and same-held-partial read-back are real. Created output is checked through original central/local/descriptor/extents and per-entry size/CRC/AES/SHA before final source validation and native commit. Large ZIP64 test actually executed successfully. Retain these tests and implementation; don't replace the design or weaken validated ZIP readers.

Repair the three reproduced findings,close the namespace-binding hole with precise evidence,then rerun original62 plus new cases. Not necessary to repeat4GiB encoding for every local collection fix; one final integrated run can include it when relevant and resources permit. Keep phase names/order; after creation slice acceptance continue7z/TAR/TGZ/TXZ and remaining native/product/resource work. Do not mark Phase A complete from82/62 counts.

Review artifacts in scratch: zip-create-acceptance-{archive,worker,clippy,fmt,workspace,extensions,clipboard,native-list,native-tests}.log,zip-create-review-probe/. No source edits,release/package,registry/service,user archive,commit/push performed. One bounded directed source review only; findings verified independently as specified,not an iterative broad review cycle.
