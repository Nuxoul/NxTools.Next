# 第六步 HZ4：真实进程崩溃／恢复测试 — outcomes

Date: 2026-10-29. Phase remains **Phase E — Format expansion and ZIP editing**. Original seven-step **Step 6 only**; parent task `04a073dc-2430-4cfd-9378-37c0dab566b7`. Test delivery for independent parent acceptance / bounded repair, **NOT an all-green HZ4 acceptance and NOT authorization for HZ5**. No additional agent/session/review was used.

## 1. Outcome and acceptance boundary

- A real original Job/worker/native process harness is implemented. Exact retained `Popen` process handle/PID/creation FILETIME/image, actual Toolhelp parent ownership and selected nonce/operation/session/stage/visit receipt are verified before `TerminateProcess`. The original Job owns and releases its exact worker. No process-name termination or live product/installed Host/Explorer/service launch.
- Ordinary ZIP/7z/TAR extraction: all **18/18** selected real write/before-publication/first-or-second-publication/cleanup boundaries passed, with unchanged source bytes/native identity/metadata and exact actual published-file inventories. Directories and unpublished partials remain real and are retained.
- ZIP/TAR/TGZ/TXZ edits: **28/28** selected real boundaries passed. Candidate/full verification, actual reader closure, FILE-only handoff, native before/after-publication and cleanup are measured at their actual format-specific positions. Independent complete old/new archive/member/metadata checks passed; no half replacement, delete-old-first or replay.
- Original 53-case compressed-header matrix: **46 PASS / 7 FAIL / 0 SKIP**. The 7 failures are all 7z editing, retained as failures. Explicit plain-header differential: four prepublication points passed; three postpublication points remain **FAIL** because stock7Zip and bsdtar disagree. Those three now also have successful fresh-process refusal and complete stock7Zip exact-member/content/metadata evidence; they are NOT changed to PASS.
- Restart supplements: latest nine logical groups pass (five untrusted-claim variants; four identity-conflict groups; repeated two-attempt interruption; three native-injection groups). They contain **14 fresh-process refusals**, not resume successes. The earlier supplemental run's two fixture-assumption failures remain immutable.
- Native regression selection: **11/13 PASS, 2 FAIL, 0 SKIP**. The two unresolved original product-helper-dependent fixtures are disclosed below, not hidden or counted as crashpoint coverage.

**Current product has no durable archive-job restart journal/parser/resume authority.** Native `CommitJournal` is process-local state projected into a terminal after cleanup, not a durable recovery log. Each fresh process calls the original Session `resume` and obtains `stale_operation`; it performs no start, replay, claim ingestion or cleanup. All source/output/partial/claim/canary disk facts are unchanged by refusal. Recovery/resume/automatic partial cleanup remain UNSUPPORTED. Private claims, capabilities, receipts, commands and terminal JSON are evidence, never authority to delete/replay user objects. No new history/recovery scheduler or ABI was added.

## 2. Minimal implementation and RED → GREEN

Product change: only `ArchiveJobSession.ProbeLifecycleForFixture`, a DEBUG-only connector to the existing SourceCheckpointProbe, OutputWriteCheckpointProbe, OutputCommitCheckpointProbe and CleanupCheckpointProbe. Its Release body throws immediately. No codec, candidate verifier, native rename/commit owner, source lease access, queue/budget, Desktop renderer, protocol/IPC schema, dependency/vendor/backend or source-reclaim changes.

New tests: `csharp/NxTools.PlatformHost.Tests/Hz4ProcessTests.cs`, `tests/integration/archive-hz4-process.py`, `tests/integration/archive-hz4-restart-cases.py`; two exact fixture registrations and effects classifications in the original test catalog. An explicit GUID-owned capability, live driver PID/creation identity and one-use claimed nonce are required. The control writes one bounded selected receipt and waits; only the owning Python driver terminates its retained exact child. There is no normal-product environment abort switch. `after_cleanup` is a test observation of the original terminal, not an invented durable seam.

1. Initial test registration and required reply-field compilation errors were infrastructure failures; logs `build-red` and `build-red-v2` retain them. They are NOT product/crashpoint REDs.
2. `build-red-v3` successfully compiled the complete original test harness (0 warnings/errors), with unchanged accepted Host/worker. `process-red` then failed the exact two planned cases: `edit-Zip-edit_verifying` and `extract-Zip-before_cleanup-1`. Both actual Jobs succeeded, exited0, cleanup completed and published1/2files respectively; neither requested hook receipt existed. This proves the missing fixture connection, not arbitrary nonzero admission failure.
3. The minimal connector was added and compiled in `build-green` (0 warnings/errors). `process-green` ran the same two cases: **2/2 PASS**, actual selected receipt → precise interruption → independent disk oracle → fresh original Session refusal. Original failures remain separate.
4. Current worker was rebuilt locked/offline and hash-matched before the main matrix. All source protection proof is preserved.

