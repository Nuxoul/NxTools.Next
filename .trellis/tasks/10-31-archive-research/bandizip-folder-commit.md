# Bandizip 7.32.0.1 — archive-list new-folder commit follow-up

## Scope and evidence boundary

Read-only static reconstruction of the shipped `D:\Tool\Bandizip\Bandizip.exe` and `D:\Tool\Bandizip\ark.x64.dll`. The trace reuses the established command `0x80aa` draft-row evidence and joins it to the real label-edit consumer, directory-member plan, ZIP writer, publication result and browser refresh. The records are Ghidra 12.1.4 pseudocode, assembly and initialized-image bytes, not original source or runtime proof. Bandizip EULA §3 and applicable-law limits remain controlling; no proprietary implementation is proposed for reuse.

The archive-list draft reaches an explicit ZIP directory record through its end-label-edit handling path. Encoding success then reaches temporary-open/test, two-move publication and reopened browser state. A draft row by itself remains non-persistent.

## Draft row enters native label editing

The established command path is:

`0x80aa → 0x14013a1e0 → 0x1401a62a0 → 0x1400ef0d0 → row+0x38=1 → 0x1401a8630 → 0x1401a99e0`.

`0x1401a99e0` does more than select the inserted row: it focuses the archive list, sends `LVM_SETITEMTEXTW` (`0x1074`) to seed the editable label, and sends `LVM_EDITLABELW` (`0x1076`). The main notification owner `0x140156740` handles list notifications. For `LVN_BEGINLABELEDITW` (`-0xaf`), it calls `0x140150b30`; that routine rechecks archive-edit admission, obtains the selected list index, captures the current archive folder path at main state `+0x1d0`, resets the saved member index to `-1`, and requires the row-data lookup to succeed. The handler then marks label editing active at main state `+0x180` and allows the editor. [B01–B04,F01,F02]

## End-label-edit acceptance and cancel behavior

`LVN_ENDLABELEDITW` (`-0xb0`) resets the edit-active flag and calls `0x140150c40` with the notification payload. The routine resolves `item.iItem` through `0x1401a6770`; `item.pszText` is the pointer at notification offset `+0x30`. It copies and trims that text, then consumes the provisional-row marker at `row+0x38`. [F01,F03]

The marker changes cancel semantics:

| End-edit input | Existing archive row, marker `0` | New-folder draft, marker `1` |
| --- | --- | --- |
| Non-empty, valid, unique label | Compare with the current path; if changed, run the ordinary rename edit plan. | Build the target path under the captured archive folder and run the directory-add plan. |
| `pszText == NULL` (native label-edit cancel) or text trims to empty | Return without an archive edit; the caller redraws the row from its existing model text. | Replace the empty result with the draft row's generated label and continue to the directory-add plan. Escape/cancel therefore **does not discard this draft**; it attempts to create the default generated folder name. |
| Forbidden character or same-name peer | Show the typed error and do not start compression/publication. | Same; the provisional row remains in the in-memory list because this validation path does not remove it or rebuild the browser. |
| Backend/compression failure after a valid plan | Delete the temporary output and reopen the original archive. | Same; that reopen rebuilds the list, so the provisional row is no longer retained. |
| Full publication success | The helper returns success and the notification owner skips a stale in-place label write. | Same; the refreshed archive catalog supplies the persisted directory row. |

The forbidden-character gate explicitly rejects `?`, `\`, `"`, `/`, `:`, `<`, `>`, `|` and `*`. The same-name branch reports `TEXT_ERR_CANT_CHANGE_FILENAME_SAMENAME`. For a non-draft unchanged name, `0x140150c40` returns without recompression. For a draft, marker `1` is passed to `0x1401518b0`; this is the real consumer that distinguishes “new directory” from rename. [F03]

If `0x140150c40` returns zero, `0x140156740` calls `0x1401a76c0` and sends `LVM_SETITEMTEXTW` (`0x1074`) with the row's current model text. That is a rejection/snap-back path, not persistence of the submitted label. A nonzero return means publication completed and the browser has already been rebuilt, so no in-place row write is performed. [F01,F04]

## ZIP directory plan

`0x1401518b0` derives the `.temp` output, creates the compressor, applies current settings, and imports the opened archive through compressor virtual `+0x70`. Its parameters are `memberIndex`, `draftMarker`, old path and new path. The joined new-folder call has `memberIndex == -1` and `draftMarker == 1`; that selects compressor virtual `+0x30` with a null filesystem source and the archive target path. A false return reports `TEXT_ERR_CANT_CREATE_FOLDER`. The edit descriptor uses action `0x0c` and runs through `0x14013be50`. [F03,F05]

