# Platform protocol v1

Transport is a duplex Windows named pipe named `nxtools-next-platform-v1`. Each frame is one
UTF-8 JSON object followed by `\n`; the JSON payload before that delimiter may not exceed 1 MiB.

```json
{
  "protocol_version": 1,
  "id": "8df2ce69-8881-48a3-a6b3-8b9c8aa1771c",
  "correlation_id": null,
  "type": "hello",
  "payload": {
    "client_name": "platform_host",
    "client_version": "0.1.0",
    "capabilities": ["foreground_context"]
  }
}
```

The receiver rejects unsupported protocol versions before interpreting the payload. Replies use
a fresh `id` and set `correlation_id` to the request id. Unknown message types receive an error
reply once request/command routing is introduced; unsolicited platform events require no reply.

## Quick-launch messages

- `quick_launch_panel_requested` is emitted by the C# adapter after its dedicated `WM_HOTKEY`.
  It carries the previous foreground/focus HWND, cursor coordinates, display index, DPI and request
  timestamp. When that exact foreground HWND is a filesystem-backed Explorer Shell window, it also
  carries an optional tagged `directory_navigation_context { host: "explorer", window_handle,
  current_directory?, host_context_id? }`; when it is a strictly recognized native Windows
  open/save dialog, `host` is
  `"file_dialog"`; when it is the real Shell desktop (`Progman`, or a `WorkerW` containing
  `SHELLDLL_DefView`), `host` is `"desktop"` and `current_directory` is the user's physical Desktop
  directory; when C# strictly recognizes a supported Directory Opus, Total Commander or OneCommander
  top-level window and captures its active pane, `host` is `"third_party_file_manager"` and
  `host_context_id` is a non-empty opaque one-shot token for that exact captured pane. Exactly one
  context is captured before the GPUI popup changes foreground focus.
  Rust rejects stale requests and replaces the whole context on every summon. Explorer capture avoids
  Shell COM and filesystem I/O on the hotkey path. File-dialog capture uses bounded native
  class/control inspection and an optional 25 ms breadcrumb read; inability to read a filesystem path
  does not discard an otherwise navigable host.
- `quick_launch_directory_context_resolved` carries a deferred high-integrity context together with
  the original target HWND and request timestamp. C# publishes the panel request before waiting on
  the helper; Rust applies the resolved context only while that exact summon remains open. A late
  resolution from a hidden or newer popup is discarded.
- `configure_quick_launch` and `set_quick_launch_hot_key_recording` keep registration and hotkey
  capture on the Windows adapter while Rust owns the persisted semantic shortcut. The configuration
  also carries independent `auto_quick_switch_enabled` and `file_dialog_search_enabled` flags.
- `quick_launch_auto_switch_requested` carries the exact known file-dialog navigation context, the
  source directory and a timestamp after the Windows adapter has observed one valid file-dialog →
  supported opened-folder provider → same-file-dialog foreground round trip. Explorer, Desktop and
  adapted third-party file managers use the same tagged directory context. First observation never
  switches, each source is consumed once, and Rust rejects disabled, stale or non-file-dialog
  requests before issuing the existing `navigate_directory` command.
- `quick_launch_file_dialog_search_host_changed` carries the exact supported dialog HWND, current
  directory, physical-pixel bounds, monitor DPI, visibility and the optional latest opened-folder
  source. Rust owns and reuses the attached GPUI search window, sets that dialog as its native owner,
  searches the global file index, and sends the dialog directory only as `preferred_directory`
  ranking context. The empty-query surface may render the latest source as a `Ctrl+G` quick-switch
  row. C# updates this host event for foreground, child create/show, move/resize, title, hide and
  destroy changes; it neither renders the search UI nor performs ranking. Both sides advertise
  `file_dialog_search`.
- `set_window_owner` attaches an auxiliary GPUI HWND to the exact file-dialog HWND without activating
  either window. `open_file_in_dialog` is used only by the attached search bar for an explicit file
  result: C# revalidates the exact supported dialog and existing absolute file, writes the full path
  to its filename control, restores focus and submits it. Directory results continue to use
  `navigate_directory`, which never submits a file.
