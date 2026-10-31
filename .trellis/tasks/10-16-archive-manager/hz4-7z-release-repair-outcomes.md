# HZ4 bounded 7z / full Release Tests repair outcomes

2026-10-29. **Phase E — Format expansion and ZIP editing**, original 第六步 HZ4 only. Parent task `3e78dffa-8255-4503-9b38-08fb0527b503`. Implementation and relevant checks are delivered for **parent narrow acceptance, not self-acceptance of HZ4**. HZ5/Step7 and experience packaging remain unstarted.

## 1. Outcome and evidence levels

| Bounded item | Delivered result | Not claimed |
| --- | --- | --- |
| HZ4-7Z01 valid stock7Zip compressed-header edit | Minimum original synthetic-header bridge repair; causal unit RED→GREEN; current real native seven-point compressed input matrix passes process/integrity/fresh-refusal checks | All possible 7z headers, coders or metadata supported |
| HZ4-7Z02 published bytes / bsdtar disagreement | Conforming partial inline WinAttributes exposes pinned secondary-reader limitation. Approved, opt-in test bookkeeping reports **reader-unsupported**, not reader PASS. Writer and metadata are unchanged | Universal interoperability, repaired bsdtar or all readers passing |
| Full Release Tests CS0162 | Original full-project RED preserved; minimum test-only preprocessing; actual full Debug and full Release Tests projects compile with **0 warnings / 0 errors**; complete Release assembly exact two inert gates **2 PASS / 0 SKIP** | Full Release fixture-suite, Release worker, packaged/installed/native product or risk-interaction acceptance |

Final process accounting is **compressed7 + plain7 = 14 process/integrity PASS**, with secondary bsdtar **8 PASS / 6 reader-unsupported / 0 unexpected FAIL**. Normal current original Job control is exit0/succeeded/one publication/no TerminateProcess, full primary and fresh refusal PASS, bsdtar **reader-unsupported**. This is not an all-readers-PASS or no-limitations result. The ordinary case `skip=0` field must always be read with these explicit unsupported counts.

Original `hz4-outcomes.md`, `hz4-matrix.md`, `hz4-acceptance.md`, the original main53 **46 PASS / 7 FAIL**, explicit plain7 **4 PASS / 3 FAIL**, all original negative logs and their frozen binaries are unchanged. New evidence does not rewrite those historical results. Previously accepted HZ3/EIC/TAR and earlier slices retain only their historical bounded scope.

## 2. Evidence location, identity and replay

Private driver/build/process evidence and the artifact indexes are retained under:

`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz4-7z-release-repair`

Below, `R` denotes that exact directory. No dependency on scratch is introduced into production or publishing material.

- `command-ledger-final.json`: exact recorded verification argv, exit, start/elapsed/PID, required environment and log SHA256; includes failed attempts rather than keeping only green commands. Wrapper records are not additional tests.
- `matrix-detail.json`, final `fc/results.json`, `fp/results.json` and each `case-result.json`: selected seam/visit/nonce/logical/operation/session identity, retained process/worker handles, actual receipt, disk native identities/metadata/content hashes, independent oracle commands and fresh original-session refusal.
- `final-normal-data.json`, `fr/results.json`, `rcf/results.json`, `native-current/results.json`: current no-termination, restart, reader-control and exact native results respectively.
- `source-identities-final.json`, `evidence-manifest-final.json`, `protection-final.json`, `checkpoint-suffix-final.json`, `delivery-seal-final.json`: final source/binary/oracle identities, immutable evidence hashes, protected drift and complete opening-suffix proof. The seal records its precise indexed counts and excludes its own recursive self-hash.
- `protection-before-docs.json` and `owner-spec-separate-protection.json` preserve the pre-document checks. The latter reconstructs the separately read owner spec, which was not in the 1186-file opening baseline; it is not falsely counted within that baseline.
- `native-owned-artifact-index.json` additionally hashes retained native test fixtures at their original GUID-owned temporary locations, selected by this run's recorded time window and exact frozen worker identity; it does not promote discovery into a separate native PASS oracle. Those fixture bytes are not silently claimed to live under `R`.