The `ark.x64.dll` primary compressor vtable starts at `0x1801f1408`. Its initialized bytes place `0x18001e180` at slot `+0x30`. The wrapper assembly preserves the incoming stack argument, supplies compressor state `+0x108`, and tail-jumps to `0x18001e1a0`; that shim reaches `0x18001e1d0`. [A01–A04]

At `0x18001e1d0`, a null source path selects the synthesized-directory branch rather than source-file metadata collection. It constructs the planned archive name, marks the member record as a directory at `record+0x7c=1`, and appends the record to the compressor member vector/name index. This is a real compressor member record, not the archive-list draft object. [A04]

ZIP writer `0x180065a70` walks planned records. For `record+0x7c != 0`, it sets the member payload extent to zero; it still emits the member header through `0x1800672d0`, but skips both new-data encoding (`0x180091070`) and imported encoded-data copy (`0x180090e20`). It then calls `0x1800666b0`, which writes one central-directory header per retained planned record (`0x02014b50`) and the ZIP end record (`0x06054b50`). The directory therefore persists as an explicit zero-payload ZIP member represented in both local/member and central-directory output. [A05,A06]

## Result, publication and refresh

After `0x14013be50`, `0x1401518b0` releases the compressor and branches on the coordinator result:

- On failure, it deletes the `.temp` output, reopens the original archive with `0x1401579a0`, releases the edit state and returns zero.
- On success, it calls `0x140154650` and returns that publication result. The internal publication label is `"rename"` for this shared edit path even when the planned operation is directory creation. [F05]

`0x140154650` closes the old browser state, opens the generated temporary archive and applies the configured post-edit test before publication. Only the admitted path derives an unused `.old` backup and performs original → backup followed by temporary → original through `0x140122080` (`MoveFileW`). [F06]

On both moves succeeding, it reopens the published original with `0x1401579a0`, restores the prior archive-folder location, selects the requested row/path when available, restores list focus, separately removes/recycles the backup, clears the saved edit selection and returns `1`. `0x1401579a0 → 0x140157cc0 → 0x1401584c0` reconstructs the reader-backed tree, list, comment pane and location state. [F06,F07]

If the second move fails, `0x140154650` captures the native error, attempts `MoveFileW(backup, original)`, reports modification failure and reopens the original path. The restoration return is still unchecked, matching the previously established non-atomic publication limitation. Temporary-open failure, user abort and configured test failure return before replacement. A successful ZIP writer is therefore not reported as a successful new folder unless `0x140154650` returns `1`. [F06]

## Joined outcome

The complete static chain is:

```text
command 0x80aa
→ provisional row, marker 1
→ LVM_EDITLABELW
→ LVN_ENDLABELEDITW / 0x140150c40
→ marker-1 directory branch / 0x1401518b0
→ compressor +0x30 / 0x18001e180 → 0x18001e1a0 → 0x18001e1d0
→ directory record +0x7c=1
→ ZIP member header, zero payload, central directory and EOCD
→ coordinator success
→ 0x140154650 temporary validation and publication
→ 0x1401579a0 browser reopen and refresh
```

The main behavioral finding is the draft-specific cancel rule: cancelling the native label editor does not remove the provisional row and does not cancel creation. The default generated draft label is substituted and submitted. Only validation rejection leaves the draft as an uncommitted in-memory row; backend failure rebuilds from the unchanged archive; full publication rebuilds from the changed archive.

## Evidence locator

Files are relative to `evidence/raw/`. `EXE` means `D:\Tool\Bandizip\Bandizip.exe`; `ARK` means `D:\Tool\Bandizip\ark.x64.dll`. Pseudocode and assembly entries are Ghidra reconstructions; the vtable entry is initialized-image byte evidence.