- `open_shell_target` expresses Rust's open/reveal/elevated/system-tool intent. The adapter validates
  the target, limits elevation to existing executables, shortcuts or allowlisted tools, and returns a correlated
  result before Rust records activation and hides the popup.
- `launch_process` is the typed custom-command execution boundary. Rust expands the configured
  executable, argument template and working directory, then sends `executable`, an independent
  `arguments[]` array, optional `working_directory`, `run_as_admin` and `run_silently`. C# maps only
  those fields to `ProcessStartInfo` with `UseShellExecute = true`, `ArgumentList`, optional `runas`
  and hidden window style. C# never parses launcher keywords or placeholders, and Rust never sends a
  caller-assembled command-line string whose quoting must be reinterpreted by the adapter.
- `navigate_directory` carries a tagged host, the exact host HWND frozen in the summon request, an
  existing target directory and the unchanged optional `host_context_id`. For `host: "explorer"`,
  the C# adapter enumerates Shell Windows on an
  STA thread, matches that HWND exactly and invokes the matched window's navigation method, then
  restores the same HWND to the foreground. It never rereads the live foreground window, opens a
  replacement Explorer, or redirects the action to another host. For `host: "file_dialog"`, the C#
  adapter revalidates the exact `#32770` structure, writes only that directory to its path/file-name
  edit, restores dialog/edit focus and submits directory navigation. It never sends a file result,
  clicks Open/Save on the user's behalf, launches Explorer, or redirects to another dialog.
  For `host: "desktop"`, the C# adapter revalidates that the frozen HWND still belongs to the Shell
  desktop and asks Windows Shell to open the existing target directory; the desktop itself is never
  misrouted through Explorer `Navigate2`. For `host: "third_party_file_manager"`, C# reidentifies the
  frozen HWND and consumes the matching `host_context_id` registry entry to navigate only the pane
  captured at summon: Directory
  Opus uses its lister command channel, Total Commander uses its documented Unicode `WM_COPYDATA`
  protocol, and OneCommander uses the captured focused UI Automation pane anchor. Tokens are bounded,
  expire, and are consumed only after a confirmed navigation; stale, missing or cross-HWND tokens
  fail without falling back to another pane. Cross-process messages, UIA calls and result verification
  are bounded; Rust and GPUI do not know product-specific details. Both sides
  advertise the single `directory_navigation` capability, without parallel host-specific Rust commands.
- `elevated_directory_helper_requested` is an internal C# rendezvous event containing only a random
  current-user pipe name and a 256-bit authentication token. Rust retains the latest request and
  forwards it as `ensure_elevated_directory_helper` only after the index-service channel is ready.
  The service derives the caller PID and desktop session from the named-pipe kernel handle, verifies
  that the caller is the administrator-installed protected `NxTools.exe`, and
  launches only the fixed `NxTools.PlatformHost.exe` from the ACL-hardened service directory; no
  caller-controlled executable path crosses the protocol.
  `elevated_directory_helper` is the correlated indexer status response and repeats the requested
  pipe name as its rendezvous generation. Rust acknowledges only the currently advertised pipe, so
  later index status events do not relaunch a completed request and a stale response cannot clear a
  newer request. Before launching, the service also requires the desktop and PlatformHost bytes to
  match SHA-256 values recorded in the protected service registry key by the elevated installer.
  The LocalSystem indexer and broker live under
  `%ProgramData%\NxTools\NxTools.Next\Service` with inherited write access removed. The
  UAC-authorized PlatformHost pins its process image and contains build-generated Indexer/Desktop
  SHA-256 values; Indexer, PlatformHost and the authorized desktop are copied from locked,
  non-reparse source handles only after matching that trust anchor, then executed only from the
  protected directory. The Release artifact directory is never registered with SCM or accepted as
  the privileged broker client.
  The helper must connect to
  the advertised current-user-only pipe, match the token, actual pipe-client PID, exact packaged path
  and elevated-token state, and share the main host's Windows session, before requests are accepted.
  It re-captures the frozen host HWND at high
  integrity and returns opaque `elevated:` context tokens; only those tokens are routed through this
  channel. Ordinary contexts continue through the medium-integrity adapters. Broker startup,
  rendezvous retries, streaming frame reads and every capture/navigation request are bounded and
  fail closed. Fatal navigation responses retire both ends immediately and trigger a fresh
  rendezvous before another capture.
