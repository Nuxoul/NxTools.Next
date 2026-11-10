# Large-count ZIP: Bandizip comparison

## Runtime result

The supplied `D:\Temp\archive-tests\samples\zip\zip-zip64-70000-entries.zip` was extracted with the installed Bandizip `bz 7.32 (Beta,x64)` into a fresh GUID-owned session-scratch directory on 2026-11-06.

```text
bz.exe x -y -o:<fresh-owned-output> <supplied-ZIP>
```

| Observation | Result |
| --- | ---: |
| Encoded ZIP bytes | 7,910,098 |
| Expected files | 70,000 |
| Total expected payload bytes | 70,000 |
| Reference exit code | 0 |
| Extraction including process startup | 9,238.618 ms |
| Independently verified files | 70,000 |
| Independently verified payload bytes | 70,000 |
| Separate file-name/content comparison | 19,594.482 ms |

Verification compared the complete output filename set to the ZIP catalog and every output's bytes to Python zipfile's decoded/CRC-verified member. There were no missing or extra files. The source was read-only. This is a high-file-count, tiny-payload case, not a large decoded-byte case. Timing is one run with ordinary workstation caches, not a cold-cache or cross-build speed ratio.

## Connected reference mechanism

The existing [ZIP pipeline trace](../../.trellis/tasks/10-31-archive-research/bandizip-core-pipeline.md) records the shipped native implementation:

- Retain one source/input object and one ZIP decoder with its central-directory member table; ZIP64/end records are handled in the directory walker.
- Resolve selected indices against that table and cache/validate local data offsets.
- Stream Stored/Deflate member data through bounded blocks and applicable incremental CRC, then check decoded size and finalize the output.
- Direct output Close applies finalization/metadata and closes the current file handle. Large-file queued output has a separate bounded-work/backpressure path; it is not the explanation for this tiny-member case.
- The [operation owner](../../.trellis/tasks/10-31-archive-research/bandizip-operation-lifecycle.md) samples progress at100ms and resolves the accumulated operation result. Decoder/output calls are in-process; supporting70,000files does not require one cross-process terminal frame containing70,000per-file records.

These mechanism statements are retained binary-analysis observations, not original Bandizip source. Conditional parallelism does not establish which thread branch this runtime invocation selected. The runtime result above independently establishes complete extraction for this concrete package.

## NxTools cumulative and active boundaries

| Owning boundary | Current behavior |
| --- | --- |
| `repo:rust/crates/nxtools-archive/src/worker.rs#WorkerRequest` | Ordinary Job Extract explicitly negotiates `job_large_count_v1`; unnegotiated requests retain legacy limits. |
| `repo:rust/crates/nxtools-archive/src/worker_job.rs#plan` | Complete selection/native-history path credit is admitted under the original entry/metadata limits before output. |
| `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs#Execute` | Cumulative begin count uses admitted budgets; active tokens retain their independent4096ceiling. |
| `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.Journal.cs#ConfigureJournal` | Retained native transactions/path storage consume cumulative metadata credit; live file streams are bounded separately. |
| `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs#CommitCore` | Ordinary committed streams/handles close immediately; reclamation-specific retention is unchanged. |
| `repo:rust/crates/nxtools-protocol/src/archive_job.rs#ArchiveJobTerminal` | Negotiated summary has complete grouped publication/cleanup counts and bytes; at most256ordinal details with explicit completeness. Legacy journals retain their old shape. |
| `repo:csharp/NxTools.PlatformHost/Protocol/ProtocolEnvelope.cs#ProtocolConstants` | Ordinary protocol frames remain1MiB. |

Completed-file count is not simultaneously open-file count. Native authoritative history is distinct from its bounded public terminal projection.

## Repair boundary and acceptance

Preserve the original Rust state/selection, C# native output, GPUI presentation, CRC, source authority and per-file publication rules. Adapt the existing shared pipeline rather than introducing an alternate extractor.

1. Admit the complete file set under actual catalog/metadata/output budgets. Bound simultaneously active resources separately from the cumulative completed count.
2. Keep authoritative per-output facts at their owning boundary; separate them from the public terminal's representation. Use a bounded summary and explicitly complete/partial detail transport as required, without silently dropping committed, uncertain or failed outcomes or enlarging the global frame limit.
3. Preserve one logical extraction owner, real cancellation and cleanup, truthful committed counts and partial-publication behavior. A single archive need not become many unrelated4096-file jobs.
4. Verify this exact70,000-file package through the original production Job/worker/native output: all names and bytes, actual terminal/cleanup, cancel after prior commits, no unowned deletion, bounded frames/memory and measured phase cost. Removing a guard or compiling is not completion.

## NxTools runtime result

The same supplied ZIP now completes through the original production Job/worker/native output with70,000files/70,000bytes, all names and every payload independently verified, cleanup complete and no partials. Debug extraction including session disposal measured373,444.161ms; the terminal is45,450bytes with21native-output log events. A native cancel at4,200publications retains exactly4,200files; a damaged final member in5,000files retains4,999verified outputs and cleans its failed partial. Legacy/deficient-metadata admission refuses before publication.

The [delivery record](../audits/2026-11-06-large-count-and-launcher.md) records actual commands, separate timing/memory observations and remaining verification. Tiny-file native publication remains costly. This result does not certify arbitrary solid/staged codec counts, installed GUI interaction or Release throughput.