### Current actual executable content

| Artifact | SHA256 |
| --- | --- |
| Current frozen Debug worker | `CA67D018EF04F8F3ACBD09F34C6D1E85768D609E16A791E8949D8B0552AFEA34` |
| Current actual Debug Tests DLL | `967609C35D233BB22ACD57C3C86AC2391855167EFC10657404CC2F25B296376E` |
| Debug production Host DLL, unchanged this repair | `9759D4C003403C4DE83FD12422F861C6DCF21C071120292FD1A7397AF8171B65` |
| Complete current Release Tests DLL | `E4DD415723BE876A19DA6E0C75F0469578BFB699CDEB6B3E6E78724F2294FDF6` |
| Complete current Release Host DLL | `AD0FD3BDD95D76B12D2B1F0CC6ED43BC7DE4F57309D82839F444FF55B7E65C57` |
| Explicit CPython executable | `7D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1` |
| Pinned independent stock7Zip | `4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F` |
| Actual `C:\Windows\System32\tar.exe` | `92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A` |

Actual secondary version: `bsdtar 3.7.7 - libarchive 3.7.7 zlib/1.2.5.f-ipp liblzma/5.4.3 bz2lib/1.0.8 libzstd/1.5.4`. Unknown hash/version is not covered. Tests apphost hash alone never identifies the assembly under test. Original failing bundle remains separate at sibling `hz4/frozen-green` (worker `B0DF89DA…DC7F7C`, Tests DLL `BF4A581D…15EF4C5`) and was used only for the initial reproducible REDs/differentials, not current-source acceptance.

## 3. HZ4-7Z01 exact cause and minimum repair

Before implementation, the reference note/manifest were extended with newly inspected entry points. Read-only reference roots are `D:\Code\archive-edit-references`: 7Zip commit `b88c6bc09339de32f7d18bbab900686b244f42ba` and NanaZip commit `555a3ff0f0e6c754dbf93c528253afeb0444e9e1`. Their `7zIn.cpp::ReadStreamsInfo` chain (`1098–1155`, `1211–1274`) gives **one unpack stream per folder when optional SubStreamsInfo is absent**. Both pinned worktrees were checked clean. No native Listary or other unavailable engine source is claimed.

Locked read-only `sevenz-rust2=0.23.0` `reader.rs:432–512` handles an encoded StreamsInfo descriptor, but its ordinary `read_files_info:661–691` requires populated SubStreamsInfo for NxTools' synthetic streamed FilesInfo entry. The valid stock7Zip descriptor omitted the optional property. The original adapter built that synthetic file without materializing the default and therefore reported corrupt_archive. **PackPos was not lost.**

Causal independent Copy-coded header tests were added before product repair:

- `hz4_encoded_metadata_explicit_substreams_control_and_crc`: explicit descriptor control.
- `hz4_encoded_metadata_omitted_substreams_default_one`: valid omitted default.
- Both cover PackPos0/7 and defined/undefined folderCRC; CRC mismatches continue to reject.

`header-adapter-red-v2` is the clean causal RED: explicit control PASS, omitted-default FAIL/CorruptArchive, exit101. The minimum repair changes `Header::streams` to return whether SubStreamsInfo was explicit; **only the private synthetic description** materializes `[8,0]` when absent. Original archive bytes, original PackPos, explicit descriptors, strict unknown-property/CRC/metadata/budget/encryption checks and native authority remain unchanged. Public `BlockDecoder.entry_count()` requires exactly one metadata stream. No dependency/registry/vendor edit or backend replacement.

`header-adapter-green-v2` full Archive lib **44/44 PASS**, exit0. Current worker real compressed `edit_verifying` then reaches the selected original hook and exact process termination/fresh refusal (`compressed-native-green`), followed by the final complete seven-point compressed matrix. Full final Archive selection is **102 PASS**, not the previous100.

## 4. HZ4-7Z02 conforming output / bounded secondary-reader limit

