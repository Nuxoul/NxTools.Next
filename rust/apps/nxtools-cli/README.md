# NxTools CLI

Existing download commands remain unchanged; run without arguments for syntax.

## Archive commands

Use one matching build with `NxTools.Cli.exe`, `NxTools.PlatformHost.exe` (and its .NET runtime
files), and approved `NxTools.ArchiveWorker.exe` together. Do not copy only a new CLI into an old
installation. Source builds deliberately fail closed on unapproved workers; the original build
manifest generator supplies actual worker identity. CLI launches only its sibling **archive-only**
PlatformHost on a private GUID pipe, never Desktop, its ordinary pipe, windows, hooks or services.

```powershell
& .\NxTools.Cli.exe archive list 'D:\Test\sample.zip' --json
& .\NxTools.Cli.exe archive test 'D:\Test\sample.zip' --json
& .\NxTools.Cli.exe archive extract 'D:\Test\sample.zip' --dest 'D:\Test\output' --json
```

- `list` returns original ordinals, names/types, nullable sizes, encryption and name warnings.
  It retains original v1 ceilings (4096 entries /4MiB metadata); it never silently truncates success.
- `test` actually decodes/verifies through the original isolated worker without writing outputs.
- `extract` preserves the archive hierarchy, including empty/implicit directories, and keeps MOTW
  enabled. Target must be explicitly supplied. Default conflict policy `fail` cancels on the original
  conflict prompt, without replacing the existing file. Previously committed outputs are not rolled back.
- `--conflict skip|overwrite|rename` is explicit per-command authorization. Skip returns
  `succeeded_with_warnings` with skipped counts; rename uses the original noncolliding suffix policy.
- Missing target requires explicit `--create-destination`. That flag does not authorize low disk
  space, risk, output-budget increases, source deletion or a different destination.
- `--codepage 0|65001|936|950|932|949|437|1250|1251|1252` uses the original name-decoding policy;
  `0` means automatic. Authoritative Unicode still wins over manual legacy pages.
- Noninteractive: password-required and other unapproved prompt cases fail with stable codes;
  **no password argv/env/config, password dialog, retry or password manager** is added. Passworded
  metadata may list if the original format permits; actual TEST/extract still requires authorization.
- Ctrl+C requests original-operation cancellation and waits for real cleanup. Deadline/disconnection
  or unconfirmed cleanup is failure, not success. Published output facts remain visible.
- `--connect-only` is download-only and rejects archive commands. There is no archive create/modify,
  selected extraction, GUI-open or virtual-drag command in this slice.

## Structured results and exits

`--json` emits exactly one JSON object on stdout, with no progress noise:

```json
{"schema_version":1,"request_id":"<uuid>","command":"extract","success":true,"result":{"terminal":{"outcome":"succeeded","files_committed":2,"cleanup_complete":true}},"error":null}
```

This is a shape example, not a complete terminal. Actual result retains original terminal fields:
verified/committed/skipped counts, commit state/journal, cleanup, elapsed and native error code.
Failure may still include committed files. Queue admission/ACK/accepted/private completion is not
reported as operation success. `success=true` requires a successful original terminal and confirmed
cleanup; `succeeded_with_warnings` remains distinguishable in the terminal.

Exit codes:0 success (including explicitly skipped warnings),2 syntax,3 unavailable host/runtime,
6 rejected/required interaction,7 execution or cleanup failure,8 incompatible protocol/capability.
Text mode escapes member names and prints human-readable terminal facts. Requested path/member
output is not diagnostic telemetry; secrets never belong in either output or logs.

## Automated verification

```powershell
# Build/test with the workstation memory policy; never run alongside another Cargo owner.
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
cargo test -p nxtools-cli --lib --test archive_cli -- --test-threads=1
# Explicit matching isolated build path and scratch root, no installed product/cleanup:
python tests/integration/archive-cli.py --cli '<scratch>\bin\NxTools.Cli.exe' --evidence-root '<scratch>'
```

The opt-in gate independently generates small ZIP fixtures, checks metadata/CRC failures, literal
bytes and full empty/implicit tree, fail/skip/rename/overwrite policies, destination authorization,
unsafe paths, missing/unsupported sources and single JSON errors. It retains every output in a fresh
GUID directory. It does not build/install/package, use user data or delete evidence.

These tests do **not** certify physical mouse capture/Explorer OLE/foreign COM/DPI/UIA or complete
P208/Phase F. Current source delivery does not change the installed sequence26 release.

## Bounded lifecycle probes (opt-in, test infrastructure only)

`tests/integration/prepare-archive-cli-lifecycle.ps1 -EvidenceRoot <fresh-session-scratch>` builds
a matching worker and a scratch-only host source-copy with original finite checkpoints. It never
adds a production command/test mode or changes installed files. The native signal driver is:

```powershell
python tests/integration/archive-cli-lifecycle.py --bin '<scratch>\fixture-bin' --evidence-root '<scratch>' --case signal
```

Cases: `signal`, `partial_signal`, `parent_exit`, `eof`, `diagnostics`; `--default-sink` selects
actual default diagnostic output after isolated-sink verification. Signals are refused unless a
new independently created console contains only the owned driver and real CLI. This is native
CTRL_C_EVENT evidence with a checkpoint-instrumented host, not a physical keypress or ordinary
uninstrumented-host claim. Results keep exact JSON, native lifecycle events, before/after source/
helper exclusive-open facts and child state. No user data/large load/recursive evidence cleanup.
The owner-only C# cleanup timeout fixture is separate; it is not proof of every CLI timeout path.