## 3. Failures / real limitations kept OPEN

### HZ4-7Z01 — independently valid stock7Zip compressed-header edit input

The stock7Zip-generated tiny non-solid LZMA2 source contains `first.txt`, `second.txt`, `empty/`. Stock7Zip test/exact extraction and bsdtar read the original successfully. Original browser open succeeds. Actual Modify terminates `corrupt_archive`, zero file commits, before the requested candidate/verification/handoff/publication points. Five requested hooks are absent; the two cleanup hooks occur only on failure, and independent expected-new checks correctly reject those cases. All seven remain FAIL, originals unchanged.

A one-variable fixture differential adds only `-mhc=off`, explicitly labeled `stock7zip-plain`, retaining the same content/empty directory/non-solid setting and all assertions. It reaches all seven real original edit points. This localizes an existing compressed-header-layout compatibility gap; it is not a codec fix or a claim that every encoded-header input fails. No dependency patch or relaxed admission was made.

### HZ4-7Z02 — independent readers disagree on published new 7z

For the explicit plain-header fixture, the three new archives after `native_published`, `before_cleanup`, `after_cleanup` pass stock7Zip test and exact individual member content/hash/time/attribute checks, with one added `published/` directory and every old member preserved. Actual system bsdtar returns1, `Damaged 7-Zip archive`. The fixture still FAILS; the secondary assertion was not removed. It now saves the primary manifest and continues fresh-process refusal before reporting that same failure. Do not assert either whole-format correctness or half-corruption based solely on this disagreement. Resolving parser/writer compatibility requires a separate bounded decision/repair within Step6, not silent codec redesign here.

### Existing native fixture / build / formatting limits

- `archive password fixture actual wait retry cancel`: actual run failed `missing real pending password prompt`. No password interaction success is claimed. Its failure was retained and not rerun seeking green; no independent terminal category was captured by that original fixture, so do not invent its cause.
- `archive cli lifecycle fixture actual cleanup timeout retains ownership`: first attempt lacked the required evidence-root environment. After explicit owned-root configuration and a fresh identical-name LIST→RUN, it failed `first_original_test_admitted`; retained `cli-cleanup-evidence/.../evidence.json` records **helper_untrusted**. It did NOT reach its cleanup fault point. Production helper verification was not bypassed. The main owned-package Host-death matrix separately proves exact worker Job termination, not console-signal/public-CLI coverage.
- Full Release Tests build fails at unchanged `ArchiveRiskTests.cs:72`, CS0162, after actual Release product Host and selection helper compiled. No warnings were disabled and no unrelated source edited.
- Broader `cargo fmt` of five packages fails only in pre-existing `network_settings.rs`, `network_settings/blacklist.rs`, `network_settings/page_rem_tests.rs`. No formatting rewrite was performed. All11 directly relevant archive boundary files pass an explicit read-only rustfmt check.
- Previous HZ3 negative reports, SourceChanged-vs-Cancelled/readmission instability, source_unavailable/helper_untrusted/FB-AC05 and physical UI/drop/DPI/IME/UIA/user/installed/release/power-loss limitations remain OPEN at their original levels. Earlier bounded HZ3+AC01–03 acceptance is not revoked or inflated.

## 4. Actual compiled Release-inert gate

After committed-memory headroom and empty Rust-lane preflight, a scratch-output Release build actually compiled product Host. Full Tests Release build was blocked as above. A scratch-only small probe compiles **the exact same HZ4 source** in Release, references that actual Release Host, and preserves friend assembly identity. It does not replace product code, suppress warnings or masquerade as the full test suite.

- Exact two-name LIST→RUN: **2/2 PASS, 0 SKIP**; compiled fixture Debug gate is false, actual Release owner connector rejects, original fresh Session has no recovery authority.
- Calling the compiled Release fixture process entry with a nonexistent owned path returns2 and fixed `release_fixture_disabled` before path/capability/worker/native execution. The path remains absent. Expected refusal exit2 is not a build failure.
- Release Host and the probe's loaded Host copy have identical SHA256 `AD0FD3BDD95D76B12D2B1F0CC6ED43BC7DE4F57309D82839F444FF55B7E65C57`.
- This is an actual Release owner/exact-source fixture rejection gate, **NOT a successful full Tests Release build, Release worker execution, package, signing, install or release acceptance**. No new Rust crash controls exist.

## 5. Verification performed

