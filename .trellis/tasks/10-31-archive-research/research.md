# Selected-entry extraction and archive-form research

Status: research baseline. [Repair results](outcomes.md) record the subsequent approved implementation and checks. This note retains the supplied ZIP, production-boundary probes, owned native-worker measurements and bounded Bandizip REA observations.

## Findings

### 1. A normal ZIP member is rejected before native drag starts

The supplied member is `companion\dist\bcprov-jdk18on-1.78.1.jar`: 40 UTF-8 bytes in the complete path, not an overlong filename. Its original ZIP name contains literal backslashes. Of the sample's 215 members, 197 have backslashes in their original names.

The metadata path preserves that decoded name. Desktop directory projection and drag validation split only on `/`; drag's component validator then rejects the remaining `\`. Ordinary extraction already accepts both separators. This is a projection/validation inconsistency, not a corrupt member or a necessary Windows restriction.

A scratch Rust probe directly includes the production model module and calls the real `freeze_drag_selection`. Original input reproduces the reported rejection (exit 2); changing only the input separators to `/` succeeds (exit 0). The rejection occurs before OLE or extraction.

Source evidence:
- `repo:rust/crates/nxtools-archive/src/formats/zip.rs#list`
- `repo:rust/crates/nxtools-archive/src/worker_metadata.rs`
- `repo:rust/apps/nxtools-desktop/src/archive/model.rs`
- `repo:rust/apps/nxtools-desktop/src/archive/model_drag.rs#freeze_drag_selection` (slash-only check at line 224)
- `repo:rust/crates/nxtools-archive/src/path_policy.rs`
- `repo:rust/crates/nxtools-archive/src/planning.rs`

A separate supported-name limitation also exists: `valid_job_name` limits UTF-8 bytes to 255. An ordinary 86-character Chinese filename can exceed that byte threshold while remaining below Windows' component limit. This is distinct from the real `FILEDESCRIPTORW` filename capacity, measured in UTF-16 units. Neither limit explains this 40-byte ASCII path.

Evidence: `repo:rust/crates/nxtools-protocol/src/archive_job.rs#valid_job_name`, `repo:rust/crates/nxtools-protocol/src/archive_virtual_drag.rs`, `repo:rust/apps/nxtools-desktop/src/archive/model_drag.rs`.

### 2. Browse is rejected before the system dialog, then leaves the form pending

The actual command chain is:

`OperationForm::pick` → `ArchiveApplication::pick_for_form` → `PlatformBridge::send_command_with_id` → `ArchivePickerCommand.Parse`.

Rust sets the envelope ID, but leaves `correlation_id` null. C# requires `envelope.CorrelationId == OperationId`. The parser therefore rejects the command before `ArchiveJobPicker.Pick` or `ShowDialog` can run.

The production Rust bridge probe captures the null correlation. A scratch C# reflection probe invokes the already-built, real parser: the current wire throws `InvalidDataException`; a correlated control is accepted. Neither parser probe opens a dialog.

The rejection returns a normal `CommandResult`. Desktop handles picker completion only through the `ArchivePicker` event; its `CommandResult` branch does not retire picker state. The application map and `OperationForm.pending_picker` remain pending, and `busy()` disables controls.

The intended valid path already uses a real Windows `FolderBrowserDialog.ShowDialog(owner)` on the bounded STA owner. Replacing it with a custom GPUI directory browser would not address this defect.

Source evidence:
- `repo:rust/apps/nxtools-desktop/src/archive/forms.rs#pick` (line 1063)
- `repo:rust/apps/nxtools-desktop/src/archive/jobs.rs#pick_for_form` (send at line 1375)
- `repo:rust/crates/nxtools-ipc/src/lib.rs#send_command_with_id`
- `repo:csharp/NxTools.PlatformHost/Protocol/ArchivePickerContract.cs#Parse` (line 16)
- `repo:csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs` (lines 348–356)
- `repo:rust/apps/nxtools-desktop/src/archive/jobs.rs#job_notification` (event at line 959; command result at line 986)
- `repo:csharp/NxTools.PlatformHost/Windows/ArchiveJobPicker.cs#Pick`

