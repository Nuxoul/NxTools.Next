# Sequence26 manual acceptance — live evidence

## Authority and evidence level

2026-10-24, same **Phase F — Explorer integration and genuine native drag**. User reports no
installation/startup problem and authorizes reading logs. This round observes existing files,
processes and service configuration only; the user owns all real GUI/Explorer operations.
Current status belongs to [the sole checkpoint](phase-a-checkpoint.md).
[Package outcomes](manual-package26-outcomes.md) and accepted AC01/tree/password source slices remain
unchanged. No source review, product fix, installation, product launch, elevation, service/bootstrap
maintenance, Registry/COM apply, packaging, Cargo, commit or push was performed by this round.

Evidence captured at 2026-10-24 08:15 +08:00:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\package26-live-20261024-081511`.
`readonly-evidence.json` contains allowlisted identity/event metadata, not archive contents/passwords.
The runner is `package26-live-readonly.ps1` in the same session scratch parent. Original logs and
ProgramData recovery/release state were read only and not repaired, deleted or reset.

## Installed release and startup: confirmed within the observed scope

- Protected updater `Releases/state.json` reports sequence **26**, release ID
  **e0a8c392-0bb3-44e6-b573-4d17ba35863b**, manifest SHA256
  **23DE45C00BE1BFBF12EAB3E9BD2A5EE4E789E3EA330F5C4355D379357FB4D515**.
  All three match the previously delivered package identity; display version0.7.0 was not used as proof.
- Six protected files match their exact sequence26 manifest size/SHA256: Desktop, PlatformHost,
  Indexer, ArchiveWorker, ShellSelectionHelper EXE and DLL. This is not a fresh all77-file/signature audit.
- Current Desktop PID8460 and its PlatformHost PID16424 run from
  `C:\ProgramData\NxTools\NxTools.Next\Service`, created at07:59:15 +08:00, after the observed deployment.
  Higher-privilege process executable paths are unavailable; no elevation was attempted.
- Read-only SCM query reports indexer PID5324 and updater PID5416 Running with their protected paths.
  Running is not a service restart/search/CPU or in-memory image verification gate.
- PlatformHost log `platform-host.log.2026-10-24.jsonl` records the same trusted-update correlation
  **d605262c-0f94-4c7b-b0bb-39ae2637b777**: client_accepted00:06:36.730556,
  client_peer_verified00:06:37.1590628, client_completed00:06:40.8571279 +08:00;
  completion elapsed4127.3543ms, native_error_code=null. This confirms the recorded client terminal,
  not independent observation of every installer/UAC/compensation branch.
- Desktop tail records starting_GPUI_application and PlatformHost connection at00:06:47–48 and
  07:59:15 +08:00; the latter silent startup also records completed. Desktop timestamps are UTC;
  these entries are in `desktop.log.2026-10-23` despite local date October24.
- No `Setup Log*.txt` was found in the current user's Temp root. Full Inno interaction was not observed.

## Non-green observations retained, no unrelated repair

The PlatformHost snapshot contains9129 physical lines,9127 valid JSON records. Lines642/643 are
malformed. An initial strict aggregation command exited1; the corrected read-only collector counted
and omitted malformed lines without changing or reconstructing them. A damaged line contains a
maintenance fragment, so it is not counted as a complete successful maintenance event.

The snapshot contains1156 error-level `shell.extract_shell_icon.completed` records, plus their
command projections; sampled success=false/native_error_code=null. Last sampled correlations:
3ccac5fb-40fa-4d90-9671-7fc35dc376db and49523230-9a1f-4a6f-b83e-1b0bc5263b44,
08:15:08.256 +08:00. These are event counts, not1156 distinct user-visible defects. No raw icon path
or content was collected; root cause and user impact are unestablished.

Other warning/error event groups include Quick Launch directory recognition/Directory Opus refresh,
elevated directory-helper connection, Quick Launch window ownership and Clipboard source-icon events.
Desktop tails also contain warnings/errors. Therefore logs are **not clean**. These observations are
recorded separately, not attributed to P208 or repaired without a scope switch. User reports no
visible problem; there is no confirmed current P208 product defect or installation failure.

## Initial 08:15 snapshot — real P208 evidence then pending

At the initial08:15 capture, valid PlatformHost records contained only archive cache sweeps, not a drag lifecycle.
No user-chosen target, exact physical steps, file count/content/structure verification, or drag
correlation has been supplied. This is absence of evidence in this snapshot, not proof that no
operation ever occurred. All following live checks remain **NOT RUN / no acceptance evidence**:

1. Small plaintext ordinary-file/multiselection delivery to a user-selected empty Explorer directory.
2. Small empty/implicit/multilevel directory tree, without duplicate/omitted/flattened output.
3. Escape cancellation followed by successful reuse and a normal drag.
4. Known small encrypted archive: locked feedback, explicit unlock without auto-start, fresh held
   gesture, correct delivery and no password dialog during GetData.
5. Only after preceding checks: refresh/close state, no late grant/old contents/stuck operation.

First ask the user to choose an empty disposable Explorer target and provide its path. Do not choose
an arbitrary directory, overwrite existing files or recursively clean. Advance one small check at a
time after concrete results. Any confirmed problem needs exact time/steps/type/expected-versus-actual/
correlation/native code and owner-boundary diagnosis before a red-capable minimal repair; a new
package requires separate confirmation.

Real foreign-process COM/Explorer, HWND/capture/button continuity, physical gesture, DPI/UIA and user
visual/lifecycle acceptance remain unproved. P206/P207 real registration remains NOT RUN with
ProductApplyAuthorized=false. Earlier A–E/JOB07/SAFE08 obligations remain; no P209, recycling or
fullP208/PhaseF/first-release completion claim. That intake stopped awaiting the user's target and results.

## First user drag — two-file delivery and cleanup observed; content confirmation pending

User selected `D:\Temp\temp`; read-only metadata confirmed it existed and was empty at
2026-10-24T08:22:18.0590365+08:00. Assistant instructed one small unencrypted ordinary-file/multiselect
drag into Explorer, without Extract, overwrite or cleanup. User subsequently reports that testing works.
This statement is user evidence, not direct assistant observation of the physical gesture or target UI.

Follow-up evidence:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\package26-drag1-20261024-082514\drag1-evidence.json`.
Only target metadata and allowlisted events were read. No file contents, executable execution,
extraction, deletion, GUI automation, product/source changes or package operation.

