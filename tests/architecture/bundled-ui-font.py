"""Check the approved private font asset and its startup/default/license wiring."""
from pathlib import Path
import re
import struct

ROOT = Path(__file__).resolve().parents[2]
DESKTOP = ROOT / "rust/apps/nxtools-desktop"
FONTS = DESKTOP / "assets/fonts"
FAMILY = "Sarasa UI SC"

faces = sorted(path.name for path in FONTS.glob("*.ttf"))
assert faces == ["SarasaUiSC-Regular.ttf"], faces
font = (FONTS / faces[0]).read_bytes()
assert len(font) == 24_049_996
assert font[:4] == b"\x00\x01\x00\x00"

tables = {}
for index in range(struct.unpack_from(">H", font, 4)[0]):
    tag, _, offset, length = struct.unpack_from(">4sIII", font, 12 + index * 16)
    assert offset + length <= len(font)
    tables[tag] = memoryview(font)[offset:offset + length]
assert struct.unpack_from(">H", tables[b"OS/2"], 4)[0] == 400

name = tables[b"name"]
_, count, strings = struct.unpack_from(">HHH", name)
values = {}
for index in range(count):
    platform, _, _, key, length, offset = struct.unpack_from(">HHHHHH", name, 6 + index * 12)
    if platform in (0, 3) and key in (1, 2):
        value = bytes(name[strings + offset:strings + offset + length]).decode("utf-16-be")
        values.setdefault(key, set()).add(value)
assert FAMILY in values[1], values
assert "Regular" in values[2], values

core = (ROOT / "rust/crates/nxtools-core/src/lib.rs").read_text(encoding="utf-8-sig")
match = re.search(r'DEFAULT_UI_FONT_FAMILY: &str = "([^"]+)";', core)
assert match and match[1] == FAMILY
startup = (DESKTOP / "src/main.rs").read_text(encoding="utf-8-sig")
positions = [startup.index(call) for call in (
    "crate::assets::register_fonts(cx)",
    "gpui_kit::init(cx)",
    "crate::theme::set_ui_font_family(",
)]
assert positions == sorted(positions)
assets = (DESKTOP / "src/assets.rs").read_text(encoding="utf-8-sig")
assert 'include_bytes!("../assets/fonts/SarasaUiSC-Regular.ttf")' in assets
assert "Cow::Borrowed(SARASA_UI_REGULAR)" in assets
assert ".add_fonts(" in assets
license_text = (FONTS / "OFL-Sarasa-Gothic.txt").read_text(encoding="utf-8-sig")
for required in (
    "Copyright (c) 2015-2025, Renzhi Li",
    "Reserved Font Name 'Source'",
    "SIL Open Font License v1.1",
    "5. The Font Software, modified or unmodified",
    "OTHER DEALINGS IN THE FONT SOFTWARE.",
):
    assert required in license_text, required
about = (DESKTOP / "src/main_window/about.rs").read_text(encoding="utf-8-sig")
assert '("Sarasa UI SC", "SIL OFL-1.1")' in about
assert "crate::assets::SARASA_FONT_LICENSE" in about
print("PASS bundled UI font: one Sarasa UI SC Regular400; embedded static bytes, default, startup and OFL")
print("Scope: asset metadata/source wiring only; native DirectWrite rendering and user visual acceptance are separate.")
