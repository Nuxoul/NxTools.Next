# 7z risk / finite output readmission — bounded implementation outcomes

2026-10-24. Authorized remainder of **Phase A — Engine contracts and safety foundation** only.
Delivery level: **7z风险/有限预算再准入切片待独立验收**. No independent acceptance was performed in
this round; no full B3/SAFE04/Phase A/Phase F/first-release completion claim. Only
[phase-a-checkpoint.md](phase-a-checkpoint.md) owns current status. The earlier UI/Explorer deferral,
ZIP acceptances and independently accepted ordinary CLI/bounded lifecycle remain in force.

## 1. Scope, references and changes

Read the existing task/exit ledger/matrix and ZIP risk/readmission references/acceptances before
implementation. Later source already connected 7z extraction preflight to original ZipRisk,
InteractionPort and PrivateOutput::preflight_output; the historical B3 row was not treated as proof
that all current hooks were missing. No replacement backend/framework or dependency/vendor change.

Verified exact locked sevenz-rust2 0.23.0 archive.rs/reader.rs stream maps and supplied SupaZip
sevenz.rs/traits.rs. compressed_size belongs only to the first file/first packed stream; global
is_solid() means any shared block, not per-member attribution. Original legacy NxTools package-ZIP
owner is unrelated. New findings are indexed in [archive reference](../../../docs/reference-code/archive.md#7z-riskreadmission-exact-metadata-follow-up-2026-10-24).
[Pre-code slice contract](sevenz-risk-readmission-contract.md) and the approved implement.md amendment
fix this round's scope. Executable seven-section owner contract:
[7z safety](../../spec/nxtools-archive/backend/sevenz-safety.md).

Two reproduced defects required four production-file changes:
- worker_codecs.rs / worker_codecs/volumes.rs: sevenz-only source entry separates finite encoded
  source admission from deficient output allowance for opt-in extraction readmission.
- ArchiveWorkerRunner.cs / ArchiveVolumeSources.cs: original source-acquire passes extract+
  OutputReadmission opt-in; only an actual 7z signature under the already-held original source lease
  selects the existing default100GiB-or-original encoded ceiling. Family/native source checks remain.
- worker_codecs.rs also refuses ambiguous/orphan packed spans instead of inflating the ratio
  denominator. No new multi-input codec support.

Original output_readmission/interaction/zip_risk/worker/worker_job/worker_password/product_job and
ArchiveJobSession are unchanged against this round's saved hashes. Rust still owns policy/tickets/
state/coordinator; C# only transports original replies and owns Windows/process/Job/handles/output.
Tests add two Rust integration files, one source unit case, two C# fixture files, exact harness
registration/effects and the independent tiny sample producer. No public protocol/CLI/UI change.

## 2. Red -> green and retained failures

Single evidence root (all names below are relative to it):
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\sevenz-risk-readmission-20261024-125931`.

| Owner / regression | Genuine red | Repair / green |
| --- | --- | --- |
| Actual Rust worker, total allowance1 below encoded length | 04-total-shortage-red.log: expected original output_budget_shortfall absent; premature encoded-source ResourceLimit | SourceSet::open_sevenz -> open_bounded with max(original output, existing default100GiB), only readmission opt-in. 05 green; final22 repeats PASS |
| Original C# runner/source acquisition | 13-native-low-red.log: exact low-total test FAIL, source_unavailable before native output/declaration | Held-source actual7z signature/finite encoded selection. 14 build,15 PASS; final29/30 repeats PASS |
| Exact metadata attribution | 07-packed-attribution-red.log: unused trailing packed stream produced Some(2), expected None | Current supported single-input block span must equal1; otherwise unknown. 08 one source-unit PASS |

These are two defects with three owner-level red proofs, not three unrelated defects. The orphan
fixture exercises structural metadata attribution only, not a complete payload decode oracle.

Retained non-product/fixture failures are not red proofs: 03 direct probe had a wrong strict request;
06 lacked SubStreamsInfo; 10 was fixture E0382 moved apphost; 11 aborted after identifying native
low-total acquisition; 18 fixture JSON used Comment rather than the original snake_case schema.
The fixture fixes were local (clone / JsonDefaults.Options), not protocol/product relaxation.
16 passed the earlier eight-scenario version;20 separately passed selected/actual additions;
**31 is the final unfiltered ten-scenario run**, not a sum of earlier directories.

Final audit tooling mistakes are retained too: invalid-array-probe-final-*.json treated a PowerShell
JSON array as one record;37a had CRLF comparison mismatch;37b had a missing script local variable.
The corrected37 audit passes. None is a product regression or a reason to rerun a passed matrix.

## 3. Independent tiny samples and evidence identity

Stock7-Zip26.00 is only a fixture producer/oracle, never a product backend:
`D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`, SHA256
`4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.
prepare-sevenz-safety-fixtures.py creates and independently l/t/x-so compares fixed literals:
- first.bin:4096bytes, cyclic0..255x16, SHA c8f5d0341d54d951a71b136e6e2afcb14d11ed8489a7ae126a8fee0df6ecf193.
- second.bin:2048bytes of B, SHA90c4a574cd6699066e08ec614a847b977c61a5e764d9b0d2b2c4d06583fcf208.
- nonsolid.7z:448bytes, SHA89136ee08dc1895a9c0f3ac0fbd1b76e224412657fc6bf799ccdb5fedf248e4f.
- solid.7z:434bytes, SHAb7efbecb1b46d0e2d7a13a75fee5157904a175299f7d2435cce794acbf24949c.
- corrupt-solid.7z:434bytes; only own packed byte35 is mutated, SHAc3f8c0cc04b6dc2f03cfb55546ad64e1d4d4b16301ce1c7f7f591aa4c422491b.

Production options: -m0=LZMA2 -md=64k -mmt=off -mhc=off -ms=off|on. Exact stock commands, inputs,
archives and hashes are in fixtures/manifest.json and individual stdout/stderr. No bomb/large file.

Final matching isolated-codecs worker SHA256:
**92C85AAF4253F34668F9155F816A4D72E2CF25A332490FA501765D741AF31159**.
Native4, chain10 and separate TEST1 each recorded identical before/after hashes in native-worker-
hashes.json, chain-worker-hashes.json and test-no-risk-worker-hashes.json.37 also rehashes the staged
identity-input worker. Original identity generator wrote only scratch ReleaseBinaryManifest.g.cs /
ArchiveReleaseManifest.g.cs, not checked-in defaults or a signed release.

## 4. Targeted command/results ledger

Every Cargo command checked the shared cargo/rustc lane, waited/refused rather than colliding with
other owners, and used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256.
Initial isolated C# host preceded native work. Final strict/features checks preceded the matching
worker rebuild25, original scratch identity generation26, final host/tests builds27/28 and native
LIST29 -> RUN30. Test-only no-Risk addition then built32 -> LIST33 -> RUN34; **4+1 separate batches**,
not one five-case invocation. All relevant C# builds finished with zero warnings/errors.

Commands/targets (PowerShell log redirection and explicit scratch paths are omitted below):

```text
cargo test -p nxtools-archive --features isolated-codecs --test sevenz_readmission_contract -- --test-threads=1
  04 RED ->05 PASS1; final22 repeats the same case
cargo test -p nxtools-archive --features isolated-codecs --lib worker_codecs::packed_risk_tests::orphan_packed_stream_is_not_an_independent_member_denominator -- --exact
  07 RED ->08 PASS1, filtered21
cargo clippy -p nxtools-archive --features isolated-codecs --tests -- -D warnings
  21 PASS
cargo test -p nxtools-archive --features isolated-codecs --test output_readmission_contract --test sevenz_readmission_contract -- --test-threads=1
  22 PASS3+1, failed/ignored/filtered0
cargo clippy -p nxtools-archive --tests -- -D warnings
  23 PASS default features
cargo tree -p nxtools-archive --features isolated-codecs -e features
  24 final feature evidence
cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker
  25 PASS
cargo test -p nxtools-archive --features isolated-codecs --test sevenz_readmission_chain -- --test-threads=1
  31 PASS1 test /10 scenarios, failed/ignored/filtered0
rustfmt --check --edition 2024 --config skip_children=true <four touched Rust files>
  36 PASS
python tests/architecture/spec-contracts.py
  35 PASS112 documents/14owners; document structure/navigation only
```

The two source files and two new Rust tests are the four rustfmt targets. C# final outputs are
final-host and final-tests, using explicit scratch ReleaseBinaryManifestPath/ArchiveReleaseManifestPath
with original helper identity inputs. Exact C# selection/ordering is below; using the four exact
names as separate --fixtures filters recreates batch29/30 without accidentally including the later
TEST case. --list enumerates only; the matching invocation without --list executes. Logs, not this
reproduction description, are the observation of what ran.

| Final batch | Exact test name, in LIST/RUN/PASS order |
| --- | --- |
| 29/30 | archive 7z safety fixture packed metadata |
| 29/30 | archive 7z safety fixture risk and solid semantics |
| 29/30 | archive 7z safety fixture publication cancellation skip |
| 29/30 | archive 7z safety fixture low total preflight |
| 33/34 (separate) | archive 7z safety fixture TEST no risk interaction |

29/30: selected4/run4/pass4/skip0/fail0/not_run0;33/34: selected1/run1/pass1/skip0/fail0/not_run0.
37-final-evidence-audit.log/json checks equal names and order, exact summaries, final ten scenario
names, worker/fixture hashes, source stability and fixture Python AST.38 scoped git diff --check PASS
(with Git's existing Program.cs CRLF-to-LF warning, no whitespace defect). No optional review pass.

## 5. Actual lifecycle and policy assertions

**Risk native evidence:** ordinary production thresholds produce no false prompt; DEBUG(1,1) only
lowers the existing predicate for tiny nonsolid fixtures. Both wanted members receive fresh original
tickets before any decode/native write/publication. Approve only exact current entry ceiling;
decline/cancel exits without subsequent output. Cross-operation/wrong-ID replies reject at the
original runner; transported larger/wrong-generation/duplicate-old replies cannot satisfy Rust's
pending ticket. Existing and foreign .nxpart bytes remain untouched on refusal. Shared-solid packed
is null per member, while actual entry/total limits still reject; there is no fake member ratio.

**TEST native evidence:** ordinary no-output TEST succeeds, no Risk prompt/confirmation/publication.
It did NOT use lowered thresholds: the existing DEBUG seam is extract-only. Release omission of the
test field + deny_unknown_fields / C# #if DEBUG was source-inspected; a dedicated Release worker
runtime-rejection test was **NOT RUN** this round. No new read/CLI test or silent authority granted.

**Original Rust coordinator -> actual native runner:** new SevenzReadmissionHost is fixture-only
stdio transport, with no approval/retry/budget policy. Every callback forwards original WorkerRequest
and returns original shortage/terminal only after exited, DrainAsync true, ActiveCount0 and exclusive
source+helper reopen. At the held-cleanup checkpoint ActiveCount1, source/helper remain protected,
no exited credit and no pending OutputBudget prompt; only explicit fixture release lets cleanup finish.
Per-attempt audit retains native result counts, elapsed/native errors, output effects and journal.

Final31 runs exactly these scenarios (final-chain-index.json selects the final successful directory
for each; earlier successful directories in chain-summaries.json are not added to the count):

| Scenario | Attempts and observed assertion |
| --- | --- |
| entry | Declared entry shortage; no first effects; actual cleanup -> explicit finite approval ->2 attempts/success |
| total | total1 below encoded source length; reliable total6144; same cleanup ->2 attempts/success |
| solid_total | Known solid declarations max/sum without full budget-estimation decode ->2 attempts/success |
| cancel | Decline original OutputBudget ->1 attempt/Cancelled |
| shared_cancel | Shared cancellation while pending ->1 attempt/Cancelled, no second dispatch |
| source_changed | Change source only after real cleanup ->2 attempts/source_changed, no output |
| selected_success | Original source-bound selection ordinal0 preserved ->2 attempts/one4096byte file |
| actual | Selected solid declaration4096 fits, required block decode6144 exceeds total4096 ->ResourceLimit, zero commits/no shortfall,1 attempt |
| second_actual | First entry shortage readmitted once; same actual solid overflow ->2 attempts/no third or pending prompt |
| committed | Original postpublication cancel ->1 attempt/cancelled_with_committed_output; one2048byte second.bin, Committed/journal retained |

Serialized original requests are compared in memory (never dumped): only attempt/control ID,
readmission/source-binding fields and two output limits may differ. Logical ID/shared cancellation
remain; codepage936, password presence, auto_rename, MOTWfalse and selected ordinals are preserved.
Original work+metadata+64MiB overhead Job memory is identical across attempts; dictionary/work/
metadata limits do not rise. Native source/version identity and Rust digest are carried into fresh
admission. Original guard contracts3 also reject incomplete cleanup, commits, wrong result ID/current
limits, hard-ceiling shortages, ordinary actual failure and invalid/stale finite approvals.

**Solid original path:** skip preserves old first.bin, still decodes/checks remaining block then
publishes second.bin2048. Owned packed corruption reports CorruptArchive without invalid publication
or replacing old bytes. Postcommit cancel preserves one2048byte native journal and removes only
unpublished owned partials; original archive remains. This is not rollback or output merging.

## 6. Support boundaries, cost and preservation

Known ratio: streamed independent one-member block with one uniquely attributed packed stream in
current supported single-input coder chains. Unknown: shared solid, empty/no-stream, ambiguous/orphan
or unsupported multi-input attribution. Never use zero/shared division/whole-archive ratio. Original
predicate >64MiB AND >1000x remains unchanged. Unknown ratio is not exemption from actual budgets.
Risk confirms the original ceiling; OutputBudget is a distinct explicit new finite admission under
32GiB/file and2^50total, only after declaration/zero effects/actual cleanup, at most once.

Cost increases: C# six-byte held-source signature read; low-output opt-in7z can now scan encoded input
up to existing100GiB-or-original finite bound instead of failing prematurely. A readmitted operation
repeats original source hashing/header/plan checks and normally decodes on the second attempt.
No complete pre-extraction is added to estimate output. Existing password preparation can already
perform a whole-archive no-output TEST, which remains a separate cost. Solid selected extraction may
need unselected decode and fail actual limits even after an accurate selected-output approval.

Preservation observations:
- final-release-audit.json: **80/80** saved sequence26 delivery files unchanged, including setup,
  release manifest/signature. final-installed-audit.json: six protected binary hashes equal package.
  Updater state still Sequence26 / e0a8c392-0bb3-44e6-b573-4d17ba35863b /
  manifest23DE45C00BE1BFBF12EAB3E9BD2A5EE4E789E3EA330F5C4355D379357FB4D515.
  File identity observation only, not in-memory-image/service/search or UI acceptance.
- final-baseline-audit.json and round-diff-* compare17 actual pre-round snapshots, not cumulative
  git diff HEAD. Original coordinator/policy owners and both ZIP acceptances match. Four production
  files are this round's bounded repair; ArchiveVolumeSources.cs was NOT in that initial17-file
  baseline, so its pre-edit attribution relies on the recorded small Edit/current source and the
  post-fix pre-final hash, not an invented opening hash. Final four production hashes stay stable.
- Eight CLI/lifecycle native-owner/generated paths match the earlier lifecycle baseline; checked-in
  identity defaults were not regenerated. Historical ledger/matrix, both ZIP acceptances and root
  Cargo.toml match the older saved baseline. Cargo.lock differs from that older snapshot only by
  existing CLI internal edges (async-channel/archive/protocol/tokio), not a new7z codec release;
  no current-round lock snapshot is claimed. Preserve existing dirty/concurrent work.
- Final own-process observation:0 executables under this round root. No process kill of other owners,
  generated package overwrite, signing/sequence reservation/install/service/Registry/COM/recycle/
  P209/elevation/commit/push or GUI safety-denial bypass. Scratch fixtures/evidence are retained.

Not run/not accepted here: dedicated Release runtime fixture rejection; all7z methods/encryption/
header/volume/resource cross-products; multi-input codec graphs; full Desktop/workspace or accepted
CLI collection; 100k/1GiB/4GiB/long-path/performance; real UI/Explorer/HWND/capture/button continuity/
foreignCOM/DPI/UIA. Previous source-drag RUN/partial and deferred fresh sample remain unchanged.
RAR/TAR/stream risk/readmission remainders and prior A–E/JOB07/SAFE08/UNC/recycling/P206/P207live
obligations remain separately pending; ProductApplyAuthorized=false. No automatic next-format work.

**Stop: 7z风险/有限预算再准入切片待独立验收。**
