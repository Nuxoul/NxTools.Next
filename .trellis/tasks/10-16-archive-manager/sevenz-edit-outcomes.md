# Ordinary single-volume 7z editing — Phase E outcomes

2026-10-28. Scope: **Phase E — Format expansion and ZIP editing** ordinary single-volume7z edit slice. This does not renumber A–F, restart accepted FB-AC01–04/HZ2/common capability work, enable TAR/RAR/volume editing, or close the entire archive manager. Implementation and the scoped automated/native evidence below are delivered; overall archive/release/physical-user acceptance remains **PARTIAL**. No package/sign/install/product/Explorer/Registry/service/ACL/UserChoice/recycle/commit/push operation was performed.

## 1. References and bounded backend decision

Read the matching [local implementation review](../../../docs/reference-code/archive-editing.md), its [manifest](../../../docs/reference-code/archive-editing-manifest.json), fixed actual bit7z update callbacks/index/new-data/new-properties,7Zip solid update/repack/password and NanaZip descendant-prefix routes before implementation. All five fixed repositories and17original file hashes were rechecked (22records in scratch `reference-recheck.json`); no upstream code was compiled or copied wholesale. Additional7Zip `7zOut.cpp::SkipToAligned` and `7zIn.cpp::kDummy` inspection proves repeated zero padding is alignment, not semantic metadata; their new hashes are recorded in the manifest. Final recheck verifies19files. Do not import upstream delete-old/move-new transactions.

Locked sevenz-rust2 **0.23.0** remains the isolated codec. Runnable bounded probe established known ctime/atime/mtime/attributes/empty-directory/AES/header encryption and readable zero-member output, and demonstrated that distinct unknown-property headers collapse to the same public Archive. Therefore full reencoding plus project-owned strict original-header admission was selected, not UpdateItems/raw block copy. No dependency upgrade, C++ ABI, DLL, external7Zip product backend or new task/commit framework. Independent7Zip/Python are test oracles only. [Route](sevenz-edit-route.md), [executable format contract](../../spec/nxtools-archive/backend/sevenz-edit.md).

## 2. Per-operation capability and actual evidence

| Operation | Implemented route and verified behavior | Important boundary |
| --- | --- | --- |
| Add files/directories | Original protected source IDs/CreationPlan; empty file, empty directory, nested tree, multiple source and conflict overwrite/skip/auto-rename | Source roots<=31 in public modify request; no link-follow/self-output bypass; apply-to-all original ticket; all-skipped returns warning2/commit0 without rewriting |
| Delete file/directory | Immutable original ordinal; directory descendants removed in one candidate; overlap dedup and `a/` vs `ab/` planner tests | Remaining streams, including deleted/replaced old data, fully decoded before publication; all-delete independently lists as legal empty7z |
| Replace file content | Distinct typed `replace_content`, exactly one existing regular ordinal and one protected ordinary source; nonempty and zero-length replacement | Not Add; no intermediate delete commit; old name/known metadata retained; directory/multi-source/stale request refuses; ZIP replacement remains unavailable |
| Rename file | Original ordinal, content/known metadata unchanged; exact final safe-name collision checks | Sorted row/display path never used as member identity |
| Rename directory subtree | Explicit and implicit prefix with real original descendant ordinal; nested descendants/empty dirs and Unicode all updated in one candidate | Optional `source_prefix` identifies selected parent level; stale prefix/self-descendant/case/file-prefix collision refuses; `a/` does not match `ab/` |
| Existing UI/task hookup | Original menu/outlined commands/F2/Delete owners, ModifyDraft/form/picker/queue/task result and refresh; new replacement button/menu, no invented shortcut | Current snapshot/source/connection/attempt checks; failed draft stays retryable; deferred cross-entity callback contract retained |

Actual UI automation covers pointer/menu/key/form dispatch once, missing eligibility, stale/generation rejection, one regular replacement, implicit parent identity, visible full-reencode warning, and fixed progress phases. It is GPUI TestWindow evidence, not physical DPI/IME/Explorer/user visual acceptance.

