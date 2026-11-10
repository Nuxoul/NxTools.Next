# Bandizip parallel ZIP extraction

Target: shipped Bandizip7.32.0.1 `D:\Tool\Bandizip\ark.x64.dll`, inspected through REA/Ghidra12.1.4. Static reconstructed behavior is not original source; no proprietary implementation code is reused. The [shared ZIP trace](bandizip-core-pipeline.md), [output/failure trace](bandizip-failure-cleanup.md) and [operation lifecycle](bandizip-operation-lifecycle.md) retain their existing scope.

## Runtime evidence supplied by the user

Same workstation, same `CapsWriter-Offline-20260914.zip`, `bz.exe x -aoa -y`:

| `HKCU\Software\Bandizip\useParallelExtraction` | Process wall time | Additional observation |
| --- | --- | --- |
| 1, current default | 1.44–1.98s | Parallel extraction enabled |
| 0 | 11.8 /12.1 /14.6s | CPU0.75–0.94s, peak about12process threads |

These are user-supplied measurements, distinct from the retained NxTools/Bandizip nine-trial results. Process thread count is not the number of active ZIP decode tasks. The configuration contrast identifies cross-file concurrency as the next product boundary; it is not a measurement of exclusive IPC time.

## Persisted setting, default and Ark options

The static configuration chain is in `Bandizip.exe` and `ark.x64.dll`. Settings singleton constructor `0x140163500` zeroes a0x6a0-byte object. Initializer `0x1401641c0` leaves settings`+0x2a0` at0 unless the observed version token returned by `0x140022780` is greater than0x11 and drive classifier `0x1401aac10("c:\\")` returns1. The classifier requires a fixed drive and successful `0x1401aaa20` storage-property checks: property8 true, property7 false, consistent with TRIM support and no seek penalty. This is a conditional static default, not a universal default1 claim; the supplied workstation has the setting enabled.

Settings reader `0x140164e40` reads `HKCU\Software\Bandizip\useParallelExtraction` into the32-bit field at settings`+0x2a0`. Assembly around `0x140166c9a` passes both the field address and its current/default value to `0x140062d20`, called at `0x140166ce8`. That helper initializes the destination with the supplied default before a four-byte `RegQueryValueExW` and replaces it only on successful query. Alternate settings-file mode uses `0x140043110` for the same field.

Operation mapper `0x1400f14c0` reads settings`+0x2a0` at `0x1400f1591` and stores it at `[RBP+0x3]`. Its0x80-byte IArk options block starts at `[RBP-0x69]`, so the field is exactly options`+0x6c`. When the source-path branch applies, an enabled value can be cleared if the source drive classifier does not return1; operation state`+0x132` can force it to1, and `param_3[2]` clears it. At `0x1400f164e/0x1400f1655`, the mapper invokes IArk vtable slot`+0x150` with this options block.

The concrete setter `ark.x64.dll!0x1800084a0` copies the0x80-byte block into `CArkMan+0x98`. The retained extractor context, referenced through decoder`+0x788`, exposes the same options`+0x6c` consumed by selected extraction `0x180059a90`. This is an options-structure boundary, not a separately observed setting enum.

The EXE initializes the qword beginning at options`+0x70` to0. Its low dword is the worker-count input; no retained settings/checkbox mapping supplies a nonzero explicit count. The `param_3[2]` branch writes1 to the high dword at`+0x74`, not to count`+0x70`. Ark therefore selects its automatic count below.

The GUI initializes checkbox control`0x4db` from settings`+0x2a0` in `0x14019779c` and writes `BM_GETCHECK` back to that field in `0x1401979f0`. Settings writer `0x14016a570` persists it as `useParallelExtraction`.

## ZIP admission and dispatch

`CArkUnZip` constructor `0x180023d70` installs primary vtable `0x1801f1770`. The selected extraction owner `0x180059a90` checks configuration `*(decoder+0x788)+0x6c`, ZIP parallel-eligibility byte `decoder+0x792` and the selected count. Ordinary ZIP needs at least two selected members. The parallel virtual slot `+0xe8`, verified at `0x1801f1858`, is `0x180075020`.

After catalog loading, `0x180023fe0` computes `+0x792` from retained member metadata. Non-directory entries with compression method other than0/8, unknown size fields, missing/inconsistent metadata, nonzero encryption-related fields or the tested special bit clear eligibility. The actual parallel worker dispatch `0x1800b5820` has Stored and Deflate branches only. This is not evidence for parallel encrypted ZIP, every extra ZIP method, or solid-format parallelism.

`0x180075020` constructs `CArkParaExtMan`, invokes `0x1800a1110` for regular files and, after successful worker completion, handles selected directory entries through the ordinary per-member owner. Directory metadata finalization is therefore after file workers, not interleaved with later child creation.

## Worker count and independent input

`0x1800a1110` uses the configured count at options`+0x70` if nonzero. Automatic count is `max(1,min(logical_processors,64)/2)`. Actual workers are limited by selected count and32. All created workers are joined before manager resources and per-worker input streams are released.

