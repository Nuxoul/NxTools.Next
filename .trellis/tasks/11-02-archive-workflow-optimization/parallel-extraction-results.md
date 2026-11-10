# Bounded parallel ZIP extraction — Release results

Date: 2026-11-09. Scope: **Phase E — Format expansion and ZIP editing**, ordinary-extraction stabilization under the [parallel contract](parallel-extraction.md).

## Verdict

**The supplied-ZIP performance and correctness gate passes.** Three genuine toolbar/dialog operations, three genuine archive-Root drags to an external DOpus directory and three Bandizip parallel=1 controls complete successfully. Button median2370ms /1.297× and Root median2283ms /1.249× are both below1.5× Bandizip median1827.211ms; the threshold is2740.816ms. All nine outputs have identical file sets, lengths and bytes:4492files /313,474,195bytes each.

[Per-trial evidence](evidence/parallel-extraction-2026-11-09.json) retains exact terminal/work counters, runtime identities, extraction/OLE correlations, independent comparisons, cleanup and restoration. The [synchronous-pump report](ordinary-extraction-pump-results.md) and [initial-alignment report](ordinary-extraction-output-results.md) remain historical evidence for their own runtimes.

## Genuine same-session trials

Input: `D:\Temp\CapsWriter-Offline-20260914.zip`,110,555,597encoded bytes. Each trial used a fresh empty destination. Bandizip's existing `HKCU\Software\Bandizip\useParallelExtraction` value was1 before and after the trials and was not changed.

| Entrance | Trial1 ms | Trial2 ms | Trial3 ms | Median ms | /Bandizip median | <=1.5× |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Toolbar “解压全部” -> dialog, Smart | 2370 | 2652 | 2362 | 2370 | 1.297× | Pass |
| Archive tree Root -> external DOpus directory | 2190 | 2283 | 2283 | 2283 | 1.249× | Pass |
| Bandizip `bz.exe x -aoa -y` | 2439.8191 | 1802.0529 | 1827.2106 | 1827.2106 | 1.000× | Reference |

Order: button1/2/3, Bandizip1, Root1, Bandizip2, Root2, Bandizip3, Root3. No builds, tests, other archive operations or disk comparisons ran during a timed trial. NxTools uses `archive.worker.completed.elapsed_ms` through actual cleanup. Bandizip uses captured-console process wall time; inventory and comparison occur outside that interval. All byte comparisons ran after all timed trials.

The runtime is a matching isolated scratch Release Desktop/PlatformHost/isolated-codecs ArchiveWorker, not an installed package. The original publishing helper generated its bound Desktop/Release/Archive identities. Frozen copies were verified against the five actual Rust Release outputs and the generated manifests, including the matching PlatformHost and four Shell helper members, before trials.

| Runtime executable | SHA-256 |
| --- | --- |
| Desktop | `79C3239BCD9F5A18095F6ED632D4F4DA03780F69DD44088591452D6FB115E298` |
| PlatformHost | `EDF02E6928F9E7186CCF290B779DBCC0DE65F8A617480A8D85F198A48866CF87` |
| ArchiveWorker | `ECB18832FD78BC33C03F4C3113B90CB74079FE1BCE5BDB6189326337605003D4` |

These identify the measured executables; ordinary output has no product fingerprint/hash/readback stage.

## Real Root gesture and operation identity

| Trial | Extraction correlation | Root OLE correlation | OLE lifecycle ms |
| --- | --- | --- | ---: |
| button-1 | `f0bf18d1-cf2a-4444-813b-87d498f9fd2f` | — | — |
| button-2 | `ae27b8be-625d-420e-9d61-baf5bc886acf` | — | — |
| button-3 | `85c66f5a-8274-448d-ade9-9f77677fc032` | — | — |
| drag-1 | `33950ac8-8114-4ad1-a5ca-5b2517107a49` | `dd255eda-a626-41ae-88b5-ec5106b7fe31` | 1898 |
| drag-2 | `590b020b-56df-490f-944e-b15ae5ff9c77` | `cc9d2f6b-bbf9-4005-b9af-9e0a15680352` | 1954 |
| drag-3 | `58842d0a-da72-4fd7-ba34-a84cc8120c49` | `cb00791e-968d-430d-9fdd-8e52765466ce` | 1844 |

Each counted drag is a real held-left-pointer gesture from the archive Root to a blank external-directory area, with current PID/HWND, screenshot and point-ownership guards. Every OLE chain is `accepted -> locate_started -> ole_return -> located -> terminal`, success and cleanup, followed by the original extraction Job. OLE lifecycle totals include gesture and target location; they are separate from the extraction times above. Entry-cache/IStream materialization is not substituted for Root extraction.

