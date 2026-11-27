#!/usr/bin/env python3
"""Source/lock/provenance gate, not native or visual acceptance."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[2]
EXPECTED = {
    "gpui-kit": "0.10.0", "gpui-base": "0.10.0", "gpui-component": "0.10.0",
    "gpui-component-macros": "0.10.0", "gpui-kit-assets": "0.10.0",
}
VENDORS = {
    "gpui-component": ("vendor/gpui-component-0100", "0.10.0", ["src/notification.rs"]),
    "gpui-base": ("vendor/gpui-base-0100", "0.10.0", [
        "src/text/editor_host.rs", "src/text/editor_seam.rs",
        "src/text/format/markdown.rs", "src/text/node.rs", "src/text/state.rs",
    ]),
    "gpui-pre": ("vendor/gpui-pre-0312", "0.3.12", ["src/scene.rs", "src/style.rs"]),
    "gpui-pre-windows": ("vendor/gpui-pre-windows-0312", "0.3.12", [
        "src/directx_renderer.rs", "src/events.rs", "src/shaders.hlsl", "src/window.rs",
    ]),
}


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--active-tree", type=Path, help="cargo tree -p nxtools-desktop --target x86_64-pc-windows-msvc --prefix none output")
    parser.add_argument("--upstream", type=Path, help="optional verified extracted published crate root")
    args = parser.parse_args()
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text("utf-8-sig"))
    require(manifest["workspace"]["dependencies"]["gpui-kit"] == "=0.10.0", "Kit must pin exact0.10.0")
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text("utf-8-sig"))["package"]
    for name, version in EXPECTED.items():
        require([p["version"] for p in packages if p["name"] == name] == [version], f"mixed/missing {name}")
    for package in packages:
        if (package["name"] == "gpui-pre" or package["name"].startswith("gpui-pre-")) and package["name"] != "gpui-pre-reqwest":
            require(package["version"] == "0.3.12", f"backend mismatch: {package['name']}={package['version']}")
    # Cargo locks optional dependencies too; only the enabled target tree proves Fast activation.
    if args.active_tree:
        tree_bytes = args.active_tree.read_bytes()
        active = tree_bytes.decode("utf-16" if tree_bytes.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
        require("gpui-fast v" not in active and "gpui-fast-platform v" not in active, "Fast backend activated")
        require("gpui-pre v0.3.12" in active and "gpui-kit v0.10.0" in active, "active graph lacks new default stack")
    for name, (path, version, changed_files) in VENDORS.items():
        require(manifest["patch"]["crates-io"][name]["path"] == path, f"wrong patch: {name}")
        directory = ROOT / path
        vendor_manifest = tomllib.loads((directory / "Cargo.toml").read_text("utf-8-sig"))
        require(vendor_manifest["package"]["version"] == version, f"wrong vendored version: {name}")
        provenance = json.loads((directory / "NXTOOLS-UPSTREAM.json").read_text("utf-8"))
        require(provenance["name"] == name and provenance["version"] == version, "provenance identity mismatch")
        require(provenance["modified_files"] == changed_files, "unexpected patch scope")
        locked = [p for p in packages if p["name"] == name]
        require(len(locked) == 1 and "source" not in locked[0] and "checksum" not in locked[0], f"vendor not selected by lock: {name}")
        require(set(changed_files) <= set(provenance["files"]), "approved file missing from provenance")
        require((directory / "LICENSE-APACHE").is_file(), "license missing")
        for relative, digest in provenance["files"].items():
            file = directory / relative
            require(file.is_file(), f"missing upstream file: {path}/{relative}")
            if relative not in changed_files:
                require(hashlib.sha256(file.read_bytes()).hexdigest() == digest, f"unapproved vendor delta: {path}/{relative}")
        extras = {p.relative_to(directory).as_posix() for p in directory.rglob("*") if p.is_file()} - set(provenance["files"])
        require(extras == {"NXTOOLS-PATCH.md", "NXTOOLS-UPSTREAM.json"}, f"unexpected vendor extras: {extras}")
        if args.upstream:
            published = args.upstream / f"{name}-{version}"
            actual = {p.relative_to(published).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in published.rglob("*") if p.is_file()}
            require(actual == provenance["files"], "provenance differs from supplied published source")
    notification = (ROOT / VENDORS["gpui-component"][0] / "src/notification.rs").read_text("utf-8")
    require("pub fn duration(mut self, duration: Duration)" in notification, "duration API lost")
    require("notification.duration.unwrap_or(Duration::from_secs(5))" in notification, "raw5s default lost")
    require("timeout: autohide.then_some(duration)" in notification, "official timeout forwarding lost")
    require("pub fn header_action(" in notification and "header_action_builder: None" in notification, "optional header action API/default lost")
    require("let header_action = if closing" in notification, "header action must retire during official exit")
    require(notification.index(".when_some(header_action,") < notification.index('Button::new("close")'), "header action must precede original close")
    require(".when(header_focused, |this| this.visible())" in notification, "focused header visibility lost")
    require('.accessibility_label("Close")' in notification and ".inset_e(rems(0.25), cx)" in notification, "upstream header semantics/layout lost")
    backend = (ROOT / VENDORS["gpui-pre-windows"][0] / "src/events.rs").read_text("utf-8")
    compact_backend = re.sub(r"\s+", "", backend)
    for token in ["WM_MOUSEACTIVATE => {", "GetWindowLongPtrW(handle, GWL_EXSTYLE)", "WS_EX_NOACTIVATE.0", "if no_activate { MA_NOACTIVATE } else { MA_ACTIVATE }"]:
        require(re.sub(r"\s+", "", token) in compact_backend, f"passive policy lost: {token}")
    core = ROOT / VENDORS["gpui-pre"][0] / "src"
    require("UnderBorder = 2" in (core / "scene.rs").read_text("utf-8"), "UnderBorder scene marker missing")
    style = (core / "style.rs").read_text("utf-8")
    require("BorderStyle::UnderBorder" in style and "let compensate_border" in style, "UnderBorder style path missing")
    renderer = (ROOT / VENDORS["gpui-pre-windows"][0] / "src/directx_renderer.rs").read_text("utf-8")
    require("DestBlendAlpha = D3D11_BLEND_ONE;" not in renderer, "source-over destination alpha is additive")
    require(renderer.count("DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;") == 3, "source-over alpha state changed")
    editor_host = (ROOT / VENDORS["gpui-base"][0] / "src/text/editor_host.rs").read_text("utf-8")
    require("pub fn table_cell_bounds(" in editor_host, "editor table-cell geometry API lost")
    require("pub fn table_cell_content_bounds(" in editor_host, "editor table content geometry API lost")
    base = ROOT / VENDORS["gpui-base"][0]
    state = (base / "src/text/state.rs").read_bytes()
    ordinary_import = b"            preserve_editor_whitespace: false,\n"
    require(state.count(ordinary_import) == 1, "ordinary TextView must retain display whitespace semantics")
    original_state = json.loads((base / "NXTOOLS-UPSTREAM.json").read_text("utf-8"))["files"]["src/text/state.rs"]
    require(hashlib.sha256(state.replace(ordinary_import, b"")).hexdigest() == original_state, "TextView state delta exceeds one default initialization")
    require(re.search(r"preserve_editor_whitespace\s*:\s*true", editor_host), "editor-only whitespace opt-in lost")
    print("PASS GPUI release: coherent Kit0.10.0/pre0.3.12; approved Component/Base/core/Windows vendor deltas (1/5/2/4 files)")
    print("Scope: source/lock/provenance only; compiled/native/physical/visual acceptance is separate.")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, KeyError, OSError) as error:
        raise SystemExit(f"FAIL GPUI release contract: {error}")
