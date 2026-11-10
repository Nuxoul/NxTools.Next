# Large-count extraction and launcher provider latency — 2026-11-06

## Delivered behavior

### Large-count ZIP

The supplied 70,000-entry ZIP completes through the original product ArchiveJob, isolated Rust worker and C# native output. CRC, source lease, per-file publication, cancellation and exact owned-partial cleanup remain in their original owners. A single archive remains one logical job.

`archive_job_large_count_v1` is enabled only by the current Desktop/CLI HelloAck. The original private worker separately acknowledges `job_large_count_v1` before native calls. Unnegotiated jobs retain their existing limits and omit the new summary. Ordinary negotiated extraction uses admitted entry/metadata budgets for cumulative native facts; simultaneously active tokens/streams remain capped independently. The complete plan reserves native history/path credit before output. Reclamation, edit, create and preview resource semantics are unchanged.

The native owner retains complete transaction facts. Public terminal `journal_summary` reports complete `(state, published, cleanup)` groups with file/byte totals; `journal` contains at most256 records, prioritizing nonordinary outcomes and preserving original ordinals. `detail_complete` explicitly distinguishes full history from bounded detail. Rust validates complete aggregates, published totals, unique groups/ordinals and detail membership under the unchanged1MiB frame limit. Desktop safe Retry checks complete summary groups as well as legacy details.

### Quick Launch

The production indexer now sends the launcher's completed file-kind candidates through its original framed IPC before starting its directory-kind query. Directory candidates and exactly one terminal batch follow. Both kinds retain their original candidate headroom and scores; Finder remains the final display-ranking owner. Empty results, homophone fallback, cancellation, session leases and generation checks remain in the original query path. Scoped, single-kind and recent queries retain their existing path.

Current-generation first/terminal provider milestones are low-frequency diagnostics. No control, layout, component, size, focus or preview-owner change is included.

## Native extraction results

Windows Debug worker, matching current C# tests, fresh GUID-owned scratch roots. Supplied source files were copied, never modified.

| Case | Actual committed files / bytes | Result |
| --- | ---: | --- |
| Supplied ZIP | 70,000 / 70,000 | All filenames and every payload independently compared with .NET ZIP decoding; no missing/extra files or partials |
| Cancel at native publication4,200 | 4,200 / 4,200 | Cancelled; surviving native files exactly match terminal; cleanup complete |
| Last stored member damaged in5,000-file ZIP | 4,999 / 4,999 | `corrupt_archive`; damaged member unpublished; failed partial removed and included in bounded detail |
| Legacy session,5,000 files | 0 / 0 | `resource_limit` before publications; summary omitted; foreign sentinel unchanged |
| Negotiated session, insufficient12MiB metadata | 0 / 0 | `resource_limit` before publications; no partials; foreign sentinel unchanged |

| Observation | Result |
| --- | ---: |
| Supplied ZIP extraction including product/session disposal | 373,444.161 ms |
| Supplied ZIP public terminal | 45,450 bytes |
| Supplied ZIP native output diagnostic events | 21 |
| Cancel terminal | 45,465 bytes |
| Corruption terminal | 45,543 bytes |
| Corruption scenario elapsed | 22,035 ms |
| Corruption scenario sampled host private-commit peak | 44,756,992 bytes |
| Corruption scenario native commit time | 15,165.856 ms |
| Corruption scenario native write time | 85.496 ms |

The host memory sample is the5,000-file corruption scenario, not a70,000-file RSS measurement. The [Bandizip comparison](../reference-code/archive-large-count.md) separately records9,238.618ms for complete extraction of the same supplied ZIP. Different build modes/runtimes and ordinary caches do not establish a speed ratio. Capacity and lifecycle correctness are verified; tiny-file native-publication cost remains substantial.

## Launcher measurement

The production query worker uses a deterministic40,000-row snapshot with20,000 distinct matching file names and20,000 distinct directory names, retaining40 candidates per kind. Responses are correlated and round-tripped through the real framed wire codec; no ready result batch is injected into the provider.

