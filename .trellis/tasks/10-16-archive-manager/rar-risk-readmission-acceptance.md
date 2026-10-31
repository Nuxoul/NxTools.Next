# RAR4/RAR5 risk / finite output readmission — independent bounded acceptance

2026-10-24. **PASS for this bounded Phase A safety slice.** No new confirmed actionable defect;
no product/test changes made by independent acceptance. This is not full B3/SAFE04/Phase A/Phase F or
first-release acceptance. Current status belongs to [the sole checkpoint](phase-a-checkpoint.md).
[Implementation outcomes](rar-risk-readmission-outcomes.md), [pre-code contract](rar-risk-readmission-contract.md),
[owner contract](../../spec/nxtools-archive/backend/rar-safety.md) and historical reports remain retained.
UI/Explorer unified interaction acceptance remains **DEFERRED / NOT RUN**, not PASS.

## Review and accepted boundary

Parent independently inspected the four changed production files, original RAR adapter, source lease,
preflight/Risk, output-readmission coordinator and native runner/Job/output boundaries. No optional
second review cycle or unrelated remediation was started.

- Original `run_rar` already entered generic extraction preflight and Risk; no missing-hook rewrite
  was needed. The reproduced defect was low output allowance rejecting encoded input before reliable
  declaration, at Rust and C# source owners. Historical worker/native red logs were read and retained;
  independent execution used final source only, not reverted source or recreated red evidence.
- Windows-only `SourceSet::open_rar` separates encoded admission only for RAR extract plus explicit
  OutputReadmission. Native `ArchiveVolumeSources` verifies actual RAR4/RAR5 signature after acquiring
  the original held lease; original runner selects that opt-in. The finite encoded ceiling is the
  original larger allowance or existing default100GiB. Family discovery, missing-volume, identity,
  version and digest checks remain; output/work/dictionary/metadata/Job authority is not raised.
- Unchanged UnRAR7.1 adapter retains packed ABI, UNKNOWN_SIZE sentinel and split-part semantics.
  Shared/solid/split or ambiguous packed data stays unknown, not an archive-total/average denominator.
  Original reliable-positive-packed predicate remains strictly >64MiB AND >1000x. Risk confirms only
  the immutable original member ceiling, not output-budget permission.
- Original coordinator permits at most one explicitly approved finite fresh admission, only after
  reliable declaration, zero commits/publication and actual cleanup. Fresh control ID, source
  revalidation, shared logical cancellation and original options remain. Only the two approved output
  limits and necessary source bookkeeping change. Actual overflow, source change, corruption, commits
  or incomplete cleanup do not authorize more attempts, output merging or rollback.

