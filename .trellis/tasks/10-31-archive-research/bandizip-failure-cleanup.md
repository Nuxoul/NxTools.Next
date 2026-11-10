# Bandizip failure, cancellation and output cleanup

## Connected settings and owners

The inspected Bandizip7.32.0.1 path separates operation stop, the current member's output object, native output close, optional deletion and final progress presentation. Canceling a worker is not itself proof that every completed output was rolled back.

Settings writer `Bandizip.exe!0x14016a570` records `bDeleteOutfileWhenUserCancel` at settings `+0x4f8`, `bUseSetEndOfFile` at `+0x4fc`, `bUseLongPathName2` at `+0x4dc` and `bOpenArchiveWithShareDelete` at `+0x4e0`. Existing options helper `0x1400f14c0` places these in the IArk block, which slot `+0x150 → ark.x64.dll!0x1800084a0` copies into the manager's shared context at `+0x98`.

| Setting | Options/context offset | Connected use |
| --- | --- | --- |
| Delete output on cancellation | `+0x38` | Progress-event cancellation invokes current output Close with deletion requested |
| Output preallocation | `+0x60` | Native preallocation helper's admission condition |
| Long paths | `+0x24` | Direct-output path conversion and retry setup |
| Share-delete source open | `+0x2c` | EXE selects input sharing value1 or5; the complete setting-to-every-source-open fallback mapping is separate from the output paths below |

The context is also the one established in [provenance propagation](bandizip-provenance.md). File deletion policy and downloaded-source ADS copying occupy different fields.

## Stop request and the current output

The [operation owner](bandizip-operation-lifecycle.md) connects confirmed Stop to worker control state0, thread completion/handle release, dialog cancellation state `+0x592` and modal result2 or modeless destruction. Declining the stop confirmation restores the prior pause state. This is distinct from releasing a native output object.

Native progress callback `0x180058040` runs on the extractor's event subobject. It publishes progress through the caller event interface, receiving stop/cancel fields at interface-relative `+0x7a4/+0x7a8`. When either is set, it assigns native cancellation result `0xa0000017`. Only when context `+0x38` is enabled **and a current output pointer was supplied** does it call output vtable `+0x48` with argument1.

The subobject adjustment matters: this callback reads its context at interface-relative `+0x780`, the same pointer retained at extractor-object `+0x788`. It is not a second independently assembled cancellation policy. Member-begin callback `0x1800583c0` can reject work before any output exists when its event sets cancellation; that path has no newly opened current output to delete.

Direct output primary vtable `0x1801f5840` binds `+0x48` to `0x180060a10`. The buffered primary table binds it to `0x180023620`. In both cases argument1 requests path cleanup after the object's close sequence. The cancellation callback does not consume that close return. Ordinary destructors call Close with argument0, not an unconditional delete.

This connects the setting to **current-output cleanup**. It does not establish a transaction that removes all earlier successfully closed members. The connected operation finish/selection owners do not provide such a rollback guarantee.

## Direct and buffered close failures

Direct close `0x180060a10` flushes any pending local bytes through the native byte writer, handles a preallocated tail, applies requested timestamps, closes the file and, when explicitly requested and a retained pathname exists, calls `DeleteFileW`. Pending-write failure affects its close result. The deletion API return is not used to turn that result into proof that the path disappeared.

Buffered close `0x180023620` requests the existing output worker to stop and waits for it before EOF/metadata/handle operations. The worker drains queued bytes rather than immediately discarding them; the task-cancel callback is upstream, not inside the ring queue. After that sequence, deletion is conditional on the close argument and retained pathname. Its `DeleteFileW` return is likewise not a separate confirmed cleanup result. [The output report](bandizip-solid-output.md) retains producer backpressure, error storage, drain events and destructor lifetimes.

Member routine `0x18005a4c0` distinguishes output preparation, decoder result, close/finalization and decoded-size validation. A failed decode closes with argument0 in that path; cancellation's separate progress callback may already have requested cleanup. Generic output failure is therefore not automatically the same as cancel-policy deletion.

Direct member finalization `0x18005a6a0` maps an output close failure with native code `0x70` (Windows disk full) to archive result `0xa0000054`; other close/write failures have their own archive result. The main EXE error renderer `0x14011e5c0` maps that archive code to the disk-full message. The presence of a destination file is not the close result.

## Preallocation is a conditional attempt

