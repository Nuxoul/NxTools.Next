"""First-party release-version alignment; no build, signing or installation."""
import json
from pathlib import Path
import re
import tomllib
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
VERSION = "0.7.2"


class ProductVersionTests(unittest.TestCase):
    def test_workspace_members_and_lock_use_release_version(self):
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertEqual(workspace["workspace"]["package"]["version"], VERSION)
        locked = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))["package"]
        for member in workspace["workspace"]["members"]:
            package = tomllib.loads((ROOT / member / "Cargo.toml").read_text(encoding="utf-8"))["package"]
            self.assertEqual(package["version"], {"workspace": True}, member)
            versions = [p["version"] for p in locked if p["name"] == package["name"] and "source" not in p]
            self.assertEqual(versions, [VERSION], member)

    def test_managed_release_version(self):
        props = ET.parse(ROOT / "Directory.Build.props")
        self.assertEqual(props.findtext(".//Version"), VERSION)

    def test_installer_and_extension_release_versions(self):
        for name in ["nxtools-next.iss", "nxtools-maintenance.iss"]:
            text = (ROOT / "installer" / name).read_text(encoding="utf-8-sig")
            match = re.search(r'^#define MyAppVersion "([^"]+)"$', text, re.MULTILINE)
            self.assertIsNotNone(match, name)
            self.assertEqual(match[1], VERSION, name)
            self.assertIn("VersionInfoVersion={#MyAppVersion}.0", text, name)
            self.assertIn("VersionInfoProductVersion={#MyAppVersion}", text, name)
        manifest = json.loads((ROOT / "browser_extension/nxtools-interceptor/manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(manifest["version"], VERSION)

    def test_packaging_entries_use_release_filename(self):
        expected = f"NxTools.Next-setup-{VERSION}.exe"
        for name in ["package-local.ps1", "package.ps1"]:
            text = (ROOT / "scripts" / name).read_text(encoding="utf-8-sig")
            self.assertIn(expected, text, name)
            self.assertNotIn("NxTools.Next-setup-0.7.0.exe", text, name)

    def test_desktop_resource_uses_cargo_version(self):
        text = (ROOT / "rust/apps/nxtools-desktop/build.rs").read_text(encoding="utf-8")
        for token in ["CARGO_PKG_VERSION", "CARGO_PKG_VERSION_MAJOR", "CARGO_PKG_VERSION_MINOR", "CARGO_PKG_VERSION_PATCH", "FILEVERSION", "PRODUCTVERSION", 'VALUE "FileVersion"', 'VALUE "ProductVersion"']:
            self.assertIn(token, text)


if __name__ == "__main__":
    unittest.main(verbosity=2)