## 3. Per-archive state, fidelity and cost

| Actual source state | Support/refusal |
| --- | --- |
| Ordinary non-solid/solid, supported bounded coder graph | Supported for scoped operations; **every edit fully decodes/reencodes** to single-thread **non-solid LZMA2**. Existing packed bytes/method/layout/ratio are not preserved |
| Uniform AES content, plain header | Supported; original private password interaction; candidate remains AES. Refuse an edit leaving only zero-stream data when content-only encryption cannot be represented faithfully |
| AES content plus encrypted header | Supported; encrypted header and content/password retained, including replacement/conflict fixtures. Password never argv/log/disk/persistence |
| Empty/empty-file/empty-directory result | Plain empty output independently validated; zero-length replacement checks real source EOF. Encryption cases that cannot preserve the source mode fail closed rather than silently become plain |
| Old known ctime/atime/mtime/Windows attributes/empty dirs | Retained and fully compared against candidate. Independent stock7Zip creates an archive with three known timestamps/read-only attribute; after Add the independent listing matches every field exactly |
| New added source metadata | Names/type/content and empty structure are protected; original source-ID contract exposes DOS-local wall time, not full UTC NTFS metadata. No claim that added-file ctime/atime/ACL/ADS/all attributes are copied; no fake local-to-UTC conversion |
| Unknown semantic FilesInfo/archive properties/external metadata, anti/link/reparse/special items | Refused before candidate publication, not silently dropped. Repeated **zero-only kDummy** alignment accepted; nonzero padding refused |
| Mixed plaintext/AES blocks, header-only encrypted data shape that cannot be retained | UnsupportedEncryption; no blanket rejection of all solid/encrypted archives |
| Unsupported coder or multi-input/output graph | UnsupportedMethod/UnsupportedOperation; finite conservative decoder/encoder simultaneous reservation remains. Runtime/native matrix is LZMA2/AES; not all BCJ/Delta/Copy/LZMA cross-products independently accepted |
| Multi-volume/SFX/tail/truncated/corrupt/nested writeback | Refused. Actual native source-family discovery and strict signature/header/CRC/extent checks, not extension permission |
| Missing old worker eligibility | Browse/preview/extract remain independently usable; edits refuse until actual opened-source eligibility is present |

Form uses official Kit0.8.0 Form/Field, original outlined actions and **Small nonclosable warning Alert**. It discloses full non-solidLZMA2 rewrite and original+candidate coexistence; unknown space stays unknown, no source-deletion credit. Original progress window distinguishes preparation/read/reencode/verify/native commit; unknown total means no invented percentage. Fixed stage transitions use original bounded event queue; same-stage byte samples remain100ms/coalesced. Cooperative pause/cancel is checked at source/payload/spool/native checkpoints; backend header encoding is not claimed instantly interruptible. No plaintext source spool.

## 4. Original transaction and diagnostics

Same logical task/FIFO(max2running/32queued), attempt, password/conflict ports and native owners. Exact path: native original source+parents protected -> source digest/size/mtime/native binding -> strict header+ordinal plan -> single anonymous encoded candidate -> finish/flush checked -> complete candidate names/kinds/known metadata/hash/decode/encryption verification -> held native partial full readback/verification -> close every Rust old-source reader -> acknowledged `edit_reads_complete` -> C# still-held source fullID/version/hash/target recheck -> final cancel -> release ONLY FILE -> one existing native atomic replacement. Accepted recheck/FILE-close/path-publish TOCTOU is **not ID-CAS**.

Source/addition change, cancellation, insufficient finite output reservation, write/readback/candidate validation failure before commit preserve original and clean only owned unpublished output. Native denial5 retains FailedBeforeCommit/code5. Unknown64 retains CommitOutcomeUnknown/evidence, no retry. Native success survives late cancel with published journal. Exact edit commit failure is frozen at its owning native call rather than replaced by worker IoFailure. Ordinary extract/Test/read sharing was not widened.

