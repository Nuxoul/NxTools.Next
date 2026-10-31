# Noninteractive archive CLI

## 1. Scope / Trigger

`archive list/test/extract` composes already implemented archive owners without GPUI/Desktop.
Evidence: `repo:rust/apps/nxtools-cli/src/archive.rs`,
`repo:rust/apps/nxtools-cli/src/archive/execution.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveCliHost.cs`.
No second engine/Job/native output, private worker caller, service or GUI authority.

## 2. Signatures

`parse_invocation(args, default_directory) -> CliInvocation { CliCommand::Archive(ArchiveCommand) }`;
`run_with_host_policy(command, executable, policy) -> Result<(),CliError>`.
`archive list|test <source> [--codepage N] [--json]`;
`archive extract <source> --dest <directory> [--conflict fail|skip|overwrite|rename]
[--create-destination] [--codepage N] [--json]`.
Headless host exact args: `--archive-cli --parent-pid PID --pipe-name nxtools-archive-cli-GUID`.

## 3. Contracts

CLI creates one original platform bridge on a current-user GUID pipe and starts fixed sibling
PlatformHost; host verifies actual ordinary same-user/session/live exact sibling CLI parent and pipe
server PID. ArchiveReadClient consumes v1 bounded batches/ACK and typed terminal; job projection uses
original ArchiveJob command/reserved cancellation/terminal. Original helper approval/worker/source/
output/Job remain native-owned. No environment/PATH/helper override or Desktop/shell adapter startup.

Default extraction Here/full selection/copy_motw=true; destination explicit. Default fail cancels the
original conflict prompt, never truncates. Explicit missing-destination consent responds only to
matching create_destination ticket; no risk/lowspace/budget escalation. Password not accepted through
argv/env/config and no GUI prompt/retry. Native committed/skipped/cleanup facts survive failure.
Ctrl+C/deadline requests original cancellation, never equates receipt to actualcleanup. Normal EOF
sets stop token BEFORE completing source channels, avoiding false pump failure during normal drain.

JSON is one schema_version1/request_id/command/success/result/error object, including errors/syntax.
No progress/mixed stdout or raw rejected argv. Requested member/path output is not diagnostics.
Original correlation goes through IPC/native/worker; host session events use safe fixed stage/count/
elapsed/native only. Same exits0/2/3/6/7/8; --connect-only must reject archive at parser AND runner.

## 4. Validation & Error Matrix

| Input/boundary | Result |
| --- | --- |
| list actual metadata | bounded exact original ordinal/name/type/nullable sizes/warnings; actual terminal |
| TEST valid / CRC corrupt | verified counts / corrupt_archive7, not metadata-only success |
| target conflict default | output_collision6 after cancel/cleanup, existing bytes preserved |
| explicit skip/rename/overwrite | original policy; skip warnings/counts remain visible |
| missing destination without/with consent | destination_required6/no target / exact native ticket approval |
| password prompt / lowspace or risk | password_required6 / interaction_required6 after cleanup, no dialog |
| missing source | source_unavailable7 plus actual native2/3 when supplied |
| unsafe / unsupported | original unsafe_path7 / unsupported_format7, no unauthorized output |
| stale operation/session/generation | ignore, no completion or foreign control |
| receipt/accepted/cancel ACK | not completion; await terminal/actualcleanup |
| cleanup false / child failure | non-success with original facts; no retry/zero-output assertion |
| invalid/secret option or ConnectOnly archive | syntax2, no host lookup/start; no echoed secret |

## 5. Good/Base/Bad Cases

Good: one exact isolated CLI invocation -> original session/worker -> literal file/tree -> actual
cleanup and childexit0 -> single truthful result. Base: tiny list/test/extract with defaults.
Bad: invoke unrestricted unpack/direct File::create, initialize full Desktop adapter, return success
on receipt, hide skip/commits, create destination on lowspace ticket, or call CLI extraction a drag test.

## 6. Tests Required

`repo:rust/apps/nxtools-cli/tests/archive_cli.rs`: actual parser/runner shape, explicit destination,
secret/duplicate/codepage/ConnectOnly refusal and download compatibility.
`repo:rust/apps/nxtools-cli/src/archive/execution/tests.rs`: original routed start/receipt/accepted,
stale terminal, prompt exact cancel and actualcleanup; committed counts retained; destination scope.
`repo:csharp/NxTools.PlatformHost.Tests/ArchiveCliHostTests.cs`: early mode/pipe allowlist, dispatch
receipt identity, denied commands zeroeffects and stale controls through original inactive sessions.
`repo:tests/integration/archive-cli.py`: opt-in matching isolated build, independently generated tiny
ZIPs, exact byte/tree and all conflict policies, corrupt/unsafe/missing/unsupported/single JSON; keeps
negative evidence. `repo:tests/architecture/archive-cli.py` is source-only, not native acceptance.
Physical Explorer/foreignCOM/pressed gesture/DPI/UIA and broad formats remain separate acceptance.
Bounded lifecycle harness and evidence distinctions are owned by [CLI quality](quality-guidelines.md)
and the [lifecycle contract](../../../tasks/10-16-archive-manager/archive-cli-contract.md#bounded-lifecycle-evidence-contract--2026-10-24).
Controlled native console events are not user physical-keyboard acceptance; owner-only Drain(false)
fixtures do not prove the whole public CLI timeout branch. Keep partial publication facts unchanged.

## 7. Wrong vs Correct

Wrong: session.Stop completes events -> Pump sees unexpected EOF -> hostexit1 despite success.
Correct: stop.Cancel -> original session.Stop -> drain actual owners -> childexit0 on normal EOF.
Wrong: parser rejects ConnectOnly but public runner starts archive child anyway.
Correct: both public entry boundaries enforce the same policy before any side effect.