| ID | Canonical JSON | Evidence ID | Subject | Boundary |
| --- | --- | --- | --- | --- |
| B01 | `bandizip-create-p-0x1401401fb.json` | `ev_62909cf3d746bcd5b408319b252fc9d385facb256bfa349b3e90ce32ef35288e` | EXE | `0x14013fc40`, command `0x80aa` dispatch |
| B02 | `bandizip-management-core-p-0x1401a62a0.json` | `ev_61856f9d4c9d181b9d6645499dc02ccefecc9d841bcb4ba91a89aea0a864c497` | EXE | draft allocation, marker `+0x38=1`, insertion and edit start |
| B03 | `bandizip-management-finalize-p-0x1400ef0d0.json` | `ev_46327adda8ce7f1406f1ba195495a16120e3bf93c0e2a02427e226b54d483957` | EXE | archive-list row allocation |
| B04 | `bandizip-management-actions-p-0x1401a99e0.json` | `ev_c8ac88cc3798e48c99c5bc81f3bfd58005b44fc808ce121ea1820e65d0523d3d` | EXE | focus, `LVM_SETITEMTEXTW`, `LVM_EDITLABELW` |
| F01 | `bandizip-foldercommit-notify-p-0x140156740.json` | `ev_301bcb75db9ddab2d3345b7f29d934fc787c9ad7f0ae42ea7a0780fd7f1cac35` | EXE | begin/end label notifications and post-result row handling |
| F02 | `bandizip-foldercommit-begin-p-0x140150b30.json` | `ev_2df3bc5ea0256a6b4d66ec04309e09ac3200caaf894de920962e8cef9e7f408d` | EXE | begin-label-edit admission/context capture |
| F03 | `bandizip-foldercommit-end-p-0x140150c40.json` | `ev_80b106c20e1e33267b524630bde768e1aea1867f8100eafb4b4cb6a694ec0eb6` | EXE | marker consumer, cancel/empty/validation and plan call |
| F04 | `bandizip-foldercommit-rowtext-p-0x1401a76c0.json` | `ev_5273d5b0e6049543ff18c932aa369e024c45ac9eb0f027dc62ac593ff8f12905` | EXE | rejected/no-op label redraw text |
| F05 | `bandizip-foldercommit-plan-p-0x1401518b0.json` | `ev_a78b9d73f6baf267183cfa342741214993db8312c81a4c630608e506b6cc2e4d` | EXE | compressor import, directory virtual, action `0x0c`, result branch |
| F06 | `bandizip-foldercommit-publish-p-0x140154650.json` | `ev_df6f093ef09d272d613b3af22525693c2362f1bc0c018fa8b2268e304d7b3852` | EXE | temporary open/test, two moves, restore attempt, success result |
| F07 | `bandizip-foldercommit-refresh-p-0x1401579a0.json` | `ev_6a438180caa8fbdf74fde0e3eebc923a1d2dda2b2a2bd07690576974378f7d29` | EXE | original/published archive reopen and browser refresh |
| A01 | `ark-foldercommit-vtable-b-0x1801f1408-136.json` | `ev_3f71413bb45d8f1505f9e4dfde549a3934e2ad32d134264a2dc98069f0413261` | ARK | primary compressor vtable; slot `+0x30 = 0x18001e180` |
| A02 | `ark-foldercommit-dirwrap-asm-0x18001e180.json` | `ev_5c0d5ea124630dd02cf2e2d454d1064f1ff0a7e5a918dd0a1b75b7ea7cadd54f` | ARK | directory-add wrapper assembly and tail jump |
| A03 | `ark-create-p-0x18001e1a0.json` | `ev_88991937132a540b78ddfc29095910ee43a290bd2e23848eefbb7e8fe0b1bc2d` | ARK | existing shim from `0x18001e1a0` to member-plan core |
| A04 | `ark-foldercommit-plan-p-0x18001e1d0.json` | `ev_7ddec4ad910b189815e01bd50f2c8140e7e64548b03c635115129012f8ebfcb3` | ARK | null-source synthesized directory and record `+0x7c=1` |
| A05 | `ark-foldercommit-zip-p-0x180065a70.json` | `ev_0e43e8e7ec7379c6f9e3e14e6b15a51a06f95bdc16a98f82e03d767fd1e96bad` | ARK | ZIP member loop, zero-payload directory path and finalizer call |
| A06 | `ark-foldercommit-central-p-0x1800666b0.json` | `ev_9c38eec1d66663d16c6dd9b0a9e13899e8b243e25fc13c8b5f4ef4e5139fc522` | ARK | central-directory `0x02014b50` records and EOCD `0x06054b50` |

Private follow-up session material is retained as `bandizip-foldercommit-session-snapshot.json`, `ark-foldercommit-session-snapshot.json`, `recipes/rea-bandizip-foldercommit-main.json` and `recipes/rea-ark-foldercommit-ark.json`. Both binary sessions were explicitly closed into those snapshots.

## Limitations

The requested control-flow edge is closed. Remaining uncertainty is limited to runtime presentation and byte-level metadata details: no GUI interaction verified the visible Escape behavior, no archive was created to inspect the exact encoded separator/external-attribute combination, and native move/test failure branches were not exercised. The static control flow does establish the draft-label fallback, directory record, zero-payload ZIP write, publication return and refresh ownership. No target executable, archive, registry/live setting, credential, log, service, build, package, Git state or product code was run or modified.