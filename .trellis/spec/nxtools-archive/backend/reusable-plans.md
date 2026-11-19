# Operation-local format and selection plans

## 1. Scope / Trigger

Read before changing catalog/qualification/Test/extract/materialize, ZIP edit admission or subtree selection. Owners: `repo:rust/crates/nxtools-archive/src/engine.rs`, `repo:rust/crates/nxtools-archive/src/formats/zip/plan.rs`, `repo:rust/crates/nxtools-archive/src/formats/tar.rs`, `repo:rust/crates/nxtools-archive/src/planning_selection.rs`, `repo:rust/crates/nxtools-archive/src/zip_edit.rs`. Reference call chains: `repo:docs/reference-code/archive.md`, `repo:docs/reference-code/archive-editing.md`. Native equivalence is separately owned by [PlatformHost](../../platformhost/archive-read-observation.md).

Reuse is confined to the actual operation/attempt. No reader, spool, validated offset, permission token or authoritative Boolean enters `ArchiveSnapshot`, browser wire data, a global table or a persistent cache. This does not add formats, edit capabilities, dependencies or a new execution owner.

## 2. Signatures

- `ArchiveEngine::open_attempt_catalog(OpenRequest, &OperationControl, stream_declarations) -> Result<(ArchiveSnapshot, RetainedCatalog)>`; `open_retained_catalog` uses the same private attempt owner.
- `RetainedCatalog` privately retains `input: Option<File>`, `tar_plan: Option<tar::ValidatedPlan>` and observed gzip member count. `execute_retained` / `extract_retained` consume these facts through the original engine guards.
- `ValidatedZip::new(&mut File, &[ArchiveEntry], &ResourceLimits, &OperationControl, retain_headers) -> Result<ValidatedZip<'_>>`. Its exclusive reader and immutable entry borrows enclose execution. Records retain original `EntryId`, data offset, encoded size, record end and optional exact local header.
- `ValidatedZip::stream(ordinal, limits, control, password, total, on_chunk)` uses only its admitted record. `zip::extract_validated` requires the identical immutable entries projection.
- `ArchiveEngine::extract_with_zip_plan(request, control, output, source_lease, &mut ValidatedZip)` enters the same private `extract_impl` as public extraction. Readmission must hold this plan across native preflight, not construct and discard it.
- `expand_selection(catalog, CatalogSelection, metadata_credit, entry, emit, checkpoint, &mut SelectionWork) -> Result<Vec<u64>>`. `bind_ordinals` binds requested original IDs to catalog positions, including noncontiguous Desktop ordinals. The emit callback receives the temporary normalized path and selection reservation, and returns its retained-output reservation.

## 3. Contracts

### ZIP admission and execution

One plan performs the original central/local name, flags, method, size, CRC, descriptor, ZIP64/AES extra, encoded extent and sorted overlap checks. Contiguous-layout qualification is computed during the same overlap traversal. Original raw names/extras/encoding/compression/encryption remain format metadata, not display-normalized replacements. Independent public operations still open and verify their own source revisions. A genuine `worker::NativeReadLease` may suppress only the original per-member protected-read revision work; it grants no edit, split-volume or reclamation authority. External split source-set checks remain complete.

Ordinary `modify_zip` uses private catalog intake and original browser/native source binding. The intent-aware candidate performs full edit admission; unqualified browse capabilities are not authority. Format/volume/application-container/backend restrictions and unsupported ReplaceContent remain. A no-change Add still runs fresh read-only modification eligibility because no candidate will discharge structural admission.

ZIP update payload responsibility is explicit:

| Bytes | Integrity owner |
| --- | --- |
| Retained old payload | Complete final candidate decode, CRC/MAC, actual size and codec-end |
| New payload | Additions structural admission, then complete final candidate payload verification |
| Removed/replaced-away old payload | Original source plan and full payload verification before publication |
| Held partial | Exact encoded length/hash equality to the fully verified candidate, plus original native flush/fingerprint/publication checks |

Old-source actual bytes (removed plus retained) and final-candidate actual bytes have separate output quotas. Copy/finish/catalog readability is not payload integrity. Raw directory records, snapshot fields/capacities, intent, copied output records, additions catalog/plan and final verification must be accounted while they coexist. ZIP plan record/range/header reservations precede growth; streaming subtracts live plan working bytes before existing codec minima and 64KiB buffer credit.