Preparation `0x1800a1b40` requires a `CArkWinIn` source, creates one independent input-stream object per worker from the original input's source identity, and fills a shared queue with selected non-directory, non-excluded original member indices. Each input has an initial64KiB buffer. If this input preparation is unsupported, the caller signals the original selected owner to use its sequential fallback.

## File assignment and large-file pressure

Worker entry `0x1800b6010` marks its state, runs `0x1800b5d70`, then marks completion. Workers pop a whole member index from the shared queue under the manager mutex. They do not split one Deflate member among several workers. There is no separate small-file bucket or size-sorted queue in this inspected preparation/dequeue path.

The worker checks active member count at state`+0x1830` and summed active member-size field`+0x1828`. If at least two members are active and the sum exceeds100MiB, it releases the mutex and sleeps1ms before retrying task acquisition. It increments the active totals before executing `0x1800b5820` and subtracts them afterward. This is task-admission pressure control, not a hard100MiB allocation ceiling: the test precedes the next member's addition.

The pre-member hook `0x180074fa0` calls existing local-header/data-offset preparation `0x1800576f0` with that worker's input. `0x1800b5820` then performs output preparation, Stored/Deflate streaming, original CRC/size/error handling, metadata/close and output release for the current file. Multiple workers can concurrently own different source cursors, codec state and output files.

## Questions, errors, cancellation and completion

Workers enqueue callbacks under the manager's critical section and signal its event. Per-member preparation uses message kind2; conflict/question forwarding uses kind4. The caller thread drains these in `0x1800a0c00`, invokes the original callback and signals the waiting worker's event. Conflict prompts therefore stay serialized even though file work is concurrent. Workers periodically check the manager stop byte while awaiting replies.

Error messages use kind1, completion notification uses kind3. The coordinator updates original error/stop state and calls the existing error/member callbacks. The reconstructed parent error field can be assigned from processed messages; this is not evidence of a universal first-error-CAS guarantee. NxTools' required first-error sticky contract remains independently binding.

The manager propagates the original cancellation/error flags into stop byte`+0x1818`; codec callback `0x1800a20c0` reads it. Stopping task acquisition is not completion. The manager continues draining queued worker messages and joins every created thread before releasing inputs/events/locks. Per-file direct-output close/delete behavior remains owned by the output object described in the linked failure/output traces; no whole-job rollback is introduced.

## Progress aggregation

The `CArkParaExtMan` secondary interface is at vtable`0x1801f7008`; `0x1800a1fe0` adds worker input/output deltas to shared totals`+0xd0/+0xc8` under the manager mutex. The coordinator `0x1800a0c00` samples the cumulative output total, computes the delta from its previous sample and forwards it to `0x180058040`. Workers do not independently replace the original overall progress snapshot. Question/error callbacks and progress aggregation remain on the original operation owner.

## Evidence map

Complete records are in the fixed local `evidence/raw/` root, with replay recipes in `raw/recipes/rea-ark-parallel-*.json` and target-matching `ark-parallel-20261107-snapshot.json`.

| Boundary | Record | Evidence ID |
| --- | --- | --- |
| ZIP constructor | `ark-parallel-dispatch-p-0x180023d70.json` | `ev_0a15f8bcd5231ada6be7d39b4057489e0144b25afd730be043da6cce8460db7f` |
| Eligibility | `ark-parallel-dispatch-p-0x180023fe0.json` | `ev_4bfb1773e4c7f1ccf04cf5a88abd379b3fc3946038ac24a75d195ea33b5b6851` |
| Virtual target | `ark-parallel-slots-b-0x1801f1850-32.json` | `ev_cb3102a849b7d0b04fb72122c4bfcaf9ef2a4e7b76fb455d61b8439d522324fe` |
| Parallel entry | `ark-parallel-scheduler-p-0x180075020.json` | `ev_a75d1d3847f61ddba83070c054076c78ea6768eb68ce31c754cf57b09928774a` |
| Count, stop, drain, join | `ark-parallel-manager-p-0x1800a1110.json` | `ev_cfe7aff527d534946f1e41bc5af61c43a630e19496971a7465e22df49260b095` |
| Independent inputs and selected queue | `ark-parallel-workers-p-0x1800a1b40.json` | `ev_52d53ce868a69c308d33b007e5442855335d16fc3a55bd77400be15ae179fbd0` |
| Dynamic dequeue and large-file pressure | `ark-parallel-member-p-0x1800b5d70.json` | `ev_372a0e25c566205161c8d160f86c6fcb2d10e0e13e854e45098ad4312176f27e` |
| Local-header preparation | `ark-parallel-member-p-0x180074fa0.json` | `ev_4159c7f0d2d36f4d284f5b0d604c0717c7850e508a0e7569b9a966131aeea898` |
| Actual member/output work | `ark-parallel-outcomes-p-0x1800b5820.json` | `ev_d8352a0ca0862f8a1b70007b8aba3b1a278d87e83765d0855680bd3cdbaf8bfe` |
| Message enqueue | `ark-parallel-outcomes-p-0x1800b6080.json` | `ev_3241f58c720e396ba09a7e5b67b8421bb758d23b57113b53745a2b4dced5d2ac` |
| Serialized callback/event drain | `ark-parallel-workers-p-0x1800a0c00.json` | `ev_1f4edac9e71792837f3cdd6ce2ae5034b9ffc76665510f88e9a2a68cc55e9ba7` |
| Progress totals | `ark-parallel-aggregate-p-0x1800a1fe0.json` | `ev_a01ac62502f8c65ecd86b3fce04c167fbba503fdd7e803ee2feda62c288839b7` |
| Codec stop callback | `ark-parallel-aggregate-p-0x1800a20c0.json` | `ev_56f689c395e53ee5434b59e2e8e02b77ab87a3a10d27dcc3d6d464bfd69574cc` |
| Conflict forwarding/wait | `ark-parallel-aggregate-p-0x1800a20d0.json` | `ev_da4e642e2b9688823aefbc1589bf0807473ae6034a983e090eb8e05d1c54e385` |