Related reachable lifecycle gaps belong to the same repair:
- An invalid/relative editable destination is passed as `current_directory`; C# rejects it and reaches the same unhandled failure path.
- Disconnect clears the application picker map, but does not clear the form's pending picker.
- Form retirement removes local bookkeeping without an explicit native-picker cancellation command.
- Generic IPC command expiry is 10 seconds, while the native picker allows two minutes. The receipt and dialog lifetimes need a coherent terminal contract.

### 3. Contextual Extract retains the source but renders the blank-entry chooser

Browser Extract already passes the opened archive and selection through `open_extract_form` into the existing `OperationForm`. `extract_fields` nevertheless appends “选择压缩包…” unconditionally, even when `has_source` is true. Choosing a replacement source clears the original selection.

The global Extract entry has an intentionally empty source and does require a chooser. The two entry contexts must remain distinct within the existing form; they do not require separate form implementations.

Source evidence:
- `repo:rust/apps/nxtools-desktop/src/archive/browser.rs` (contextual extraction at line 1139)
- `repo:rust/apps/nxtools-desktop/src/archive/jobs.rs#open_extract_form`
- `repo:rust/apps/nxtools-desktop/src/archive/forms_layout.rs#extract_fields` (lines 359–387)
- `repo:rust/apps/nxtools-desktop/src/archive/forms.rs#picker_result` (source replacement at line 1161)
- `repo:rust/apps/nxtools-desktop/src/archive/launch_entry.rs` (global Extract at line 67)

### 4. Single-member native extraction is slow and can time out after publishing correct output

The existing owned-test-package seam ran the real, already-built PlatformHost DLL and unchanged Debug ArchiveWorker. Browser open returned the target's actual central-directory ordinal **0** and its browser context; all metadata batches were acknowledged. Both extraction operations retained the real source/browser/output validation. No timeout, resource budget, manifest, or identity check was weakened.

| Operation | Measured wall time | Result |
| --- | ---: | --- |
| Python ZIP member decode to discard | 51.584 ms | Successful decode and ZIP CRC check |
| Bandizip `bz.exe x`, one exact member | 110.731 ms | Exit 0; exactly one 8,324,412-byte file |
| NxTools browser open | 10,195.0772 ms | 215 entries, one batch, succeeded, cleanup complete |
| NxTools single-entry materialize | 24,536.2446 ms | Succeeded; 8,324,412-byte file; returned lease released successfully |
| NxTools ordinary selected-member job | 30,130.4285 ms | `failed / timeout`; cleanup complete; one committed, published 8,324,412-byte file |

Bandizip's time includes process startup. The NxTools job reports native elapsed **30,111 ms**, `files_committed=1`, `bytes_committed=8324412`, `commit_state=Committed`, `output_published=true`, and no native error code. Its fresh destination contains only `companion/dist/bcprov-jdk18on-1.78.1.jar`.

Direct streaming byte comparison confirms that both the timed-out NxTools job output and Bandizip output equal the decoded ZIP member for all **8,324,412 bytes**. Successful publication and a successful terminal result are separate facts here.

Confirmed source costs:
- ZIP browser open computes the entire archive revision before listing and recomputes it afterward. `revision` reads all bytes and updates SHA-256 in 64-KiB chunks.
- Extraction checks the revision on entry, after ZIP preflight, after each selected file's decode, and after ZIP extraction returns. The per-file check makes archive-sized work scale with selected file count.
- ZIP preflight inspects local headers/descriptors/extents for the entire snapshot, including unselected entries. It is structural inspection, not decoding every unselected payload.
- Worker output sends 64-KiB chunks as JSON byte arrays through synchronous native calls with replies. It does not flush the disk on every chunk; commit owns flush/readback.
- Materialization additionally seals the cache output; stream-open and release paths perform their existing output validation.

