# OPT-05 — Comparable Debug measurements

Three completed trials per boundary; identical retained inputs, build mode, disk, route and stable execution paths. No system-cache flush or best-run selection. Separate owner latencies are not added together.

ZIP: 1,024 metadata records or 128 × 65,536-byte payloads. TAR kernel: independent USTAR with two directories and 16 × 65,536-byte files beneath TAR/TGZ/TXZ. Native compound: two independently decoded 65,536-byte members per TGZ/TXZ. Selection: 65,536 entries / 4,096 selected IDs. Native owner: retained 8,388,730-byte ZIP.

Full three-trial values, work counters, source reports and metric scopes: [measurement data](opt05-measurements.json). Delivery and verification: [OPT-05 result](opt05-reusable-plans.md).

## ZIP engine/update

| Boundary | Before ms: median [min–max] | After ms: median [min–max] |
| --- | --- | --- |
| add | 342.334 [336.920–342.388] | 336.455 [331.672–339.314] |
| comment | 340.653 [339.216–343.700] | 334.680 [332.392–339.054] |
| delete | 338.574 [336.830–344.788] | 338.549 [335.778–342.103] |
| metadata_extract_one | 24.261 [23.222–24.882] | 24.970 [24.470–25.195] |
| metadata_open | 57.825 [57.470–58.072] | 44.788 [42.092–46.730] |
| metadata_test | 33.328 [32.816–35.941] | 23.080 [22.530–25.448] |
| overwrite | 338.955 [333.621–346.598] | 334.436 [334.296–335.016] |
| rename | 344.509 [338.734–349.704] | 333.532 [327.139–337.739] |

## TAR engine/worker

| Boundary | Before ms: median [min–max] | After ms: median [min–max] |
| --- | --- | --- |
| Tar/engine_open | 73.231 [69.974–77.288] | 75.620 [74.378–80.585] |
| Tar/worker_extract | 437.514 [432.843–442.376] | 463.174 [462.720–477.001] |
| Tar/worker_materialize | 161.376 [156.905–162.431] | 166.508 [165.169–170.478] |
| Tar/worker_retained | 231.890 [230.060–234.805] | 248.927 [240.329–252.584] |
| Tar/worker_test | 122.119 [122.005–146.496] | 132.087 [131.003–132.203] |
| TarGz/engine_open | 11.589 [11.077–12.527] | 7.054 [6.594–7.461] |
| TarGz/worker_extract | 360.951 [357.920–372.331] | 382.050 [372.283–383.683] |
| TarGz/worker_materialize | 82.821 [82.677–88.442] | 85.668 [85.285–86.972] |
| TarGz/worker_retained | 112.813 [110.988–117.065] | 124.227 [121.093–124.286] |
| TarGz/worker_test | 63.980 [61.370–64.241] | 59.934 [59.720–60.242] |
| TarXz/engine_open | 54.794 [53.919–59.136] | 24.867 [23.954–25.595] |
| TarXz/worker_extract | 401.817 [400.693–414.903] | 393.340 [392.510–403.896] |
| TarXz/worker_materialize | 125.359 [125.037–129.322] | 102.266 [101.323–102.380] |
| TarXz/worker_retained | 135.557 [135.205–142.585] | 145.867 [142.957–150.856] |
| TarXz/worker_test | 105.286 [102.643–106.336] | 81.722 [81.159–82.177] |

## Native compound

| Boundary | Before ms: median [min–max] | After ms: median [min–max] |
| --- | --- | --- |
| targz/browser_open | 124.520 [123.144–279.343] | 140.552 [139.785–273.695] |
| targz/extract_all | 196.835 [179.698–208.299] | 190.012 [182.816–197.361] |
| targz/extract_selected_last | 153.156 [151.911–168.734] | 185.636 [145.585–189.940] |
| targz/materialize_last | 172.706 [169.673–222.416] | 173.673 [156.631–241.447] |
| targz/test | 155.010 [107.534–167.727] | 154.780 [109.520–183.871] |
| tarxz/browser_open | 185.683 [154.310–187.675] | 154.468 [123.647–168.021] |
| tarxz/extract_all | 213.782 [213.020–245.655] | 210.381 [181.332–212.955] |
| tarxz/extract_selected_last | 186.423 [154.253–216.668] | 202.265 [188.631–216.069] |
| tarxz/materialize_last | 219.331 [203.381–233.727] | 201.951 [192.001–221.500] |
| tarxz/test | 186.269 [182.728–200.822] | 154.668 [145.779–186.492] |

