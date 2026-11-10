# Bandizip selected extraction and picker lifecycle

## Selected extraction

Bandizip7.32.0.1 uses an in-process `bz.exe` → `ark.x64.dll` engine boundary. The inspected selected-member path is distinct from archive Test and extract-all:

1. `bz.exe!FUN_14001d080` loads its sibling `ark.x64.dll`, resolves `CreateArk`, and constructs the engine. `FUN_140020ab0` prepares options/events and calls `FUN_140021160` → `FUN_1400214f0`.
2. `FUN_1400214f0` dispatches extract operations to `FUN_1400219a0`. Test uses a different branch/virtual entry.
3. `FUN_1400219a0` opens the source through `FUN_140021840`, obtains the member count at virtual slot`+0x68`, obtains indexed metadata at`+0x70`, normalizes requested forward slashes to backslashes for matching, and marks matching indices at`+0xe8`. With no member filter it uses extract-all`+0xb0`; with matches it uses selected extraction`+0x108` once for the marked set.
4. `ark.x64.dll!CreateArk` at`0x180001740` constructs `CArkMan` through`0x180004190`. Its observed vtable at`0x1801ef958` maps`+0xe8` to`0x1800067b0`, `+0x108` to`0x180006a10`, and`+0xb0` to`0x180007260`.
5. `FUN_1800067b0` inserts member indices into an ordered, duplicate-free array. Selected extraction`FUN_180006a10` →`FUN_180006a30` forwards that array/count (`this+0x70/+0x78`) to the selected decoder's`+0x28` method. Without selected indices it delegates to decoder`+0x38`. The selection array is released after the extraction call. Extract-all`FUN_180007260` →`FUN_180007280` directly uses decoder`+0x38`.

This establishes explicit selection planning and one engine call for the selected set. It does not recover the format decoder's complete source or prove that no checksum/validation exists inside it.

### Runtime corroboration

A fresh `bz.exe x` process extracted only `companion\dist\bcprov-jdk18on-1.78.1.jar` from the supplied137,737,166-byte ZIP into a new GUID session-scratch directory:

| Measurement | Observed |
| --- | ---: |
| Wall time, including process launch |72.7695ms |
| Process read-transfer bytes |8,355,741 |
| Process write-transfer bytes |8,324,608 |
| Read/write operation counts |452 /809 |
| Selected member output |1 member;8,324,412bytes |
| Exit |0 |

A streaming byte comparison against the decoded ZIP member matched all8,324,412bytes. The original ZIP was read only, extracted code was not executed, and no existing output was overwritten.

I/O counters cover the entire spawned process, not only the ZIP; they do not attribute memory-mapped page access. CPU counters were below the available accounting resolution and are not used for a CPU-cost conclusion. The observed ordinary read-transfer count is far below the archive length and supports the selected-extraction model, but cannot by itself exclude every possible mapped read.

### NxTools repair direction

The current NxTools source still makes full-archive SHA-256 revision passes during open and extraction, including `formats/zip.rs`'s per-selected-file check before commit. `engine.rs::revision` reads the full source; this is not merely a native metadata identity check. The shared output path also transfers decoded64-KiB chunks through synchronous JSON/native replies.

The reference-backed direction is to keep one selected-member plan and original native source lease per attempt, consolidate redundant whole-source revision work at the operation boundary, and stream the selected payload. Preserve entry/source/browser-generation validation, payload CRC, final source/result validation, native output publication and actual cleanup. Standalone/unleased engine callers must retain their own source-change contract. A held-lease proof must not be inferred from an arbitrary caller flag.

Measure source revision, preflight, decode/output, commit and final validation separately before changing their implementation. The reference supplies an efficient selection model; it does not establish how much of NxTools' approximately27second Debug sample is hash work versus IPC/native work. Raising the timeout or copying the proprietary engine is not the selected repair.

## Native directory Browse

The directly verified system Browse chain is in **CArchiveDlg**, the archive-creation/options form, not proof of the extraction-specific`IDD_SELECT_FOLDER` form:

- Constructor`FUN_1400f4d60` installs `CArchiveDlg` vtables; destructor`FUN_1400f5200` tears down its child controls.
- Its message dispatcher`FUN_1400f3e20` handles the Browse click by calling`FUN_1400fa3d0` synchronously. Its `WM_COMMAND / IDCANCEL` branch calls`EndDialog(hwnd,2)`.
- `FUN_1400fa3d0` passes the current form HWND, initial directory and `TEXT_SELECT_DEST_FOLDER` title to`FUN_140024f10`. No worker launch or asynchronous dispatch appears in this call chain.
- `FUN_140024f10` assigns `BROWSEINFO.hwndOwner`, callback, initial-directory data and flags`0x45`, then calls`SHBrowseForFolderW`. Callback`FUN_140024ee0` supplies initial selection with`BFFM_SETSELECTIONW` on initialization.
- Cancel returns an empty result; only a nonempty selected result updates the form's destination control. The helper/handler contains no independent `SetFocus`/`SetForegroundWindow` call after selection; the owner is supplied to the native chooser.

