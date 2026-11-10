# Ordinary extraction output — Release measurements

Date: 2026-11-07. Parent: **Phase E — Format expansion and ZIP editing / in_progress**.

**Implementation and directed regression checks are complete. Release byte correctness passes; the performance gate does not pass.** Button and external-directory Root extraction remain above the required 1.5× Bandizip median. [Contract](ordinary-extraction-output.md), [machine-readable evidence](evidence/ordinary-extraction-output-2026-11-07.json).

## Release workload and method

- Input: `CapsWriter-Offline-20260914.zip`, 110,555,597 encoded bytes; 4,492 files, 313,474,195 decoded bytes, including 4,252 files below 64 KiB.
- Current Release Desktop, bound Release PlatformHost and isolated-codecs Release ArchiveWorker ran from one scratch runtime. No installer was produced, no installed binaries were replaced, and no service was installed or restarted.
- Three actual archive-browser **Extract All → Extract** submissions used Smart extraction, Ask conflicts, reclamation off and independent empty destinations.
- Three actual archive **Root tree-row drags** used Windows OLE into independent empty Directory Opus directories. Each has `accepted → locate_started → ole_return → located → terminal` evidence followed by the original extraction Job. No entry-cache materialization was substituted.
- Three controls used `bz.exe x -aoa -y -o:<fresh-directory> <archive>` in the same session. No Cargo/compiler or other archive worker ran during timed trials.
- NxTools timing is the original `archive.worker.completed.elapsed_ms` from Job start through resolved cleanup. Bandizip timing is process wall time. Root pointer movement and OLE target selection are separate from extraction timing; the three OLE location lifecycles took 1,665 / 1,674 / 1,649 ms, including the automation's held-pointer gesture.

## Results

| Entrance | Trial 1 (ms) | Trial 2 (ms) | Trial 3 (ms) | Median (ms) | / Bandizip median |
| --- | ---: | ---: | ---: | ---: | ---: |
| NxTools button | 4,617 | 4,474 | 4,373 | **4,474** | **2.957×** |
| NxTools Root → external directory | 4,378 | 4,405 | 4,315 | **4,378** | **2.894×** |
| Bandizip CLI | 1,773.013 | 1,468.158 | 1,512.826 | **1,512.826** | 1.000× |

The required maximum is **2,269.239 ms** for both NxTools medians. Neither entrance meets it.

All six NxTools terminals report success, 4,492 completed files, zero skipped files, `Committed` and `cleanup_complete=true`.

### Correlation IDs

| Trial | Extraction correlation ID | Root OLE correlation ID |
| --- | --- | --- |
| button-1 | `a4e8aebc-ea1a-400a-99cd-d7ff89ac9e51` | — |
| button-2 | `e54b038f-7038-4be4-be9b-c35b48f1eabb` | — |
| button-3 | `0e5e3cd9-fb99-43ca-8369-27bc7634f7a3` | — |
| drag-1 | `fabb9031-eb3d-4e6d-b0d6-3e69b1fa8e48` | `4c68cea6-6097-454f-80a9-63531ce353c7` |
| drag-2 | `3a4b3b2b-ac81-48d1-b75d-bee161cba006` | `c4084930-2d93-4745-a16d-07c89d3dffd7` |
| drag-3 | `bbe63edf-f82d-46a8-83ab-b53d3b2d075f` | `72797274-37b2-4f14-9c9b-26fe759864c3` |

## Work and native counters

| Trial | Revision (ms) | Inflate (ms) | Payload read (ms) | Pipe send/backpressure (ms) | Native write (ms) | Metadata/close (ms) | JSON stdin / stdout frames |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| button-1 | 84.565 | 670.855 | 884.360 | 350.800 | 373.006 | 304.908 | 9,349 / 9,393 |
| button-2 | 84.635 | 664.724 | 881.448 | 331.618 | 363.897 | 297.040 | 9,349 / 9,392 |
| button-3 | 86.086 | 657.676 | 868.945 | 320.954 | 354.314 | 292.374 | 9,349 / 9,391 |
| drag-1 | 79.880 | 657.550 | 867.868 | 320.133 | 350.390 | 293.887 | 9,349 / 9,391 |
| drag-2 | 78.990 | 656.895 | 866.735 | 321.553 | 356.372 | 292.445 | 9,349 / 9,392 |
| drag-3 | 79.703 | 655.965 | 865.017 | 320.373 | 347.145 | 287.942 | 9,349 / 9,391 |

Every trial has:

- one full source revision, 110,555,597 bytes; one catalog load;
- 4,492 member decodes, local checks, descriptor checks and record seeks;
- 13,200 bounded binary output records, 313,474,195 payload bytes;
- **zero write ACK frames, native fingerprint time and postpublication time**;
- zero target/output fingerprint read attempts or bytes;
- zero payload-conversion and transport payload-copy bytes;
- one payload buffer at high water, at most 32,768 bytes, within the existing 1,048,576-byte reservation.

`native_commit_ms` now means metadata/drain/close. `output_ipc_ack_us` is the retained field name for pipe send/backpressure, not an acknowledgement wait. Work intervals overlap; they are not exclusive phases and must not be summed or subtracted to manufacture an IPC duration.

## Independent output verification

Bandizip trial 1 was the independent reference. Each of the six NxTools outputs and both other controls matched its complete file set, lengths and chunked byte comparison: **4,492 files / 313,474,195 bytes per trial**. No content hashes were used.