- At08:24:32 +08:00 the target has exactly2 direct files and0 directories: one.exe2021888bytes and
  one.xml291bytes. Both creation times are08:23:35 +08:00. These are not the originally suggested
  two text files. Selected member identities, source format/encryption and byte equality have not
  independently been established; no executable was or should be run for acceptance.
- Drag correlation **045624e3-f141-4c4c-9d53-b72a9ab2889c** logs accepted08:23:33.9554335,
  started08:23:33.9573438, count2; contents requested/ready for indices0 and1; terminal
  08:23:35.4721717, **success=true, cleanup_complete=true, native_error_code=null**, elapsed1516ms.
- Entry/worker correlations **175f803d-42e8-4d3c-b006-d1eea6ff39a7** and
  **851fe1ec-1eb8-4225-9039-0cea5618256e** each record success, one committed output, terminal and
  actual cleanup_complete=true. Read correlationa9495ff0-b99d-421d-a0ab-35abb29b9b2c completes before
  drag. These surrounding correlations share the time window; do not claim a logged parent link.
- No matching malformed lines in the captured correlation records. Earlier damaged records and
  unrelated warnings remain unchanged. No confirmed P208 defect found in this operation.

Scoped outcome: actual production virtual-drag start, indexed materialization, successful native-owner
terminal and cleanup, together with2 target files and the user's positive report, are now observed.
Do not keep this first check labeled NOT RUN; it is **RUN / partial evidence, content and exact sample
confirmation pending**, not full PASS. Ask whether.exe/.xml are exactly the two selected members and
whether output matches the originals; if unverified, request the non-sensitive test archive path for
an authorized read-only comparison, never execute.exe. Do not advance to the tree yet.

Tree/Escape/encrypted/refresh-close remain NOT RUN. Foreign-target identity, full physical capture/
button continuity, arbitrary external COM, DPI/UIA and full lifecycle remain unproved, not all closed
by one success event. P206/P207 and A–E/JOB07/SAFE08 obligations remain as above.

## CLI coverage question — scope decision pending

User asks whether the assistant should test autonomously and expose archive functions through
NxTools.Cli.exe. Read-only inspection of nxtools-cli/src/{main,lib}.rs and Cargo.toml confirms the
current CLI parses download commands and uses DownloadIpcClient only; there is no archive command.
This is a missing automation surface, not a newly reproduced P208 defect. Noninteractive archive
commands should reuse existing archive owners; they cannot substitute for physical Explorer drag.
No CLI implementation/build/package or further user test was started. Pause manual progression
pending the user's explicit choice to switch scope to archive CLI work. First-drag partial evidence
and its missing content/sample confirmation remain as recorded above.

## Approved scope switch to CLI — 2026-10-24

User subsequently approved the proposed archive CLI list/test/extract + structured-result slice.
Manual progression remains paused, not abandoned or accepted. New [CLI outcomes](archive-cli-outcomes.md)
record original-owner implementation, isolated automatic checks and limits. Source/Debug CLI work
does not change the installed sequence26 or retroactively complete first-drag content confirmation,
tree/Escape/encrypted/refresh-close/native-user acceptance. No new release package was authorized.
