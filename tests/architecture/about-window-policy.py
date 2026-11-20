"""Guard About's modeless creation policy against the locked Windows modal branch."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
source = (root / "rust/apps/nxtools-desktop/src/main_window/about.rs").read_text(encoding="utf-8-sig")
windows = (root / "vendor/gpui-pre-windows-039/src/window.rs").read_text(encoding="utf-8-sig")
events = (root / "vendor/gpui-pre-windows-039/src/events.rs").read_text(encoding="utf-8-sig")
assert "if params.kind == WindowKind::Dialog" in windows
assert "EnableWindow(parent_window, false)" in windows
assert "EnableWindow(parent_hwnd, true)" in events
assert "SetForegroundWindow(parent_hwnd)" in events
assert not re.search(r"kind\s*:\s*WindowKind::Dialog", source), (
    "About must not enter the framework modal path that disables/re-enables the main window"
)
assert re.search(r"kind\s*:\s*WindowKind::Normal", source)
assert re.search(r"let\s+options\s*=\s*about_window_options\(cx\)\s*;", source)
assert re.search(r"cx\.open_window\(\s*options\s*,", source)
assert "fn about_window_options(" in source
assert "is_resizable: false" in source and "is_minimizable: false" in source
assert 'caption_action(' in source and '"about-close"' in source
assert "WindowControlArea::Close" not in source
print("PASS About policy: modeless fixed window; no modal parent disable/re-enable path; one client Close")
print("Scope: source/native-policy wiring only; physical parent-window presentation is separate.")