Button Smart output is rooted at `CapsWriter-Offline`. Root drag preserves its existing archive-name destination wrapper, `CapsWriter-Offline-20260914/CapsWriter-Offline`; comparison starts beneath that explicit wrapper and verifies all remaining relative names. The wrapper itself is checked exactly.

All nine owned output directories were removed after comparison. The original archive was not modified and extracted programs were not executed.

## Implementation and regression boundary

The shared ordinary-output path now uses held-relative final-name `CREATE_NEW`, conflicts only after `target_exists`, direct decided overwrite, bounded original-pipe writes without per-record ACK, commit-carried metadata, drain/close completion and incomplete-only cleanup. Create/Edit candidate publication, HZ5 and entry-cache Seal/cursor behavior retain their separate contracts.

Directed verification:

- Rust output regression rejects per-write ACK waiting and checks begin/close-only controls, exact sequences/offsets, <=32 KiB chunks and commit-carried metadata.
- Final C# direct-output, ordered-pipeline, actual ZIP conflict/extraction, mid-write and post-close cancellation, long-path extraction/creation, source-deletion refusal, large-count cancellation and mixed Create/Extract/Add fixtures: **LIST 14 / RUN 14 / PASS 14 / SKIP 0 / FAIL 0 / NOT_RUN 0**; build zero warnings/errors. The 5,000-file cancellation fixture stops after exactly 4,200 closed files. The mixed fixture completes nine original Jobs.
- The ordered-pipeline fixture covers success, native `write_file / 112` reported at close, and the same first native failure retained when EOF precedes close. Failure/cancel/CRC keeps closed outputs and removes the incomplete output.
- Rust library/unit tests: **99 passed**. OPT-06 bulk tests: **8 passed**. TAR-plan tests: **12 passed, 1 opt-in benchmark ignored**. Cross-format prompt tests: **2 passed**. Metadata-on-close and TAR-stream readmission contracts pass.
- Original RAR/7z/TAR-stream coordinator → C# runner/Job/native-output chains: **3 targets passed, 0 failed/ignored**, covering **13 / 10 / 17 internal scenarios**. Post-close cancellation reaches the actual `native_closed` fixture checkpoint after the native handle is closed and bridge completion counts are recorded; earlier closed outputs remain and new admission is refused. Creation/editing continue to use their distinct `native_published` checkpoint.
- Strict `nxtools-archive` all-target Clippy with isolated codecs passed. Final scoped rustfmt checks all 11 touched archive Rust files without recursively formatting unrelated children.
- Final documentation contract gate passes: **126 documents / 14 workspace owners**. Scoped diff whitespace checks pass.
- Release Desktop and worker built; bound Release hosts published for the scratch runtime. Desktop reports five existing download-settings warnings. There is no archive build warning or package/install acceptance claim.

## Changed owners

This slice changes the ordinary-output portions of existing files, not every dirty change in those files:

- Rust `worker.rs`, `worker_output.rs`, `worker_conflict.rs`, `worker_codecs.rs`, `worker_formats.rs`; new `worker_output_direct_tests.rs`.
- C# `ArchiveOutputSession.cs`, new `ArchiveOutputSession.Extraction.cs` and `.ExtractionConflict.cs`, `ArchiveWorkerOutputBridge.cs`, `ArchiveWorkerRunner.cs` and `.Bulk.cs`.
- C# direct/pipeline/conflict/actual-worker fixtures and registrations, RAR/7z/TAR readmission fixtures, Job/password/large-count close checkpoints, long-path and source-deletion matrices, and the ordinary branch of mixed bulk measurements; Rust worker-wire, prompt, metadata, TAR-plan and bulk peer adapters.
- Operative archive/platformhost/Desktop specs, archive reference note, parent extraction requirements and the existing 11-02 task documents.

Two minimal build-enabling edits are separate from archive behavior: removal of duplicated delimiters in `nxtools-ipc/src/lib.rs`, and `pub(super)` visibility for two already parent-called download handoff methods. Concurrent trusted-uninstall and other workspace edits are not attributed to this slice. No commit is created.

## Known issues and next decision

- **Performance remains above the required limit.** The removed flush/hash/reopen/ACK work is absent, but 4,492 files still require 8,984 synchronous begin/close controls plus directory/operation overhead. Existing counters do not isolate those round trips from native path-open and control serialization cost.
- `product_read_chain::v2_cumulative_exact_byte_boundaries_and_one_less_are_enforced` remains failing in the 23/24 product-read selection: the next run is limited to the preceding run's `private_bytes - 1`, while serialized microsecond counters vary in length. Product accounting is unchanged. The full archive suite is not certified green.
- Real Release performance here covers the supplied ZIP and these two entrances, not every format/strategy, installed deployment or human visual acceptance.

The next bounded investigation should separate begin/close control time, native path-open time and directory controls on this same workload. A duplicated-output-handle design would change the Rust/C# write boundary: C# would retain safe-open, metadata and cleanup authority while the worker writes through a duplicated handle. That requires user approval before implementation and a new lifecycle/error/cancellation regression at the changed boundary.

Entry-cache optimization remains separate. [Bandizip gap investigation](bandizip-gap-investigation.md) retains the different 208-file sample's Seal and cursor double-hash measurements; those figures do not explain or accept the current ordinary-output benchmark.
