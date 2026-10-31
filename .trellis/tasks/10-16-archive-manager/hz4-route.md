# Step 6 — HZ4: Real-process crash / restart route

Status: BEFORE-CODE contract, 2026-10-29. Same **Phase E — Format expansion and ZIP editing**; only original seven-step Step6, not HZ5. Parent task04a073dc-2430-4cfd-9378-37c0dab566b7; HZ3+AC01–03 acceptance is preserved. Results and acceptance remain in the sole checkpoint, not this plan.

## Discovery and authority

Opening checkpoint209948bytes/SHA256 `95B653E959C51E0B57217D9550C25D0E819314F94F06FC797E8855D4A4FCC8E8` is fully saved in session scratch `hz4/checkpoint-before.bin`. Fresh991-file baseline includes protected owners and historical reports. HatchZip nine reference files match recorded hashes; actual debug crash gate/commit/keep/CLI child/formats/streaming/restore/CI read, not executed. Verified source map is appended to [reference note](../../../docs/reference-code/hatchzip/crash-recovery-tests.md).

**Current product has no durable archive-job restart/resume journal or parser.** Native CommitJournal is an in-memory history projected after cleanup, not a recovery log. Original JobSession has only process-local active/retired records; fresh process `resume` rejects `stale_operation`. Cache/update recovery is unrelated. This task does not invent a second recovery/history/scheduler or ABI. Recovery success, automatic resume and restart partial cleanup remain UNSUPPORTED. Fresh-process evidence will exercise that actual refusal and preserve all disk evidence. A fixture's disk request/receipt/terminal is private evidence only, never authority to delete/replay, even if syntactically valid. Missing/malformed/old/new schema claims cannot grant unsupported recovery authority.

## Invariants

- Ordinary extraction source bytes/metadata remain unchanged; outputs are real complete/partial/native-published sets, including directories, not manufactured rollback. Owned unpublished partials left by process death are retained.
- Edits ZIP/7z/TAR/TGZ/TXZ use original candidate/full verification/actual reader closure/edit_reads_complete/FILE-only handoff/one native publication. Original must independently validate complete old or complete new, never half replacement. No delete-old-first or sharing relaxation.
- Original2running/32queued/2GiB, FIFO/head overlap/cleanup slots, immutable source+attempt/generation+ordinal, HZ3 three-state output_published/real result_location, retirement64, EIC/TARraw safeguards unchanged.
- Test processes are exact Python Popen-owned testapphosts in GUID scratch packages. Save retained process handle, PID, creation FILETIME/image and parent PID; validate checkpoint receipt PID/nonce/operation/stage/visit, process still alive and worker-child ownership before termination. Kill only retained exact handle. Job kills only its original assigned worker after Host death. No name kills, product/installed/Explorer/service/Registry/ACL actions.
- All controls live in test assembly, debug-only with explicit private capability/one-use nonce. Existing internal probes only; no production env abort switch. Release fixture entry rejects before filesystem/native execution and must be actually built/run in scratch. No Rust controls added; frozen current worker source/hash reverified/built.
- No source reclaim/delete/truncate/punch or expanded source FILE access. HZ5 remains Step7/default off. Process death is not power loss, network outage or Windows rename durability proof.

## Exact seam selection

| Operation | Point | Meaning / expected disk state |
| --- | --- | --- |
| Edit all5 | edit_verifying | ZIP already codec-verified candidate spool;7z encoded candidate before full verify;TAR inner candidate before verify/outer encode; no native publication |
| Edit all5 | candidate_write | original native staging write; owned candidate may remain; old source intact |
| Edit all5 | edit_source_rechecked | full candidate/held readback verified, actual worker readers closed/completion accepted; source FILE and parents still protected |
| Edit all5 | commit_in_flight | original FILE-only release complete, single native rename not yet called; old source intact |
| Edit all5 | native_published | real native success/publication fact recorded, before output close; new complete source |
| Edit all5 | before_cleanup | exact worker/pumps drained; original resources still held; new complete source |
| Edit all5 | after_cleanup | original terminal observed after actual disposal/freeze; new complete source |
| Extract ZIP/7z/TAR | candidate_write, commit_in_flight, native_published visit1/2, before_cleanup, after_cleanup | real partial vs first/all complete files; source preserved; file and directory inventory explicit |