## Parallel work, output and lifecycle

| Trial | Planned lanes | Active-lane high water | Native pending high water | Completed members |
| --- | ---: | ---: | ---: | ---: |
| button-1 | 6 | 6 | 8 | 4492 |
| button-2 | 6 | 6 | 7 | 4492 |
| button-3 | 6 | 6 | 9 | 4492 |
| drag-1 | 6 | 6 | 7 | 4492 |
| drag-2 | 6 | 6 | 7 | 4492 |
| drag-3 | 6 | 6 | 7 | 4492 |

- Six NxTools terminals report success,4492committed,0skipped and `cleanup_complete=true`.
- Each retains one source revision/catalog,4492ZIP member decodes/local checks/descriptor checks/record seeks,4492begin and4492close controls,360directory controls and no directory-metadata controls.
- Each writes13,200binary records /313,474,195payload bytes. Ordinary write ACKs, fingerprint/postpublication time and target/output fingerprint-read bytes are zero; payload-buffer high water is at most32,768bytes.
- All nine relative file sets, lengths and streaming bytes match independently decoded Bandizip1 output. Button Smart produces `CapsWriter-Offline`; Root produces exactly `CapsWriter-Offline-20260914/CapsWriter-Offline`, including the separately verified outer wrapper. No product-output hashes were used for comparison.
- After workers exited, the source ZIP reopened with exclusive sharing, verifying extraction lease release. All nine verified owned output directories were removed; the source, reports, logs and frozen runtime remain.
- The scratch Desktop34544 and Host43088 stopped. The installed Desktop38444 and its Host39164 were restored from their original paths and no-argument launch. Only the floating assistant is visible; the main window is hidden. DOpus18048 remains running with zero listers, matching the starting baseline. Unowned Host processes were untouched.

Rust begin/close waits and native create/write/metadata-close counters are retained in the evidence. They are inclusive, overlapping aggregates across lanes and can exceed operation wall time. They must not be added or subtracted to claim exclusive IPC cost.

## Implementation and compiled verification

Whole-member Stored/Deflate decoding uses bounded dynamically claimed Rust lanes and independent source cursors under the original protected lease. The original Job/worker,32KiB payload records and pipes remain. Only privately negotiated ordinary extraction permits bounded keyed controls and overlapping native begin/close; collision/risk decisions serialize. True joins precede incomplete-output cleanup, lease release and terminal projection. Completed outputs remain intact on cancellation or later failure. Creation/edit candidates, cache/HZ5, other formats and solid-integrity owners keep their existing routes.

Verification completed before timed trials:

- Full isolated-codecs archive suite:60targets,392passed,0failed,1existing opt-in benchmark ignored, using `cargo test -p nxtools-archive --features isolated-codecs --no-fail-fast -- --test-threads=1`. This includes library103, parallel contracts9 and the original RAR/7z/TAR-stream native readmission chains.
- Strict archive all-target isolated-codecs Clippy with `-D warnings`: pass. Scoped Rust formatting: pass.
- C# build:0warnings/0errors. Parallel fixtures10/10, ordinary/native preservation15/15 and default production Job preservation4/4 pass with0skips/0failures.
- Actual Rust-to-native fixtures prove mixed Stored/Deflate overlap and exact bytes, cancellation with two incomplete native files after one completed close, CRC failure retaining completed output, original Job/pipe drain and source lease release. Other directed fixtures cover bounded32 admission, serialized ApplyAll/stale tickets, first real native/worker error after provisional cancellation, late payload rejection, monotonic aggregate progress and nonaccumulating pending diagnostics.
- Preservation includes actual encrypted ZIP/7z/RAR, ordinary conflict/source/long-path/metadata/MOTW behavior and existing candidate/transport/other-format contracts. Release build and bound identity generation succeeded.

Final source/document verification: `python tests/architecture/spec-contracts.py` passes126documents/14workspace owners, including navigation/evidence/contracts; scoped `git diff --check` passes. These checks are separate from product behavior and genuine timing.

## Limitations

The performance verdict covers this supplied ZIP and machine under the recorded protocol. Overall Phase E, installed-package/deployment acceptance and human visual/DPI/interaction acceptance remain separate. No package, installation, deployed-binary replacement, service change, commit or push was performed.

The complete current archive run passed; the historical private-byte digit-boundary instability remains attributed to its earlier run and is not claimed repaired by this result. Optional external codec corpora without their opt-in environment are not additional corpus acceptance. Static `bz.exe` configuration handoff remains unproven; the Bandizip controls above are runtime evidence, while the inspected configuration/scheduler chain belongs to Bandizip.exe/Ark.
