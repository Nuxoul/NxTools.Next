"""Approved project palette/schema/state contract; not screenshot or user acceptance."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "rust/apps/nxtools-desktop/assets/themes"
schema = (ROOT / "vendor/gpui-component-0100/src/theme/schema.rs").read_text(encoding="utf-8")
known = set(re.findall(r'#\[serde\(rename = "([^"]+)"\)\]', schema))

def luminance(value):
    values = [int(value[i:i+2], 16) / 255 for i in (1, 3, 5)]
    linear = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4 for v in values]
    return sum(a*b for a,b in zip(linear, (.2126, .7152, .0722)))

def contrast(a, b):
    a, b = luminance(a), luminance(b)
    return (max(a,b)+.05)/(min(a,b)+.05)

for mode in ("light", "dark"):
    theme = json.loads((ASSETS / f"nxtools-graphite-{mode}.json").read_text(encoding="utf-8"))["themes"][0]
    colors = theme["colors"]
    assert theme["mode"] == mode
    assert not set(colors) - known, set(colors) - known
    for foreground, background in [("foreground", "background"), ("foreground", "list.active.background"), ("tab.foreground", "sidebar.background"), ("muted.foreground", "sidebar.background")]:
        assert contrast(colors[foreground], colors[background]) >= 4.5, (mode, foreground, background)
    for role in ("primary", "secondary", "success", "warning", "danger", "info"):
        for state in ("", ".hover", ".active"):
            assert contrast(colors[f"{role}.foreground"], colors[f"{role}{state}.background"]) >= 4.5, (mode, role, state)
        assert colors[f"{role}.hover.background"] != colors[f"{role}.active.background"]
    assert colors["list.active.background"] != colors["list.hover.background"]
    assert theme.get("radius") is None and theme.get("font.size") is None
    print(f"PASS graphite {mode}: {len(colors)} schema roles, readable text/state pairs")
print("Source/numeric contract only; GPUI/native/visual acceptance is separate.")