Hooks use original SourceCheckpointProbe/OutputWriteCheckpointProbe/OutputCommitCheckpointProbe/CleanupCheckpointProbe. Add only a debug-rejected internal fixture connector if original Job facade does not expose existing source/write/cleanup probes. after_cleanup is the fixture observing the original terminal, not a new durable seam. No per-format fictional journal seams.

## RED / GREEN and matrix

1. Tiny ZIP edit `edit_verifying` and extract `before_cleanup` with only current exposed commit probe: actual admitted Job/worker/native success but requested hook absent -> test driver FAIL. This is missing instrumentation RED, not product corruption or arbitrary launch failure. Preserve source/output/oracle and terminal proof.
2. Connect existing probes minimally, rerun those exact cases; require receipt visit before exact termination and fresh child refusal. Preserve every failed run separately.
3. Run above format/point matrix, plus repeated interruption then fresh restart, missing/malformed/unsupported schema claims, owned source/target replacement/hardlink/reparse conflicts, original injected112/flush/native5/64 cases and necessary password cancel/IPC-loss coverage. Unsupported restart always refuses without replay/cleanup. New explicit attempt is not automatic recovery; logical-member duplication remains covered through original Desktop/HZ3 tests.
4. Independent CPython zipfile/tarfile, explicit bsdtar and stock7Zip oracles: exact file sets/kinds/length/content hashes/archive metadata, old/new encoded hashes; source/native identities, retained partials, private record schema/hash, unrelated canary unchanged. Encrypted fixtures are not claimed unless an oracle actually opens them.

## Diagnostics / evidence contract

Reuse existing production accepted/started/source or fixed edit phase/native publication/terminal with operation correlation, stage/outcome/count/elapsed/native code; no new production logger or payload. Private fixture receipts record schema1, nonce, operation/session/logical/attempt IDs, stage, exact visit, process PID/creation/image/content DLL hash, monotonic elapsed; one bounded receipt per selected seam, no chunk logs. Nonce and paths remain private evidence only. Process killed before terminal has **no terminal/cleanup-complete assertion**; fresh process cannot reconstruct publication from memory. Disk inventory/oracles independently determine facts and preserve unknowns.

Each case retains exact command argv/exit/time, before/after inventories and metadata, receipt, exact handle/creation/parent proof, child exit, fresh restart identity/refusal, native baseline/refusal where relevant, hashes and PASS/FAIL/SKIP. Test envelope is not a product durable record. Journal absence is a measured limitation, not a synthetic durable journal success.

## Verification and stop

Effects audit -> exact LIST -> identical-name RUN for C# affected native fixtures; skip is not pass. Freeze current worker/actual TestsDLL+HostDLL and explicit oracle/env hashes. Required same-process Cargo settings jobs1/incremental0/codegen-units256, idle lane before every invocation, locked/offline/serial and adequate timeout. Initial headroom ~16GiB commit/13GiB physical; recheck before builds. Actual compiled Release test entry refusal goes to owned scratch, no packaging/sign/install/manifest override.

Affected Desktop archive/HZ3/EIC, Protocol/IPC/Core, Archive targeted codec/native tests plus strict/workspace/scopedfmt/diff/spec and framework/editor provenance gates. Do not rerun old readmission instability seeking green. Keep source_unavailable/helper_untrusted/FB-AC05 and physical GUI/drop/DPI/IME/UIA/user/installed/release/power-loss OPEN. Deliver hz4-outcomes.md and hz4-matrix.md, commands/cases/drift/limitations; reread checkpoint then prepend only, prove entire209948byte opening exact suffix. Stop for host callback and parent independent acceptance; do not launch HZ5.
