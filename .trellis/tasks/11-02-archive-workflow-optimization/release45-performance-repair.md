# Release 45 — extraction performance repair

Parent: **Phase E — Format expansion and ZIP editing / in_progress**.

## Result and implementation

Extraction now coalesces original decoder writes into one reusable32KiB buffer in `worker_output::PrivateOutput`. It stays inside the original transport reservation, uses the original token/sequence/ACK and drains before metadata, source verification and commit. Abort/Drop discard pending bytes; first flush failure prevents replay and publication. Direct Create/Edit writes retain their existing path. Concurrency, timeouts, CRC/size/native publication/cache integrity and cleanup are unchanged.

The bounded ABBA comparison supports retaining this change: ordinary Extract wall time averaged4014.97→3251.30ms (19.0% lower), controlled Root terminal8106.60→7414.17ms (8.54% lower), and first demand→last-ready6766.37→6104.39ms (9.78% lower). Output records fell16980→6436 for identical206572163 output bytes. The sequential three-trial Root series did not improve; both series are retained below.

## Workload and boundaries

- Read-only source: `D:\Tencent\xiugou-tv-windows-x64-v1.0.3-portable.zip`,137737166 archive bytes,208 files,206572163 expanded bytes. No member was executed.
- Matched optimized Rust helper and C# Release test host; no package/install. Baseline includes progress and timing instrumentation, without coalescing; candidate adds production32KiB coalescing. Each run uses fresh owned outputs, no system cache flush.
- Ordinary route: original ArchiveJobSession Extract Here,208 committed files, actual worker cleanup. ZIP work counts215 record-processing calls (including7 directories),2 catalog loads on both sides.
- Root route: original retained read/EntrySession/DragSession with255 wrapper-first FILEDESCRIPTORW slots,208 requested original ordinals, one retained worker/catalog. A controlled native COM receiver copies every returned IStream to an owned wrapper before requesting the next member.
- Every result is independently byte-compared with streamed .NET ZipArchive content outside the timed operation. All20 measured operations passed content/count/cleanup checks. Each Root run preserves208 Seal and208 independent cursor checks, each covering206572163 bytes. Host transport payload high-water was1 buffer, within the existing2-buffer ceiling.
- Root demand→last-ready includes interleaved receiver copies; it is not pure decoding. Cache counters are captured before EndOperation removes leases. Receiver End→cleanup is measured separately.

## End-to-end observations

All times in ms. Each table cell retains every successful observation, not the best run.

| Series / side | Ordinary Extract wall | Controlled Root wall | Root demand→last-ready |
| --- | --- | --- | --- |
| Sequential baseline | 3513.50,3002.92,3056.92 | 6765.27,7018.69,7615.99 | 5588.06,5783.84,6312.17 |
| Sequential32KiB | 3223.89,2814.05,2925.98 | 7088.38,7574.62,7663.33 | 5819.10,6139.18,6336.54 |
| ABBA A1 baseline | 3960.26 | 7920.08 | 6652.87 |
| ABBA B1 32KiB | 3232.51 | 7588.14 | 6262.60 |
| ABBA B2 32KiB | 3270.08 | 7240.21 | 5946.19 |
| ABBA A2 baseline | 4069.69 | 8293.11 | 6879.87 |

ABBA uses a fresh process for each A/B invocation and one trial of each route. Sequential runs use three trials per side. This bounded comparison measures workstation variation as well as the changed output policy; it establishes a useful measured improvement, not a universal latency guarantee.

## Stage means

All values are ms except records. Columns compare baseline→32KiB within the same route/series.

| Stage | Sequential Extract | Sequential Root | ABBA Extract | ABBA Root |
| --- | ---: | ---: | ---: | ---: |
| Whole operation | 3191.11→2987.97 | 7133.32→7442.11 | 4014.97→3251.30 | 8106.60→7414.17 |
| Raw Deflate execution | 444.24→456.32 | 448.07→455.74 | 481.08→451.38 | 472.42→443.72 |
| ZIP payload Read, inclusive | 560.86→561.35 | 564.24→560.73 | 630.05→557.21 | 610.58→543.27 |
| Output IPC/ACK, inclusive | 958.03→570.61 | 836.01→509.63 | 1358.20→686.61 | 979.05→478.53 |
| Native FileStream.Write | 79.58→88.52 | 79.04→87.52 | 104.12→89.41 | 92.30→82.57 |
| Native commit, inclusive | 774.53→863.70 | 808.18→899.22 | 921.38→906.68 | 910.69→907.62 |
| Prepublication fingerprint | 152.53→166.89 | 160.12→174.57 | 178.16→171.91 | 179.35→172.44 |
| Postpublication verification | 150.02→164.19 | 152.28→166.50 | 178.24→167.24 | 168.48→162.34 |
| Cache Seal | — | 697.67→748.33 | — | 764.00→760.00 |
| Verified cursor | — | 265.00→292.67 | — | 306.00→293.50 |
| Receiver stream copy | — | 215.46→256.90 | — | 265.78→246.60 |
| Receiver End→cleanup | — | 1202.74→1307.32 | — | 1287.31→1253.14 |
| Binary output records | 16980→6436 | 16980→6436 | 16980→6436 | 16980→6436 |

