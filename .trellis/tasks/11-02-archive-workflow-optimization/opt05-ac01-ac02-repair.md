# OPT-05 — O5-AC01/02 directed repair

## Result

**Both findings are repaired and pass directed automated/owned-native verification. Independent closeout remains pending.** The original [independent partial verdict](opt05-independent-acceptance.md), delivery records and OPT-02/03/04 independent acceptance retain their attribution. This is repair evidence, not another broad independent review.

The archive remains **Phase E — Format expansion and ZIP editing**. This work stops at **OPT-05 — Reusable format and update plans**; OPT-06/07 are unstarted. The [parent checkpoint](../10-16-archive-manager/phase-a-checkpoint.md) remains status authority.

## O5-AC01 — production role-separated native observations

`ArchiveWorkerRunner.DiagnosticTerminal` aggregates the existing `Run.Source.ReadObservation` and primary output observation after the original cleanup and terminal resolution. It retains an established source even if `BindEditSource` verification was interrupted before the bridge stored that source. No hash/read, codec work, source authority, wire field, publication decision or retry is added.

The original `archive.worker.completed/cancelled/failed` event exports these fixed numeric fields under the original operation correlation:

- `native_source_attempts/full_reads/read_bytes/completed_bytes`;
- `native_target_attempts/full_reads/read_bytes/completed_bytes`;
- `native_output_attempts/full_reads/read_bytes/completed_bytes`;
- independent `partial_read_calls` and `partial_read_bytes`.

Actual bytes include interrupted reads. Complete-read credit requires EOF and digest completion. Category, native error, elapsed time, commit facts and actual cleanup are copied from the same resolved event that is published. Projection/sink exceptions cannot change that event. No paths, identities, hashes, members, payloads or per-chunk events enter diagnostics. The older `archive.output.*` totals remain local to their output owner, not total source I/O.

`Opt05NativeObservationTests.Terminal.cs` uses the actual browser, worker, Run and same-path native replacement. The production sink assertion was red for both success and established-lease cancellation before the terminal export; the same fixture is green with the export. A third mode rejects every log write and still succeeds with one publication and actual cleanup.

Final integrated run, original source **262,266 bytes**, candidate **262,304 bytes**:

| Terminal case / role | Attempts | Full reads | Actual bytes | Completed bytes |
| --- | ---: | ---: | ---: | ---: |
| Success / source | 5 | 5 | 1,311,330 | 1,311,330 |
| Success / target | 2 | 2 | 524,532 | 524,532 |
| Success / output | 2 | 2 | 524,608 | 524,608 |
| Cancel EditVerification / source | 2 | 1 | 327,802 | 262,266 |
| Cancel EditVerification / target | 1 | 1 | 262,266 | 262,266 |
| Cancel EditVerification / output | 0 | 0 | 0 | 0 |

Success held-partial readback is **6 calls / 262,304 bytes**, including EOF; cancelled readback is zero. The interrupted source attempt returned **65,536 bytes** without receiving another full-read credit. Success and rejecting-sink modes each publish once; cancellation publishes nothing and leaves the source unchanged. All three finish actual Run drain, retire owned partials and permit an exclusive source reopen. Source/target's seven reads exclude the separate two output reads.

The instance-only runner fixture callback installs the existing observation chunk probe after the source lease belongs to the Run. Production callers install no probe. All five fresh source stages, single-use reader completion, actual reader close, FILE-only handoff, parent/candidate protection and native publication precedence remain unchanged.

## O5-AC02 — all-Skip fresh qualification regression

The checked-in actual-worker `eic-edit-repair.py` assertion now protects both implicit and explicit merged directories:

| Case | Eligibility | Local checks | Descriptor checks | Member/source/addition/candidate decodes | Begin / commit | Source unchanged |
| --- | ---: | ---: | ---: | --- | --- | --- |
| Implicit all-Skip | 1 | 2 | 2 | 0 / 0 / 0 / 0 | 0 / 0 | Yes |
| Explicit all-Skip | 1 | 3 | 3 | 0 / 0 / 0 / 0 | 0 / 0 | Yes |

