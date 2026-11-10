# Bandizip auxiliary malware scanner

## Artifact and boundary

The installed main program is `D:\Tool\Bandizip\Bandizip.exe`7.32.0.1. Its scanner helper is a different artifact: `D:\Tool\Bandizip\data\Amsiman.x64.exe`,234,552bytes, file version7.17.0.4. Helper addresses below are in that executable; the same image-base address in Bandizip.exe is not the same function. Ghidra12.1.4 records use the independent `amsiman-reference-session-snapshot.json` and `amsiman-reference-*` records. No helper, scan provider, suspicious payload, user archive or GUI operation was executed.

[Main scan admission and output stream](bandizip-browser-scan-details.md) records which archive members are selected, buffer limits, worker results and filesystem extraction. This note connects that caller to the helper's actual request processing, stream adapter, verdict and exit paths.

## Request and helper lifetime

Main-side `0x140186940` derives the helper path; the main request object and `0x140187d70` create/fill a named mapping `BANDIZIP_AMSI_MMF_%d`. The mapping includes the observed protocol marker `0x20210607`, a64-bit byte length, a result area and the scan bytes. The main caller sends registered `WM_AMSIMAN_CHECK_BUFFER` through `SendMessageTimeoutW` with a5-second bound.

Helper window callback `0x14000579c` registers the same check/exit message names. A check message calls `0x140005a40` with the supplied mapping identifier and returns that function's request status as the message result. An exit message posts `WM_CLOSE`; window destruction posts quit. Timer1 opens/retains a synchronization handle to the parent process and requests close if that parent cannot be opened or has exited. This is lifetime coupling to the invoking process, not a new global scanner service.

`0x140005a40` opens the named file mapping, maps its view, obtains the mapped-region facts through `0x1400023b4`, checks the observed minimum header/marker conditions and resolves the byte span/result area. It creates and retains the COM scan interface when needed. Initialization, mapping and scan failures return distinct request-level errors. After each request it unmaps the view and closes its mapping-related handles; it does not retain a live pointer to a previously unmapped archive buffer.

The helper executes the request on its window-message path. A5-second caller timeout does not establish that a busy provider was canceled or that a queued close message has already run. Main-side helper shutdown has its own bounded wait; provider work and request/window lifetime remain different events.

## AMSI stream and verdict

`0x1400066a4` wraps the mapped bytes in an `XMemoryReadStream`, creates `CAmsiMemStream`, supplies the application name `Bandizip`, obtains `IAmsiStream` and enters verdict function `0x140006840`. The helper uses the COM stream interface, not an observed direct imported `AmsiScanBuffer` function.

`CAmsiMemStream` attribute method `0x140002604` exposes its compiled string/size attribute paths, reports required output sizes and distinguishes unsupported attributes or insufficient buffers. Read method `0x14000274c` delegates provider reads to the retained memory stream and returns an error when the underlying read fails. These are provider-facing adapter calls; they do not publish normal extraction files.

`0x140006840` calls the scan interface and separates provider failure from verdict. For a successful call it maps the observed AMSI malware threshold (`0x8000`) to helper verdict1 and a lower result to verdict2, with a separate compiled test-signature special case. A failed provider call produces helper verdict3. Provider description is copied into scanner-owned string state when available; the scan result object is released before returning.

On a completed request, `0x140005a40` writes verdict and a bounded UTF-16 description to the shared result area and returns request status0. Failure returns a request-level error rather than a clean verdict. The two channels must remain distinct:

| Layer | Meaning |
| --- | --- |
| Window-message/request return | The mapping/protocol/provider request completed or failed |
| Shared verdict1 | Detection under the inspected threshold/test handling |
| Shared verdict2 | No detection under that handling |
| Shared verdict3 | Provider scan failure |
| Shared description | Optional result/provider text, not the operation's success authority |

The main wrapper `Bandizip.exe!0x140187200` maps helper verdict1/2/3 to main status4/5/2. `0x140187410` associates reportable scan results with member context; `0x1401b2df0` queues them under the original worker lock. The progress result consumer and extraction terminal then choose detection, cancellation, provider/init fault or ordinary completion presentation.

## Reporting a finding versus blocking extraction

The inspected in-process main path `0x140186f20` increments main scanner detection count `+0x90` on a finding. Normal pre-extract `0x1401b3ff0` checks the count returned by `0x1401b3a40` and exits before filesystem extraction when it is nonzero.

