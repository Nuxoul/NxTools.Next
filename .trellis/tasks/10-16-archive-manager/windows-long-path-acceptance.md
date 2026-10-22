# Windows local long paths — independent acceptance

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains incomplete.
Reviewed [contract](windows-long-path-contract.md), [implementation evidence](windows-long-path-evidence.md),
[result ledger](windows-long-path-results.json) and [reference sources](research/windows-long-path-sources.md).

## Verdict

**PASS for the tested local ordinary-drive Windows ZIP long-path slice. No concrete in-scope
product/test defect remains from this independent review and execution.** Existing production
implementation passed; no production fix/red-to-green is claimed. This is not complete ENG08,
SAFE01/03/05/06/07, PhaseA/A–F, UNC, all-format, UI, native-user or deployment acceptance.

Acceptance modified no implementation/tests or submitted historical reports/results. Added this
report and scratch verification scripts/logs only; no optional reviewer needed. No100k/1GiB/large
ZIP64/performance sampling, formalDesktop/installedhost, Release/package/install/sign/commit/push,
source-recycle/delete API or systempolicy/LongPathsEnabled/registry/drive-map change.

## Independent source and boundary review

Read reference ledger, actual C#ArchiveLongPathTests/independentZIPoracle helper, Rust
product_read_chain long-source case/Fixture ownership, shared verification and native creation
contracts. Prior reference source inspection is source evidence, not reference long-path runtime.

- Public paths stay ordinary local DOS paths; internal volume-GUID namespace derives from held
  C#handles. Oracle-only extended path is supplied solely to read-only external CPython after
  publication, not admitted as public source/output/member authority.
- C#tests run original runner/Job/worker/source/output sessions. Rustlongsource runs original
  ArchiveReadClient/typedpipe/productionArchiveReadSession/originalrunner and actualworker.
- Original same-connection open/test metadata, literalnames/ordinals/sizes/encoding, real snapshot,
  owner=remote terminals, sourceunchanged/writable, exacthelperrelease and normalhostexit are
  asserted. Test teardown retires the original endpoint after realoperations; no fake replacement
  product path. A failed teardown cannot write the retained passed evidence.
- Native extraction layout checks fullfiles/directories/literalbytes/emptyfile/emptydir/DOSmtime.
  Creation uses real held-ID enumeration, partialreadback and independentPythonZIPverification.
- Both8case publication matrices reach original barriers. native_published asserts committedcount1
  immediately; terminalcount/state agrees. Precommit failures retain old/externaltarget; cancellation
  duringcommit and postcommitverificationfailure retain actualpublishedoutput/count. Injected
  native5 is not an observed systemdenial. Approved targetrecheck-close->publish TOCTOU unchanged.
- Unsafe8members still require exact private UnsafePath,0commits,emptydestination and outside
  sentinelunchanged. Publicsource/output4internalprefixes refuse. No long-path policy relaxation.
- Diagnostic assertions compare correlated category/native/count/state/elapsed and boundedrecords,
  including unescaped fieldvalues to avoid JSONescaping masking payload/path leaks.

## Actual independently rerun checks

C#isolated apphost built FIRST, then matching isolated-codecs worker. Every direct Cargo command
inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; lane checked before
commands, workerexecution/relink serial. Foreground1800s verification completed, no lostbackground
result. No unrelated process killed, no production code or dependency changed.

| Independent gate | Actual result |
| --- | --- |
| C#Debug testapphost build |exit0,0warnings/0errors |
| Matching isolated-codecs actualworker build |exit0 |
| Exact23C#LIST |selected23/run0,exit0 |
| Same23C#RUN |selected23/run23/pass22/skip1/fail0/not_run0,exit0 |
| Only skippedcase supplement, exactLIST->RUN |selected1/run1/pass1/skip0/fail0/not_run0,exit0 |
| C#unique PASScoverage |all23names, including all4new long-path fixtures; NOT one integrated23/23PASS |
| Affected Archive9integrationgroups |62pass/0fail/0ignored,1existing100ktest explicitly filtered,exit0 |
| product_read_chain within affectedtests |22pass, including actual long-source open/test/cleanup |
| Protocol/IPC lib |53/30pass,0fail/ignored,exit0 |
| Extensions archive_import /Clipboard archive_ |7/5pass,0fail/ignored,exit0 |
| Strict Clippy Archive tests/examples isolated/default |both exit0, -Dwarnings |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0, compilation ONLY, not workspace tests |
| Docs/catalog/context/selector/diff |all exit0 |
| Independent offline preserved-file/source verifier |470checks PASS/exit0 |