## Independent execution and identity

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\rar-independent-20261024-155432`.
Session scratch runner: `rar-independent-verify.ps1`. Raw logs, `commands.jsonl`,
`worker-meta.json`, `14-independent-oracle.json` and `independent-results.json` retain actual results.
Initial isolated C# host came first; strict/features preceded matching worker, original generator
wrote scratch approval inputs, then final host/tests and native chain ran. Every Cargo command used
one shared lane with jobs1/incremental0/codegen256. No signing or checked-in identity replacement.

| Independent check | Actual result |
| --- | --- |
| Initial/final isolated PlatformHost and final Tests builds |PASS,0warnings/errors;01/09/10logs|
| output_readmission_contract + rar_readmission_contract |3+1PASS,0failed/ignored/filtered;03-contracts.log|
| Archive strict Clippy --tests, isolated-codecs and default |bothPASS,-Dwarnings;04/05logs|
| Final feature tree then matching isolated worker build |PASS;06/07logs|
| Exact C# native fixture selection |LIST8=RUN8=PASS8,same names/order,skip0/fail0/not_run0;11/12logs|
| Original Rust coordinator -> actual native runner/Job/output |1test/13scenariosPASS,0failed/ignored/filtered;13-chain.log|
| Worker SHA256 before/after native and chain |63BE2E7BA2A432F52B9AA2D753B5BF008A86EB349C7F393DFE58D868FC5F08C4,identical|
| Four affected Rust files rustfmt |PASS;15-format.log|
| Spec/task/diff documentation checks |PASS;16/17/18logs;spec113documents/14owners,not native acceptance|

The independent worker is a fresh Debug build, not implementation worker922E699B...69B65F or
installed/package27 identity. Neither is substituted into the protected release tuple.

Exact eight native suffixes (prefix `archive RAR safety fixture `), in LIST/RUN/PASS order:

1. ordinary open TEST metadata
2. low total preflight
3. conflicts skip overwrite
4. password preparation
5. postpublication cancellation
6. current budget risk tickets
7. solid skip integrity
8. volume packed unknown

One unfiltered Rust test,
`original_rar_coordinator_requires_cleanup_and_preserves_one_new_admission`, executed all13 scenarios
in order: `rar4_entry`, `rar4_total`, `rar5_entry`, `rar5_total`, `cancel`, `shared_cancel`,
`source_changed`, `selected`, `encrypted_total`, `actual`, `solid_total`, `second_actual`, `committed`.
`NXTOOLS_RAR_CHAIN_CASES` was removed. These are scenarios in ONE test, not13 Rust tests or aggregated
partial runs. All13 attempt records were audited for actual cleanup, active0 and source/helper release.

Entry/total first attempts produce ResourceLimit/zero commits/zero bytes with reliable shortfall,
then one new approved success. Cleanup-held active1/source/helper protection gives no Budget prompt;
actual exit/drain/active0/exclusive reopen precedes approval. IDs/options/source binding remain checked.
Job memory stays138412032bytes across attempts. Decline/shared cancellation do not create a second
attempt; source change rejects it. Unknown-size actual overflow gives no shortfall; second_actual
ends after its second ResourceLimit, with no third attempt. Solid total succeeds with4files/16384bytes.
Postpublication cancellation preserves1file/11bytes, Committed and native journal. Password preparation,
conflict skip/overwrite, stale/larger/cancel Risk replies, solid skip integrity, volume packed-unknown
and missing-member rejection exercise original owners, not new policies.

## Fixture attribution and independent oracle

No RAR creation was added or performed. Small ordinary/encrypted inputs retain published UnRAR test
provenance and notices. Genuine RAR4/RAR5 volume families are copied whole with retained manifests,
not manufactured by splitting archives; this acceptance adds metadata/missing-member evidence, not
another full-volume interoperability matrix.

Three genuine compressed libarchive samples at fixed revision
`79c571f845dbdd250d69bff9d93ae034f674135e` retain raw uu data, hashes and BSD notice:
`test_read_format_rar_compress_normal.rar`, `test_read_format_rar5_compressed.rar`,
`test_read_format_rar5_multiple_files_solid.rar`. Independently stock-listed/tested and individually
decoded with7-Zip26.00 SHA256
`4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f`.
All8 member length/hash comparisons passed:3RAR4regular members,1RAR5member,4solid members.
Native assertions compare independent expected data, not self-roundtrip output. RAR4 link handling
is not counted as another successful regular-file decode. Positive Risk fixtures lower DEBUG
thresholds against genuine compressed data; they do not claim production-size bomb testing.

`rar4-unknown-size.rar` and `rar5-solid-first-unknown.rar` are explicitly header/CRC-mutated
**structural negative copies** retaining original packed payloads. They prove actual callback limits
and no third attempt, NOT independently produced samples or general unknown-size compatibility.
No RAR4solid/all-method/encryption/volume cross-matrix, executable launch or full predecode estimator.

## Preservation, concurrency and remaining obligations

Opening baseline contains3200files. The initial all-unchanged audit rejected5 concurrent non-RAR
changes; this failed audit is not a product failure or frozen-workspace proof. Only these separately
identified paths were allowed by the subsequent preservation audit, with no resets or edits here:

- `csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs`
- `rust/apps/nxtools-desktop/src/main_window/tools_page.rs`
- `rust/apps/nxtools-desktop/src/ui/components/native_context_menu_tests.rs`
- `rust/apps/nxtools-desktop/src/ui/components/styled_menu_surface.rs`
- `rust/crates/nxtools-protocol/src/lib.rs`

They are ordinary pipe/menu/UI/protocol-index changes, not the RAR owner/tests. Dedicated RAR stdio
host execution does not traverse ordinary Desktop routing. This limits whole-workspace conclusions;
no exact before/after semantic claim is inferred from cumulative HEAD diff. All other opening files
were unchanged before report writes, missing0. Acceptance writes only this additive report, the sole
checkpoint and scratch evidence; historical checkpoint body is retained below the new status.

All84 monitored package/installer files remain unchanged; six protected Desktop/Host/Indexer/Worker/
helperEXE/helperDLL hashes match packaged27. No current-evidence-root residual process was observed.
These checks are not a full release-ledger audit, service restart or live installation acceptance.
Installer27 SHA256 remains
`BFB2C30EFA1798262BDBD2F72B879D6FB3E01561050669A993074F15D4F8E49B`.
**RAR changes are not in that previously generated installer. No packaging is authorized by this
acceptance request.** No installation, product startup, service/bootstrap mutation, Registry/COM,
UNC expansion, recycling, P209, dependency/vendor update, commit or push was performed.

At final documentation closure,5 additional concurrent files changed after the preceding audit:
`csharp/NxTools.PlatformHost.Tests/Program.cs`, `csharp/NxTools.PlatformHost.Tests/TestEffects.cs`,
`rust/apps/nxtools-desktop/src/network_settings.rs`,
`rust/apps/nxtools-desktop/src/ui/components/native_context_menu.rs`,
`rust/crates/nxtools-protocol/src/system_proxy.rs`. RAR's eight registration names remain present;
these proxy/menu/test-registration changes are preserved, not accepted or freshly compiled here.
The resulting10-file concurrency boundary limits PASS to tested isolated binaries and unchanged
RAR owners/tests, not the final whole workspace. Initial final-audit drift rejection is retained in
`22a-concurrent-drift.json`; the scratch comparator's earlier separator-newline assertion was also
corrected without editing historical checkpoint text. No product failure or broad rerun is inferred.

Final documentation/preservation follow-up logs are `19-final-spec.log`, `20-final-task.log`,
`21-final-diff.log` and `22-final-preservation.json` in the same evidence root.
No full Desktop/CLI/workspace/accepted-format matrix,100k/large/performance run or extra review loop.
Dedicated Release fixture-field runtime rejection remains NOT RUN; strict cfg/parser source evidence
is not runtime acceptance. Bounded encoded/header/source rereads and a fresh admission are the cost;
original password whole-archive no-output TEST remains, with no new full budget-estimation decode.

UI/Explorer/foreign COM/DPI/UIA and P206/P207 live gates remain deferred/unaccepted. TAR/stream safety,
trusted UNC, strict source recycling/default-off native_protection_unproven/NotAttempted,
JOB07/SAFE08 and earlier A–F obligations remain open; ProductApplyAuthorized=false. ZIP/7z and CLI
acceptances stay closed. Stop at **RAR4/RAR5 finite source/native slice accepted**, not whole-phase or
first-release completion, and do not automatically advance to another format or package.