`zip_inflate_us` is strictly inside flate2 raw Deflate calls. `zip_payload_read_us` includes decoder, encoded input and checkpoints. `output_ipc_ack_us` includes native write and pipe/ACK waiting. `native_commit_ms` includes flush, metadata, fingerprint, publication and postpublication verification. FileStream.Write can buffer OS work. Cache/receiver observations have their own clocks. **Do not sum these overlapping intervals.** Retained terminal native counters aggregate every member output bridge. Diagnostics are aggregate or low-frequency lifecycle events, never per-record payload logs.

Both series reduce IPC/ACK time substantially. Raw Deflate execution is about0.44–0.48s here; the whole-operation delta is not evidence of a faster decoder. Native/cache/cleanup costs remain material. No decoder replacement or expanded scheduling/resource budget is part of this repair.

## Evidence and fixture

New evidence directory: [20261104-r45-repair](../../../.diagnostics/archive-root-drag/20261104-r45-repair/measurements-summary.json). Original Release45 topology/counters remain unchanged.

| Series | Raw report |
| --- | --- |
| Sequential baseline | [3 trials](../../../.diagnostics/archive-root-drag/20261104-r45-repair/r45-measurements-baseline-9d34a398-c23a-4433-908b-e5e98b445d98.json) |
| Sequential32KiB | [3 trials](../../../.diagnostics/archive-root-drag/20261104-r45-repair/r45-measurements-32k-4f383720-f6ee-49be-8fb2-8cdd20499a72.json) |
| ABBA A1 | [baseline](../../../.diagnostics/archive-root-drag/20261104-r45-repair/r45-measurements-baseline-b1a8bbc7-c158-4d25-abc9-100d6b1e549a.json) |
| ABBA B1 | [32KiB](../../../.diagnostics/archive-root-drag/20261104-r45-repair/r45-measurements-32k-91813527-a793-4e50-aa38-fe3d4472f5b1.json) |
| ABBA B2 | [32KiB](../../../.diagnostics/archive-root-drag/20261104-r45-repair/r45-measurements-32k-fa6a8dd0-c51a-4198-9b08-d31c6a9e88d5.json) |
| ABBA A2 | [baseline](../../../.diagnostics/archive-root-drag/20261104-r45-repair/r45-measurements-baseline-886c41c2-6cc6-42d5-9a59-6716f225a3c8.json) |

`Opt04DragTests.Release45.cs` registers the effects-reviewed exact selection `archive R45 same-source extract and Root drag measurements`. Use matching `NXTOOLS_ARCHIVE_WORKER_FIXTURE`, `PI_SCRATCH_DIR`, report-only `NXTOOLS_ARCHIVE_R45_LABEL=baseline|32k` and test-only `NXTOOLS_ARCHIVE_R45_TRIALS=1..3`. Exact LIST precedes identical RUN. The fixture creates GUID-owned outputs/helper, restores TEMP/TMP, compares all output bytes, then removes only its verified owned outputs after actual cleanup. These env keys do not switch production coalescing.

Output regression evidence covers real begin/write/ACK,32KiB blocks plus exact tail, metadata/source_precommit/commit ordering, pause/cancel/abort, first failed write and no replay. Final7 output-boundary tests and8 actual-process bulk tests pass;10 extraction contracts retain CRC/source/readmission guarantees. [Complete verification](release45-repair.md).

## Reference and limitations

Bandizip7.32.0.1 remains a static inspected reference:16KiB Deflate working capacity, direct in-process writes and conditional32KiB background output with tail drain.204 sample members are below the20MiB size gate;4 meet it, totaling116120809 bytes. Configuration/type/parallel eligibility and the actual Bandizip branch for this ZIP/DOpus operation are unverified. No live Bandizip or hot-loop comparison was run.

Controlled COM is not physical DOpus or ordinary button UI acceptance. DOpus visible scatter/collect remains unresolved. Physical GUI/DOpus/Bandizip execution, new reverse engineering and Release packaging/install need separate scoped authorization. The historical9.578s live drag and single-member Bandizip CLI result are not comparable baselines for these timed routes.
