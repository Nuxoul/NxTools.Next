# Architecture

## Process model

```text
NxTools.exe (Rust + GPUI)
  |-- owns the compact main window plus on-demand popup surfaces
  |-- owns GPUI rendering, domain state, SQLite, commands and orchestration
  |-- named pipe: nxtools-next-platform-v1
  |
  +---- NxTools.PlatformHost.exe (.NET 8)
  |       owns user-session Windows API adaptation only
  |
  +---- NxTools.Indexer.exe (Rust)
          owns USN/MFT indexing and finder snapshots

NxTools.Cli.exe
  |-- owns argument parsing and terminal presentation only
  +-- current-user pipe: nxtools-next-download-v1
        -> the Rust desktop/background download host remains the sole transfer and database owner

Browser extension
  +---- NxTools.BrowserHost.exe (Rust Native Messaging adapter)
          -> browser-owned lifetime; exits when the native stdio channel closes
          -> opens NxTools.exe --browser-dialog for confirmation-window requests;
             GPUI creates the main window hidden, then PlatformHost shows it minimized/no-activate
             while the add-download window claims foreground
          -> opens NxTools.exe --background only for non-interactive download commands
```

The desktop process is the source of truth. Adapter processes may reconnect and be restarted
without reconstructing product state. Messages use newline-delimited JSON during development;
the envelope is versioned so payload encoding can change without blurring process ownership.
The LocalSystem indexer intentionally remains available across GUI restarts, while BrowserHost
follows the browser extension connection rather than the GUI lifetime.

## Rust owns

- every GPUI surface and UI state machine;
- download planning, segmentation, scheduling and persistence;
- clipboard history, deduplication, categorization, search and retention;
- finder indexing, ranking, fuzzy search and actions;
- floating assistant profiles, actions and orchestration;
- settings, migrations, storage, telemetry and lifecycle.

## C# owns

- clipboard listener, format extraction, write-back and OLE interaction;
- semantic-to-Win32 hotkey mapping, registration and low-level user-session hooks;
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
  -> C# snapshots every available text, image and file-drop representation plus its source window/icon
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

The GPUI executable embeds the selected Google Material Symbols SVGs through its own `AssetSource`.
Icons are presentation assets only: they do not introduce a web runtime or move behavior into the
view layer. The source program remains responsible for supplying process icons for history rows.

The popup uses a variable-height GPUI list: text and file rows remain compact, while image rows
preserve aspect ratio without upscaling and cap the preview at 270 logical pixels. Source-process
icons are extracted by the Windows adapter, content-addressed by Rust, and shown with a process-name
tooltip; the visual treatment remains NxTools.Next's Zed-inspired design rather than copying the
reference Avalonia surface.

Favorites, pins, edits and soft deletion remain Rust domain operations. GPUI sends typed item
intents to `ClipboardApplication`; the application updates SQLite and reloads the current query.
Pinned records sort before unpinned records, editing removes preserved rich formats and produces a
plain-text record, and deletion clears the pin while retaining the favorite flag. The panel keeps
only the most recently deleted id for immediate undo; a persistent recycle-bin surface is a later
slice.

Clipboard categories are also Rust-owned. SQLite stores five built-in filters plus user-created
categories, their ordering and case-insensitive source-process membership. GPUI routes category-bar
wheel input horizontally, persists drag/drop ordering, and exposes add/rename/delete plus source
assignment without involving the Windows adapter. Built-in categories are immutable definitions but
may be reordered; custom categories filter all visible records captured from their assigned source
programs.

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

## Quick-launch phase-1 slice

```text
double Ctrl (or a configured combination shortcut)
  -> C# low-level gesture/WM_HOTKEY adapter captures the previous foreground/focus HWND and cursor display
  -> Rust reuses one hidden GPUI WindowKind::PopUp near the upper quarter of that display
  -> Rust nxtools-finder searches its in-memory application catalog and usage weights
  -> C# extracts visible Shell icons on demand into a bounded staging cache
  -> Enter/Ctrl+Enter/Ctrl+Shift+Enter sends a correlated open/reveal/run-elevated command
  -> successful result increments Rust's SQLite-backed usage history and hides (not destroys) HWND
```

