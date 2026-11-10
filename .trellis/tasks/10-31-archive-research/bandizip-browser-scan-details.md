# Bandizip encoding, archive thumbnails and scan admission

## Encoding selection reopens the catalog

`Bandizip.exe` menu handler `0x14014d4a0` loads menu resource `0x8c`, marks the current code page, obtains the selected command from `TrackPopupMenu`, maps it to the four-field table at `0x14026adc0`, and calls `0x14013a8d0` only when the filename code-page selection changes. The table has separate filesystem/name/comment/password values, including default, regional pages, UTF-8 and automatic detection. A menu label does not mean one universal encoding is applied to every field.

`0x14013a8d0` compares the old and new parameter block, updates the four fields at application state `+0x90..+0x9c`, and updates the toolbar presentation. When a usable archive is open and the block changed, it captures the current browser location through `0x140155d50`, reopens the current source through `0x1401579a0`, then attempts to restore that location through `0x1401aa610`. This is catalog re-reading, not a text-only redraw.

The open path is `0x1401579a0 → 0x140157cc0`: the latter copies the four application fields into the open work descriptor. The original open worker reaches `0x1400f0020`, which closes prior reader state, sends the block to archive virtual `+0x130`, installs callbacks and options, then calls archive `Open` at virtual `+0x20`. The automatic choice has a distinct conditional detection branch after a successful open and can call the archive name-decoding adjustment at virtual `+0x198`; a failed or low-confidence detection is not presented here as guaranteed correction.

After successful open, `0x1401579a0` updates the source/recent state and runs `0x140159440`, `0x14015ddd0`, `0x14015dee0` and the name-quality notice path. The actual refresh therefore consumes the newly read catalog. The captured location is a path-like browser location; these functions do not establish preservation of every prior multi-selection, row ordinal or focus state across a changed decoding result. NxTools must keep original metadata identity distinct from its decoded projection and explicitly settle selection/output plans when that projection changes.

## Archive thumbnail request and cache

The connected archive-image route is the browser's thumbnail/icon projection, not every possible image-viewer surface. List notification handling `0x140156740` resolves the row's original catalog member index. It first asks `0x1401a2ce0` for an existing image-list/cache result. On miss, a regular member obtains an extension-based placeholder, then `0x1401a2fe0` submits the source path, original member index and member name when that browser mode and name are eligible. Directory records use their separate placeholder path.

`0x1401a2fe0` ensures the preview process exists through `0x1401a31d0`. That launcher builds `Arkview.x64.exe /previewprocess` arguments carrying archive/member context, parent HWND and thumbnail dimensions, starts the process and resolves its window. Requests are not pixel buffers sent from the browser: the helper opens the archive itself through the shared library.

The request protocol uses registered messages `WM_BANDIZIP_TO_PREVIEW` and `WM_PREVIEW_TO_BANDIZIP`. Main-side request kind999 carries the member index. When the retained request context changes, kind1000 first clears the helper's pending request queue. A separately transferred password-context update has its own `WM_COPYDATA` type100; no actual password was supplied, read or recorded during this investigation.

### Helper processing and native output

`Arkview.x64.exe` preview window `CPreviewProcessWnd` handles messages through `0x1400a5550`. Kind999 appends a member index and starts timer2 at10ms. That timer takes work from the queue, calls `0x1400a6690`, and posts kind999 plus the completed index to the parent only when loading succeeded. It checks elapsed time between completed items; the observed10ms batch budget is not a deadline that interrupts one large image decode.

Archive initialization `0x1400a6470` configures and opens a retained reader, remembers source and auxiliary context, and derives a cache path. `0x1400a6690` obtains the catalog entry, refuses directories/ineligible image names, and first checks the compound-cache setup at `0x1400a7680`. Its decode path allocates storage based on the declared member size, extracts the selected member into that memory via archive virtual `+0xc0`, then passes an `XMemoryReadStream` into image-header and pixel-data loading (`0x1400619e0`, `0x140061ec0`). It scales/composites the image for the requested dimensions, encodes the result into an `XMemoryWriteStream`, and writes it under a dimension/member-derived cache key through `0x140020b10 → 0x140020cb0 → 0x140020dd0`. The last path writes the stream and commits the structured-storage object; it checks stream/read/write outcomes and returns failure on those branches. No success notification is sent after a failed load/cache write.

The reference materializes one encoded image member in memory before image decode; it is not a bounded incremental-image decoder merely because archive extraction is streamed elsewhere. This is a concrete resource/latency tradeoff, not a design to copy into NxTools without its current member/dimension/decode limits.

### Result publication, clearing and close

Main-side callback `0x1401a46d0` accepts preview results only in the applicable browser modes. It calls `0x1401a2ce0` with the completed catalog index, reads the encoded thumbnail from the compound cache through `0x1401a3de0`, decodes it, adds the bitmap to the browser image list and remembers the member→image mapping. The callback resolves the member in the current row map before sending `LVM_SETITEMW`; it does not treat the helper's member index as a list-row index.

