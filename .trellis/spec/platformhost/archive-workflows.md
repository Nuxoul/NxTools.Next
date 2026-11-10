# Archive batch secrets and owned folder picker

## 1. Scope / Trigger
OPT-07 original Job pipe-session password reuse and New/Add native multi-folder input. Read [batch reference](../../../docs/reference-code/archive-batch-workflow.md), [picker reference](../../../docs/reference-code/archive-picker.md) and [Desktop contract](../nxtools-desktop/backend/archive-browser-workflows.md). Evidence: `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.Password.cs`, `repo:csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs`, `repo:csharp/NxTools.PlatformHost/Windows/ArchiveJobFolderPicker.cs`, `repo:csharp/NxTools.PlatformHost/Windows/ArchiveJobPicker.cs`. State/policy/layout stay Rust-owned; Windows APIs remain here.

## 2. Signatures
- Original `ArchiveJobSession` owns `Dictionary<Guid,BatchSecret>`; `RememberBatchSecret`, `TryReuseBatchSecret`, `InvalidateBatchPassword`, `RetireBatchPassword` and `RetireBatchSecrets` run under its existing gate.
- `BatchSecret` uses `CryptProtectMemory`/`CryptUnprotectMemory` flags0 (SAME_PROCESS), 16-byte padded protected buffer and Dispose zeroing. `ArchiveWorkerRunner.RespondPasswordBytes` feeds the original Run input pump; no retained managed password string.
- Additive Job `BatchId`, reply `RememberBatchPassword` and `retire_batch` use the original typed wire/parser. See [protocol contract](../nxtools-protocol/backend/directory-structure.md#scenario-batch-password-wire).
- `ArchiveJobFolderPicker.Show(owner,initialDirectory,multiselect)->string[]`; native `IFileOpenDialog.Show(original HWND)` -> `GetResults` -> `IShellItemArray` -> `SIGDN_FILESYSPATH`. Original `ArchiveJobPicker.Pick` owns admission, bounded STA, cancellation and result envelope.

## 3. Contracts
Protected candidates are per original pipe-session and batch, maximum34 (original two-running/32-queued budget); no global/persistent/history/preview/OLE cache. Each new prompt defaults no retention. Retention requires explicit current password reply and eligible generation1 Test or ordinary Extract with one path/no reclamation/not cancelling/not retired. Match operation/session/generation/batch/current one-use ticket before accepting. A protect failure reports fixed category/native code and does not alter the accepted password operation. Transient unprotected padding, copied plaintext and serialized input bytes are zeroed by their owning scopes/pump.

Reuse is once per original member, only first `PasswordRequired` attempt1; never auto-answer a later error/retry. Confirmed `WrongPassword` notification/failure invalidates before further reuse; ambiguous `AuthenticationOrDataError` stays ambiguous. Explicit retirement clears candidate and marks still-active same-batch members retired; generation>1 Start does the same. Cancel, cancelled/rejected/unclean terminal, queue failure, Stop/disconnect and disposal clear their owned candidates. These actions do not release Run/source/output/cleanup ownership early or authorize a new attempt.

The original detached platform command reader stays responsive. `ArchiveJobPicker.Pick` validates source HWND and pipe peer before presentation and after success. One existing bounded archive-picker STA reservation remains held through actual modal return and cleanup, even after logical cancellation. `StaDialogRunner` closes only that STA's dialog windows; do not add another thread/runtime. Source folders use FOS_PICKFOLDERS|FORCEFILESYSTEM|PATHMUSTEXIST|NOCHANGEDIR|ALLOWMULTISELECT in one native dialog. Destination omits ALLOWMULTISELECT; files/save retain original semantics. Existing initial directory is seeded by SHCreateItemFromParsingName/SetFolder. Balance every COM reference/HRESULT allocation; ERROR_CANCELLED yields no selection.

Validate the entire returned vector<=32 paths/131072 UTF-8 bytes and original PathValid before serialization. Overflow/invalid vector -> `picker_limit` with zero paths, never truncation. Cancel/retired owner yields none; native failure retains native code without path/exception text. Native selection grants no execution/overlap/write authority; Rust validates combined existing/incoming roots and frozen form context. Advertise `archive_multi_folder_picker_v1` through original Hello; absence refuses this source-folder feature locally, without a new wire version.

Diagnostics remain low-frequency and correlated: `batch_password_remembered/reused/invalidated/retired/capacity/protection_failed`, original Job accepted/start/state/terminal and picker accepted/start/terminal with elapsed/count/fixed category/native code. Never log password/query/path/member/payload/raw commands; a throwing sink cannot change outcomes.

## 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
| Explicit same-batch candidate + first eligible password prompt | Original pump auto-answer once; no published prompt |
| Wrong password / ambiguous authentication-data error | Invalidate candidate / preserve ambiguity category |
| Other batch/session, stale ticket, generation2 or retired active sibling | No cross-use/late retention; original fresh prompt |
| Protected-memory failure/capacity | No retained candidate; ordinary operation/explicit prompt remains |
| Folder2/16/32 / Destination | One multiselect vector / one selected destination |
| Count/UTF-8/path overflow, cancel or owner retirement | Whole refusal/empty result, no partial vector |
| Cancel before blocked modal exits | Keep reservation until actual exit; no overtaking |

## 5. Good/Base/Bad Cases
Good: TestA explicit opt-in, ExtractB same batch reuses once, other batch and retired/retry generation prompt anew. Base: absent new fields keeps original explicit password behavior. Bad: retain plaintext, interpret ambiguous corruption as WrongPassword, clear only the dictionary while active members can reinsert, or emulate multiselect by repeated single dialogs.

## 6. Tests Required
`repo:csharp/NxTools.PlatformHost.Tests/Opt07BatchPasswordTests.cs` covers original Session/Run/serialized tickets/input pump/protected memory across six controlled cases. `Opt07PickerTests.cs` covers seven original parser/Pick/STA/option/vector/owner/reservation/diagnostic cases; original strict picker fixture remains. Effects review -> exact LIST -> identical RUN, nonzero selected and zero skips/not_run. `Opt07NativeWorkflowTests.cs` runs five actual worker/codec Test/Extract operations over owned AES ZIP copies, proves reuse/other batch/retirement/generation isolation, source byte equality, real publication/location/cleanup/exclusive reopen and no partial. Preserve actual ZIP ambiguity/revoke and native empty-directory location fixtures. Controlled COM/hidden HWNDs do not exercise a system dialog, real OLE/Explorer or physical focus/keyboard/DPI. Recycling remains unavailable under its separate original-object prerequisite.

## 7. Wrong vs Correct
Wrong: UI checkbox -> reusable plaintext/global entry grant; logical cancellation -> free STA slot. Correct: current original ticket -> session/batch-protected candidate -> one eligible pump reply -> zeroing/retirement; cancellation -> actual modal exit -> original reservation cleanup.
