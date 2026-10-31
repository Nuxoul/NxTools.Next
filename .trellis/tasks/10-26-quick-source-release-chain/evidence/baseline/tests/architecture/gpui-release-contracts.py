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
    "gpui-kit": "0.8.0", "gpui-base": "0.8.0", "gpui-component": "0.8.0",
    "gpui-component-macros": "0.8.0", "gpui-kit-assets": "0.8.0",
}
VENDORS = {
    "gpui-component": ("vendor/gpui-component-080", "0.8.0", "src/notification.rs"),
    "gpui-pre-windows": ("vendor/gpui-pre-windows-039", "0.3.9", "src/events.rs"),
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
    require(manifest["workspace"]["dependencies"]["gpui-kit"] == "=0.8.0", "Kit must pin exact0.8.0")
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text("utf-8-sig"))["package"]
    for name, version in EXPECTED.items():
        require([p["version"] for p in packages if p["name"] == name] == [version], f"mixed/missing {name}")
    for package in packages:
        if (package["name"] == "gpui-pre" or package["name"].startswith("gpui-pre-")) and package["name"] != "gpui-pre-reqwest":
            require(package["version"] == "0.3.9", f"backend mismatch: {package['name']}={package['version']}")
    # Cargo locks optional dependencies too; only the enabled target tree proves Fast activation.
    if args.active_tree:
        tree_bytes = args.active_tree.read_bytes()
        active = tree_bytes.decode("utf-16" if tree_bytes.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
        require("gpui-fast v" not in active and "gpui-fast-platform v" not in active, "Fast backend activated")
        require("gpui-pre v0.3.9" in active and "gpui-kit v0.8.0" in active, "active graph lacks new default stack")
    for name, (path, version, changed_file) in VENDORS.items():
        require(manifest["patch"]["crates-io"][name]["path"] == path, f"wrong patch: {name}")
        directory = ROOT / path
        vendor_manifest = tomllib.loads((directory / "Cargo.toml").read_text("utf-8-sig"))
        require(vendor_manifest["package"]["version"] == version, f"wrong vendored version: {name}")
        provenance = json.loads((directory / "NXTOOLS-UPSTREAM.json").read_text("utf-8"))
        require(provenance["name"] == name and provenance["version"] == version, "provenance identity mismatch")
        require(provenance["modified_files"] == [changed_file], "unexpected patch scope")
        require((directory / "LICENSE-APACHE").is_file(), "license missing")
        for relative, digest in provenance["files"].items():
            file = directory / relative
            require(file.is_file(), f"missing upstream file: {path}/{relative}")
            if relative != changed_file:
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
    backend = (ROOT / VENDORS["gpui-pre-windows"][0] / "src/events.rs").read_text("utf-8")
    compact_backend = re.sub(r"\s+", "", backend)
    for token in ["WM_MOUSEACTIVATE => {", "GetWindowLongPtrW(handle, GWL_EXSTYLE)", "WS_EX_NOACTIVATE.0", "if no_activate { MA_NOACTIVATE } else { MA_ACTIVATE }"]:
        require(re.sub(r"\s+", "", token) in compact_backend, f"passive policy lost: {token}")
    print("PASS GPUI release: coherent Kit0.8.0/pre0.3.9; exactly two narrow licensed vendor-file deltas")
    print("Scope: source/lock/provenance only; compiled/native/physical/visual acceptance is separate.")


if __name__ == "__main__":
    try:
        main()
    except (AssertionError, KeyError, OSError) as error:
        raise SystemExit(f"FAIL GPUI release contract: {error}")