`nxtools-finder` recursively catalogs `.lnk`, `.url` and `.exe` entries from the per-user/common
Start Menu and desktops, plus an explicit safe system-tool set. Search never scans these directories
or reads SQLite per keystroke: both the application entries and activation weights are memory-backed
after startup. C# does not parse the query or rank results.

The main window's 速启 page exposes registration and catalog status only. The actual input and
result list exist exclusively in the standalone popup. Escape clears a non-empty query first and
hides the popup second; external focus/pointer changes hide it after the opening grace period, while
a repeated hotkey toggles and reuses the same native window.

### Quick-launch contextual directory navigation

Every supported host crosses one tagged protocol seam:
`DirectoryNavigationContext { host, window_handle, current_directory?, host_context_id? }` at
summon and
`NavigateDirectory { host, window_handle, target_directory, host_context_id? }` at execution. Rust
freezes that context, exposes one “切换到此目录” action and records only confirmed successful
directory visits. It treats `host_context_id` as opaque and does not call COM, UI Automation or
file-manager-specific APIs.

C# maps the tag to Windows adapters. Explorer uses exact-HWND Shell COM navigation; the Shell desktop
opens the target through Windows Shell; native open/save dialogs use their captured address control
without submitting a file. The generic `third_party_file_manager` tag enters a C# registry whose
built-in adapters support Directory Opus, Total Commander and OneCommander. Every successful capture
creates a bounded, one-shot registry entry keyed by `host_context_id`, so a later summon from the same
HWND cannot overwrite the pane selected by an earlier request. Directory Opus and Total Commander use
their native command channels with bounded sends and post-action path verification; OneCommander
freezes the focused UI Automation pane during summon, runs UIA work on bounded STA operations and
reuses that exact pane only for the requested action. A timed-out UIA operation keeps an operation-
name lease until its worker actually exits, so repeated summons cannot accumulate blocked STA
threads. No adapter polls while Quick Launch is idle or searches.

Automatic quick switch remains split across the same boundary: the C# foreground adapter recognizes
one known file-dialog → Explorer → same-dialog transition and publishes the frozen source path;
Rust owns enablement, freshness validation and the `NavigateDirectory` command. The attached
file-dialog search surface is a reusable GPUI window owned by the exact dialog HWND. C# supplies
physical placement/DPI/current-directory facts and executes exact-HWND navigation or explicit file
submission; Rust owns the query generation, current-directory scope, result state and all rendering.