| Configuration boundary | Record | Evidence ID |
| --- | --- | --- |
| Setting-name references | `bandizip-create-xrefs-0x1402a6350.json` | `ev_588b666bb1ae7704b181e13b9699b0ce43b8a8d18c49a54560249f67ecbd1d52` |
| Zero-filled settings singleton | `bandizip-create-p-0x140163500.json` | `ev_b7628eb2f16482d8ccdf3a595e340219314fe251924370fe64b5c7419135de1e` |
| Conditional default | `bandizip-create-p-0x1401641c0.json` | `ev_b647d7e7e3fca79b912809027ca6166598516ba8f794e725c6199d5611461f74` |
| Drive classifier | `bandizip-create-p-0x1401aac10.json` | `ev_8d211b3e560eb02bbc33b2787d6f9108b1ca725ac6cd9ad34c3430dcafe2d7d5` |
| Storage properties | `bandizip-create-p-0x1401aaa20.json` | `ev_8403783b12a139437374af38796af90637da8a820e28d431bc39f4224abab3de` |
| Reader field/default assembly | `bandizip-create-asm-0x140166cc9.json` | `ev_0b238ced433319afccc7a5ba829cea32f16dffb63f107e54706c51019b0e8723` |
| Registry/default helper | `bandizip-create-p-0x140062d20.json` | `ev_b15776c459de6710686bc9f0cbcaa2cc73d7ae138d94bb77544a8fd62d1fd124` |
| Settings reader | `bandizip-create-p-0x140166cc9.json` | `ev_780bf49c1a9103f28fbec93b8876abce53a551760d67954b5cf464d02e654d0b` |
| Settings writer | `bandizip-create-p-0x14016b4ae.json` | `ev_f7eb1f34393d2af2b6f901eba01fee1e278ebb82bfaec99a52188e7517dff6cf` |
| Application-to-IArk options assembly | `bandizip-provenance-asm-0x1400f14c0.json` | `ev_9c145989c2db9fee4fa0a2cbf0444dfa8aa29eb0ca7e6a62a426fe98a0e90821` |
| Application-to-IArk options reconstruction | `bandizip-provenance-p-0x1400f14c0.json` | `ev_c921b0129e4d31f5d08591463ec7a657fcc5c64ce302e572a4b005d5ee1548a2` |
| Concrete options setter | `ark-provenance-asm-0x1800084a0.json` | `ev_c0a550573bd61f489f764e017788c72f2d6a4a9f1eb82e51f610867daab0c33a` |
| Checkbox initialization | `bandizip-create-p-0x14019779c.json` | `ev_ef5dd45638a132418d47b152479fb046683f12e3bd0511f4f82d151227d7b3e5` |
| Checkbox writeback | `bandizip-create-p-0x1401979f0.json` | `ev_d6138a496e8cda8875f3b0344d5ccf1b65add296ebd2d8db0ef35f8ca717650e` |
| bz defined-string search | `bz-create-parallel-config-20261107-bz-strings1-strings-0.json` | `ev_c1d378908d07bef226f30545bfaa1a5ffd2ba60a0afffe32a9fe30eaa16a652c` |

## Limitations

The retained `bz.exe` defined-string search has no matching setting string and does not establish a static bz-to-Bandizip configuration handoff. The complete static chain above belongs to `Bandizip.exe` and Ark; the supplied bz setting/timing contrast remains separate runtime evidence. The exact semantic name of the numeric version token used by the default gate is not established. Additional formats, encrypted/special-method ZIP parallelism, live conflict/cancel interaction and independent Bandizip output-cleanup trials are outside this static path. Existing NxTools source/path/reparse, actual-byte, integrity and cleanup contracts are not weakened by the reference.
