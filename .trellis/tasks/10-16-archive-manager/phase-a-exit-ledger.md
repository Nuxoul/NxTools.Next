# Phase A exit ledger

2026-10-20. **Phase A — Engine contracts and safety foundation: NOT PASSED.**
[Canonical exit](implement.md#phase-a--engine-contracts-and-safety-foundation) requires
ENG01–ENG09, SAFE01–SAFE08, P201 and IPC01 for touched payloads. This ledger maps subconditions;
it does not amend those IDs, safety contracts, first-release A–F scope or phase order, and does
not authorize Phase B. **One next implementation slice is selected, not implemented.**

## Key and independent evidence

**A** implemented + applicable independent PASS; **B** confirmed missing behavior/defect;
**C** implementation exists but a specific necessary assertion is missing; **D** contract or
authorization decision; **E** original later-stage/native-user/Release work.
Evidence levels: **S** source; **T** compiled domain/protocol tests; **R** small actual worker/
native/typed chain; **L** authorized large input/resource observation; **U/Rel** UI/live-user/
Release. S/T/R/L never imply U/Rel. All runtime evidence here is reused, not rerun this turn.

| Ref | Independent report and accepted scope |
| --- | --- |
| Z | [ZIP read/AES PASS](zip-aes-acceptance-review.md#sourcecoverage-checks): plain0/8/9/12/14/93/95, ZipCrypto; AE1/AE2 all strengths Store/Deflate and AE2AES256 extra-method samples; directory integrity/comment-budget repairs. T/R. |
| X | [XZ/Zstd PASS](xz-zstd-acceptance-review.md#source-and-contract-inspection): bounded standalone/compound reads, including earlier Gzip/BZip2/TAR native regressions. T/R. XZ BCJ is not 7z BCJ evidence. |
| K | [Codec/source/Job/encrypted-output repairs](acceptance-remediation.md#1-stable-source-identity--repaired-at-the-real-workernative-seam), later independently exercised in [Z](zip-aes-acceptance-review.md#independent-verification-actually-executed) and [TR](tar-time-acceptance.md#independently-executed-verification) unions: RAR4/5 and7z plain/encrypted, exact oracles and source/budget protections. T/R. |
| CR | [Tree creation/preparation acceptance](tree-create-acceptance-review.md#independently-executed-verification) + subsequent [7z verifier typed-I/O PASS](sevenz-create-typed-io-acceptance.md#verdict): required creation formats, full readback, independent roundtrips; invalid internal-output side effect closed. T/R. |
| L64 | [Independent4GiB+17 FastDeflate ZIP64](tar-time-acceptance.md#independently-executed-verification): native publication and independent streaming size/hash. L; not all formats/methods×large. |
| O | [Approved overwrite review](overwrite-acceptance-review.md#confirmed-delivered-behavior), [outcome repair](overwrite-acceptance-remediation.md), later independent TR/LP: native publication, true counts/errors/cleanup. T/R. Strict-CAS FAIL32 remains historical. |
| M/TR | [Metadata/MOTW acceptance](extraction-metadata-acceptance.md#independently-rerun-integration) + [TAR-time P2 PASS](tar-time-acceptance.md#verdict): mtime/readonly/hidden/fixed source mark, directory ownership and guarded cleanup. T/R. ADS is a captured snapshot, not a lifetime ADS lock. |
| SD | [Source-delete P1/P2 PASS](source-deletion-remediation-acceptance.md#verdict): original Run/lifecycle authority, once-only qualification and recording backend. T/R; NOT real recycling or directory coverage. |
| P | [Product read P2 PASS](product-read-p2-acceptance.md#verdict): original-generation typed read/cancel/credit/Drop/full-queue/EOF cleanup and helper trust. T/R. |
| H | [Finite100k PASS](product-100k-post-status-acceptance.md#verdict):100000 exact members/391 batches, Debug required-PID samples, bounded quotas and original-connection postcleanup reuse. R/L; first batch≠first paint, sampled≠absolute peak. |
| LP | [Local ZIP long-path PASS](windows-long-path-acceptance.md#verdict): source446/final767–782 UTF16; actual product open/test, native extract/create/publication/unsafe-member cases. R; excludes trusted UNC/all-format matrix. |
| Q | [Worker cancellation PASS](worker-cancel-acceptance.md#verdict): actual receive/observe/reply/credit/pause and cleanup-held resources/slots. T/R; not arbitrary blocked-call interruption or full ENG09. |

## Exit ID ledger

Production path is [ArchiveEngine](../../../rust/crates/nxtools-archive/src/engine.rs) / original
[worker](../../../rust/crates/nxtools-archive/src/worker.rs) and isolated codecs, under the original
[C# runner](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs) and held
source/output sessions. Product open/test uses ArchiveReadClient→typed IPC→production
ArchiveReadSession→that runner. No fixture execution is substituted for UI/Release acceptance.

Test navigation: Rust [engine](../../../rust/crates/nxtools-archive/tests/engine_contract.rs),
[crypto](../../../rust/crates/nxtools-archive/tests/zip_crypto_contract.rs),
[methods](../../../rust/crates/nxtools-archive/tests/zip_methods_contract.rs),
[plans](../../../rust/crates/nxtools-archive/tests/extraction_plan_contract.rs),
[interaction](../../../rust/crates/nxtools-archive/tests/interaction_contract.rs),
[wire](../../../rust/crates/nxtools-archive/tests/worker_wire.rs);
C# [ZIP create](../../../csharp/NxTools.PlatformHost.Tests/ArchiveZipCreationTests.cs),
[tree create](../../../csharp/NxTools.PlatformHost.Tests/ArchiveTreeCreationTests.cs),
[codec](../../../csharp/NxTools.PlatformHost.Tests/ArchiveCodecWorkerTests.cs),
[metadata](../../../csharp/NxTools.PlatformHost.Tests/ArchiveMetadataTests.cs),
[overwrite](../../../csharp/NxTools.PlatformHost.Tests/ArchiveOverwriteTests.cs).
Exact existing names below locate relevant assertions; refs supply independent execution evidence.

| Original ID / necessary subconditions | Production entry / exact existing tests | Current conclusion and specific remainder |
| --- | --- | --- |
| **ENG01** required writable formats, relative Unicode/empty files/dirs, levels, streaming and independent roundtrip | create_zip/7z/tar/tgz/txz/gzip/xz; `ArchiveZipCreationTests.DirectoryLevelsAndRoots/AesCreationInterop/LargeZip64Streaming`; `ArchiveTreeCreationTests.SevenzOptions/TarPlain/TarGzip/TarXz` plus its `ReadProduced`; `ArchiveStreamWorkerTests.ActualGzipCreation/ActualXzCreation` | **A T/R**,CR; multichunk128/256KiB inputs; **A L** ZIP,L64. **C C1** empty single-source Gzip/XZ assertion. TAR does not acquire a payload checksum. No new multi-GiB sample per codec. |
| **ENG02** ZIP methods/crypto and7z coder/solid/AES/header variants | `zip_methods_contract.independent_zip_lzma_without_eos_uses_known_size_and_consumes_exact_input`; `zip_crypto_contract.independent_aes_ae1_ae2_all_strengths_require_authentication_and_exact_bytes/independent_encrypted_method_combinations_zip64_descriptors_and_extra_failures`; `ArchiveCodecWorkerTests.IndependentRarAndSevenzReadTest` | **A T/R**,Z,K. **C C2** required7z BCJ actual coder/native bytes. Not every AES×method×size combination; typed rejection cannot replace required BCJ. |
| **ENG03** typed password/auth/data errors, secrets, no invalid publication; retry preserves options/selection | `ArchiveCodecWorkerTests.PasswordBudgetCorruptionAndCancel`; `ArchiveExtractionWorkerTests.ActualSevenzToHandleSession/ActualRarToHandleSession`; crypto tests | **A T/R**,Z,K for errors/integrity. **B B2** operation-integrated retry. Standalone `interaction_contract.generation_scoped_password_answer_resumes_once_and_old_ticket_cannot_apply` is not real retry. Public password UI **E D/JOB03**. |
| **ENG04** signature/compound/ZIP aliases/random; signed containers read-only; no inflate-to-size | `ArchiveEngine::open`; `stream_contract.independent_gzip_streams_list_unknown_size_and_verify_all_members_before_publish/independent_tar_gzip_checks_outer_integrity_before_any_inner_output`; XZ/Zstd tests | **A T/R**,X for actual formats/compound integrity. **C C3** explicit disguisedZIP/JAR/APK/random assertions. Signed-container editing refusal is **E Phase E**, not an A editing implementation. |
| **ENG05** Unicode authority, manual pages, detectable/ambiguous Auto and changed-decoding safety | `encoding::decode`; `engine_contract.manual_code_pages_and_authoritative_unicode_keep_their_provenance/shift_jis_trail_byte_is_not_a_directory_separator`; `ExtractionPlan::build` | **A T/R**,Z for Unicode/GBK/ShiftJIS/CP437/uncertain fallback. **B B1**,**C C4** below. Menu/browser state **E Phase C**. |
| **ENG06** optional/unknown metadata, solid shared packed size, truthful progress | `worker_codecs::run_sevenz`; `ArchiveCodecWorkerTests.IndependentRarAndSevenzReadTest`; `engine_contract.declared_budget_rejects_before_decode_and_terminal_progress_is_exact`; stream unknown-size tests | **A T/R** stream unknown/progress,X,K. **C C5** exact solid packed-null/optional-field assertions. |
| **ENG07** stale revision/object/ordinals refused | `engine_contract.old_snapshot_cannot_address_a_replaced_source`; `ArchiveRemediationTests.SourceRaceBarriers/MappedSourceRace`; `ArchiveSourceBindingTests.NamespaceReplacement/HeldOutputNamespace` | **A T/R**,K,CR,SD. No demonstrated current source-binding defect. UI last-good/reload **E C/LIFE06**. |
| **ENG08**100k metadata/resources/reuse;>4GiB ZIP64;legal long paths;firstpaint/target | Original product read,H; `ArchiveZipCreationTests.LargeZip64Streaming`; `ArchiveLongPathTests.ExtractionLayout/ExtractionPublication/CreationPublication/UnsafeMembers`; product long-source test | **A R/L**,H+L64+LP for the named conditions. Firstbatch is not firstpaint; Debug is not Release1.5s acceptance. **D D1** dependency/measurement disposition;trustedUNC **D D3**. No repeat accepted matrices. |
| **ENG09** cooperative chunk/reply/credit/pause, cleanup/busy;promptwait/finalization | `ArchiveCancellationBoundaryTests.StreamingReceiveObserveCleanup`; `worker_cancel_contract.actual_worker_credit_wait_records_received_then_observed_without_an_ack`; `control::cancellation_tests.paused_checkpoint_is_really_waiting_then_cancel_wakes_without_resume`; product explicit credit cancel/readmission; `ArchiveOverwriteTests.PostCommitAndCancellation/ActualWorkerOverwriteLifecycle` | **A T/R**,Q+O+M/TR; `ArchiveWorkerTests.StuckAdmissionRetainsSlots`/`ArchiveRemediationTests.NativeJobLimitAndCleanup` cover retained busy/finite strategy. **B B2** actual operation prompt wait. Standalone `interaction_contract.cancellation_wakes_prompt_wait_and_busy_prompt_never_overwrites_current` passes but is not operation integration. Original1GiB wording **D D1**,not universalcodec200ms. |
| **SAFE01** unsafe member classes visible/refused, no escape/sanitization | `engine_contract.unsafe_names_are_visible_and_never_sanitized_into_valid_outputs`; `extraction_plan_contract.unsafe_and_colliding_archives_never_get_a_writable_plan`; `ArchiveOutputTests.NestedAndUnsafe`; LP UnsafeMembers | **A T/R**,Z,K,LP for traversal/root/drive/UNC/device/ADS/control/reserved/trailing ambiguity. Unsafe UNC member rejection≠trusted UNC source support. |
| **SAFE02** duplicates/case/file-parent/normalization and changed-decoding collisions | `engine_contract.duplicates_case_and_file_directory_conflicts_remain_visible`; `creation_plan_contract.public_plan_rejects_case_names_unsafe_components_invalid_globs_and_finite_limits` | **A T/R**,Z,CR. **C C4** reprojection collision/no-write. Design§4.1 makes NFC opt-in,not an invented default:if exposed later,validate its collisions before writes. |
| **SAFE03** real no-reparse/parent swap/input loop | `ArchiveOutputTests.ConcurrentTargetAndReparse`; `ArchiveOverwriteTests.ChangesBeforeRecheck`; `ArchiveZipCreationTests.NativeSourceLimitsAndReparse`; source-binding namespace tests | **A T/R**,O,CR,LP. Approved output recheck-close→publish TOCTOU remains;not universal filesystem/powerloss safety or sourceRecycle authority. |
| **SAFE04** entry/model/actual bytes/dictionary/Job limits;benign risk escalation | `engine_contract.finite_metadata_and_actual_stream_budgets_are_enforced`; `zip_boundary_contract.eocd_comment_raw_lossy_and_central_share_one_metadata_reservation`; `ArchiveRemediationTests.BudgetAdmission/SevenzWorkingBudget/SevenzHeaderClassification/NativeJobLimitAndCleanup` | **A T/R/L** finite limits,K,Z,H. **B B3** ratio warning/operation finite approval. Five limit scalars or standalone Risk prompt do not implement it;100kv2 is not general risk approval. |
| **SAFE05** CREATE_NEW partial/full integrity/flush/precommit old-target and ownership | `transaction_contract.output_failure_aborts_owned_partial_and_cancel_before_open_has_no_effects`; `ArchiveOverwriteTests.FailuresAndNativeOccupation/ExclusivePartialOwnership`; `ArchiveWorkerOutcomeTests.ActualTargetFailures`; LP/Q | **A T/R**,O,Z,CR,LP,Q. No generic “broader safety” block or hardware/powerloss guarantee. |
| **SAFE06** overwrite/skip/autorename/ask-all plus target recheck | `ArchiveOutputTests.RecheckedAtomicPathOverwrite`; `ArchiveOverwriteTests.ChangesBeforeRecheck`; `ArchiveWorkerOutcomeTests.ActualTargetFailures`; standalone conflict-ticket test | **A T/R** overwrite/no-replace/native errors,O,LP. **B B2** real per-job skip/autorename/apply-all/re-prompt;enum/preconfirmed map alone insufficient. Public dialog **E Phase D**. |
| **SAFE07** pre/in/postcommit cancel,unknown truth,owned cleanup | `ArchiveOverwriteTests.PostCommitAndCancellation/ExclusivePartialOwnership/ActualWorkerOverwriteLifecycle`; `ArchiveWorkerOutcomeTests.ActualOperationIsolation`; `ArchiveMetadataTests.GuardedMetadataCleanup`; Q | **A T/R**,O,M/TR,LP,Q. Published/unknown/replacements/external edits retained;optional cleanup only unchanged owned NEW outputs. No full matrix repeat. |
| **SAFE08** self-inclusion;eligible full success/test/consent;original object/once-only/per-item result | `ArchiveSourceBindingTests.SelfInclusion/MissingInternalOutput/MissingInternalMultiAndIdentity`; `ArchiveSourceDeletionTests.OriginalSessionRejectsChangedEvidence/RetiredEvidenceCannotBindReplacement`; `source_deletion_contract.late_extraction_finish_cannot_restore_facts_invalidated_by_reentrant_test` | **A T/R** foundation/refusals/recording outcomes,CR,SD. **D D2**,NOT realRecyclePASS:backend absent,directory/exclusion coverage refuses. Delete integration **E D/JOB07**,but A exit names SAFE08;dependency cannot silently disappear. |
| **P201** RAR4/5 independent plain/encrypted/corrupt/read-only/license | `ArchiveCodecWorkerTests.IndependentRarAndSevenzReadTest/PasswordBudgetCorruptionAndCancel`; `ArchiveExtractionWorkerTests.ActualRarToHandleSession`;[attributed inputs/hashes](../../../rust/crates/nxtools-archive/tests/fixtures/upstream/README.md) | **A T/R**,K and independent Z/TR/SD unions. No demonstrated current core defect. Volumes **E Phase E/P202**;RAR creation excluded. |
| **IPC01** touched worker/native/create/metadata/eligibility/cancel and product readv1/v2 payloads | `worker_wire.private_frames_are_bounded_strict_and_preserve_operation_identity/creation_fields_are_opt_in_schema2_and_roundtrip_without_changing_legacy_requests/read_v2_requires_exact_transport_contract_and_never_grants_write_authority`; `metadata_wire_contract.actual_worker_preserves_old_schema2_native_actions_and_explicit_metadata_negotiation`; `ArchiveAdmissionTests.AdmissionBounds/ErrorCompatibility`; `ArchiveProductReadTests.WireValidation/MetadataValidation`; `ArchiveProductSessionTests.LargeReadNegotiationAndFiniteBudgets` | **A T/R/L**,CR,M/TR,SD,P,H,Q for EXISTING bounded payloads. Future coreprompt payloads require symmetric tests under B2/B3;future pickers/write/launch **E B–D**. No arbitrary public delete/write authority or speculative futurepayload blocker. |

## Finite remaining actions

| Item | Category / exact gap | Bounded closure condition |
| --- | --- | --- |
| **B1 Auto filename candidates** | **B**: [encoding.rs39–73](../../../rust/crates/nxtools-archive/src/encoding.rs) makes Auto always CP437+UncertainCp437. PRD§A1/design§4.1 require detectable AND ambiguous cases. | Bounded candidate/provenance;Unicode precedence;genuine ambiguity/manual choice;stable source/ordinal and safe reprojection. **Sole next slice.** |
| **B2 real operation interaction** | **B**: `InteractionPort` occurs only in its own production module,not engine/worker;standalone ticket/cancel tests are not encrypted retry or real conflict decisions. | Original operation/lease/partial-bound passwordretry and Skip/AutoRename/Overwrite/apply-all;cancel/stale reply. Core A17,public UI D. |
| **B3 ratio/risk core** | **B**: no ratio policy/operation finite approval;ResourceLimits has five count/byte fields. | Declared/actual risk check,finite approval under hard ceiling,no write before approval,cancel preserves output. Keep immutable admission,not unsafe accepted-budget mutation. |
| **C1 empty stream creation** | **C**: Gzip/XZ creation oracle tests use nonempty256KiB. | Zero-byte source→independent decode→public read/native extract,empty counts/cleanup,valid levels. |
| **C2 actual7z BCJ** | **C**: [budget/reader admits BCJ](../../../rust/crates/nxtools-archive/src/worker_codecs/sevenz_budget.rs),but indexed fixture does not assert actual coder chain. | Explicit x86BCJ+LZMA2 producer/coder proof + independent exactbytes/native output + bounded damage/budget controls. Representative,not every advertised BCJ architecture. |
| **C3 signature/alias assertions** | **C**: dispatch exists,no named independent disguisedZIP/JAR/APK acceptance identified. | Tiny ZIP renamed.bin/.jar/.apk stays ZIP;random/corrupt/misleading compound suffix typed-refuse;no mutation authority. |
| **C4 pages/reprojection** | **C**: Big5/CP949/1250/1251/1252 branches exist;current explicitoracle covers GBK/ShiftJIS/CP437. | Independent raw-name literals;changed decoding rechecks unsafe/case/prefix collision before writes. Verification within B1,not a second implementation feature. |
| **C5 solid optional metadata** | **C**: run_sevenz emits packed=None for solid;current openfixture checks nonempty entries,not per-membernull. | Exact shared-block packed_size=null/non-solid values and absent-field semantics;no throughput/RSS sample. |

M/TR metadata/MOTW/time and O publication are delivered scoped PASS,not missing-feature buckets.
No automatic obligation for every AES×method×size/filesystem or largeinput percodec. Typedreject
still cannot waive required BCJ,Auto/risk behavior or A–F scope.

## Decisions, dependencies and original stage ownership

| Item | Original clause / conflict | Minimal proposal — NOT applied here |
| --- | --- | --- |
| **D1 A exit / UI / measurement wording** | implement22 requires ENG08/09;acceptance21 asks firstpaint;PRD1991GiB/2011.5s target. Browser is C39–53;publicprompt UI D58–60;core exchange A17. H≠firstpaint;Q only small cooperative proof. Historical [1GiB native data-cancel98–99](resource-acceptance-review.md#independent-resource-rerun--one-run-per-scenariono-best-score-repetitions) exists,but later tool defects preclude calling it clean new timing acceptance. | Explicit Aengine/native versus Cfirstpaint/DUI evidence split,without dropping requirements. Keep1.5s a target. Resolve literal1GiB proof using existing historical+Q evidence or separately authorize ONE necessary functional check;no rerun queued. Product1GiB credit already has [smallfixture amendment](resource-acceptance-contract.md#recorded-bounded-amendment-after-first-actual-runs). |
| **D2 strict sourceRecycle** | SAFE08 in Aexit;PRD125/146,D60/JOB07 require eventual recycle. [Native investigation](research/source-recycle-native-capability.md#identity--sharing-blocker):held leases block ShellDELETE;closing permits replacement;no inspected fullID/version-conditioned AND never-permanent recycle API. SD excludes nativebackend/directorycoverage. | Keep source and `native_protection_unproven/NotAttempted`;no outputTOCTOU reuse/permanent fallback/syntheticbackendPASS. Explicitly decide foundation versus later nativeproof ownership;safe API/live actions need separate authority. NO safety reduction accepted. |
| **D3 trusted UNC scope** | design154 requires real UNC tests;[sourcelease](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs) permits ordinary localdrive only;LP excludes UNC. | Confirm trustedUNC first-release boundary. If required,it is missing native capability + separately authorized networkfixture,not mere absent tests. Unsafe memberUNC always rejected;no networkshare/mapping here. |
| **Count ceilings,NOT a new benchmark gate** | design158/model finite1,000,000engine ceiling;[later productv2](product-100k-contract.md#budget-table-bytes-not-observed-rss)100000;nativecreate65536. Different layers/operations. | Maximum≠promise every millionarchive fits256MiB/30s. No original million PERFORMANCE benchmark found;do not add one. Record advertised operation-specific limits;changing first-release limits requires explicit contract,not H as blanket waiver. |
| **E original stages/levels** | B:settings/pickers;C:browser/lifecycle/firstpaint;D:forms/publicprompts/FIFO/notifications/launch/deleteintegration;E:splits/ZIPedit;F:registry/OLE;separate native-user/Release | CFG/UI/LIFE/JOB/IPC02–05/P202–03/P206–08/DIAG01 keep implement.md owners. Signedhelper/protecteddeployment/userShell/DPI is not A touchedIPC01. Literal A dependencies remain D1/D2,not falsePASS or unilateral stage move. |

**One question before eventual A signoff:** may an explicit plan-only amendment split A's
engine/native foundation exits from the Cfirstpaint/Dpublicprompt/recycle and separately
authorized native/Release dependencies, preserving all A–F requirements and strict source safety?
1. **Recommended:** approve an explicit dependency split later;D1–D3 details remain visible,no
   requirement/safety waiver. NOT approval already granted or permission to enter B.
2. Keep literal all-ID A exit and remain in A until its cross-stage/native dependencies are
   explicitly resolved;do not pretend missing UI/liveRecycle passed.
3. Supply another precise plan/API boundary. Any sourceTOCTOU relaxation is a SEPARATE safety
   decision,not bundled here or with approved output overwrite.

No answer is needed to implement B1 later;one IS needed before claiming A closed. No plan/spec
exit amendment,phase reorder or native execution is assumed in this ledger.

## Sole recommended next slice — B1

**ZIP bounded Auto filename candidates and safety re-projection (ENG05,SAFE01/02).**
Why first:confirmed required behavior gap,small deterministic red,benefits future listing/extract;
no UI/Recycle/UNC/count decision or largeinput observation. C4 is limited verification within it.

- **Code:** Rust encoding.rs owns bounded candidate/confidence/ambiguity. Only necessary
  provenance/model/ZIP/path_policy/planning changes;extend Rust/C# provenance allowlists and
  worker metadata symmetrically IF needed. Reuse original worker/typedIPC/runner/native owner;
  raw names private/budgeted,source revision/session/ordinal stable. No C#codec/GPUI logic.
- **Minimum real red:** independent tiny unflagged rawGBK/ShiftJIS ZIP names detectable under a
  documented deterministic rule;Auto currently yields uncertainCP437. Assert candidate/provenance
  through actual productopen AND ZIP nativeextract against independentname/content oracle.
  Short multi-candidate-valid names remain uncertain/manual;UTF8/validUnicodeextra wins. Changed
  manual decoding that creates case/prefix/unsafe collision refuses before writes. Decoder-only
  strings or universal language-detection claims are insufficient.
- **Acceptance:** finite candidate/byte work charged before allocation;no wholearchive detection
  copy/pass;ambiguity/Unicode/manualpages(C4)/cancel/budget/source/name safety intact;exact metadata/
  native output match;affected tests,exact C#LIST/RUN,strictlint/fmt/staticgates,independent acceptance.
- **Not doing:** B2/B3/publicprompt/FIFO/UI,allcodec/crypto matrices,million/UNC,100k/1GiB/largeZIP64/
  performance,resource tools,sourceRecycle/delete,dependencyupgrade,system/package/install,PhaseB.
- **Authority:** follow-up implementation instruction needed:THIS turn is docs-only. Existing A
  source/isolatedfixture scope needs no extra system/native-mutation authority or safety downgrade
  for B1. New dependencies still require normal exact license/platform justification.

## Preservation and stop

Only ledger/currentcheckpoint/currentmatrix/README/context navigation change. Historical reports,
results and historical sections,old Failed/Stopped and unproved causes stay byte-identical.
No delegation,build,worker,performance or system actions. Only permitted spec/context/diff and
necessary link/JSON/hash static gates;these are not product acceptance. **Stop after gates.
B1 is not implemented. Phase A/A–F remain incomplete.**

Static closeout:spec106documents/14owners,context82implement/79checkentries and gitdiff--check
exit0;19exitIDs/40localfile+sectionlinks checked.234file openingbaseline:5authorized currentdocs
changed,229unchanged including93historicaltaskfiles;unrelatedtrackeddirtydiff unchanged. Original
checkpoint history reconstructs its opening whole-file hash. Existingnative-contracts35497byte
context warning remains;no new injection oversize. These are document checks,not runtime PASS.
