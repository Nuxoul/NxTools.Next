# Identity-bound native overwrite gate

## Current approved amendment2026-10-17

User explicitly replaces strict confirmed-target-object atomic CAS with complete validated
sibling temporary + immediate target recheck + one atomic path publication. The TOCTOU window
after closing the old-target recheck handle is accepted. Parent/reparse/temp/source protection,
budgets/full integrity/flush remain; only that old-target check handle may close. No delete,
truncate or backup-first. Native sharing/access failure is not capability failure. No unverified
path fallback. Preparing→Verified→CommitInFlight→Committed / FailedBeforeCommit; preserve
postcommit verification/change and unknown states without destructive retry/partial cleanup.
Current implementation/7-fixture result is recorded in [overwrite remediation](../overwrite-remediation.md); the historical probe/red below is unchanged.

SDK checked locally: Windows SDK10.0.26100.0 um/minwinbase.h FILE_INFO_BY_HANDLE_CLASS
FileRenameInfoEx=22, FileIdInfo=18; NT FileRenameInformationEx=65 is a different enum.
um/winbase.h FILE_RENAME_FLAG_REPLACE_IF_EXISTS=1, POSIX_SEMANTICS=2; FILE_RENAME_INFO
union DWORD Flags, HANDLE RootDirectory, DWORD FileNameLength, WCHAR FileName[1]. x64 offsets
0/8/16/20 and full FILE_ID_INFO64-bit volume+128-bit file ID. Primary implementation will use
SetFileInformationByHandle with a live relative-parent/temp capability; no arbitrary fallback.

## Historical strict-contract evidence (superseded requirement, unchanged results)

Date:2026-10-17. Scope: Phase A output transactions/SAFE05–SAFE07. **Blocked, not an approved product behavior change.** No user files, real Shell/registry/service state or package artifacts were mutated.

## Contract and exact red

Design§4.3 requires verified atomic replace/no-replace, preserving old target on failed create and refusing a newer target after a conflict prompt. Current C# session opens target relative to a held non-share-delete parent, checks volume/fileID/size/mtime/SHA256, and retains exclusive target access through commit. This prevents target replacement between check and use.

Real harness:

```powershell
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list "archive output fixture identity bound atomic overwrite"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "archive output fixture identity bound atomic overwrite"
```

List: selected1/run0. Execution: selected1/run1/pass0/skip0/fail1, exit1; `FAIL ...: replace_file:32`. `ArchiveOutputTests.IdentityBoundOverwrite` remains registered, not marked skip and not weakened. The passing16-test subset excludes it explicitly; default/full C# fixtures remain non-green.

## Native source and discriminating probe

Production `ArchiveOutputSession.Commit`→`OutputNative.RenameReplace`: NtSetInformationFile class65/FileRenameInformationEx, flags3 (REPLACE_IF_EXISTS|POSIX_SEMANTICS), x64 native structure offsets0flags/8parent/16UTF16bytecount/20name. Native status maps with RtlNtStatusToDosError. API reference inspected:

- https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle

Documentation describes POSIX replacement with existing handles, but does not prove this exclusive protection combination works on this workstation. Both C: and D: report NTFS. The probe uses only unique self-created temporary trees, exact same native rename operation and changes one handle variable per row. Source/log retained in session scratch `archive-rename-probe/` and `archive-rename-probe.log`.

| Target/source/parent combination | Observed status / publication |
| --- | --- |
| no target handle; production source/parent |0/new bytes |
| target production read/write/delete, share0 |0xC0000043/32; old bytes |
| target read-only, share0 |0xC0000043/32; old bytes |
| target read/write without delete access, share0 |0xC0000043/32; old bytes |
| target shareREAD |0xC0000043/32; old bytes |
| target shareREAD+WRITE, not DELETE |0xC0000043/32; old bytes |
| target shareDELETE |0/new bytes |
| target production; source adds shareDELETE |0xC0000043/32; old bytes |
| target production; parent adds DELETE_CHILD/write access |0xC0000043/32; old bytes |

This rules out an unsupported filesystem/rename path or simply missing parent/source access in the tested setup. It does not prove every possible Windows primitive is impossible. Releasing the protected target or allowing shareDELETE before replacement reintroduces a check→name-replace race; no such fallback is implemented. No surviving probe/worker process or fixture tree was observed after cleanup.

## Minimum proposed amendment — awaiting explicit approval

1. Keep original target protected and identity-checked.
2. Rename that exact held file handle no-replace to a unique owned backup in the held parent (bounded collision retries).
3. Publish verified partial to final name with no-replace. External name competition must never be clobbered.
4. On publish failure restore backup no-replace; if another program occupies final name, retain backup, return typed recovery-needed and expose its location locally, never log path or delete user data.
5. Delete backup by exact held handle only after successful publish and defined durability/recovery policy. Cancellation/disconnect/crash semantics need an owned recovery journal/fixtures before acceptance.

Trade-off: a brief missing-name interval exists between moving the old file and publishing/restoring. This is **not atomic replacement**. It needs a deliberate design§4.3/acceptance amendment and red-capable failure/competition/recovery tests before product implementation. Do not infer approval from the already-authorized A–F scope or worker architecture. The native atomic red remains a genuine gate while this is undecided.

## Bounded Win32 parameter verification for approved path publish

Three self-owned tiny-tree cases, SetFileInformationByHandle class22/flags3: held-parent+relative
name→87, same with trailing NUL→87, full target path/RootDirectoryNULL→success and exact new
bytes. Not a capability failure and no generic fallback. Microsoft SDK-API FILE_RENAME_INFO
docs currently describe relative RootDirectory, but this workstation's Win32 wrapper rejects
it; the implementation uses the verified full-target form. Derive that target from the held
parent using GetFinalPathNameByHandleW(VOLUME_NAME_GUID), append one validated component,
keep the no-share-delete/no-reparse ancestor chain and source temporary handle live. This
remains the main handle-based SetFileInformationByHandle call, not MoveFileEx or a close/reopen
of the temporary. Raw probe/script/log stay in session scratch, no real user files involved.
https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info
https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew

Second bounded/discriminating ABI probe: full target path, explicit FileNameLength, suffixTAIL
immediately after the counted name. Without WCHAR NUL, Win32 returned success but created
oldTAIL and left old bytes; with NUL beforeTAIL, correct old-target replacement succeeded.
The implementation therefore allocates counted bytes+sizeof(WCHAR), writes explicit zero,
and excludes that terminator from FileNameLength. The API docs call FileName NUL-terminated
even though the FileNameLength paragraph says no terminator required; actual wrapper behavior
is locked by this probe and actual target-byte/post-ID tests. No fallback/retry hides this issue.