Directory Rename validates the original selected ID and kind, normalizes slash projection, and matches an exact selected record or a slash-ended descendant prefix. Attribute-defined directory `a` is valid without a stored slash: rename to `b/` produces `b/` and `b/file`, not `b//file` or a rewrite of `ab/file`. Only affected names are rewritten; unrelated raw records remain unchanged.

### TAR/compound ownership and budgets

Catalog discovery fully validates the outer stream/footer and inner TAR structure/metadata once and retains the anonymous spool. Qualification reuses those bytes with the original strict TAR/gzip/XZ policy; execution uses the same catalog and original-ordinal locator Vec. A separate edit/candidate/readback operation remains fresh. Gzip member count comes from the complete original decode; it is not guessed from the spool.

Catalog estimate is the sum of `1024 + raw_name_bytes * 32`; actual locator capacity times eight is admitted before growth. Strict edit records receive only metadata remaining after catalog/locators. Fresh and retained Test/extract both use `ValidatedPlan::admit_execution`: execution requires `256KiB + entry_count * 128 + actual_locator_bytes`, together with retained catalog metadata. Codec and whole-spool limits remain separate from selected output. Original cancellation, source checks, metadata provenance and anonymous-file retirement survive every early return.

### Selection and lifecycle

Caller checks original session/snapshot and IDs before helper execution. Bind selected IDs once; normalize each catalog path only as needed; merge nested/overlapping slash-ended prefixes; emit one expansion in catalog order. Missing nested roots still refuse even when an ancestor covers the same bytes. Preserve `a/` versus `ab/`, slash/backslash projection, explicit/implicit/empty directories, collision rejection and original logical limits. The required scale is 65,536 entries/4,096 IDs, not a new runtime cap.

Scratch credit is remaining metadata bounded by working allowance, including catalog, selected tables/roots/ordinal output and callback-retained output. No second complete normalized catalog is retained. Existing drag `BTreeMap` presentation/collision order and COM descriptor indices remain distinct from expansion order. Browser final edit admission/refresh uses fresh original IDs, not names as write authority. Cross-entity results remain deferred and connection/generation-bound.

Aggregate `SelectionWork` before propagating success or failure; do not double count at outer callers. Existing operation/attempt diagnostics carry bounded counts and elapsed times at terminal. Never log members, paths, payloads, passwords or chunks. `zip_record_seeks` covers record admission only; `tar_record_scans` excludes strict edit parsing; `tar_plan_bytes` is cumulative locator backing capacity, not peak memory; normalization allocations count owned results, not allocator calls. Unobserved total/internal I/O stays unavailable.

## 4. Validation & Error Matrix

| Condition | Boundary result |
| --- | --- |
| Changed source/session/projection/original ID | `SourceChanged`; no stale-plan execution |
| Local/central/descriptor/extent/overlap or final CRC/MAC/codec-end disagreement | Original corrupt/authentication classification; no invalid publication |
| Additions bad payload or removed corrupt source member | Refuse before native publication |
| Metadata/working/actual-byte/spool/count limit | `ResourceLimit` at the original budget boundary; no enlarged allowance |
| Unsafe output name, missing selected root or collision | Original path/collision/source refusal; no partial selection acceptance |
| Unsupported edit/format/volume/application container | Original unsupported refusal; no backend fallback |
| Cancellation | Original attempt cancellation and actual retirement; committed/unknown native facts remain authoritative |

## 5. Good/Base/Bad Cases

Good: one TGZ Run lists/qualifies/executes from one fully verified spool; three retained demands reuse one ordinal Vec. ZIP readmission admits four records once and executes through `extract_impl`. Rename/comment decode 128 retained payloads in the final candidate, not on the source too.

Base: a separate public Test revalidates its own source; split ZIP still checks every volume; no-change Add publishes nothing. The same native source/output owners still flush, commit once and report actual cleanup.

Bad: serialize offsets as browser authority, reuse a prior-stage digest, subtract codec memory without live plan memory, treat removed corrupt bytes as irrelevant, match `a` as the raw prefix of `ab/file`, or alter descriptor sorting to optimize membership.