The original prompt, Skip summary, zero incoming payload opens, no candidate bytes and zero publication assertions remain. The expected record count comes from the independent fixture entries.

A controlled mutation bypassing only the fresh `modification_eligibility` call makes both cases fail the new eligibility assertion: the worker still completes with one Skip prompt, unchanged source and zero begin/commit, but eligibility/local/descriptor counts are zero. Mutant source/worker and both RED results are retained separately. The production source is restored byte-for-byte to its saved correct version; the complete four-test actual-worker contract is green. There is no permanent Rust product behavior change or test-only qualification switch.

## Verification

| Check | Result |
| --- | --- |
| Actual worker EIC contract | **4/4 PASS**, 0 ignored; 14 underlying owned scenarios cover both Skip cases, genuine additions/empty directory/component boundary/kind conflict, ZIP/TAR/7z native cancellation and other native failures |
| New production terminal fixture | RED on missing role export, then **LIST1/RUN1/PASS1/SKIP0**; success, established-lease cancellation and throwing sink |
| Integrated native preservation | **LIST9/RUN9/PASS9/SKIP0**, not_run0: original terminal diagnostics and operation isolation, identity/version/eight chunk cancellations, new terminal observations, five original ZIP same-path/handoff/failure/unknown/no-retry fixtures |
| C# Debug build | **0 warnings / 0 errors** with `--no-restore -m:1 -p:UseSharedCompilation=false` |
| Rust source restoration | Exact byte equality to the saved correct `worker_zip_edit.rs` |
| Source/document gates | Spec contracts: 122 documents/14 owners; coherent Kit0.8.0/pre0.3.9; pinned editor57files/16serializer cases; task JSON parses. Source/provenance checks only. |

Every Cargo command used one lane, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, scratch TEMP/TMP and a 2400-second timeout. Native runs used the actual workspace apphost, reviewed exact LIST then identical RUN, an explicit matching frozen worker and GUID-owned scratch packages. No skip or zero-selection outcome is accepted. Prior performance samples are not rerun or reattributed by this repair.

## Evidence

Session root: `D:\.pi-desktop\data\scratch\a2072588-1f47-4232-be7a-3951a25df615`.

- `opt05-ac02-qualification/`: correct/mutant Rust source and actual workers, checked-in Python fixture, implicit/explicit RED and GREEN `wire.json` / `result.json` and logs.
- `opt05-ac02-current-green.log`, `opt05-ac02-mutant-red.log`, `opt05-ac02-restored-green.log`.
- `opt05-ac01-red/`: diagnostic RED source and compiled C# artifacts; `opt05-ac01-red-list.log` and `opt05-ac01-terminal-red.log` retain the exact selection and intended failures.
- `opt05-ac01-green-build.log`, `opt05-ac01-terminal-green.log`, `opt05-ac-native-preservation-list.log` / `opt05-ac-native-preservation-run.log`.
- Final integrated terminal packages: `archive-product-opt05-terminal-62395034-788e-4a9c-a918-63a7eab0b628` (success), `archive-product-opt05-terminal-9afce58c-013f-477c-803b-b72682d8ca06` (cancel), `archive-product-opt05-terminal-9b222306-6e53-49e1-a79c-88737c003a60` (rejecting sink). Each retains its actual terminal and production diagnostic records.
- `opt05-ac-repair-source/` and `opt05-ac-repair-pair/Debug/`: final directed source/documents and matching compiled Host/test/worker artifacts. Frozen binaries preserve attribution; native execution above used the actual workspace apphost.
- `opt05-ac-static-gates.log`: source/document/provenance gate output.

## Limitations and stop point

Independent directed closeout is pending; the original partial report is not rewritten as an approval. G-00, OPT-00/01 overall, complete G-04, overall Phase E and physical/Release acceptance remain open. Historical unavailable total/internal I/O, peak memory and codec timing remain unavailable; no new speedup claim is made.

Only controlled fixture files and Debug artifacts were used. No real GUI/Explorer/clipboard/picker, user archive mutation/recycling/HZ5, service/registry, packaging/signing/install, commit or push occurred. OPT-06/07 remain unstarted.
