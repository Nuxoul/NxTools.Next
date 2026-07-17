# Architecture

## Process model

```text
NxTools.exe (Rust + GPUI)
  |-- owns UI, domain state, SQLite, commands and orchestration
  |-- named pipe: nxtools-next-platform-v1
  |
  +---- NxTools.PlatformHost.exe (.NET 8)
  |       owns user-session Windows API adaptation only
  |
  +---- NxTools.Indexer.exe (Rust)
          owns USN/MFT indexing and finder snapshots
```

The desktop process is the source of truth. Adapter processes may reconnect and be restarted
without reconstructing product state. Messages use newline-delimited JSON during development;
the envelope is versioned so payload encoding can change without blurring process ownership.

## Rust owns

- every GPUI surface and UI state machine;
- download planning, segmentation, scheduling and persistence;
- clipboard history, deduplication, categorization, search and retention;
- finder indexing, ranking, fuzzy search and actions;
- floating assistant profiles, actions and orchestration;
- settings, migrations, storage, telemetry and lifecycle.

## C# owns

- clipboard listener, format extraction, write-back and OLE interaction;
- global hotkeys and low-level user-session hooks;
- foreground process/window/caret and keyboard modifier state;
- Shell/COM operations, Explorer integration and file-dialog adaptation;
- screenshot and window enumeration adapters.

## Boundary rules

1. C# has no product database and never decides retention, ranking, categorization or actions.
2. Rust never embeds the CLR and C# never loads Rust through in-process FFI.
3. Cross-process messages are versioned, bounded and correlation-aware.
4. Events describe observed platform facts; commands express Rust-owned intent.
5. GPL repositories are behavioral references only. Zed's Apache-licensed GPUI crate is the
   sole Zed code dependency.

## Reference use

| Reference | Used for | Source reuse |
| --- | --- | --- |
| Zed | GPUI interaction and visual principles | Apache GPUI dependency only |
| SwiftList | index/service separation and search behavior | concepts and measurements |
| Ditto | clipboard formats and paste behavior | clean-room behavior only |
| AB Download Manager | download domain and recovery behavior | Apache-compatible concepts |