## 6. Tests Required

- `repo:rust/crates/nxtools-archive/tests/opt05_zip_plan_contract.rs`: single structural plan; retained/new/removed payload responsibility; corrupt retained/removed/additions refusal; encrypted codec and snapshot coexistence budgets; attribute-directory Rename exact names/payloads/source preservation.
- `repo:rust/crates/nxtools-archive/tests/opt05_tar_plan_contract.rs`: outer reuse, original ordinal plan, failed footer/truncation/cancellation/source changes, fresh/retained execution budgets, no output on refusal. Opt-in baseline must actually execute for three-trial evidence.
- `zip_edit_worker_contract.rs`: normal source plus final candidate each admitted once, source readers closed before handoff, corrupt readback refuses. `worker_extraction_contract.rs`: ZIP readmission successful business terminal and three real commits before asserting four record checks.
- `repo:rust/crates/nxtools-archive/tests/eic_repair_worker_contract.rs` and `tests/fixtures/eic-edit-repair.py`: actual implicit/explicit all-Skip Add must report exactly one `eligibility_checks`, respectively two/three `zip_local_checks` and `zip_descriptor_checks`, and zero member/source/addition/candidate payload decodes. Preserve the original prompt/Skip summary, zero incoming payload opens, zero candidate/output/publication and unchanged source. Bypassing only fresh modification eligibility must fail this worker assertion while the no-publication behavior remains.
- `planning_selection_tests.rs`, `sevenz_edit_plan_tests.rs`, `zip_edit_selection_tests.rs`, `tar_edit_tests.rs`: exact member set/order first, then 65,536 visits instead of selected-root times catalog; metadata/working/cancellation/refusal behavior.
- Desktop `browser_tests.rs` / `model_tests.rs`: noncontiguous IDs, stale guards, refresh binding, empty/implicit/nested roots and unchanged descriptor presentation. Original form/destination and OPT-04 preview/native lifetime tests remain.
- Effects-audited C# original Read/Job/Entry/source/output tests: exact LIST then identical RUN, matching worker, nonzero selection, zero skips, actual bytes/publication/source exclusive reopen and cleanup. Synthetic peers are not native acceptance.

## 7. Wrong vs Correct

Wrong: `preflight_created` builds a plan -> drop -> ordinary extraction rediscovers it; or public `open` qualifies every edit source -> candidate qualifies it again.

Correct: reader-bound private plan -> native preflight -> original `extract_impl` guards/finalization -> payload execution -> attempt retirement. Actual edits use catalog/native binding -> intent-aware complete candidate admission -> exact held readback -> original native handoff/publication. Public browse qualification remains fresh and independent.

## Scenario: Ordinary ZIP password admission shares its catalog

### 1. Scope / Trigger
Default single-volume unencrypted ZIP Test/extract jobs with password interaction and the original acquired native source. Owners: `repo:rust/crates/nxtools-archive/src/worker.rs#OpenedAttempt` and `repo:csharp/NxTools.PlatformHost.Tests/ArchiveOptimizationTests.cs#SharedReadWork`.

### 2. Signatures
Private `OpenedAttempt { source_lease: Option<&NativeReadLease>, catalog: Option<(ArchiveSnapshot, RetainedCatalog)> }`; `job_password_format(request, control, &mut opened)`; `execute_opened_attempt(..., opened)`. `execute_attempt(..., source_lease)` retains the independent-entry interface used by password preparation.

### 3. Contracts
The ZIP encryption probe opens one catalog without edit qualification. If no entry is encrypted and the original `NativeReadLease::protects` covers that snapshot, transfer its owned snapshot/catalog into formal execution instead of reopening it. Keep browser/source binding, exact original ordinals/projection, fresh execution-before/final source revisions, complete payload/CRC/MAC/size/codec-end and native publication. No opened state enters requests, snapshots, wire fields or another operation. Unleased, split and encrypted paths retain their independent admissions; encrypted password Test/retry and once-only extraction are unchanged.

### 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Default protected plain ZIP Test/all/selected1/2/16 | One catalog, four complete source revisions independent of member count; exact output |
| Encrypted ZIP | Original no-output password preparation and ticket/attempt isolation |
| No matching held source, split source, changed source/browser/ordinal | Original fresh admission or typed refusal; no borrowed authority |
| Corruption, quota, cancellation or native publication failure | Original failure/commit/cleanup facts |