## Selection

| Boundary | Before ms: median [min–max] | After ms: median [min–max] |
| --- | --- | --- |
| extraction_plan | 164915.449 [163061.090–179398.011] | 5919.188 [5874.848–6094.364] |
| sevenz_edit_plan | 19461.484 [19431.353–19519.247] | 74.969 [73.323–79.047] |
| extract | 7088.868 [7069.453–7240.986] | 80.663 [76.941–85.262] |
| delete | 7660.492 [7573.246–7907.170] | 96.079 [88.097–97.043] |
| edit_admission | 1947.310 [1877.106–2021.746] | 19.252 [18.687–20.093] |
| browser_model_drag | 102370.244 [102124.124–102603.683] | 37990.884 [35898.795–39643.270] |

## ZIP work

| Boundary | Local checks B→A | Descriptor checks B→A | Record-admission seeks B→A | Source/addition/candidate payload decodes B→A |
| --- | --- | --- | --- | --- |
| add | 774→258 | 516→258 | 1034→388 | 128/1/129→0/0/129 |
| comment | 768→256 | 512→256 | 1024→384 | 128/0/128→0/0/128 |
| delete | 766→255 | 511→255 | 1021→382 | 128/0/127→1/0/127 |
| metadata_extract_one | 1025→1024 | 1024→1024 | 1537→1536 | 0/0/0→0/0/0 |
| metadata_open | 2048→1024 | 2048→1024 | 3072→1536 | 0/0/0→0/0/0 |
| metadata_test | 2048→1024 | 1024→1024 | 2560→1536 | 0/0/0→0/0/0 |
| overwrite | 772→257 | 515→257 | 1031→386 | 128/1/128→1/0/128 |
| rename | 768→256 | 512→256 | 1024→384 | 128/0/128→0/0/128 |

## Protected native owner

| Boundary | Before ms: median [min–max] | After ms: median [min–max] |
| --- | --- | --- |
| latency_ms | 130.377 [126.809–170.213] | 87.545 [87.252–91.267] |
| cleanup_ms | 0.446 [0.410–2.144] | 0.559 [0.538–0.588] |
| edit_open | 13.894 [13.486–30.405] | 7.185 [7.179–8.314] |
| target_admission | 6.744 [6.685–10.078] | 7.193 [6.959–7.816] |
| edit_verification | 12.364 [12.342–14.158] | 6.891 [6.588–6.943] |
| candidate_write_readback_verify | 24.754 [23.339–32.646] | 24.580 [24.184–26.446] |
| complete_edit_reads | 12.923 [12.687–15.970] | 7.111 [6.749–7.842] |
| verified_native_commit | 60.103 [57.861–66.951] | 34.791 [32.788–36.497] |

Old-object full reads: 14→7; actual old-object bytes: 117,442,220→58,721,110. Output fingerprint/postpublication reads remain 2; exact held-partial readback remains 8,388,728 bytes. Every native trial publishes once, confirms FILE-only handoff/reader close, completes cleanup and exclusively reopens its final source.

## Interpretation and limits

- One verified compound outer decode replaces two; payload routes use one ordinary TAR scan instead of two. Three retained demands use one locator Vec instead of three HashMaps. Locator bytes are cumulative backing capacity (1,344→256 in this kernel fixture), not peak/RSS equivalence.
- Selection expansion visits fall from 268,435,456 to 65,536. Drag retains descriptor presentation/collision costs; its prefix probes fall from 268,431,360 to 557,055 and original ordinal probes from 133,236,736 to 69,632.
- ZIP update timings do not establish a uniform end-to-end speedup; most ranges overlap. Small native compound ranges also overlap. These cases support work-reduction claims, not a blanket speedup; the metadata single-extract time is reported without a speedup claim.
- Native owner times include fixture staging/checkpoint assertions, not exclusive codec time. TAR worker terminal-to-exit is separately retained in JSON. Native compound cleanup completion is asserted but its separate cleanup duration is unavailable.
- Total/internal I/O and peak memory remain null. Revision bytes are not total source I/O; record-admission seeks are not all seeks; strict edit parsing is excluded from ordinary TAR scan counts. Near-exit process sampling can miss peaks.
- Debug fixtures do not establish Release, all-format, Bandizip-equivalent or physical Explorer/UI performance. G-00, complete G-04, overall Phase E and physical/Release acceptance remain open.