| Milestone | Elapsed |
| --- | ---: |
| First provider batch | 15.079 ms |
| Terminal provider batch | 23.370 ms |
| Test receiver completes framed decoding | 31.217 ms |
| Returned candidates | 80 |

This is one Debug fixture measurement, not installed keyboard-to-paint latency or a percentile. The regression requires the first nonempty frame before provider completion, compares final candidates/scores to the original complete query, and checks supersession and empty/fallback behavior. The separate Desktop tests cover arrival-to-publication, selection/action eligibility and refresh lifecycles.

## Verification

| Boundary | Result |
| --- | --- |
| Large-count original Job/native fixtures | Four distinct fixtures passed: supplied source, cancellation, corruption, legacy/metadata admission; zero skips |
| Actual C# terminals decoded/validated by Rust | Success/cancel/corruption transcripts passed the explicit native-transcript test |
| Original native output and empty-directory guards plus admission | LIST9 / RUN9 / PASS9 / SKIP0 |
| C# strict wire fixture and required native/progress exports | Passed; matching original worker |
| Protocol / IPC library | 110 / 56 passed |
| Archive library / product Job worker contracts | 98 / 2 passed |
| Indexer launcher provider regressions | 4 passed |
| Indexer search preservation selection | 17 passed; overlaps2launcher regressions, not an additional distinct total |
| Desktop Job collection | 128 passed with required current C# fixture exports |
| Desktop refresh/selection | 16 passed |
| Desktop first-ready-batch publication/action test | 1 passed; consumer seam only |
| Archive/indexer/protocol/IPC/CLI all-target Clippy | Passed with `-D warnings`, archive isolated-codecs enabled |
| C# build | Zero warnings/errors |
| Quick Launch lifecycle / GPUI release / Markdown editor source gates | Passed |

Every Rust invocation used jobs1, incremental0 and codegen-units256. The archive worker and C# test binaries were rebuilt; Desktop production code compiled and linked into its test executable. Changes remain in the working tree.

## Remaining verification and limits

- The Computer Use tool reports `cua-driver not installed`. Installed input-to-first-visible-results, physical focus/keyboard/DPI behavior and user acceptance remain unverified.
- No Release package, installation, protected-binary replacement, service restart or commit was performed. The installed application/service has not acquired these source changes. Packaged LocalSystem acceptance remains separate.
- Desktop all-target strict Clippy is blocked by the existing `unnecessary_lazy_evaluations` diagnostic in `archive/browser_drag.rs:448`; that unrelated file is unchanged by this repair. The five other changed Rust owners pass strict lint.
- Staged codecs retain their simultaneous native-resource admission. The supplied ordinary ZIP result is not acceptance of arbitrarily large solid7z/RAR/TAR workloads or every archive resource boundary.

## Reproduction

Set `NXTOOLS_ARCHIVE_WORKER_FIXTURE` to the rebuilt `target\debug\nxtools-archive-worker.exe`, `NXTOOLS_ARCHIVE_COMPAT_CORPUS` to the supplied corpus, and TEMP/TMP to PI_SCRATCH_DIR. Use exact C# LIST/RUN selection `archive large count`. The fixtures export three Job envelopes into that scratch directory. Set `NXTOOLS_ARCHIVE_NATIVE_TERMINALS` to it and run:

```text
cargo test -p nxtools-protocol --test archive_large_count_native -- --ignored --test-threads=1
cargo test -p nxtools-indexer --lib launcher_ -- --nocapture --test-threads=1
cargo test -p nxtools-desktop search_latency_ready_indexed_rows_publish_before_terminal_batch -- --test-threads=1
cargo test -p nxtools-desktop quick_launch::tests::refresh_tests -- --test-threads=1
```

The Desktop Job collection additionally requires current exports from `archive OPT01 AC repair production progress wire` and `OPT03 create actual native result locations`, with `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE` and `NXTOOLS_OPT03_CREATE_WIRE` set for both producers and consumers.