### 5. Good/Base/Bad Cases
Good: the no-password decision and extraction share the same admitted catalog. Base: standalone public operations still open and validate their own source. Bad: trusting a request Boolean or retaining a catalog across source acquisition/operation boundaries.

### 6. Tests Required
`ArchiveOptimizationTests.SharedReadWork` drives the original native Read/Job/Entry pipeline and independently verifies output for Test/all/1/2/16 selection, one catalog/four revisions and actual cleanup. Preserve `read_work_contract`, worker CRC/cancel/readmission, encrypted-extraction password/native failure and changed-source tests. The R45 paired measurement probe may record one or two ordinary catalogs to compare versions; it does not replace the strict current-work regression.

### 7. Wrong vs Correct
Wrong: open solely to discover encryption, discard the catalog, then open the same protected source again. Correct: retain the probe's operation-local catalog under its genuine native lease and enter the original execution guards with it.

## Scenario: Selected extraction output prefix

### 1. Scope / Trigger
Archive Browser drag-out uses one native locate folder, followed by the original ordinary selected Extract. Source and output namespaces remain independent. [Drop contract](../../../tasks/10-16-archive-manager/drop-selection-repair.md).
### 2. Signatures
`ArchiveJobSelection { browser, ordinals, strip_prefix: Option<String> }`; C# `ArchiveJobSelection(..., string? StripPrefix=null)`; `ExtractionPlan::strip_output_prefix(&mut self, prefix: &str)` and `directory_output_path(entry)`.
### 3. Contracts
Absent prefix preserves legacy selected extraction. Present prefix requires Here and a safe slash-ended directory, native255-UTF-16 components and32767 UTF-8 bytes. Original browser/source binding and IDs are frozen before OLE. The plan removes the common prefix only from final output paths; the receiver-owned exact directory root emits no duplicate child or empty-path metadata call. Codec lookup uses original names/ordinals; directory metadata uses mapped output paths. Ordinary password/conflict/progress/cancellation owners remain. Logical selections have no4096 cap; actual envelope, metadata, working and output budgets still apply.
Collision warnings are archive-wide metadata. Whole-archive extraction preserves their refusal; selected extraction reuses the original path annotator on participating collision candidates and retains actual selected collisions. A codec job plan owns selected path/count admission; full metadata validation and complete required decoding/source checks still run. SevenZ/RAR gate each output by its original ordinal in the plan before name-to-output mapping. Native codec catalogs use monotonic original ordinals and the plan retains catalog order for binary membership lookup; same-named records never grant each other output authority.
### 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
| Invalid prefix or non-Here prefix | InvalidRequest before execution |
| Selected original outside prefix or changed source | SourceChanged before output |
| Selected output collision | OutputCollision; no partial selection |
| Unselected unsafe/colliding record | No output authority; does not reject safe selected output |
| Empty selected directory | Successful receiver-owned folder, no nested duplicate |
| Retired/cancelled/reconnected locate receipt | No job submission |
### 5. Good/Base/Bad Cases
Good: `top/sub/a.txt` under prefix `top/sub/` writes `a.txt` using the original member ID. Base: absent prefix keeps `top/sub/a.txt`. Bad: mutate snapshot names or use current selection after the gesture.
### 6. Tests Required
Protocol omission/strict prefix/UTF-16/4097-selection tests; plan exact IDs/files/empty directories/outside-prefix and selected-collision assertions; actual C# selected ZIP/7z/RAR, long-CJK prefix, duplicate original ordinal and changed-source native output. The two-Copy-block 7z fixture with interleaved empty files must select ordinals2/3 as payload `two` and empty-b, never ordinal0 payload `one`. Desktop actual held gestures, frozen selection, current receipt/deferred original owner and retired-connection tests. Controlled fixtures are separate from physical Explorer/Directory Opus acceptance.
### 7. Wrong vs Correct
Wrong: strip the source name, trust archive-wide collision warnings for a single selected member, or let virtual-drag lifetime govern extraction. Correct: immutable source IDs plus a mapped ordinary output plan, ordinary task lifetime and one correlated locate-to-job handoff.
