# OPT-06 — Comparable private-transfer measurements

Three completed trials per route/side;18 raw samples. Original browser/Job/Run/native owners, Debug, deterministic1MiB seeds606/607, identical disk/inputs/stable worker and route/trial execution paths. No concurrent build, cache flush or best-run selection. [All raw samples and summaries](opt06-measurements.json).

## Completed operation and wire

| Route | Before wall ms, median [min–max] | After wall ms, median [min–max] | Before complete wire bytes | After complete wire bytes |
| --- | ---: | ---: | ---: | ---: |
| create_zip_store | 691.335 [601.668–972.028] | 183.681 [183.278–335.040] | 6,789,553 [6,789,334–6,789,760] | 3,172,163 [3,172,163–3,172,163] |
| extract_zip | 542.544 [517.313–571.784] | 246.609 [244.524–279.812] | 3,755,971 [3,755,912–3,755,971] | 1,059,473 [1,059,461–1,059,473] |
| modify_zip_add | 1,260.297 [1,209.646–1,315.571] | 495.964 [494.115–498.746] | 12,054,485 [12,054,485–12,054,701] | 5,280,000 [5,280,000–5,280,000] |

Wall time includes actual Run drain and excludes independent decoding. Complete wire is bidirectional JSON plus binary framing, not source-file I/O. Create/Extract/Add median wire reductions are respectively 53.28%, 71.79%, 56.20%.

| Route / side | Trial0 ms | Trial1 ms | Trial2 ms |
| --- | ---: | ---: | ---: |
| create_zip_store / before | 972.028 | 691.335 | 601.668 |
| create_zip_store / after | 335.040 | 183.681 | 183.278 |
| extract_zip / before | 571.784 | 542.544 | 517.313 |
| extract_zip / after | 279.812 | 246.609 | 244.524 |
| modify_zip_add / before | 1,315.571 | 1,260.297 | 1,209.646 |
| modify_zip_add / after | 495.964 | 498.746 | 494.115 |

## Payload, native work and host transport ownership

| Route | Source payload bytes/calls | Output payload bytes/calls | Held-partial bytes/calls | After binary stdin bytes/frames | After binary stdout bytes/frames | After envelope copy bytes | Host payload buffers/bytes high-water |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| create_zip_store | 1,048,576/17 | 1,048,706/22 | 1,048,706/18 | 2,100,082/35 | 1,050,466/22 | 2,097,282 | 2/131,072 |
| extract_zip | 0/0 | 1,048,576/16 | 0/0 | 0/0 | 1,049,856/16 | 0 | 1/65,536 |
| modify_zip_add | 1,048,576/17 | 2,097,691/33 | 2,097,691/34 | 3,150,347/51 | 2,100,331/33 | 3,146,267 | 2/131,072 |

Source/output/partial payload bytes and calls, native source/target/output attempts/full reads/actual bytes/completed bytes are equal before/after for every corresponding trial. Add retains source5/target2/output2 full reads; Create/Extract output2 remains. Every sample has one native publication, Committed journal, actual cleanup, zero active Runs, exclusive reopen and no owned partial. Original source bytes remain exact.

All nine after samples report `payload_conversion_input_bytes=0`. Host source/partial replies perform one bounded envelope copy; writes borrow the received body without decimal conversion. Each binary frame adds80wire bytes, including prefix/header. EOF is an empty read reply, not an empty write. The1MiB reservation is inside the original64MiB runtime/IPC overhead. Before did not instrument conversion/copy/buffer high-water; those values remain null.

## Sampled CPU and memory

| Route | Before sampled worker CPU ms | After sampled worker CPU ms | Before sampled worker max working bytes | After sampled worker max working bytes | Before sampled worker max private bytes | After sampled worker max private bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| create_zip_store | 31.250 [15.625–78.125] | 0 [0–15.625] | 9,990,144 [9,940,992–10,285,056] | 10,051,584 [9,990,144–10,051,584] | 2,068,480 [2,068,480–2,121,728] | 1,695,744 [1,691,648–1,695,744] |
| extract_zip | 31.250 [0–62.500] | 15.625 [0–31.250] | 10,055,680 [9,875,456–10,059,776] | 9,703,424 [9,703,424–10,129,408] | 2,080,768 [2,080,768–2,093,056] | 1,638,400 [1,630,208–1,642,496] |
| modify_zip_add | 203.125 [140.625–218.750] | 31.250 [31.250–46.875] | 11,190,272 [11,120,640–11,190,272] | 11,251,712 [10,874,880–11,251,712] | 2,412,544 [2,318,336–2,412,544] | 2,027,520 [2,023,424–2,027,520] |

Requested sampling interval is20ms, with actual intervals and failed native counter attempts retained per raw sample. CPU is an observed worker-process interval, not exclusive transport/codec CPU; coarse zero samples do not mean zero total CPU. Memory values are maxima among successful samples and may miss peaks. Owner-process samples, baselines, tails, counts and failure/native-code details remain in JSON; a longer-lived shared test process is not an isolated owner-memory benchmark.

## Attribution and limitations

- Observed before: `opt06-before/Debug`; final pair: `opt06-delivery/Debug`, with independently frozen source snapshots. Stable owned input/report root: `D:\.pi-desktop\data\scratch\a2072588-1f47-4232-be7a-3951a25df615\archive-product-opt06-82039bbc-817a-4c87-b80e-2ab73d6f14ee`.
- Before raw: `reports/before.json`; after raw: `reports/after-complete.json`. Both are complete9-sample reports and remain immutable. The workspace test/Host path and owned worker execution path are identical between sides.
- Wall intervals demonstrate improvement for these three Debug workloads. Already accepted source/plan savings are held constant by equal native/payload work; these are not new source-I/O savings.
- Complete/internal source I/O, exclusive codec/transport CPU, exact process peaks and separately timed cleanup remain null/unavailable. Cleanup is actually asserted and included in wall, not reported as zero.
- No reference-program execution, GUI/Explorer target, system clipboard/picker, user-source mutation, service/registry or Release acceptance is represented. G-00 and original overall/physical/Release gates remain open.
- This is implementation measurement evidence, not an independent OPT-06 acceptance verdict.