High-integrity hosts use the same directory-navigation contract without elevating the Rust/GPUI
desktop. The panel request is published first; high-integrity capture runs off the hotkey/message
thread and returns through a summon-timestamp/HWND-scoped context-resolution event, so a slow helper
cannot consume the 60 ms popup budget or attach to a later summon. The medium-integrity C# host
creates a random current-user-only pipe and a 256-bit
authentication token, then asks the already installed LocalSystem index service to start the fixed
packaged C# helper in the requesting desktop session. For that privileged command, the service
derives the real client PID and session from the connected pipe handle, accepts only the desktop
path and bytes approved by the elevated installer, and never accepts an executable path from the
request. The installer takes ownership of the ProgramData product root, rejects reparse points,
replaces its DACL, and deploys the indexer, broker PlatformHost, browser Native Messaging host and
authorized desktop under
`%ProgramData%\NxTools\NxTools.Next\Service`. Only SYSTEM/Administrators retain write access;
SCM, the broker and privileged requests never execute or trust the user-writable Release directory.
The UAC-authorized PlatformHost pins its own process image at startup and embeds build-generated
SHA-256 values for the Rust desktop, browser host and indexer; the elevated installer accepts only
source handles
matching that compiled manifest before it publishes protected copies.
When an already protected desktop repairs ACL, registry or SCM state, identical source and
destination paths skip binary replacement while all security descriptors and hashes are still
revalidated.
The Release executable acts only as an update launcher: after a successful protected deployment it
restarts the byte-identical protected desktop, which waits for the previous single-instance lock.
Ordinary bounded
index queries retain the existing authenticated local-pipe contract. The broker uses the session token and its linked
elevated token to launch `NxTools.PlatformHost.exe` on `winsta0\default`; it does not invoke `runas`
or show a per-navigation UAC prompt. The helper is authenticated by token, pipe-client PID, exact
packaged path and elevated token state, then re-captures the exact HWND at high integrity and returns
another bounded one-shot `elevated:` context token. Pipe frames are rejected while streaming once
their cap is reached; rendezvous retries are bounded, peer session IDs must match and Explorer COM
navigation runs behind a bounded STA boundary. Successful or failed launch acknowledgements are
matched to the pipe generation, so normal index status events cannot repeatedly start helpers.
Fatal COM/UIA timeouts are carried in the helper response: both the helper process and the client
connection are retired immediately, so the next summon uses a fresh bounded process rather than a
stale pipe or a late side-effect thread.

## Quick-launch phase-2 tracer bullet

The packaged desktop connects to `NxTools.Indexer.exe --service`, an automatic LocalSystem Windows
service. The service keeps the index alive independently of the user-session UI, publishes protocol
events through a versioned local named pipe and accepts only local authenticated clients. The C#
Windows adapter owns the one-time UAC/SCM installation and start operation; it never reads MFT data.
The Rust/GPUI desktop always remains medium integrity and reconnects when the service or UI restarts.

Search requests cross this bounded process seam with monotonically increasing generations. The
indexer refuses stale generations and the desktop independently discards late batches. Handshake and
volume discovery are published before the initial MFT build is queued on a worker thread, so a slow
volume cannot block search or heartbeat handling. Searches also run against an `Arc`-backed immutable
snapshot with a per-request cancellation token, so new input never waits for an older full-snapshot
scan. Rebuild requests are coalesced to the newest pending drive set and a completed immutable snapshot
is swapped in atomically. If the service stream closes, the
desktop immediately clears volume status and invalidates the active query generation while the
connector retries, so the settings page cannot report stale availability.

### Initial NTFS snapshot

The current phase-2 slice attaches a Windows-only adapter to the Rust indexer. It opens each discovered
drive-letter NTFS volume read-only, queries its journal cursor and enumerates MFT records with
`FSCTL_ENUM_USN_DATA` in bounded 1 MiB batches. Variable-length `USN_RECORD_V2` fields are validated
before decoding; file/parent references rebuild full paths into an immutable per-volume snapshot.
Only a completed drive snapshot is published. There is deliberately no recursive directory-walk
fallback that could turn popup input into foreground disk I/O.

Completed snapshots are persisted as immutable, versioned per-volume files under the machine-wide
index cache. A service restart memory-maps the newest valid version instead of reconstructing every
path and persists USN cursor/replacement/deletion state in a small matching delta file. Full rebuilds
write a new filename before publication, so Windows never has to replace a file that is still mapped;
the next restore removes obsolete versions and interrupted temporary files. Short-lived journal rows
whose parents have already disappeared cancel out of the delta instead of forcing a volume rebuild.

Search remains in the indexer process. A completed generation returns ranked file/directory summaries
that the desktop merges with application results before updating the existing standalone GPUI popup.
Opening and revealing those paths still goes through the C# Shell adapter; C# does not receive MFT
records or participate in ranking.

Windows restricts direct volume access to administrators. The dedicated LocalSystem index service is
therefore a deployment boundary, not an optional performance mode: only the service opens raw NTFS
volumes. The desktop must not suggest running the whole application elevated and must not silently
fall back to recursive foreground filesystem scans.
