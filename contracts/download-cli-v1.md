# NxTools download CLI contract v1

Status: frozen for download phase 0

Last updated: 2026-07-27

## Process model

`NxTools.Cli.exe` is a caller of the Rust download host, not a second download engine.

1. CLI connects to the current-user local pipe `nxtools-next-download-v1`.
2. If the host is absent, CLI starts the packaged `NxTools.exe --background` without showing the
   main window.
3. CLI waits up to five seconds for the pipe and performs a protocol-version handshake.
4. The desktop/background host remains the sole database and transfer owner.
5. CLI never falls back to opening SQLite directly.

The local pipe must use a current-user ACL and bounded frames. Commands use the v1 download command
envelope and always carry a request ID.

## Commands

```text
nxtools download add <url> [--dir <path>] [--name <name>] [--header <k:v>]
nxtools download list [--active|--completed|--failed] [--json]
nxtools download show <id> [--watch] [--json]
nxtools download pause <id...>
nxtools download resume <id...>
nxtools download cancel <id...>
nxtools download remove <id...> [--delete-file]
```

`show --watch --json` emits newline-delimited JSON: one complete event object per line. Human watch
mode may redraw a table, but it consumes the same event stream.

CLI output never prints Cookie, Authorization or complete request headers unless a future explicit
secret-reveal option is designed and confirmed.

## Exit codes

| Code | Meaning |
| ---: | --- |
| 0 | Command accepted or requested query completed |
| 2 | Invalid CLI syntax or argument |
| 3 | Download host unavailable, disconnected or timed out |
| 4 | Task not found |
| 5 | Command invalid for the task's current state |
| 6 | Download request rejected |
| 7 | Accepted operation failed |
| 8 | Protocol version incompatible |

For batch actions, any rejected target produces a non-zero exit. JSON output includes per-task
results so automation can distinguish partial success.

## JSON envelope

Non-watch JSON output has:

```json
{
  "schema_version": 1,
  "request_id": "uuid",
  "ok": true,
  "result": {}
}
```

On failure, `ok` is false and `error` contains the stable engine fault code plus a display message.
Field additions must be backward compatible within v1.