Output vtable `+0x50` enters direct `0x180060c10` or buffered `0x180023880`, both reaching `0x18006fb90`. That helper requires enabled context `+0x60`, a requested size greater than4096bytes and at most1GiB, current position0 and a successful seek to the requested size. It then calls `SetEndOfFile` and seeks back to0.

Successful expansion records the preallocated extent for later close/truncation. A failed `SetEndOfFile` does not by itself make this helper return failure; restoring the position is a separate result. Requests outside the observed range skip the attempt and return success. This range is a preallocation choice, not a maximum supported output-file size. Subsequent actual writes and final close still own disk-full/output errors.

## Long-path output setup

Direct output open `0x180060660 → 0x1800606c0` uses context `+0x24`. When enabled and the resolved path exceeds259 UTF-16 code units, it invokes `0x180009f80` and adopts the converted string if available. It closes any previous handle, tries the native create helper, and can invoke `0x18000ab70` before retrying. Failure of the create/retry remains an output failure; enabling long paths is not evidence that every pathname or filesystem will accept the destination.

The source input's sharing fallbacks and the long-path output conversion are different owners. The observed `0x18000dac0` source helper tries read-only native opens with progressively different sharing values on sharing failures, but these records do not establish a complete mapping from the EXE share-delete setting across every source interface.

## Capacity confirmation and native error presentation

Extract-path callback `0x140113fd0` dispatches `WM_COMMAND/1` to submit `0x140115d10`; Cancel command2 calls `EndDialog(...,2)` without that submission. Submit reads the all/selected controls into its selection field and calls `GetDiskFreeSpaceW`, multiplying reported sectors-per-cluster, bytes-per-sector and free clusters. If that query succeeds, the selected retained size is positive and required bytes exceed available bytes, it formats destination/free/required values through `0x140044be0` and asks whether to continue. Non-Yes returns without ending the dialog; Yes reaches ordinary option/history persistence and `EndDialog(...,1)`.

The all/selected branches use retained totals at form `+0x168/+0x170`. Unknown/nonpositive size or failed disk-space query does not take that warning branch. It is a pre-submit comparison, not reserved capacity or a guarantee against later disk-full failure. The complete form constructor/show boundary remains distinct from this connected submit path.

Renderer `0x14011e5c0` selects typed text for cancellation, disk full, unsupported data, corrupt data and access denial. For destination access-denied code `0xa0000062`, helper `0x14002c6b0` reads the current process token's elevation type to select ordinary versus UAC-specific copy. The renderer builds a message; it does not relaunch the job with elevation or authorize a retry. [Batch lifecycle](bandizip-operation-lifecycle.md) separately records archive-open failure logging/next-item advancement and explicit stop ending the batch. A message renderer alone does not determine every decoder's per-member continuation policy.

## Opening a later volume

First-volume resolver `0x1400d2b50` constructs a candidate from supported filename patterns, including the inspected RAR and EGG branches. It returns a candidate/result; that is different from successful admission of all required volumes.

Browser open owner `0x140157cc0` uses that resolver after applicable open errors and constructs `TEXT_ASK_OPENERR_FIRST_VOLUME`; accepting its prompt reaches the reopen path with the candidate. Batch open `0x14011bca0` has its own applicable error/count conditions: it replaces its current source path with a resolved first-volume candidate and calls the open worker again. Neither path is a demonstrated general missing-next-volume chooser, and a resolved name does not prove all parts exist or decode correctly.

## Evidence map

Files are relative to `evidence/raw/`. EXE and DLL addresses refer to their separately retained7.32.0.1 subjects. Ghidra12.1.4 reconstructions and supplementary Capstone instruction spans are static observations, not original source or execution.