Normal original Job `mode=run`, no termination, exit0/succeeded/one FILE publication already reproduced stock7Zip full content+metadata PASS and bsdtar exit1. This excludes active process termination as its cause; it is not evidence of a half-replaced archive.

One-variable private copies retained encoded PackCRC omission, folder-versus-substream CRC placement, raw versus encoded header, main PackCRC omission, mtime omission and attributes omission controls. Valid CRCs were independently checked. Removing CRCs/mtime or making the header raw did not repair the disagreement; removing attributes did. These are diagnostic copies only, not product rewrites.

Pinned7Zip `7zIn.cpp::ReadBoolVector2:1158–1176`, WinAttrib `1386–1391`, `7zOut.cpp::WriteAlignedBools:482–491`, attributes `808–809` establish canonical ordering: all-defined, optional MSB bitmap, inline/external flag, defined DWORD values. Actual output property:

`15 0f 00 e0 00 20 00 00 00 20 00 00 00 10 00 00 00`

It describes four entries / definitions `[true,true,true,false]`, inline external0, original values32/32/16; the last newly added directory attribute stays **undefined**. The exact decoded four-entry header and packed file streams are independently bounded and verified by the test-only classifier.

Read-only upstream libarchive v3.7.7 snapshot `archive_read_support_format_7zip.c:2698–2724` takes two bytes before its partial bitmap read. Snapshot SHA256 `8C9CA81FBCD15CD12B2E9701B232FC810E00FE83F49FF0DE2556948F5D6A2821`, fixed URL recorded in the reference manifest. This is **not asserted to be byte-identical source provenance of Microsoft's executable**; actual binary behavior is independently established below.

| Executable private control, same packed file body / valid recomputed CRCs | stock7Zip `t` | actual bsdtar `-tf` | Classifier |
| --- | --- | --- | --- |
| Canonical valid partial attributes | 0 | 1 | reader-unsupported |
| Swap only bitmap/external bytes (nonconforming) | 2 / HeadersError | 0 | Reject; cannot exempt |
| All-defined private diagnostic witness | 0 | 0 | Outside exemption; actual reader PASS remains mandatory |

The all-defined diagnostic adds a fourth0x10 solely to discriminate reader behavior. **That fabricated metadata was never transferred to production.**

The parent independently replayed all six reader calls and verified hashes/start/next CRCs/body equality before approving only this contract in `hz4-7z02-reader-decision.md` (SHA256 `E3295B1D7B9F9D8F81F4BD841FC285C303C6977AAB475A9B792B4AEC60979FE2`). This technical decision is not HZ4 acceptance.

`--qualified-bsdtar-partial-attributes` remains opt-in; default seed remains stock7Zip **compressed**. bsdtar is still executed and its exit/output hashes/failure retained. The exception requires the witnessed exit1/damaged-header report **and** exact pinned primary/secondary identities/version **and** exact bounded canonical full metadata, signature/start/next/encoded-pack/decoded CRCs, packed file CRCs and exact decoded payloads. Extension/error string alone is never sufficient. Unknown structure/identity/version/CRC and nonconforming order fail closed. Primary `t`, exact member/kind/content/known metadata, candidate/readback/readers/FILE-only publication, process receipt/source integrity and fresh refusal remain mandatory; any such failure keeps the case FAIL.

Controls: final `reader-controls-final` exit0, three actual-binary witness pairs + current encoded-output positive + nine negative rejections (start/next/main-pack/encoded-pack CRCs, other valid-CRC metadata, trailing data, both unknown hashes, unknown version). `final-process-checks` also proves unrelated secondary exit/message remains FAIL and verifies import from an independent caller. `current-unqualified-control` deliberately remains overall FAIL despite process/integrity PASS, proving the default did not silently waive bsdtar.

## 5. Full Release Tests blocker and actual inert gates

`full-release-build-red` reproduces original complete Tests project `ArchiveRiskTests.cs(72,17): CS0162`, exit1. Release preprocessing left an unconditional throw inside the loop, making the rest unreachable. Saved headroom preflights observed zero Rust processes and 6876028928 bytes before RED / 7769300992 before current build. No Release Cargo/package was started.

