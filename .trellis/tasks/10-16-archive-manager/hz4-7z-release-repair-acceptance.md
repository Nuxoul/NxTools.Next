# HZ4 7z / Release repair — independent narrow acceptance withheld for HZ4-TERM01

2026-10-29. **Phase E — Format expansion and ZIP editing**, original Step6 only. Parent examined user-provided completed delivery and [repair outcomes](hz4-7z-release-repair-outcomes.md)/[matrix](hz4-7z-release-repair-matrix.md). **The compressed-header fix, approved reader classification and complete Release Tests compile gate are independently verified at the levels below. HZ4 overall remains withheld because one actual native compatibility test failed.** No HZ5 or experience package starts. Original reports, old FAILs and [reader decision](hz4-7z02-reader-decision.md) remain unchanged. No parent product edits or additional reviewer.

## Independent baseline / identities

Parent root: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\h4r-220225`. Full68 expanded argv/exit/time records in `results.json`; exact counts/dispositions in `accounting.json`, frozen IDs in `identities.json`, raw logs and new owned-process/native evidence retained. `normal-check.log` and `nn/result.json` are supplemental, not silently included in68. Opening checkpoint **222502bytes**, SHA256 `D82E09BE9BC246EDB7580CEC6108982F2C0B43886493F1B9714F526C0D013ED5`, saved in full. Delivered source hashes/frozen binaries verified before tests; **1045-file test-time/final baseline zero drift**, including Cargo/installer/historical evidence documents.

Current frozen worker **CA67D018EF04F8F3ACBD09F34C6D1E85768D609E16A791E8949D8B0552AFEA34**; actual DebugTestsDLL **967609C35D233BB22ACD57C3C86AC2391855167EFC10657404CC2F25B296376E**, unchanged DebugHost **9759D4C003403C4DE83FD12422F861C6DCF21C071120292FD1A7397AF8171B65**. Parent copied the whole frozen Debug bundle into its own short scratch path; test executable alone is not used as content identity. Stock7Zip/system bsdtar hashes and decoded fixed witness contract match the approved decision.

Every Cargo operation checked idle shared cargo/rustc/clippy lane and inherited jobs1/incremental0/codegen-units256 in the same process; no duplicate/name kill. Initially another repository was compiling, so parent performed source/native work first and waited; did not demand pause or terminate it. Release .NET preflight measured **8.35024642944336GiB** committed headroom before building to parent-only scratch output. No Release Cargo or production package/sign/sequence allocation.

## Actual independent checks

| Check | Actual parent result |
| --- | --- |
| Current compressed7 + explicit plain7 real process matrix | **14 process/integrity PASS**, all intended receipts/exact retained child handles/worker exits and fresh refusals; bsdtar **8PASS / 6reader-unsupported / 0unexpected-readerFAIL** |
| Default unqualified compressed native_published | expected overallFAIL/exit1, while process/integrityPASS; confirms no silent blanket waiver |
| Reader controls on parent's current encoded output |3actual binary witness pairs, encoded positive,9fail-closed negative rejections PASS |
| Current normal original Job, no termination | succeeded/exit0/one publication, exact primary manifest/native/fresh refusalPASS, secondary reader-unsupported explicitly retained |
| Supplemental wrong reader exit / unrelated exit1 message | both rejected by original oracle call path, remain FAIL; not process cases |
| Current restart supplement | **9groups PASS /14fresh-process safe refusals**, not recovery successes |
| Complete Debug Tests csproj build | exit0,0warnings/0errors |
| **Complete Release Tests csproj independent build** into parent scratch | exit0,**0warnings/0errors**; actual TestsDLL **E4DD415723BE876A19DA6E0C75F0469578BFB699CDEB6B3E6E78724F2294FDF6**, matching delivery |
| Complete actual Release assembly exact2LIST -> identical RUN | **2PASS/0SKIP**, including early risk-fixture refusal; no mini probe substitution |
| Complete Release --hz4-owned-process entry | expected exit2/release_fixture_disabled before FS, specified nonexistent path remains absent |
| Effects-reviewed exact23 native LIST -> identical RUN | **22PASS /1FAIL /0SKIP**; original same-name failure retained, not rerun for green |
| Desktop full archive, fresh parent native export | **349PASS/0FAIL/1167filtered** |
| Archive lib+11 targets | **102PASS/0FAIL/0ignored/0filtered**, including new default/explicit header tests and original EIC/TARraw/ZIP/7z controls |
| Protocol/IPC/Core | **91/43/100PASS**,234total; doc-tests0 separate |
| strict Desktop/Protocol/IPC and isolated Archive tests -Dwarnings; workspace --tests check | exit0 |
| scoped2Rustfmt / source diff /spec118/14owners /release-editor provenance | exit0; source/static only |

The68-command driver itself records all results and continues collection; its top-level exit0 is **not** an assertion all child commands pass. `accounting.json` independently identifies exactly three nonzero children: two intended negatives (default unqualified exit1, Release entry exit2) and **native-11-run exit1 unexpected**. A progress message prematurely said23native passed before final accounting; that message was explicitly corrected. Final result is22PASS/1FAIL, not implementation's23PASS. No failure logs were overwritten.

## Verified bounded repairs

**HZ4-7Z01 source fix verified:** original edit-only `Header::streams` reports presence of optional SubStreamsInfo; only synthetic bridge materializes default `[8,0]` when absent. Original file bytes/PackPos/explicit descriptors unchanged; require exactly one metadata stream. Real current stock7Zip compressed-header source now reaches all7original points, complete old/new archive/member/known metadata and exact native handoff protected. Causal Copy controls (default/explicit,PackPos0/7,folderCRC defined/undefined/mismatch) and full lib44 pass. Writer, dependencies/backend and native publication owner were not changed.

**HZ4-7Z02 approved accounting verified:** classifier is test-only/opt-in and requires exact pinned readers/version and fixed full4entryTTTFinline-header/payload/signature/encoded+decoded+main CRC structure. No metadata removal or fabricated fourth attribute, writer change, defaultplain seed or blanket error-string exemption. Actual bsdtar still executes; known canonical structure is separately unsupported, not a reader PASS. Primary full t/member/content/metadata and process/native/fresh-refusal remain mandatory. Default-off, swapped nonconforming/all-defined and nine corruption/identity controls keep other failures closed. No claim all readers interoperate.

**Release test-only gate verified:** outer Release method throws original exact skip message before effects; original Debug risk assertions remain active. Parent independently builds the **whole** project Release with0warnings/errors and runs its actual2gates and pre-FS entry rejection. This closes reproduced CS0162 compile blocker, not full Release fixture/risk behavior, Release worker, package, install or user acceptance.

## HZ4-TERM01 — native failure cause5 masked by worker cleanup cancellation (required bounded repair)

Exact unmodified native test **`archive sevenz edit native conflict skip native terminal`**, parent `native-11-run.log`, fails `native denied edit truth, no retry` at `SevenzEditTests.cs:444`. First seed and all-skipped warning phases pass. At original `commit_in_flight` hook, test injects `ArchiveOutputException("replace_file",5)` once. Original expected invariant is outcome failed, native5,0commit,FailedBeforeCommit; the actual terminal is instead:

```text
outcome=cancelled; code=cancelled; native_error_code=5
files_committed=0; commit_state=FailedBeforeCommit; cleanup_complete=true
journal=[unpublished,partial_removed]; output_published=false
```

Parent did **not** rerun it seeking a green timing. Shallow read-only discovery by exact time window + current worker hash located the matching retained GUID fixture `C:\Users\Administrator\AppData\Local\Temp\archive-product-1fb17e86-38ed-406a-b099-9f2c0f11f85d`. Exact before/after archive SHA256 strings match. Its original3terminal/diagnostics were copied unchanged into parent `native11-retained/`, with source index in `native11-retained-index.json`; no reparse recursion or cleanup.

Correlated operation **1df5eede-6c89-430e-84b8-25200ba6fabc** diagnostics establish the real sequence: `archive.output.failed` stage replace_file/native5 -> unpublished partial cleanup -> **archive.worker.failed category replace_file/native5/FailedBeforeCommit** -> **archive.job.terminal category cancelled/native5**. This is a user-visible false cancellation, **not demonstrated data loss or publication**. It occurs after native cause is correctly preserved in Runner; source/candidate ownership stays safe. Not asserted newly introduced by this synthetic-header repair.

Actual mapping chain: `ArchiveWorkerRunner::FailNative` records native failure then requests worker stop; worker failure may be Cancelled as a cleanup consequence. `ArchiveJobSession::OnFrame` stores that in `op.Failure`; `OnRunnerEvent` lines164–167 prefers `op.Failure` unless native category is in its explicit authoritative/nativeCause set. `replace_file` is not in that set, so stored cleanup cancellation masks original native cause. No cancel request was issued by this test before the failed native5 step. This is an actual production native/Job observed failure, not a speculative code concern or reader/tool limitation.

Return **only HZ4-TERM01** to original implementation session: minimally preserve an actual owning native failure cause over consequential worker cancellation at the real Job projection boundary. Require deterministic event-order regression (native failed output plus worker Cancelled frame before native exit), plus actual native5 case and normal real user-cancel/no-native-error control. Preserve genuine cancellation, unknown/native64/publication/latecancel/sourcechanged/timeout/cleanupfailure authority and error privacy. Do not undo accepted exact native-cancelled mapping, globally turn every cancellation with an error into failed, weaken assertions, broaden unrelated error framework or rerun broad reviews. Original failed23sweep and native fixture remain immutable. Subsequent narrow verification must disclose this first22PASS/1FAIL history.

## Stop and retained limits

Parent does not attach host acceptance or dispatch HZ5 while this real required compatibility defect is unresolved. Package remains **not built, no sequence reserved, not installed**; no SkipVerification/unsigned fallback. After this one bounded repair and independent verification, reassess HZ4 scoped acceptance and the user's requested stable experience package.

Original readmission/source_unavailable/helper_untrusted/password-fixture/FB-AC05/physicalGUI-DPI-IME-UIA-Explorer-user-installed acceptance stays OPEN; no unrelated fix requested. Fresh Session stale_operation is explicit refusal, no trusted durable archive recovery/resume/partial cleanup exists. Process termination is not power loss. Full53historical matrix,fullfixture/workspace test/fullnative/100k/performance/Release worker were not run this round. Prior source-preserving successful cases retain their original proof; no blanket crash-safe/recovery/release claim.

No parent product code modification, package/sign/install/system/real-user-source actions, dependency/vendor/backend upgrade, commit/push or new review. Checkpoint may only prepend; complete222502byte opening must remain exact suffix.
