# Root drop and extraction read-pass repair

Parent: **Phase E — Format expansion and ZIP editing / in_progress**.

## Root drop

Root drags now follow Bandizip's Fast DnD outcome: the receiver creates one folder named by the archive stem and an ordinary whole-archive extraction fills it. The receiver no longer copies hundreds of virtual members, so the scattered-then-collected presentation cannot occur and the drop costs the same as Extract Here. Mechanism, reference functions, invariants and tests: [Root drop destination location](../../../docs/reference-code/archive-virtual-drag.md#root-drop-destination-location--2026-11-05).

## Extraction read passes

| Pass | Before | After |
| --- | --- | --- |
| Source revision (protected ordinary source) | Full SHA-256 at open, after catalog, before and after payload and per ZIP member when unprotected; OPT01 fixture asserted 4 per job, 2 per browser open | One full read when the worker opens the source; later checks compare size and modification time of the same handle |
| New-file prepublication fingerprint | Full re-read of the closed partial | SHA-256 accumulated from the exact bytes written |
| New-file postpublication verification | Full re-read of the published file | Volume, file ID and size of the renamed object |
| Replacement, edit and reclamation commits | Full reads | Unchanged |

**Source.** The native runner holds an ordinary single-volume source with `FILE_SHARE_READ` only and every parent without `FILE_SHARE_DELETE` for the whole worker lifetime ([ArchiveSourceLease](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs)). `worker::execute` registers that path with `OperationControl::protect_source`; `engine::ensure_revision` then validates metadata instead of re-hashing. Split volumes, reclamation and unprotected engine use keep the full re-read. Browser digest binding and readmission still compare the digest taken at open.

**Output.** The partial is created exclusively and opened without sharing, so the written-bytes digest equals the closed candidate's content. A new name is published with no-replace rename of that handle; identity after rename proves the published object is the one written. Verified owned cleanup still hashes the real file and compares it with the stored fingerprint.

## Measurements

`archive OPT00 completed read baseline`, debug worker with `isolated-codecs`, 17-member stored ZIP, 3 trials each, same C# host, runs ordered new/baseline/new. Median ms:

| Operation | New | Baseline | New |
| --- | ---: | ---: | ---: |
| browser_open | 511.85 | 850.78 | 489.86 |
| test | 542.03 | 1620.83 | 550.13 |
| materialize | 500.50 | 1656.04 | 507.15 |
| extract_all | 684.31 | 1793.68 | 662.27 |
| extract_selected_1 | 504.68 | 1575.97 | 487.76 |
| extract_selected_16 | 571.54 | 1656.94 | 568.40 |

The baseline column is the same worker source with only `protect_source` removed, so it isolates the source-revision change. The output-commit change is covered by read counters (`archive output fixture new file streamed fingerprint`), not timing. These are debug-build fixture timings, not the Release 45 production workload.

## Verification

- Rust: `nxtools-protocol` drag tests 14/14; `nxtools-ipc` drag tests 3/3; `nxtools-archive` lib 65/65 including `protected_revision_tests`; all `nxtools-archive` integration targets pass except two `tar_stream_readmission_*` tests that require `NXTOOLS_TAR_STREAM_FIXTURES`.
- Desktop `archive::` 468 passed; the 7 failures are `ac_repair_tests`/`opt03_result_tests`, which require `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE`/`NXTOOLS_OPT03_CREATE_WIRE` exports.
- C#: Root locate 3/3, virtual drag 4/4, output fixtures 7/7, `archive OPT01 native shared read revision work` (revision count 1), source-binding 9/9. With the Debug test apphost and `NXTOOLS_ARCHIVE_WORKER_FIXTURE`, the actual-worker job, drag and output selection passes 22/22, including `archive job fixture actual risk pause prompt timeout`. That test lowers the risk thresholds through `ArchiveJobSession.ConfigureControlFixture`, which is compiled only under `DEBUG`; a Release test apphost leaves the default 64 MiB threshold, so its 8 KiB member completes without a prompt.

## Limitations

- No physical Explorer, Directory Opus or desktop drop was run; receivers are controlled `CF_HDROP` copy fixtures.
- Receivers that do not copy folders to a local or removable volume (shell namespaces, network shares not under a watched root and not open in Explorer) report `drop_location_unknown`; nothing is extracted.
- Release packaging and installation were not run.
