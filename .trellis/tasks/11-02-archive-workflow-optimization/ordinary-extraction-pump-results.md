# Ordinary extraction output — synchronous-pump Release results

Date: 2026-11-07. Scope: **Phase E — Format expansion and ZIP editing** stabilization under the [ordinary-output contract](ordinary-extraction-output.md).

## Verdict

The current Release completes all six genuine NxTools operations with correct output and actual cleanup. All nine trial outputs agree on file paths, lengths and bytes. **The required <=1.5× Bandizip performance gate does not pass:** button median4241ms /2.873×; Root-to-external-directory drag median4214ms /2.855×; Bandizip median1476.011ms. The threshold is2214.016ms.

[Per-trial evidence](evidence/ordinary-extraction-pump-2026-11-07.json) retains operation correlations, terminal/work counters, separate OLE events, independent comparisons, causal controls and environment restoration. The [earlier nine-trial report](ordinary-extraction-output-results.md) and its evidence remain unchanged and belong to their earlier runtime.

## Genuine same-session trials

Input: `D:\Temp\CapsWriter-Offline-20260914.zip`,110,555,597encoded bytes,4492files /313,474,195decoded bytes;4252files are below64KiB.

| Entrance | Trial1 ms | Trial2 ms | Trial3 ms | Median ms | /Bandizip median | <=1.5× |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Toolbar “解压全部” -> dialog, Smart | 4378 | 4241 | 4233 | 4241 | 2.873× | Fail |
| Archive tree Root -> external DOpus directory | 4214 | 4253 | 4186 | 4214 | 2.855× | Fail |
| Bandizip `bz.exe x -aoa -y` | 1476.0109 | 1443.6921 | 1510.8874 | 1476.0109 | 1.000× | Reference |

The order was button1/2/3, Bandizip1, Root1, Bandizip2, Root2, Bandizip3, Root3. Every destination was independently empty. No builds, tests, other archive operations or byte comparisons ran during a timed trial. NxTools uses `archive.worker.completed.elapsed_ms` through actual cleanup. Bandizip uses captured process-only wall time; inventory and comparison are outside that interval.

The runtime is the matching scratch Release Desktop/PlatformHost/isolated-codecs ArchiveWorker built from the synchronous-pump source. The original publishing helper generated the bound Release identities; this is not an installed package. Runtime path and generated identity manifests are retained in the evidence.

| Trial | Extraction correlation | Root OLE correlation |
| --- | --- | --- |
| button-1 | `282f8e3c-828d-49e7-ad3d-2bd1620b593f` | — |
| button-2 | `43e1b3ee-eb3a-48fd-8a3b-0d264f1e4a7f` | — |
| button-3 | `871a380e-65a3-40d5-9524-149957a19ca8` | — |
| drag-1 | `add3acec-24f3-42e8-8e7f-e122e03466d9` | `9328c06e-db76-4db9-ae91-0a4c6dd4fa75` |
| drag-2 | `08a6bcc7-846d-4186-a627-1964a5198ee6` | `b715899c-c7f9-4eb6-8427-221dc5ca79c3` |
| drag-3 | `1c223df6-4e3c-49b5-b6c8-95ec6a705e4a` | `8d304b91-b7f6-4044-adb5-0f2c26c42852` |

All three drags used one real held-pointer gesture from the archive Root to a blank external-directory area. Each retains `accepted -> locate_started -> ole_return -> located -> terminal`, success and cleanup, followed by its original extraction Job. Separate OLE lifecycle totals1628/1651/1640ms include the scripted gesture and target location; they are not extraction time. No entry-cache/IStream materialization substitutes for Root extraction.

## Output and lifecycle

- Six NxTools terminals report success,4492committed,0skipped,`Committed` and `cleanup_complete=true`.
- Every NxTools output and Bandizip2/3 was independently compared with Bandizip1: exact relative file sets, lengths and chunk bytes,4492files /313,474,195bytes each. No product-output hashes were used.
- Button Smart output is `CapsWriter-Offline`. Root output is exactly `CapsWriter-Offline-20260914/CapsWriter-Offline`; comparison validates the outer wrapper before comparing beneath it.
- Each NxTools operation retains one source revision and catalog,4492member decodes/local checks/descriptor checks/record seeks,4492begin and4492close controls,360directory controls and no directory-metadata controls.
- Every operation writes13,200binary records /313,474,195payload bytes, with zero ordinary write ACKs, fingerprint/postpublication time, target/output fingerprint-read bytes, payload conversion or payload-copy bytes. Payload-buffer high water is one buffer /32,768bytes.
- All nine owned output directories were removed after successful comparison. The source ZIP, evidence and runtimes remain. Scratch Desktop/Host exited; original installed Desktop/Host restarted from their existing installation paths. DOpus retains its original `Down` tab and geometry; the three result tabs are closed. No package, installation, deployed-binary replacement or service change occurred.

## Current boundary measurements

All values are milliseconds. Worker begin/close totals include the complete corresponding control await. Native creation/inspection and native commit are narrower owner timers.