The one initialskip was `archive overwrite fixture actual worker integrity and commit lifecycle`:
acceptance script omitted NXTOOLS_ARCHIVE_CODEC_FIXTURES. No productfailure or PASS inferred from
exit0. Set explicit existingupstreamfixturepath, rebuilt matchingworker after feature-unifying
checks, LISTed and executed ONLY thatcase,1/1PASS. Originalskip retained. No unnecessary full-suite
repeat to manufacture an integratedPASS. LISTnames==RUNnames; PASSnames+supplement equal exact23.

Spec106documents/14owners, catalog79pages/79rows/78locked, context77implement/74checkentries;
fixture/native-selection sourcegatePASS. Existing native-contracts.md35497bytes>32768warning remains.
Main reference32654bytes within injectionlimit; no unrelated documentation restructuring.

## Fresh retained long-path facts and offline verification

Independent new roots in this acceptance session scratch:
- layout: archive-long-path-ac06765f-3c91-43d9-9d87-8dabb004984d
- extractionpublication: archive-long-path-492b3cdc-566d-4f5d-ac32-c925149d3ccf
- creationpublication: archive-long-path-432021ec-f70c-46e8-90cd-06743b7e562b
- unsafe: archive-long-path-26b5ec5f-4e0a-4a13-bc8a-3f5ea4c5cd0f
- product: archive-product-chain-03d4f449-a5ff-4839-9b5b-da39a8462d2e

All retained passedrecords actualcleanuptrue. Source446UTF16/512UTF8bytes on realproductopen/test;
creationfinal436–447UTF16/sourcepayload766; extractionfinal771–782UTF16 (layout767). ASCII/CJKBMP,
legal47unit nestedcomponents, ordinarydrivepaths. No universalWindowsmaxpath/depth claim.

Offline own checker directly inspected newpublicationtargets: successful/committedcancel/postcheck
bytes match expectedpayload or exact10memberZIP/fullcontent; nonpublished oldtarget or externalchanged
bytes remain. Exactlyoneexternal.nxpart-external per case retains its bytes; no ownedpartials. ZIP
members/kinds/Store-directory vsDeflate-file methods match independentexpecteddata. Layoutpayload/
emptyfile/emptydir and unsafeemptydestination/outside sentinel checked. Runtimefixture independently
checks complete directorysets and mtime; offline evidence is not a replacement for runtimeassertions.

Rehashed all186submitted rawfiles across17roots, including failedattempts: unchanged. Opening
baseline comparison independently verified67production/archiveprotocol/resource-toolsourcefiles and
85historicaltaskfiles unchanged; acceptance's current sourcebaseline also unchanged. Newrawfilehashes
saved in offline results. Rehashed acceptednew100k and oldFailedattempt/samplepairs unchanged.

One scratch offlinechecker initially looked at outer create-mode directory instead of its nested
longdirectory. Corrected exactunique-final lookup; no product rerun, file repair or changedexpected
outcome. Final470checks passed. Initialcheck failure is retained as a script incident, not a product
longpath defect. Implementer's test-budget/oracle/teardown failures remain as documented originals,
not falsely presented as productionred->green. Its stoppedoptionalreview is not a reviewPASS.

## Evidence / stop

Acceptance scratch:
C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df.
- long-path-independent-verify.ps1, commandledger/logs, exactnativeLIST/RUN and supplementlogs.
- long-path-independent-offline.py /long-path-independent-offline-results.json(470PASS), SHA256
  `17eb314b0dacfcbfaac193a4a04113f9c8af095e7dd474caa7e72d6d863f8969`.
- commandledgerSHA256 `23a82249f9545efa2b134e041bd1ed1a46c72d77cd177aacb65503d33ea6af65`.
- nativeRUNlogSHA256 `98bc5ffae5ed705c3e45ea631f95b25931a21b01baaef3d03207dae295cc976f`;
  supplementSHA256 `95e1ce692672c2b11e264579812e95f54491efbc9bd3e76549db7e32269ac04a`.

Closing observed0archivefixtureprocesses,0product/resourcepipes, fixedtesthelperabsent. All new
longpathinputs/outputs/evidence remain retained; no unknownrecursivecleanup. Production remained
unchanged; no newperformance/tooloptimization/nativeRecyclework. NativeRecycle remains
native_protection_unproven/NotAttempted. **This bounded long-path slice is accepted and stops here;
broaderPhaseA/ENG08/A–F/native/UI/deployment gates remain incomplete.**