| Boundary | Canonical file | Evidence ID |
| --- | --- | --- |
| Settings fields | `bandizip-browser-code-from-0x14016ae74.json` | `ev_f9fb926fd126cdfa26cdb64777d5f3c89aa2efa8a5ca0940a26c332774e531d2` |
| EXE options mapping | `bandizip-provenance-p-0x1400f14c0.json` | `ev_c921b0129e4d31f5d08591463ec7a657fcc5c64ce302e572a4b005d5ee1548a2` |
| Stop confirmation/join | `bandizip-operations-code-0x140118b20.json` | `ev_dd748f9d958c1bc6e1f72703044baea926c18a473983d3b0d9e46dfa3def283f` |
| Cancel-to-current-output close | `ark-failure-p-0x180058040.json` | `ev_6f8c0b32275d936f89b821b350d2b457368c97f7b855061a5af1262b0f792682` |
| Cancellation before member output | `ark-rar-code-0x1800583c0.json` | `ev_deaca93290ab11dc530d9e034dde7821ea9e98d46e164556b2410df9603b9102` |
| Direct close slot | `ark-failure-bytes-0x1801f5840-168.json` | `ev_4fd3110bb3f5968aa628505aadb190eeb93c28595d73e2ac9d964c90c84a2d8a` |
| Direct close/native deletion | `ark-core-code-0x180060a10.json` | `ev_6a65cd62820ad25a5643c36a76470513b005f409ba31acdac8019aef547c678d` |
| Buffered drain/close/deletion | `ark-provenance-p-0x180023620.json` | `ev_deef005e936c46e7ca136320149235e4092e76d10ae07fd684004097f96bd1c2` |
| Prepare/decode/finalize/size result | `ark-core-code-0x18005a4c0.json` | `ev_42e091e0a8474bcb7b5d9238ae7639aea0f3b153c16638c4ce02203538d7ee74` |
| Finalization/disk-full mapping | `ark-provenance-p-0x18005a6a0.json` | `ev_35dab75b48ae90fc1767d58510761308d0a50f44be8e811619e2f670f858f261` |
| Preallocation conditions/API/result | `ark-failure-p-0x18006fb90.json` | `ev_f6f361dafc0e26306bca09cf56ffbc72518cc3456dd07e4cfb0cad7a507478e0` |
| Direct output path/setup/retry | `ark-failure-p-0x1800606c0.json` | `ev_6385fb878a9bfc1a683a68fcb9c58ee92a90a0abee1bb3169a03dc1a9f67a064` |
| Source sharing fallback | `ark-core-code-0x18000dac0.json` | `ev_dd864ee1eba9429a3a9b7c2f9d7dcabe76292e9afdd02583d0cdf50175414d24` |
| Extract-path capacity/acceptance | `bandizip-failure-p-0x140115d10.json` | `ev_b462cc46a2eaafbeecd83fe07d344823d5546a8633f2509cadfc7070a91f6701` |
| Extract-path OK/Cancel commands | `bandizip-failure-p-0x14011407d.json` | `ev_07cffed50affeadb97a9d6d944351a5a8fc3bc200a4cb34e01d8fd0740b18ce4` |
| Capacity message fields | `bandizip-failure-p-0x140044be0.json` | `ev_5c98dc61b8c82173c0c2ae5e1f8d9ba27e691c2473561ee6296426e1679000ba` |
| Typed native/archive error rendering | `bandizip-failure-p-0x14011e5c0.json` | `ev_a287be309a227a0d44adf00386c7e69ba4382dbdf86f9737b4b2680da05296d3` |
| Elevation-aware error wording | `bandizip-failure-p-0x14002c6b0.json` | `ev_908cc7cdb89f4116c17ac368698d65bbd72919db2a955c10c6186278930aa1f6` |
| First-volume name resolver | `bandizip-failure-p-0x1400d2b50.json` | `ev_905566db3240c7f0a2ca4cb82fdaab97052babe570341326b605958e71b56377` |
| Browser first-volume prompt/reopen | `bandizip-create-p-0x140157cc0.json` | `ev_fd828c460cd299f6931bf2da0cc4fd23035aefb5e9d25ae67394473bed03715d` |
| Batch first-volume reopen | `bandizip-operations-code-0x14011bca0.json` | `ev_12f0c4c436c53ee99b76c039aff032b86710f562369b0bb75c7a49751cadfb73` |

Supplementary `ark-failure-static-output.txt` retains actual destructor/close arguments and the direct/buffered preallocation wrappers; `ark-failure-static-cancel-tail.txt` retains the event-subobject adjustment and imported `SetEndOfFile` call. These have no REA Evidence IDs.

## Limitations

No stop, disk-full, permission, volume, long-path or cleanup failure was induced. This report establishes connected static branches, not live responsiveness, deletion success, every codec's error policy, a complete missing-volume workflow, all source-sharing modes or a whole-task rollback contract. The full extract-path form constructor/native show boundary and special output types remain separate coverage gaps. No product logic, archive, output directory, user settings, service, installed binary or package was changed. Applicable-law/EULA restrictions remain in [the reference note](../../../docs/reference-code/archive.md).