- `query_shell_actions` lazily opens one bounded STA `IContextMenu` session for the selected result.
  C# resolves Windows shortcuts and allowlisted system executables only at this selected-result boundary.
  Its correlated `data` contains one snake-case `ShellActionMenu` level with an optional final
  `navigation_directory`, command identifiers, separators, disabled state, optional staged `icon_path`
  values and opaque `submenu_id` values. Rust uses that final directory only for contextual directory
  navigation; normal launch/copy/Shell actions keep the original result target. A later root query
  replaces the previous session, so the adapter never accumulates per-result Shell objects.
- `query_shell_submenu` carries the live session id and an opaque submenu id. Only that submenu receives
  `WM_INITMENUPOPUP` and is enumerated; opening the root action mode never recursively initializes slow
  Shell extensions in unopened descendants.
- `invoke_shell_action` carries the matching session id, Shell command id and GPUI owner HWND. The C#
  adapter invokes that command through the same live COM session and consumes the session; Rust owns
  action-mode navigation and GPUI owns all rendering. Both sides advertise `shell_context_actions`.
- `extract_shell_icon` asks the adapter for a bounded Shell icon. The result `data` is a staged PNG
  path; Rust caches the association and GPUI renders it without moving ranking into C#.
- `set_window_no_activate` toggles `WS_EX_NOACTIVATE` for an existing GPUI HWND. Quick Launch applies
  it only to the interactive preview window before showing that window, so pointer interaction with
  the preview cannot take keyboard focus from the activatable search panel. Both sides advertise
  `window_no_activate`.
- `set_window_bounds` atomically applies device-pixel position and size with `SWP_NOACTIVATE` before
  a hidden auxiliary GPUI window is first shown. This avoids exposing the Windows `CW_USEDEFAULT`
  creation bounds while Rust/GPUI remains the owner of logical layout. Both sides advertise
  `window_bounds`.
- `set_taskbar_progress` maps a GPUI-owned window and Rust-owned download state to Windows
  `ITaskbarList3` progress. The C# host neither calculates progress nor retains task state; both
  sides advertise `taskbar_progress`.
- `begin_file_drag` starts a native Shell-compatible file drag for one completed artifact while the
  pointer is held over the GPUI drag affordance. It carries the artifact path and visible source
  window handle so C# can release GPUI's pointer capture before entering `SHDoDragDrop`. Rust owns
  which artifact may be dragged; C# owns the Windows OLE drag loop. Both sides advertise
  `native_file_drag`.
- `place_file_dialog_search_window` carries Rust-owned visible width, visible height and the
  result-list overlap above the owner bottom edge. The C# adapter re-reads the live file-dialog and
  GPUI HWND DWM bounds, compensates the popup's non-client frame, centers their visible horizontal
  bounds, and performs one bounded post-resize correction. This keeps layout dimensions in Rust
  while Windows-native coordinates and frame insets remain in C#.
- `pick_directory` carries an optional initial directory and a Rust-owned dialog title. The C#
  adapter only opens the native Windows folder picker and returns the selected path; index settings
  remain Rust-owned.
- `pick_program` carries the current custom-command program value. The C# adapter opens the native
  Windows file picker and returns the selected executable or script path; Rust retains ownership of
  the unsaved editor state and command configuration.

## Clipboard messages

- `paste_image` carries the persisted image path, `include_file_drop_fallback` and
  `paste_after_write`. Rust classifies the foreground target: rich editors receive native image
  representations only, while allowlisted terminals and file managers also receive `CF_HDROP` so
  they can paste the managed image path. C# owns only the Windows data-object mapping and must not
  second-guess this product policy.