Minimum **test-only** repair places a non-async `Task` method under outer `#if !DEBUG`, immediately throwing the same exact `TestSkippedException` message before effects; original async Debug body is under `#else`. No warning suppression, no `-SkipVerification`, no weakened Debug thresholds/assertions. `risk-debug-preservation.json` proves Debug effective source is byte-for-byte identical to the opening fixture:12398bytes, SHA256 `4065978096EF3E17E23870A68DBA43555829B41055C3D2FBACA0116A3833C365`.

Actual complete Debug and complete Release Tests csproj builds both exit0 / zero warnings/errors. Actual complete Release assembly LIST→identical RUN of:

1. `archive HZ4 process fixture capability release gate`
2. `archive HZ4 process fixture no durable restart authority`

returns **2 PASS / 0 SKIP / 0 FAIL**; the first gate explicitly catches only the exact risk-fixture early Release refusal. Complete Debug bundle's same two gates also pass. This does **not** relabel the risk interaction itself as a Release PASS or hide its refusal as an executed risk test. Actual full Release `--hz4-owned-process` entry returns2 / `release_fixture_disabled`, and the specified nonexistent path remains absent.

## 6. Current regression and ownership boundaries

- Final current 7z matrices: compressed7 and explicit plain7, every original requested seam reached; exact retained-Popen-handle termination, PID/creation/image/parent ownership and owned worker Job exit; no name-based termination. At `edit_source_rechecked` exclusive source read remains denied; at `commit_in_flight` actual reader closure/FILE-only handoff permits exclusive read. Prepublication original bytes/native identity/metadata remain exact. Postpublication complete new manifest and original known metadata remain exact. All fourteen fresh original Sessions refuse `stale_operation` without disk mutation.
- Current normal run: original candidate/native chain succeeds exactly once, no termination, exact primary manifest and fresh refusal; secondary reader-unsupported remains explicit.
- Final restart supplements:9groups / **14 fresh refusals PASS**, covering five untrusted claims, four identity conflicts, two successive logical attempts and native112/5/64 owned injections. Terminal/CommitJournal/private claim never become restart authority. No replay or path cleanup.
- Exact effects-reviewed C# LIST→identical RUN: **23/23 PASS**, zero skips. Includes non-solid+solid7z actions/all-delete, AES/header encryption/conflict, empty/subtree/Unicode, independently seeded metadata, source/cancel/handoff/unknown and native5/64/latecancel; HZ3 publication2, EIC2, ZIP Ask/handoff, TAR actions/native failure/lowspace, global admission/cancel and HZ4 inert gates. `native-current/native-terminals.json` is fresh current-worker data, fed to final Desktop tests. Legacy password/CLI failure names were deliberately not selected.
- Desktop archive **349 PASS**; Protocol91 + IPC43 + Core100 = **234 PASS**; Archive lib44 +11 integration targets58 = **102 PASS**, including TAR raw-header controls. All selected counts are actual logs, not imports or expected totals.
- Strict Desktop/Protocol/IPC and strict Archive isolated-codecs checks pass; `cargo check --workspace --tests` passes. These are check/lint results, **not** a full workspace test-suite run.
- Spec navigation/contracts (118documents/14owners), archive CLI source16 assertions, release/editor provenance, explicit C# test-selection source gate, four Python AST checks, scoped two-file rustfmt and `git diff --check` pass. Whole-workspace fmt was not changed or claimed; the three unrelated network formatting differences remain untouched.