Helper kind1000 discards queued indices and releases queue storage. It does not set the helper's archive-cancellation field or forcibly interrupt an already active `0x1400a6690` call. The archive event interface at helper object `+0x48` uses `0x1400a6680`: it reports cancellation when the window's close flag (`object+0xc0`, interface-relative `+0x78`) is set. Window destruction releases the compound cache, sets the close flag and posts quit; a periodic parent-HWND check requests close when the parent no longer exists. That is cooperative/lifecycle cancellation, not proof of instant abort while a synchronous image decoder occupies the helper window thread.

The helper result message contains a member index rather than a demonstrated generation token. The main browser's cache context and current row-map check are the connected guards; this trace does not establish that they cover every source replacement, decoding change or rapid request interleaving. NxTools should retain its own generation-scoped preview/lease/controller boundaries rather than replace them with this protocol.

## Scan admission is distinct from normal extraction

### Settings and operation intent

The settings writer `0x14016a570` establishes these separate fields:

| Setting | Offset |
| --- | --- |
| `bCopyZoneIdentifier` | `+0x4a0` |
| `bScanMalwareBeforeExtract` | `+0x4a4` |
| `amsiScanBufferSizeFreeMB` | `+0x4a8` |
| `amsiScanBufferSizePaidMB` | `+0x4ac` |

Extraction coordinator `0x14013ce80` copies the scan setting into operation field `+0x11c` only under its additional global eligibility condition. The offsets for scanner enablement, buffer size and source-provenance copying must not be conflated.

`CWorker` function `0x1401b3c80` dispatches standalone scan operation8 to `0x1401b3a40`. Ordinary extraction operations5/6/0x12 instead enter `0x1401b3ff0`. When their descriptor's scan field is set, that function computes a candidate vector with `0x1401b3860`, selects the edition-dependent buffer size, and invokes the same scan core before ordinary output extraction.

Candidate selection retains original indices from the whole catalog or requested subset, excludes directories/empty names and applies the observed size/known-size conditions. It checks a fixed executable/script/document extension set, including EXE/COM/MSI/SCR/BAT/CMD/PIF/LNK and several Office/OneNote types. This is not content scanning of every archive member, a recursively unpacked nested archive or all possible executable formats.

### Scan stream and results

`0x1401b3a40` retains a scanner object, allocates the configured bounded scan buffer, installs its `CAmsiStream` interface and invokes the archive's all/selected extraction-to-stream interface. It initializes an in-process or auxiliary-process provider path according to configuration. This scan phase does not create the ordinary filesystem outputs.

`CAmsiStream` write `0x1401883a0` accepts bytes into the scan buffer only on its owning thread and within the current member/buffer bounds. It records a distinct too-large/skipped result when capacity is exhausted. Member completion `0x140188520` scans accumulated bytes unless that result already excludes the member, then resets member state. `0x140187410` calls the provider wrapper, associates the verdict with member/name context and forwards reportable results through the worker callback. Callback `0x1401b2df0` appends those records under the worker's existing critical section for later UI consumption.

The in-process verdict function `0x140186f20` maps an AMSI result at/above the observed malware threshold to internal result4 and increments scanner detection count at `+0x90`. The pre-extract function observes the returned count: a nonzero count sets a separate detection terminal and returns before the ordinary filesystem extraction call. No-candidate/disabled scans proceed to normal extraction; scanner failures and initialization failures have separate report paths and must not be described as a clean scan.

The auxiliary wrapper `0x140187200 → 0x140187d70` copies bytes to its shared mapping, sends `WM_AMSIMAN_CHECK_BUFFER` with a5-second wait and maps helper verdict1/2/3 to main detection/clean/fault results4/5/2. Unlike `0x140186f20`, this branch does not increment the detection count used by the pre-extract stop decision. On request failure it returns1 without assigning the member result; `0x140187410` initializes that result to0, ignores the wrapper return and queues only4/5. This is neither an explicit failed member item nor a clean result. [The helper protocol trace](bandizip-amsi-helper.md) connects mapping validation, `IAmsiStream`, verdict, cleanup and parent lifetime in the separately versioned helper7.17.0.4. The UI result list and request completion must not be substituted for the permission-to-extract decision.

Download provenance is a separate feature: [the propagation trace](bandizip-provenance.md) connects its setting to source ADS capture, the retained holder and suffix-filtered direct/buffered writes. Main-file success does not establish a successful ADS copy; a scan verdict and a provenance setting remain different results.

## Evidence map

Canonical JSON is under `evidence/raw/`; [the index](evidence/index.json) records subject, provider and query parameters. Arkview addresses below belong to its own image, not `Bandizip.exe` at the same image base.

