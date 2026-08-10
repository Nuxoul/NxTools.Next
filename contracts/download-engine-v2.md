# Download engine contract v2

Status: active for download phase 4

Last updated: 2026-08-10

This contract retains every v1 command and invariant from `download-engine-v1.md`, with
`protocol_version = 2`. Version 2 adds the bounded browser Blob artifact boundary; v1 envelopes are
rejected rather than guessed or silently upgraded.

## Browser Blob commands

- `browser_open_blob_dialog { handoff_id, blob, save_directory }` delivers private staged metadata
  to the GPUI confirmation surface without performing a network probe.
- `browser_import_blob { handoff_id, blob, save_directory, file_name }` commits a completed staging
  artifact as one `Completed` browser task. The persistent handoff mapping is checked before the
  staging path so a response-loss replay resolves to the original task.
- `browser_discard_blob { handoff_id, blob_id }` removes unconfirmed staging while preserving the
  original correlation ID and handoff ID.

`blob` contains only a random upload ID, bounded file name/MIME metadata and exact byte size. The
engine derives the staging path from its private data root; no browser-provided path is accepted.

The Native Messaging frame remains a metadata/control plane. Blob bytes use BrowserHost's bounded,
random loopback capability and never enter this command envelope. All results repeat the request ID
and use the existing accepted/dialog-accepted/rejected outcomes.