Every Cargo invocation used a fresh idle-lane check and same-process `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, `--locked --offline`, serial tests and full-operation timeout. No duplicate Cargo or name-kill was used.

Production changes are confined to the original Rust encoded-header bridge. C# production Host/native source/commit/queue, protocols/IPC, GPUI, codec writer/options, dependencies/vendor and backend remain unchanged this repair. Original2running/32queued/2GiB/FIFO/cleanup-held slots, immutable ordinal/revision/attempt authority, complete candidate/readback verification, `edit_reads_complete`, FILE-only single publication and three-state publication/unknown-no-retry rules are preserved. No new production diagnostic or cross-process operation is introduced.

## 7. Negative evidence, infrastructure issues and protected drift

All earlier attempts remain in the command ledger and original files:

- Clean product REDs: `compressed-native-red`, `plain-native-red`, `header-adapter-red-v2`, `full-release-build-red`.
- Early header test expectation/attempted private Rust field access and differential harness encoding/raw-LZMA/filename/source-symbol errors are recorded separately, **not product REDs** and not crashpoint PASS.
- First new long-path matrix `current-compressed-seven` had three inventory WinError3 failures: original interruption/receipt PASS, but274-character extended `.nxpart` path /270normal exceeded the private CPython path handling. The FAIL results are retained. Only private evidence-root length was shortened, with all product/disk/reader assertions unchanged; final matrices reran all seven points per profile.
- `process-matrices-v2` wrapper failed its private assumption that normal mode has no passive hook receipt. Its saved normal Job actually exited0/succeeded/one publication/no termination; `normal-current-assertions` verified those disk/terminal facts. Final normal control also uses the corrected assertion, never treats a hook receipt alone as termination.
- Initial protection wrapper expected an owner-spec path inside a baseline that did not contain it. Corrected bookkeeping explicitly separates that spec, rather than ignoring unexpected drift.

Before final documents/checkpoint: **1186 opening files,7 declared changes,1179 unchanged**; separately read owner spec has only the declared contract insertion (reconstructed opening SHA256 `CE6FBFE99059CBF9B2C9383731CA19448827660DABB07EF32FB4C2F48BA5086D`). Final baseline additionally permits only checkpoint prepend, so final expected baseline drift is8 /1178unchanged. Original HZ4 **1645 indexed evidence files were all rehashed unchanged**, and original reports/parent decision remain protected. This is the named relevant-file baseline, not an assertion that every unrelated workspace path or private workstation material was inventoried.

Actual opening checkpoint was saved in full: **217143bytes**, SHA256 `29DD7AD68D18BA20305326D6A897B332A66F1408DF47FF0E28E35ACA1A24F060`. Only a top prepend is permitted. Final `checkpoint-suffix-final.json` proves the **entire actual217143-byte opening is an exact suffix**, rather than matching only a title or a historical shorter prefix. No reset/staging/commit/push or overwrite of old reports occurred.

## 8. Remaining blockers and explicit unrun layers

- **Await parent narrow HZ4 acceptance.** This implementation does not self-accept Step6 or dispatch Step7.
- Pinned bsdtar's demonstrated partial-inline-WinAttributes limitation remains real and explicitly unsupported; other structures/versions are not exempt.
- No trusted durable archive-job restart journal/parser/resume/cleanup authority exists. Fresh `stale_operation` is safe refusal, not resumed recovery. Process termination is not power loss, rename durability, real disk exhaustion or network loss; native112/5/64 are owned injections.
- Historical password missing-pending, CLI `first_original_test_admitted`/`helper_untrusted`, readmission/source_unavailable/FB-AC05 issues remain **OPEN**. No broad password/trust fix, unstable rerun-green campaign or self-app GUI bypass.
- This repair did not rerun the whole historical53-process matrix, full C# fixture suite, full workspace test suite, fullnative/100k/performance/large-resource cases, a Release worker, real user/Explorer/installed Host, physical user interaction, power-loss test or release package. Other original-format crash results remain their original frozen-bundle evidence, supplemented only by the stated current regression selection.
- No real-user archive write/delete/truncate/punch/reclaim, new recovery controller/sidecar cleanup, Registry/service/ACL/UserChoice/Recycle action, package/sign/install, key/ledger/installer mutation or publishing sequence allocation. HZ5 and experience package require the parent's later authorization after stable acceptance.

Detailed point/accounting/command index: `hz4-7z-release-repair-matrix.md`. Stop after the final seal for automatic host completion callback and parent narrow acceptance; do not send a duplicate callback task.