The standard owner-modal interpretation is that the system chooser prevents ordinary user interaction with its owner until it returns, rather than requiring the form to close behind it through a separate cancellation protocol. This interpretation is consistent with the synchronous owner-bound code; actual disabled-state/focus readback was not performed here. The complete CArchiveDlg native creation/base boundary and the extraction-specific folder form remain unresolved. An unrelated `DialogBoxParamW` caller was not used as evidence for this form.

### NxTools repair direction

Keep the existing C# `FolderBrowserDialog.ShowDialog(owner)` and original HWND/process validation. Verify the actual modal boundary: opening disables the intended Rust form; Cancel/X preserves its target; OK writes back once; return enables/restores that same live owner without activating a retired/replaced window. The pending receipt and native dialog lifetimes must not contradict one another.

Do not treat the absence of a separate per-picker cancel command alone as proof of a missing user interaction. If normal owner-close is blocked by the actual modal chooser, that is the reference-shaped behavior. Programmatic owner destruction, disconnect or application shutdown is a separate native-retirement scenario; any required cancellation belongs to the original native STA/dialog owner, not a competing Rust chooser or generic cancellation framework.

## Evidence

Complete REA JSON, reusable snapshots and measured runtime/probe artifacts are retained in the fixed local [evidence archive](evidence/README.md), with file names and exact Evidence IDs in [index.json](evidence/index.json). The ignored `evidence/raw/` directory is independent of session scratch and contains reconstructed artifact evidence, not product source.

| Observation | REA Evidence |
| --- | --- |
| CLI loader |`ev_c1af4a82b02eb8fe72a4def49a4348fda851d4038186d8b8429d0782977d9614` |
| CLI options/events |`ev_a1e9b44d38a36976801c54cf72f1217ee73e06edcee6a3d1c006e700abdfe21e` |
| Operation dispatcher |`ev_b9ee03aa384eea718441eb16959e16dec0e4016a80987505fa68d547e9f52e39` |
| Selected matching/dispatch |`ev_d953bad311908d31cfde48057b8fd6c583e05ee394258eed754d504b518f8e43` |
| Engine factory |`ev_6fe98db86dbad40a02b91d7295cb4be59a29f59c6da8f473025bab23a6397317` |
| Engine vtable memory |`ev_994d1f1622606e88f4766dd1707756eca6e1971bc09bef5b049130918597e816` |
| Selected-index storage |`ev_606921f98e5163a259829426aa5d5d66b4e89c4ae6f23fd8dbb18afd8c271c40` |
| Selected decoder adapter |`ev_b1a0bbe243c328ae0ac0fb1028f715e1a0b5b33bfc0c663b269e5bb5b4762cae` |
| Extract-all adapter |`ev_e1ecbd9ef20e65a3952d1fed63c86f164117da11ca7a66d9bad3b34c7343473d` |
| CArchiveDlg constructor |`ev_9058ee7844bc6eca10a3517c2f3a59e5ccead0d674a1bb39c56ff818d62ca0e8` |
| CArchiveDlg message dispatcher |`ev_45dce621d914338476bf680c64368f36c474b97ff674b903ae6873f3de4f939b` |
| CArchiveDlg destructor |`ev_2ea98ffdb6faf2d424e9c3234afc8f2a438eb184c82c4b5ca48a3698310e5963` |
| Browse handler |`ev_a73e5e87d202f9d9c233b061a8bed1570df329b5e0fa9c0a5ab81b54610cab4b` |
| System Browse helper |`ev_8a25fa16bb447eee63faa242145772809478d34e3a333f37fa6c88786c2dddce` |

Retained runtime artifacts: `evidence/raw/measure-bandizip-selected-io.ps1`, `evidence/raw/bandizip-selected-io.json`, `evidence/raw/bandizip-selected-io-output-check.json`.

## Limits

This follow-up changes reference documentation only. Static reconstructed code is not original source or observed execution. Native engine internals beyond the selected adapter, exact NxTools stage attribution, complete form creation, live modal disable/focus/close behavior, and installed/release acceptance are not established. Product code, budgets, services and protected binaries were not changed; no new product build or package was run.