| Trial | Begin wait | Close wait | Native `NtCreateFile` | Post-open inspection | Native payload write | Native metadata/close |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| button-1 | 1803.026 | 702.044 | 1469.248 | 43.560 | 344.285 | 256.065 |
| button-2 | 1809.315 | 698.079 | 1490.602 | 43.155 | 353.000 | 254.751 |
| button-3 | 1788.701 | 681.381 | 1473.725 | 43.166 | 337.526 | 253.269 |
| drag-1 | 1842.306 | 683.142 | 1525.620 | 43.074 | 337.823 | 256.195 |
| drag-2 | 1862.804 | 691.225 | 1541.408 | 43.630 | 338.095 | 256.368 |
| drag-3 | 1863.689 | 674.343 | 1544.609 | 43.050 | 330.176 | 252.745 |

Worker `output_*_wait_us` starts after flushing the preceding Rust tail and covers serialization/send/reply wait. A close await includes queued payload the single stdout consumer must process before close. `native_commit_ms` starts at `session.Commit` only after that preceding payload has been processed and measures native metadata/handle close, not the earlier drain wait. Intervals are inclusive and overlapping; summing or subtracting them does not produce exclusive IPC cost.

## Measured optimization

The existing `CreatePipe` handles are synchronous and their `FileStream` wrappers use `isAsync:false`. The original independent input/stdout pumps now perform synchronous I/O directly: input cancellation precheck then `Write`, input bufferSize1 for complete small control frames, and stdout `Read`/`ReadExactly` including binary bodies.

The original anonymous pipes, bounded channels/backpressure, pump separation, protocol/capabilities, <=32KiB records, zero ordinary payload ACKs, Job termination, real drain, metadata ownership, native permissions and flags remain unchanged. No duplicated handle, new pipe/channel/runtime/request map or process-authority boundary is introduced. Ordinary final-name creation, conflict decisions, first native error, incomplete-only cleanup and completed-file retention remain the same. Creation/edit candidates, cache Seal/cursor verification and HZ5 retain their separate owners.

Original CLI -> ArchiveJob -> worker -> C# native controls isolated the relevant costs before this change:

| Control | Trial1 ms | Trial2 ms | Trial3 ms | Median ms |
| --- | ---: | ---: | ---: | ---: |
| Original pumps, begin/path/directory counters | 4454 | 4396 | 4361 | 4396 |
| Original pumps, split create/inspection counters | 4489 | 4378 | 4414 | 4414 |
| Synchronous original input/stdout pumps | 4518 | 4304 | 4301 | 4304 |

Initial parent resolution totals52–60ms and360directory operations117–120ms. Detailed native creation totals1368–1410ms, versus about43ms of post-open inspection. These owner measurements identify final-name native creation and8984synchronous controls as remaining material boundaries; they do not support removing path/source/inspection checks. Payload handle relocation alone leaves those controls and native creation intact. The bounded synchronous-pump improvement does not close the performance gate.

The controls are separate from genuine GUI acceptance. Their15successful product outputs across measured variants were independently byte-compared with Bandizip and removed; raw successful baseline/detail/current-pump records and comparison results are included in the evidence.

## Directed compiled verification

The current synchronous-pump source has the following verification, completed before timed trials:

- Rust archive library:99passed,0failed,0ignored, including the actual ordinary protocol sequence and begin/close counter assertions.
- Strict `nxtools-archive` all-target Clippy with isolated-codecs and `-D warnings`: pass.
- Fresh Debug worker and C# build: success; C#0warnings/0errors.
- C# preservation group: exact LIST14/RUN14/PASS14/SKIP0/FAIL0/NOT_RUN0. Direct output, first-write-error/close/EOF, conflict policies, real ZIP/cancellation, Job late cancellation, long-path extraction/creation, source-deletion matrix,5000-file cancellation after4200closes and mixed Create/Extract/Add Jobs pass.
- C# transport/candidate group: exact LIST17/RUN17/PASS17/SKIP0/FAIL0/NOT_RUN0. ACK ordering, malformed frames, EOF/short reads, capability/privacy, bounded cancellation/drain, first-error retention and staged-candidate verification pass.
- Actual cross-format chains: RAR13scenes/22attempts,7z10/16,TAR-stream17/27; all3targets,40scenes/65attempts pass, with cleanup complete, no active work and released source/helper ownership.
- Final matching Release Desktop/Indexer/CLI/BrowserHost and isolated worker builds: success. Desktop retains three existing download-settings visibility warnings; no archive compilation error.

Final source/document gates after results synchronization: `spec-contracts.py` passes126documents/14workspace owners; scoped `rustfmt --check` passes13Rust files; scoped `git diff --check` passes. These are source/document checks, separate from the genuine Release trials above.

## Limitations

The <=1.5× ordinary-output performance gate remains open; neither the measured controls nor the genuine trials authorize a performance pass. Phase E overall, installed-package/deployment acceptance and human visual/interaction acceptance remain separate.

The full archive suite is not certified green. The retained `product_read_chain::v2_cumulative_exact_byte_boundaries_and_one_less_are_enforced` failure compares `None` with `Some(MetadataBudget)` at `product_read_chain.rs:1027`; its reused private-byte boundary is sensitive to serialized microsecond-counter digit lengths. The actual encrypted-password extraction fixture was updated and compiled but was not explicitly rerun in the final14; its result is not included in that pass count.