The helper protocol returns a verdict, not that count. The main helper branch in `0x140187200` writes the mapped verdict but does not perform the increment shown by `0x140186f20`; the worker's result-list callback only queues a record. The helper-mode path can therefore report a finding without taking that count-based stop branch in the inspected main code. This is a concrete static distinction, not evidence that every scan-before-extract configuration blocks the same way.

Communication failure has another connected path. `0x140187d70` returns false on failed `SendMessageTimeoutW` or a nonzero helper request status, retaining its own request error. In that case auxiliary `0x140187200` falls through with return value1 without assigning the result output. Member caller `0x140187410` initializes that output to0, ignores the wrapper's return value and queues only result4/5. A failed helper request therefore does not become an explicit failed member item at this boundary; it is also not a clean result5. Main scanner initialization failure and provider verdict3 have different paths. Neither the nominal wrapper return nor absence of a member finding certifies a clean scan or denial of output publication.

For NxTools, request status, provider verdict, scanned/skipped coverage, cancellation and permission to publish outputs should be typed outcomes owned by the existing job/native boundaries. A successful RPC and a clean scan are different results; a result-list notification is not the publication gate.

## Evidence map

Canonical records are under `evidence/raw/`; [the index](evidence/index.json) retains subject and provider identity. Main-program records in the table are explicitly named separately from helper records.

| Boundary | Canonical file | Evidence ID |
| --- | --- | --- |
| Helper check/exit/parent lifetime | `amsiman-reference-p-0x1400057ea.json` | `ev_66a86f9c4b414bcf5e5c8248b81bd8417a047ce84befa72d120197aab7b90f15` |
| Mapping/protocol/provider request | `amsiman-reference-p-0x140005b76.json` | `ev_7c91c4f160bcbb360346c3a85d411ff1d16544f1cbffa981d600404047becc61` |
| IAmsiStream setup | `amsiman-reference-p-0x1400066a4.json` | `ev_b78feb6149cc3f8ae910bc0f323c55f82fdbde3267eaea2c22526a556ef2cdad` |
| Stream attributes | `amsiman-reference-p-0x140002604.json` | `ev_fad2a1356fea2e097e486a996fd0e0467b0274468d9cf9d9531d68e6876c7437` |
| Stream read | `amsiman-reference-p-0x14000274c.json` | `ev_80c0b6b10e2d7edfd99e8e59ea885da61a04f0ee96b1bae3cc3f863b9265fd4a` |
| Provider verdict mapping | `amsiman-reference-p-0x140006840.json` | `ev_fc6ba2120db71fb4410b4f35a266039c645dd5e791219282162d282119020572` |
| Main shared-buffer request | `bandizip-create-p-0x140187d70.json` | `ev_cfe10765132e58beef81cb4c89cdd95ac22fc89987fe5fae7f48ceb5929a00a6` |
| Main helper-verdict mapping | `bandizip-create-p-0x140187200.json` | `ev_510475de6ded48387d07753f9ab2c48c53b00c47a4e56b4338b45ad1bc7b9561` |
| Main member result handling | `bandizip-create-p-0x140187410.json` | `ev_5c239c479c6dfe3b4cbb432f08c4831702518bd46ffce6d4050aad863cab81f6` |
| Main count-based pre-extract decision | `bandizip-create-p-0x1401b3ff0.json` | `ev_690bdc305c3401028af1b8593d98a1e21ea0ee3fe3e2e443c6cb30ceec729c64` |
| Main in-process detection count | `bandizip-create-p-0x140186f20.json` | `ev_6463c241f4c9b4d2ba00f905b73d8c2f3db0aa099c8485b5c58e3f2382961e37` |

`amsiman-target-metadata.json` records the helper's installed version/size observation. Query recipes use `rea-amsiman-reference-*`; the helper cache is independent of the main EXE and Arkview caches.

## Limitations

Static reconstruction does not establish live provider availability, definition freshness, scan effectiveness, all callback behavior or the duration of a blocked provider. The installed helper is version7.17.0.4; this report does not assert that every Bandizip7.32 distribution contains the same helper. No scan, failure injection, user payload, registry access, helper execution, product edit or installed-file change was performed. The helper/main count difference is a static control-flow finding, not a reproduced malicious-output incident.