| Verification | Result |
| --- | --- |
| Full Debug C# Tests build / matching current worker build | PASS, zero warnings/errors; worker unchanged |
| New HZ4 fixtures exact LIST→RUN, Debug / exact-source Release | 2/2 each, no skips |
| Process main matrix | 46/53 PASS,7z failures retained |
| Explicit7z plain profile | prepublication4 PASS; postpublication3 FAIL, fresh refusals PASS |
| Latest restart/claims/conflicts/repeat/native supplements | 9 logical groups,14 fresh refusals PASS; earlier2 test-assumption failures retained |
| Exact native regression names | 11/13 PASS; password/CLI admission limits above |
| Desktop archive including HZ3 AC01–03, actual fresh native export bridge, EIC and retirement/attempt guards | **349 PASS**,0failed/ignored |
| Protocol / IPC / Core | **91 / 43 / 100 PASS**, Core100=93unit+6storage_management+1storage_runtime; doc tests0 |
| Archive targeted real worker/codec/EIC/TARraw/native handoff/wire suites | **100 PASS**,0failed/ignored |
| Clippy Desktop/Protocol/IPC tests, `-D warnings`; workspace check with tests | PASS |
| Eleven archive owner files, rustfmt read-only / broader five-package fmt | PASS / FAIL unrelated3files disclosed |
| archive-cli static16assertions; spec contracts118docs/14owners | PASS, source-only |
| GPUI release / pinned Markdown editor provenance | PASS, source-only |
| git diff --check | PASS (existing CRLF notices only) |

Every direct Cargo invocation inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` and a pre-launch idle-lane check; build/test/clippy/check used locked/offline flags, whereas fmt is a read-only formatter command. Sufficient timeouts, no duplicate build or name-kill. Rust results use the fresh HZ3 native export from this run, not a prior session's memory/report. Source checks do not certify behavior, and compiled synthetic GPUI tests are not physical interaction acceptance.

## 6. Evidence, exact identities and preservation

Private evidence root (retained): `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz4`.

- `command-ledger.json`: initial61 exact argv/PID/exit/time/env/log hashes, including expected negatives. Individual `<label>.json/.log` remain immutable; post-document checks are recorded separately and in the final seal.
- `evidence-manifest.json`: initial1633 evidence files with absolute path/size/SHA256. `*-final` seal adds the final documents/checkpoint/checks without overwriting this initial snapshot.
- Full cases in `process-red`, `process-green`, `process-matrix`, `sevenz-plain-hypothesis`, `sevenz-plain-boundaries`, `sevenz-disagreement-restart`, `restart-supplement`, `native-supplement-green`: each results.json + identities.json + GUID case-result/evidence files. Exact source/target/partial/native IDs, old/new archive hashes, member hashes/metadata, receipt/terminal/claim hashes, exact process identity/handle and restart refusal are retained. No terminal/journal is invented for a process killed before terminal.
- Native selection: `native-regressions/results.json`, exact effects list, env and each LIST/RUN command; latest CLI native evidence retains helper_untrusted. Fresh original native terminal export at `native-regressions/native-terminals.json` feeds Desktop tests.
- Worker SHA256 `B0DF89DAE9527D0E16471434ABBCDCFA949DAEAD0CCB07878293333491DC7F7C`; Debug Tests DLL `BF4A581D905744FD83D6BD4F7A7D38A9F16855E7E41DFF736E230573315EF4C5`; Debug Host DLL `9759D4C003403C4DE83FD12422F861C6DCF21C071120292FD1A7397AF8171B65`. Tests DLL, not apphost hash, identifies test content. Exact Python/stock7Zip/bsdtar paths and hashes are recorded.
- `source-protection.json`: **991 opening files**, only four declared old-file changes before checkpoint prepend. Removing precisely the4Program lines /2effects lines reproduces opening bytes; removing only the connector reproduces opening JobSession; the reference note retains its exact7717-byte opening prefix. All other987 recorded files, historical reports, Rust owners, codec/backend/dependency/ABI/queue/budget files are unchanged.
- `checkpoint-before.bin`: complete original209948bytes, SHA256 `95B653E959C51E0B57217D9550C25D0E819314F94F06FC797E8855D4A4FCC8E8`. Checkpoint is only top-prepended; final seal must prove this entire opening remains an exact byte suffix. No reset/stage/commit/push or unrelated cleanup.

Process termination is not power loss, filesystem rename durability, whole-disk exhaustion, network loss or disk corruption. Native112/5/64 cases are small owned injections. No user archive writes/delete/truncate/punch, source-reclaim privilege, arbitrary sidecar cleanup, recursive rollback, package/sign/install, Registry/service/ACL/UserChoice/Recycle operation, or self-app GUI refusal bypass occurred. **Stop here for host completion callback and parent independent acceptance / bounded Step6 repair. Do not begin HZ5/Step7.**