- `paste_native_formats` follows the same target policy. For native-image targets Rust removes a
  captured `CF_HDROP` entry and leaves `image_path_fallback` empty; fallback targets retain the
  current managed image path so C# can replace stale file-drop data before writing the clipboard.

## Indexer process messages

`NxTools.Indexer.exe --service` is a LocalSystem Rust Windows service. It exposes bounded
newline-delimited v1 envelopes over the local pipe `nxtools-next-indexer-v1`; the ordinary user-session
desktop is the client. The pipe rejects remote clients and its DACL grants duplex access only to
LocalSystem, administrators and authenticated local users. Diagnostics go to the indexer log. The
desktop begins with `hello` and requests `index_status` and `file_search`.

The C# platform adapter owns service deployment commands. `ensure_indexer_service` validates the
packaged sibling `NxTools.Indexer.exe`, starts a correctly registered service without elevation, or
launches a one-time elevated installer for create/update. Its correlated command result reports the
service state or a user-cancelled/install error. No MFT records cross the C# platform protocol.

- `indexer_event/status` reports the current phase, discovered NTFS mount points, indexed counts,
  journal cursor metadata and the latest actionable error. `volumes_ready` means discovery completed;
  it does not claim that the MFT snapshot is ready. `ready` means every published drive snapshot is
  complete; an unavailable drive is omitted from search and described by `last_error`.
- `indexer_command/search` carries a monotonically increasing generation, query, result limit and
  file/directory context. Every `search_batch` repeats that generation and marks its final batch.
  Returned items contain an executable full path, display name, file/directory kind, best-effort
  metadata and a Rust-owned score; unavailable size/time fields are omitted. The desktop may merge them
  with application results but does not re-query disk.
- Search runs on a worker against an immutable snapshot. A newer generation invalidates all older
  work, and an equal-generation retry receives a new internal task token so the previous retry cannot
  publish late. Both the indexer and desktop enforce the generation rule, leaving handshake, rebuild
  coordination and ping responsive during a large query. One search worker consumes a latest-only
  pending slot, and its top-K working set is capped at 200 results before applying the caller limit.
- `refresh_volumes` repeats safe volume discovery and queues a fresh MFT snapshot. `rebuild_index`
  queues an MFT rebuild for the current drive set. Initial and requested snapshots run on a worker;
  handshake, search and ping handling remain available while that worker is busy. Repeated rebuild
  requests are coalesced to the newest pending drive set.
- `update_settings` carries `indexed_volumes` and `excluded_paths`. Empty `indexed_volumes` means all
  discovered NTFS volumes. Volume scope and exclusions are enforced inside the index service for both
  synchronous and worker searches; a leading `*` query may bypass exclusions but not volume scope.
  Repeating settings after reconnect does not rebuild an already searchable volume.
  The service persists the normalized settings through a flushed temporary file and a last-good
  copy. Corrupt primary and backup settings fail closed until the desktop sends a valid update.
  Build results and failures are merged per volume so scoped recovery cannot erase another
  volume's searchable snapshot or unresolved error.
  The service persists the normalized setting beside its snapshot cache and restores it before
  deciding which missing volumes need a build.
- USN batches publish into the immutable-snapshot overlay before maintenance is scheduled. When that
  overlay crosses its bounded threshold, the service compacts it on a background worker while the last
  published snapshot remains searchable; replacement is accepted only for the same journal ID/cursor.
  All affected volumes from one poll are scheduled independently. Journal expiry/replacement rebuilds
  only the affected mount points and merges those snapshots without forgetting healthy or disabled
  volumes.
- A valid envelope routed to the indexer with an unsupported message kind receives a correlated
  `invalid_indexer_message` fault; malformed frames terminate that client session and trigger reconnect.
- Frames are subject to the shared 1 MiB limit and command queues are bounded; disconnection rejects
  new commands instead of retaining stale work for a restarted session. A closed indexer stream also
  invalidates the desktop's visible volume status and active search generation until a fresh handshake.

Foreground and global-pointer events are shared facts routed by the desktop dispatcher to both the
clipboard and quick-launch application modules. Neither feature consumes the other feature's event
stream or owns the named-pipe receive loop.