Source evidence:
- `repo:rust/crates/nxtools-archive/src/engine.rs` (ZIP open at line 214; extraction at line 531; final check at line 676; revision at line 725)
- `repo:rust/crates/nxtools-archive/src/formats/zip.rs#preflight_created` (line 432)
- `repo:rust/crates/nxtools-archive/src/formats/zip.rs#extract` (line 633; per-file revision check at line 706)
- `repo:rust/crates/nxtools-archive/src/worker_output.rs`
- `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs`
- `repo:csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs`

These costs are confirmed code behavior. The existing event stream measures admission/terminal boundaries, not individual revision, decode, IPC or commit durations; their shares of the measured delay remain unmeasured.

## Bandizip native reference findings

Target: `D:\Tool\Bandizip\Bandizip.exe`, version 7.32.0.1, x64 PE. REA uses Ghidra **12.1.4** from `D:\Tool\ghidra_12.1.4_PUBLIC`. The completed overview contains 5,494 analyzed procedures and 4,439 defined strings. Addresses below are linked static virtual addresses, not runtime observations.

| Boundary | Verified static chain / observation |
| --- | --- |
| Native drag | `FUN_1401130a0` and `FUN_140147200` call `FUN_140053460`; the latter constructs an `XDataObject`, supplies `CF_HDROP` with `TYMED_HGLOBAL`, and calls `DoDragDrop` at `0x1400535bc`, then releases data/drop-source references. |
| Selection and fast drag | `FUN_140147200` enumerates selected list/tree items and builds paths under a temporary root. It has distinct ordinary/fast-drag branches, with fast-mode error strings. On its successful fast-drop branch it obtains a destination and calls `FUN_1401481d0`. This does not prove which branch the supplied member uses in the GUI. |
| Extraction coordination | `FUN_1401481d0` copies the retained selected-member index array and configures the destination before calling `FUN_14013ce80`. The common coordinator retains the request, checks engine metadata through virtual calls, initializes operation state through `FUN_1401170c0`, and delegates UI execution through `FUN_14014efc0` or `FUN_14013f700`. Engine virtual-call targets and payload decoding internals are not resolved by this chain. |
| Destination Browse | `FUN_1400fa3d0` references `TEXT_SELECT_DEST_FOLDER`, passes its owner HWND and current directory to `FUN_140024f10`, and updates the destination control only for a nonempty result. |
| System directory dialog | `FUN_140024f10` sets `BROWSEINFO.hwndOwner`, title, initial-directory callback and flags `0x45`; calls `SHBrowseForFolderW` at `0x140024ff0`; cancellation returns an empty result. Selection is converted with `SHGetPathFromIDListW`. Callback `FUN_140024ee0` sends the initial directory on initialization. |
| Engine loading | `FUN_1400ee3e0` uses `LoadLibraryW`, resolves `CreateArk` and `CreateArkCompressor`, creates the engine through the recovered factory and handles missing module/export/factory results. Factory use is observed; original interface types and proprietary engine source are unavailable. |

This supports one Windows-owned OLE gesture boundary, object-bound selection/destination state, and an owned system directory chooser. It does not justify copying Bandizip's proprietary implementation or transplanting its temporary-file/fast-drag scheme into NxTools' existing virtual-file protocol.

Key REA Evidence IDs:
- Overview: `ev_a03b4e591fc4b8afdd0d1197da119cde0211ce552ba3fa1057d8950c28dada58`.
- Native drag helper: `ev_3df516b92dc4fc03133ac28811ec14aa63edf1b56d4bf78911c8d31c101eae3a`.
- Main drag path: `ev_49a445304fe63a2ef300708f8beff67e1418a331f47d6d5ed18080d790421a29`.
- Selected extraction setup: `ev_c965cdac544d536c1e2e433d8e34d2f180d4fff4214a798d3aa2adc81c10aa14`.
- Extraction coordinator: `ev_4097dbb6ac321e92e92665d68c8d9cb3fcddd625159abe7dd23cf1b7cf8f856f`.
- Destination handler: `ev_a73e5e87d202f9d9c233b061a8bed1570df329b5e0fa9c0a5ab81b54610cab4b`.
- System folder helper: `ev_8a25fa16bb447eee63faa242145772809478d34e3a333f37fa6c88786c2dddce`.
- Engine factory loader: `ev_a40f54baaaca1719244ee79de770b79fdd51c9f27b2eceabcc9933b0b65dfd29`.

