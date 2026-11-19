# 0.7.2 packaged indexer acceptance

## Package identity

The accepted package is release sequence98, ID`44027b71-475c-4047-b63f-65c8fc16baed`. The installer SHA256 is`739A204CF4C519784111745B935B2CE51B3A0E6DA6CB08DF1119B82086FF7A49`.

The signed manifest authenticates108members and the package contains exactly110files. The protected Desktop, PlatformHost and Indexer files match the package and administrator-approved registry tuple. `NxToolsIndexerV1` runs as LocalSystem from the fixed protected Service directory.

## Native evidence

`tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression` passed against that installed package. The service PID changed from16276to17252. Both versioned base filenames, lengths and last-write timestamps remained unchanged, and both persisted delta files remained available.

| Observation | Result |
| --- | ---: |
| Restored base files | 2 |
| Persisted delta files | 2 |
| Sample duration | 8seconds |
| Average normalized CPU | 0.00% |
| Average process reads | 0.00 KiB/s |
| Average process writes | 0.94 KiB/s |
| CPU threshold | <2.00% |
| Read+write threshold | <64.00 KiB/s |

A uniquely named temporary file was created after the bases already existed. Its name was observed in a persisted delta, and the actual approved Desktop found its exact path before and after restart. The ordinary protected Indexer executable search also passed after restart. The probe restored its input to empty after every query.

## Probe ownership

Service probes use the existing `QuickLaunchComparisonDriver.cs` and the product's real Desktop/PlatformHost. They require the exact protected paths and hashes, select the actual child PlatformHost, preserve pre-existing operator input, and assert expected result paths and service-PID stability. They do not open the service pipe under a PowerShell identity or relax native admission.

The idle probe measures the service process, not the UI driver. Process counters include logical file/IPC traffic, not physical-disk attribution. Base metadata, persisted delta membership and before/after actual searches provide separate restoration evidence; a delta filename alone does not prove replay.

The probe source regressions pass2tests. The Windows PowerShell parser accepts all three probe scripts.

## Evidence location

Local evidence is retained under`.diagnostics/packaging/20261119-231820-947b1f16-published98/formal-acceptance/`: the idle sample, ordinary search, before/after witness results, cache metadata and`summary.json`. The temporary witness is synthetic and is not application data.

The reference boundary remains the documented Listary managed/history source and opaque native engine boundary; no current Listary native-engine source was available.

## Limits

This certifies the matched packaged LocalSystem restart, persisted-base/delta recovery, actual Desktop search, approved/runtime identity and idleCPU/I/O boundary. It does not certify a fresh installation, all Desktop feature tests, all-volume initial-build latency, cross-DPI visuals or a separate normal Desktop-exit interaction. Existing product-wide test/lint limitations remain recorded in the release notes.
