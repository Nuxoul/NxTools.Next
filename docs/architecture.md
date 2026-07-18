# Architecture

## Process model

```text
NxTools.exe (Rust + GPUI)
  |-- runs without a main window
  |-- owns popup UI, domain state, SQLite, commands and orchestration
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

## Clipboard slice

```text
Windows copy
  -> C# snapshots every available text, image and file-drop representation plus its source window
  -> C# stages image bytes; file paths remain platform metadata
  -> Rust selects the primary history type, ingests content-addressed images and persists it in SQLite

Alt+C
  -> C# WM_HOTKEY captures target HWND + focused child HWND + cursor
  -> Rust opens a GPUI WindowKind::PopUp near the cursor
  -> user searches/clicks a Rust-owned history item
  -> Rust sends paste_text, paste_image or paste_files with the original target
  -> C# restores Windows focus and emits Ctrl+V
  -> C# returns a result correlated to the paste command
  -> GPUI popup closes on success, or stays open with an error on failure
```

The Rust `ClipboardApplication` is the application-layer module: it owns SQLite history,
search, platform notifications and commands. The GPUI `ClipboardPanel` only owns view state and
translates search/click interactions into application intents. The C# PlatformHost owns the
Windows-only clipboard format extraction/write-back, topmost operation, focus restoration,
modifier release, input injection and `WM_PASTE` fallback.

The clipboard panel is not a page in an application main window. Its GPUI window does not exist
until the global shortcut is pressed. Escape, a successful paste, or a foreground-window change hides
the native panel without destroying its GPUI window. On the next shortcut, Rust creates the new
panel and retains the previous hidden window until the asynchronous topmost result arrives. A
topmost failure is nonfatal: the new panel stays usable and only the previous hidden panel is
released. The native event loop therefore never observes its last window closing.

## Reference use

| Reference | Used for | Source reuse |
| --- | --- | --- |
| Zed | GPUI interaction and visual principles | Apache GPUI dependency only |
| SwiftList | index/service separation and search behavior | concepts and measurements |
| Ditto | clipboard formats and paste behavior | clean-room behavior only |
| AB Download Manager | download domain and recovery behavior | Apache-compatible concepts |