## Minimal repair boundaries

1. **Unify member-path projection.** Interpret accepted ZIP separators consistently for tree projection, selection and drag; retain raw archive names and ordinals for engine identity. Keep traversal, reserved-name and native descriptor checks. Correct byte-vs-UTF-16 limits at their actual owning boundaries rather than removing all path validation.
2. **Repair the existing picker lifecycle.** Send a correlated command; settle form/application state on rejection, failure, cancellation, expiry and disconnect; retain stale-generation protection. Treat an invalid editable directory as an invalid hint rather than a permanently busy form. Preserve the existing C# owned native dialog and GPUI deferred cross-entity updates. Align receipt lifetime with the dialog and define retirement behavior before adding another picker implementation.
3. **Bind contextual Extract to its object.** Display the opened source as context, preserve the captured selection, and focus the form on destination/range/action. Keep source selection in the global empty-source entry. Reuse the current `OperationForm` and official components.
4. **Measure and remove repeated work within the original extraction owners.** Add bounded, correlated stage durations for source admission/browser validation, plan/preflight, decode/native output, commit, post-validation and cache seal. Then consolidate redundant whole-archive scans under the existing held-source lease and browser generation, and evaluate bulk output transfer instead of decimal JSON chunk round trips. Preserve worker approval, source binding, output identity, payload CRC, publication/cleanup semantics and cancellation. Raising the timeout alone is not a performance repair; a published file must not be silently equated with successful completion.

The path and picker regression probes already provide red-capable boundaries. Implementation should add durable tests at those boundaries, preserve the real selected-member native run, and verify the system chooser's show/cancel/focus/close lifecycle and actual external OLE interaction. A larger passing unit-test count is not acceptance of those interactions.

## Evidence artifacts

Session scratch root: `C:\Users\Administrator\.pi-desktop\scratch\1aaee474-c442-4dd3-b493-c29dceaff0a6`.

- `archive-sample-inspect.py`, `bandizip-single.log`, `bandizip-single-result.json`: sample metadata, normal decode and exact-member Bandizip run.
- `archive_research_probe.rs`, `drag-raw.json`, `drag-normalized.json`, `picker.json`, `picker-control.json`: real Rust model/bridge probes.
- `picker-parser-probe\Program.cs`: real C# parser invocation and correlated control.
- `archive-product-d58624f2-e50a-4ac1-b3c2-839d10c7367b\harness\Program.cs` and `selected-member-results.json`: owned-package browser/materialize/selected-job invocation and structured terminal measurements.
- `compare-selected-output.py`, `selected-output-byte-comparison.json`: direct comparison of both outputs against the decoded member.
- `bandizip-native-snapshot.json`, `bandizip-overview.json`, `bandizip-*-trace.json`, `bandizip-function-0x*.json`: complete REA Evidence; recovered proprietary pseudocode is not copied into the repository.

## Limitations

- The NxTools runtime run uses the intentional owned-test-package seam and existing Debug binaries, not approved packaged/protected production binaries. It is not a release-optimized comparison.
- Browser, materialize and ordinary selected-job operations are separate measurements. Their harness total is 64,963.2039 ms; that total is not a reproduction of one production-UI operation exceeding 60 seconds.
- Progress was drained when session events arrived; its recorded drain timestamp is not the time an internal phase started. Internal stage-level attribution, cold/warm distributions and the exact production-UI slow terminal remain open.
- The owned native run exercises worker/source/output boundaries without driving the Desktop gesture/form layer. It does not certify real pointer drag, external OLE target behavior, system-dialog focus, DPI or accessibility acceptance.
- REA pseudocode is a provider reconstruction, not Bandizip source. Direct static calls are distinct from unresolved virtual calls and actual runtime reachability. Bandizip GUI extraction, native folder interaction and fast-drag selection were not exercised here; the selected-member CLI operation was exercised.
- The supplied ZIP was read only. Normal outputs were confined to fresh scratch directories; extracted code was not executed. Production code, services and installed/protected binaries remain unchanged by this research.
