#!/usr/bin/env python3
"""Phase F ownership/source guard, not physical OLE or Explorer acceptance."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / "csharp/NxTools.PlatformHost"
session = (base / "Windows/Archive/ArchiveVirtualDragSession.cs").read_text("utf-8-sig")
data = (base / "Windows/Archive/ArchiveVirtualFileDataObject.cs").read_text("utf-8-sig")
content = (base / "Windows/Archive/ArchiveVirtualContent.cs").read_text("utf-8-sig")
plan = (base / "Windows/Archive/ArchiveIntegrationPlan.cs").read_text("utf-8-sig")
pipe = (base / "Protocol/PlatformPipeClient.cs").read_text("utf-8-sig")
browser = (root / "rust/apps/nxtools-desktop/src/archive/browser.rs").read_text("utf-8-sig")
gesture = (root / "rust/apps/nxtools-desktop/src/archive/browser_drag.rs").read_text("utf-8-sig")
application = (root / "rust/apps/nxtools-desktop/src/archive/application.rs").read_text("utf-8-sig")
ipc = (root / "rust/crates/nxtools-ipc/src/lib.rs").read_text("utf-8-sig")
cancel_route = (root / "rust/crates/nxtools-ipc/src/archive_virtual_drag.rs").read_text("utf-8-sig")
freeze = (root / "rust/apps/nxtools-desktop/src/archive/model_drag.rs").read_text("utf-8-sig")
for token in ("ApartmentState.STA", "TryReleaseSourcePointerCapture", "pressed()", "DoDragDrop(data,source,1", "source.QueryContinueDrag(false,keys)", "var keys=(GetAsyncKeyState(1)&0x8000)!=0?1:0;"):
    assert token in session, token
assert session.count("var hr=DoDragDrop(") == 1
for token in ("FileGroupDescriptorW", "FileContents", "TYMED.TYMED_ISTREAM", "format.lindex", "Volatile.Read(ref retired)"):
    assert token in data, token
for token in ("entries.DispatchVirtualMaterialize(command,remaining,authorizationId)", "Task.Run", "entries.OpenVirtualStream", "Action=\"release\"", "items[index].Directory", "remaining-="):
    assert token in content, token
assert 'archiveVirtualDrag.Dispatch(envelope,desktopProcessId)' in pipe
assert 'archiveVirtualDrag.Stop()' in pipe
for token in ("VirtualDragSpike", "on_drag(", "BeginFileDrag", "DoDragDrop"):
    assert token not in browser + gesture, "competing or test-only production gesture: " + token
for token in ("this.drag_down(", "this.drag_move(", "this.drag_up()", "self.invalidate_drag()", "archive-drag-feedback"):
    assert token in browser, token
for token in ("event.pressed_button != Some(MouseButton::Left)", "< 4.", "native_window_handle(window)", "send_archive_virtual_drag(", ".validate()", "self.drag.pending = Some", "model.freeze_drag_selection()"):
    assert token in gesture, token
for token in ("for entry in &self.entries", "ARCHIVE_DRAG_MAX_DESCRIPTORS", "ARCHIVE_DRAG_MAX_FILES", "entry.encrypted.is_none()", "OutputCollision", "ordinal"):
    assert token in freeze, token
assert gesture.index("self.drag.pending = Some") < gesture.index("// Still-pressed MOVE callback")
for token in ("view.configure_drag(", "view.drag_result(", "view.drag_disconnected(", "view.invalidate_drag()"):
    assert token in application, token
assert 'commandName is "archive_virtual_drag" or "archive_virtual_drag_spike"' in pipe
for token in ("archive_drag", "take_drag_cancel()", "observe_virtual_drag_cleanup(&notification)", "held_drag_ids"):
    assert token in ipc, token
for token in ("start_dequeued", "cancel_id", "active.command_wake.try_send", "virtual_cleanup_incomplete", "drag.start_id == *correlation_id"):
    assert token in cancel_route, token
assert "let drag_connection = read.operation.connection_generation();" in application
assert "self.drag.connection = connection;" in gesture
assert "origin_connection.is_none_or(|origin| active.generation == origin)" in cancel_route
assert cancel_route.index("active.generation == origin") < cancel_route.index("router.pending.insert(")
assert "self.drag.candidate = None;" in gesture and "authorization_result" in gesture
assert '"archive-unlock-drag"' in browser and 'Command::UnlockDrag' in browser
assert "TryCaptureVirtualAuthorization" in session and "authorizationToken" in content
secret_owner = (base / "Protocol/ArchiveEntrySession.Password.cs").read_text("utf-8-sig")
for token in ("CryptProtectMemory", "CryptographicOperations.ZeroMemory", "ExpireAuthorizationsLocked", "Matches(", "CancelAuthorizedEntry"):
    assert token in secret_owner, token
assert "PasswordPrompt" not in content
assert "browser_authorize" in (root / "rust/crates/nxtools-archive/src/worker_browser.rs").read_text("utf-8-sig")
for token in ("Registry.CurrentUser", "Microsoft.Win32", "SHChangeNotify", "UserChoice\\", "LocalServer32"):
    assert token not in plan, "pure P206 plan must have no system mutation: " + token
for token in ("Capture(change.Target)", "attempted = index;", "store.Write(", "ContentEquals(fact.AppliedValue)"):
    assert token in plan, token
for token in ("OleSetClipboard", "SetClipboard", "SendInput", "CF_HDROP"):
    assert token not in session + data + content, token
print("PASS Phase F source ownership: formal row threshold/whole-selection/retirement, one STA OLE, delayed original entry/lease, pure P206 plan")
print("Scope: source only; physical gesture/real OLE target/Registry/Explorer acceptance separate.")
