"""Keep product material targets alpha-capable without a second runtime writer."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
DESKTOP = ROOT / "rust/apps/nxtools-desktop/src"

# Production constructors with explicit backgrounds; caption-based owners inherit below.
OWNERS = {
    "main.rs": 1,
    "network_settings.rs": 1,
    "main_window/tools_page.rs": 1,
    "quick_launch.rs": 2,
    "quick_launch/file_dialog_search.rs": 1,
    "downloads/window_support.rs": 1,
    "downloads/new_download_view.rs": 1,
    "downloads/task_windows_impl.rs": 1,
    "downloads/browser_handoff_impl.rs": 1,
    "downloads/artifact_actions.rs": 1,
    "downloads/completion_shutdown_impl.rs": 1,
}

failures = []
for relative, count in OWNERS.items():
    source = (DESKTOP / relative).read_text(encoding="utf-8-sig")
    backgrounds = re.findall(r"window_background:\s*WindowBackgroundAppearance::(\w+)", source)
    if len(backgrounds) < count or any(value != "Transparent" for value in backgrounds):
        failures.append(f"{relative}: expected transparent product targets, found {backgrounds}")

caption = (DESKTOP / "ui/components/titlebar.rs").read_text(encoding="utf-8-sig")
helper = caption.split("pub(crate) fn caption_window_options()", 1)[1].split("\n}", 1)[0]
if "window_background: gpui_kit::WindowBackgroundAppearance::Transparent" not in helper:
    failures.append("caption_window_options: ordinary material targets require transparent creation")

# The material paint plugin is the single runtime owner: Solid backdrop/transient windows render
# Opaque for ClearType, and its return to Transparent (which rewrites the WCA accent) re-applies
# the native material.
SETTER_OWNER = Path("ui/material/paint.rs")
for path in DESKTOP.rglob("*.rs"):
    if path.stem.endswith("tests") or path.stem == "tests":
        continue
    relative = path.relative_to(DESKTOP)
    source = path.read_text(encoding="utf-8-sig")
    if re.search(r"\.set_background_appearance\s*\(", source) and relative != SETTER_OWNER:
        failures.append(f"{relative}: runtime background setter competes with native material")

owner = (DESKTOP / SETTER_OWNER).read_text(encoding="utf-8-sig")
sync = owner.split("fn sync_background(", 1)[1].split("\n}\n", 1)[0]
if "native_appearance::invalidate" not in sync:
    failures.append("ui/material/paint.rs: returning to Transparent must re-apply the native material")

assert not failures, "\n".join(failures)
print(f"PASS material creation: {sum(OWNERS.values())} explicit targets plus shared caption owners; one runtime background owner")
print("Scope: constructor/source wiring; native composition and first-show lifecycle require separate acceptance.")
