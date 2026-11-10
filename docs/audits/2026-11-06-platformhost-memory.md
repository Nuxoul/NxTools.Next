# PlatformHost memory observation — 2026-11-06

## Observed incident

Two resident `NxTools.PlatformHost` processes reported approximately 19,726 MiB and 19,637 MiB private commit. The workstation had approximately 4 GiB available commit. One process was confirmed as the child of the protected Desktop under `C:\ProgramData\NxTools\NxTools.Next\Service`; the second process's executable path was unavailable from the ordinary diagnostic process.

Both original processes exited before a managed-heap or native-allocation snapshot could be captured. Their private commit must not be described as measured managed-heap size.

The confirmed ordinary Host's log interval was 08:13:07–13:05:16. It contained 249 received and completed commands, 70 completed clipboard captures, five archive worker starts and 176 Shell icon requests. The daily PlatformHost JSONL file was approximately 1.4 MB. These counts do not establish a command or logging storm and do not identify the retained allocation.

## Controlled observation

The installed protected Desktop was started with authorization. Its newly created ordinary PlatformHost was verified by exact executable path and parent PID. Only that Host was attached to `dotnet-counters`; no elevation-helper attachment, clipboard inspection or archive/download operation was performed.

The process sampler captured 300 samples over 606.3 seconds. The runtime sampler used `System.Runtime` counters at a two-second interval.

| Measurement | First | Minimum | Maximum | Last |
| --- | ---: | ---: | ---: | ---: |
| Private commit, MiB | 33.293 | 33.141 | 39.223 | 38.922 |
| Working set, MiB | 101.234 | 101.234 | 108.895 | 108.773 |
| Process handles | 777 | 674 | 839 | 674 |
| Process threads | 33 | 27 | 36 | 35 |
| GC heap size, decimal MB | 6.115 | — | 11.459 | 0.936 |
| GC committed bytes, decimal MB | 11.694 | — | 15.946 | 15.946 |

The maximum observed allocation rate was 466,672 bytes per two-second sample. This observation did not reproduce the incident or demonstrate sustained multi-gigabyte growth. No forced collection or automatic process-restart policy was introduced.

## Source findings

`WindowsPlatformAdapter` installs OneCommander focus tracking at ordinary Host startup. `OneCommanderFileManagerOperations.OneCommanderFocusTracker` retains an `AutomationElement` per remembered top-level HWND. Entries for closed windows are removed only when that particular HWND is subsequently queried and found stale; a closed window may never be queried again. This is a concrete supported-lifecycle retention candidate, but its contribution to the observed private commit has not been measured.

The inspected parent-process wait is created once. Pipe session teardown cancels and awaits forwarding tasks; pending clipboard events are removed after publication or session retirement. The ordinary event and outgoing channels are bounded. These source observations do not exclude other managed or native allocations in the original processes.

## Limitations and next diagnostic boundary

The 19 GiB allocation source remains unconfirmed because the original large processes exited before heap/allocation capture. The controlled observation covered ordinary residency, not a replay of the original five-hour interaction history or an elevated Host.

If growth recurs, capture process private commit and GC committed/heap size for the exact live image before shutdown. Managed growth calls for heap type and root statistics; private growth without corresponding GC growth calls for native allocation/resource ownership evidence. UIA retention requires a focused create/focus/close-window reproduction before attributing the incident to that cache. Heap payloads are not diagnostic log material.

## Runtime follow-up — 2026-11-06 23:12 UTC

A read-only `Win32_Process` inventory found no running `NxTools.PlatformHost.exe`.
At 23:12:50 UTC, the exact protected-image capture entry returned:

| Evidence | Result |
| --- | --- |
| Matching ordinary Host processes | 0 |
| Capture status | `no_matching_ordinary_host` |
| Process samples | 0 |
| GC counters | Unavailable |
| Entry exit code | 2 — blocked |
| Capture directory created | No |

There is no current process on which to reproduce the private-commit growth or distinguish
managed retention from non-GC commit. The incident remains unattributed. Earlier UIA/image/queue
lifetime fixes do not establish their contribution to the original allocation.

### Reusable capture entry

`scripts/observe-platformhost-memory.ps1` reuses the installed `dotnet-counters` tool
(observed version 9.0.661903). Run from the repository root against an already-running ordinary
Host; the entry never starts Desktop or Host:

```powershell
$installedDirectory = 'C:\ProgramData\NxTools\NxTools.Next\Service'
.\scripts\observe-platformhost-memory.ps1 `
    -HostExePath "$installedDirectory\NxTools.PlatformHost.exe" `
    -DesktopExePath "$installedDirectory\NxTools.exe" `
    -OutputDirectory (Join-Path $env:TEMP ('nxtools-host-memory-' + (Get-Date -Format yyyyMMdd-HHmmss))) `
    -DurationSeconds 60 -IntervalSeconds 5
```

Use the actual exact image paths for a different installation. Optional `-TargetProcessId` and
`-ExpectedParentProcessId` additionally select a previously identified pair. Admission requires
one matching image pair, actual parent PID equal to `--parent-pid`, same session, and valid ordered
creation times. The ordinary argument shape is verified in Desktop `runtime.rs::spawn_platform_host`
and PlatformHost `Program.cs`; trusted-update, elevated-helper and archive-CLI roles are excluded.
PID/parent/creation identity is rechecked on every sample; replacement stops capture, not follows it.

The default window is 60 seconds at five-second intervals; duration is bounded to 600 seconds and
the interval cannot be less than five seconds. The entry starts only its own counter collector with
`--resume-runtime:false` and the explicit EventCounters `System.Runtime` provider:
`gc-heap-size`, `gc-committed`, `gen-2-gc-count`, `alloc-rate`. Native sampling runs in the same
fixed-process observation window. Collector shutdown has a bounded wait and can terminate only
that owned collector, never Host or Desktop. Exit 0 means complete collection, not a leak verdict;
exit 1 means incomplete/failed collection, and exit 2 means no unique admissible target.

Artifacts in the new output directory:

- `process.csv`: UTC timestamps, elapsed seconds, private commit/working-set bytes, handles, threads.
- `runtime.csv`: original counter-export timestamps, names, units and values.
- `summary.json`: pinned image/PID/parent/start/session identity, observation window, exporter time
  zone, first/last/peak native samples and whether both GC size and committed counters were present.
- `counter.stdout.txt` / `counter.stderr.txt`: collector status, not application payload output.

Compare overlapping timestamps using the recorded time zone, not CSV row numbers. Native byte
values and runtime decimal MB must be converted consistently. GC committed is not live-object
size, and private commit minus GC committed is not an exact native-allocation measurement.

### Interpretation and verification

Increasing GC heap/commit across natural collections identifies a managed-retention investigation
boundary, not a root. Increasing private commit without corresponding GC growth identifies a
non-GC-commit investigation boundary, not a specific native allocator. Preserve the same live
process for a subsequent bounded type/root or native-region/allocation investigation before
changing an implementation. Working-set reduction does not establish commit reclamation.

The isolated sampler checks cover role/image/parent/session rejection, absent/ambiguous targets,
overlapping collection, missing GC commit, target replacement and collector failure. They use
synthetic CIM/process/counter boundaries only; their numbers are not runtime evidence.

### Limitations

No live Host or GC samples were available in this follow-up, so neither the original symptom nor
the full live counter attachment was revalidated. The entry collects aggregate counters only: no
clipboard/credential/heap payload, dump, forced GC, working-set trim, automatic restart, product
launch, service operation, stress run or package build. A short stable window would not exclude
long-lifetime growth. Further attribution requires the same affected Host to remain alive.
