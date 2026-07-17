# Diagnostics

NxTools.Next writes daily logs under:

```text
%LOCALAPPDATA%\NxTools\NxTools.Next\data\logs
```

- `desktop.log.YYYY-MM-DD` contains Rust/GPUI/application and IPC events.
- `platform-host.log.YYYY-MM-DD.jsonl` contains structured Windows adapter diagnostics.

Every paste request has one UUID correlation ID in both files. Search that UUID to reconstruct
the complete path from the clicked Rust history item through clipboard write, target activation,
modifier recovery, each `SendInput` key event and the final command result.

The Windows log records window handles, process/thread IDs, clipboard sequence numbers, every
clipboard-write attempt, input structure size and numeric Win32 errors. It records the text length
but never the clipboard text itself; sensitive field names are redacted again at the log sink.
Writes use a bounded background queue so disk I/O cannot block the Windows paste thread.
Both desktop and PlatformHost logs are rotated daily and retained for 14 days.

For a failed paste, collect the matching sections beginning with `paste.start` and ending in
`paste.completed` or `paste.failed`, plus the Rust line carrying the same `correlation_id`.