| Boundary | Canonical record | Evidence ID |
| --- | --- | --- |
| Code-page menu and command mapping | `bandizip-create-p-0x14014d682.json` | `ev_70fac5506fbff8314fc16af71fff6705471cb09f84e776815ace80aaf6623956` |
| Four-field update/reopen/restore | `bandizip-create-p-0x14013a989.json` | `ev_3f7bad7a36432d16e9f3b4fb82a46a36354f6ecaee30119de54cb3f581ceeb70` |
| Open work descriptor | `bandizip-create-p-0x140157cc0.json` | `ev_fd828c460cd299f6931bf2da0cc4fd23035aefb5e9d25ae67394473bed03715d` |
| Reader settings/open/automatic branch | `bandizip-create-p-0x1400f0020.json` | `ev_e941f92f844c86d7af3513131ba0cef457fb38ec34e15d20a99d8ecbc66b9ee8` |
| Browser image request | `bandizip-create-p-0x140156be3.json` | `ev_ca5f4a5be613491a0814bb2354d10607e30866d46b33375f9ee6a3f6f8714687` |
| Main request/queue clear | `bandizip-create-p-0x1401a3062.json` | `ev_133776f638241d1fe08eff6d1bf69f9c511fb4cc02b8aadc82edbc1ded3ef9bf` |
| Preview process launch | `bandizip-create-p-0x1401a353b.json` | `ev_470dfef1b4b212b59f8bc64f60db29117422e0a623e6fb012e7220e36a866328` |
| Helper queue/result/close | `arkview-reference-p-0x1400a56b6.json` | `ev_48047c1f839853c370c9a63788eef6dd365a1a56fe3e893b4a9e27d99563c7f6` |
| Member memory extraction/image/cache | `arkview-reference-p-0x1400a6690.json` | `ev_422168061946be50b89e0f9208d847a8aa078da517a4c8d639e3434f6f2ca86a` |
| Helper cooperative cancel | `arkview-reference-p-0x1400a6680.json` | `ev_c51a1db87dabeef025b8a61738b15e5b70e8fc9f1205347337171710fff0133c` |
| Structured cache write | `arkview-reference-p-0x140020dd0.json` | `ev_0db8bfd07a52d7ce748d509c8ad535983c0f12d1cb4b998f64c2f80c6aac2da2` |
| Main cache read/image-list mapping | `bandizip-create-p-0x1401a2ce0.json` | `ev_4ef14a356ec2a64ed772aba1394c17f48f7c817b3e784ea804efae5250be1206` |
| Main result/current-row lookup | `bandizip-create-p-0x1401a48f6.json` | `ev_0c981ee123cdfdf158c6d2825fa15958262687d128ed4849d049272083a574e2` |
| Scan versus extract dispatch | `bandizip-create-p-0x1401b3c80.json` | `ev_1e9792297dc29037af59e283d218dde985855d1e867712764b1a5dad91573ae9` |
| Scan before filesystem extraction | `bandizip-create-p-0x1401b3ff0.json` | `ev_690bdc305c3401028af1b8593d98a1e21ea0ee3fe3e2e443c6cb30ceec729c64` |
| Candidate member selection | `bandizip-create-p-0x1401b3860.json` | `ev_13e4360d55ac21973dc4f0d5089001c307a54a857d5ec72ef68e119decb62cbf` |
| Bounded extraction-to-scan stream | `bandizip-create-p-0x1401b3a40.json` | `ev_2a5468be816736e0b48ef0a10e8a7300ef238af5b4575e59a3a93b1dfe50eeda` |
| Scan stream member completion | `bandizip-create-p-0x140188520.json` | `ev_0d24b063a8c6d38b7406d476f9c96a0bd02a170598b8a41f327e10a1908406cc` |
| In-process verdict/count | `bandizip-create-p-0x140186f20.json` | `ev_6463c241f4c9b4d2ba00f905b73d8c2f3db0aa099c8485b5c58e3f2382961e37` |
| Auxiliary verdict mapping | `bandizip-create-p-0x140187200.json` | `ev_510475de6ded48387d07753f9ab2c48c53b00c47a4e56b4338b45ad1bc7b9561` |
| Worker result queue | `bandizip-create-p-0x1401b2df0.json` | `ev_91be2f752eac937ab3a4517f001e9c8bc10f9e21826953c00268da4ff3e34d9d` |

## Limitations

These are static Ghidra12.1.4 reconstructions of the shipped main7.32.0.1 images; the separately traced Amsiman helper is7.17.0.4. No real GUI action, password, provider scan, malicious file, registry value or user archive was exercised. Encoding re-read does not guarantee universal auto-detection or unchanged multi-selection. The connected preview is the archive thumbnail route; other preview surfaces and all image codecs are not covered. Queue clearing and cooperative close do not prove bounded interruption of synchronous extraction/image decoding. Scanner count/request-result differences are static findings; live provider effectiveness and cross-mode behavior remain untested. Product code, validation budgets, dependencies, services and installed settings are unchanged.