One correlation crosses Rust/IPC/C#. Existing accepted/start/fixed-stage/terminal/elapsed/native diagnostics remain; no paths/member names/content/passwords, no byte-tick logging. Added source errors retain their first type through codec wrappers. Source closure is proven by actual Windows exclusive reopen at worker handoff, not just a flag.

## 5. Genuine RED → GREEN evidence

Evidence root for this round: `C:\Users\Administrator\.pi-desktop\scratch\a03c841f-63f5-4955-8c7b-3f30a8aac0cf\sevenz-edit`. Earlier failures/logs remain intact.

| Boundary | RED evidence | GREEN evidence |
| --- | --- | --- |
| Distinct replacement wire | `wire-red.log`: RUN1FAIL1 unknown replace_content | Protocol89 and C# strict wire3 pass |
| Actual worker edit/reader closure | `worker-red2.log`: compile0/RUN1FAIL1, missing modify_7z yielded EOF not complete | Final real worker1PASS, normal single commit/exclusive reopen; corrupt readback abort/no handoff |
| Conflict apply-all / first source error | `safety-red.log`: RUN3,1PASS2FAIL (reprompt2, SourceChanged collapsed to IoFailure) | Final safety4PASS; skip2/commit0, overwrite/auto-rename, empty replacement, typed faults |
| Original7Zip alignment metadata | `alignment-red.log`: zero padding rejected; native independent metadata refused | Final header and independent native metadata pass; no semantic-property relaxation |
| Cancelled all-delete plan | `header-plan-first.log`: cancellation test returned success | Final planner8PASS incl cancel before mutation |
| AutoRename directory reservation (single bounded review finding) | `review-autorename-red.log`: OutputCollision for occupied explicit/implicit numbered directories | `review-autorename-green.log`/final safety pass; next free `a (4).txt` chosen |
| Real progress stage wire | `stage-real-red.log`: phases[] | `stage-worker-green.log`, final worker and final native phase assertions pass |
| Native exact failure code | `native-alignment-run.log`: last test failed; journal was correct but native5 absent | Final nine tests pass with actual code5/state, no retry and latecancel publication |

Intermediate compile/fixture mistakes (generic/API spelling, owned-root naming, malformed synthetic descriptor, pure-text semantic snapshot expectation, synthetic low-space error spelling and progress consumer observation) are retained as diagnostic history, **not counted as genuine product RED**. One bounded review performed; no second broad review.

## 6. Final verification, exact scope and frozen worker

Final frozen isolated worker: `sevenz-edit\native-worker\nxtools-archive-worker-final.exe`.
SHA256 **28AFDFEEC7BFC8E46C03800F08B2657F01D702B558C34DB21C550DD56D33A066** (`native-worker/final-hash.json`). Intermediate `hash.json`, `current-hash.json`, `alignment-hash.json` are historical, not final identity. Final source build and native tests use this frozen file; no package/release acceptance claim.

- Every Cargo call checked project/global Trellis channels and live Cargo/rustc, one serial lane, same-process `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, `--locked --offline`, serial tests and full-operation timeouts. No build timeout/duplicate build/process termination.
- **Archive236PASS**,39 executed targets (lib+38 integration files), fail0/ignored0. `archive-final-unified.log`, exact target/count JSON. All existing ZIP record/ZipCrypto/AES/ZIP64/unknown-delete/edit-handoff tests pass. Explicitly filtered the documented100k performance test; did not miscount it as a pass. Three standalone native-coordinator chains are excluded/not accepted below.
- **Desktop archive319PASS**, fail0/ignored0 (`desktop-archive-final-stage.log`): original310 plus9new7z tests. Protected38 all present: **HZ1 18 / icon-layout8 / WAC12**. Narrow `archive::`285/286 counts were partial filters, not the baseline comparator.
- Core93unit+7integration, IPC43, Protocol89 passed (`layers-tests.log`). Five-owner strict Clippy `--tests -D warnings` with isolated-codecs passed (`strict-final-all-second.log`). Workspace `check --workspace --tests` passed after final source (`workspace-final.log`). Default feature Desktop compilation also passed; unrelated ordinary-bin warnings during test builds are recorded, no unrelated source cleanup.
- C# final build0warnings/0errors. Effects audited; exact LIST→identicalRUN **7z9selected/9RUN/9PASS/0SKIP/0FAIL/0not_run** (`native-stage-final-*`). Three wire tests plus six native scenarios, independent stock7Zip exact members/content/encryption/metadata; fixed progress phases traverse actual Rust/native/Job event route.
- Original accepted **56** C# compatibility names executed individually with exact LIST/RUN and final frozen worker: **55PASS/1FAIL/0SKIP/0not_run** (`compat-native-final/results.json`). Includes ZIP edit6, HZ1/space, real task/pause/password, preview/OLE/drag lease and directory-only result protections. The one failure is unchanged historical `archive job fixture actual readmission shortfall no output`: `source_unavailable`. No budget/selection/assertion relaxation.
- Scoped rustfmt45paths+check, scoped/global diff whitespace, GPUI release, pinned Markdown editor and PlatformHost test-selection source gates passed. Spec/navigation and reference-manifest checks recorded with final documentation. These static gates do not certify product/native/user acceptance.

Final Archive target breakdown: lib35; creation-plan4; creation-worker1; encoding4; engine14(one100kfiltered); extraction-plan5; interaction4; job-execution2; metadata-wire1; output-readmission-chain1; output-readmission-contract3; product-cancel10; product-read-chain24; product-consumer7; prompt-display2; rar-readmission-contract1; sevenz-edit-safety4; sevenz-edit-worker1; sevenz-readmission-contract1; source-delete9; stream12; tar-stream-readmission-contract2; tar-stream-guard1; tar-time6; transaction5; tree-create-worker1; worker-cancel1; worker-extraction9; worker-wire19; xz5; ZIP-boundary2; ZIP-create1; ZIP-crypto5; ZIP-edit10; ZIP-edit-worker1; ZIP-methods13; ZIP-volume5; zstd-read1; zstd-safety4.

## 7. Remaining / not executed / stop boundary

- Overall still PARTIAL: known native readmission `source_unavailable` remains1FAIL; historical CLI cleanup `helper_untrusted` report unchanged/not rerun. FB-AC05 annotated two-line screenshot issue remains OPEN, not part of this slice.
- Standalone `rar_readmission_chain` and `sevenz_readmission_chain` initially failed missing explicit evidence/fixture environment in unrestricted Archive invocations; those failed logs are retained. `tar_stream_readmission_chain` was not executed. Fresh tiny7z/TAR-stream data was generated in current scratch and TAR-stream contract2PASS; the three full native coordinator chains were **not run to acceptance**. Their coverage is not replaced by declaring the full Archive suite green.
- Not a complete adversarial/large/resource matrix: no direct edit-specific injected local encoder finish/flush storage fault, hard-kill/power-loss/restart acceptance, exhaustive coder/filter/encryption cross-product, huge archives/disk-fill stress or all added-source metadata fidelity. Original checked finish/flush and first-error code paths plus existing creation fault tests pass, but they are not falsely labeled an edit-specific injected fault.
- No physical pointer/IME/DPI/Light-Dark screenshot/user acceptance, installed protected tuple, Explorer end-to-end, signing/package/update/service validation. Source/compiled/native-fixture evidence is separated.
- Checkpoint update is prepend-only; original **163934bytes** retained as exact suffix, old reports untouched. No overall PhaseE/A–F/release completion claim. No TAR editing/HZ3–HZ5/source reclamation started.
