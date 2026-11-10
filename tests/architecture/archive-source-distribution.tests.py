"""Archive worker source-distribution checks; no publisher or product execution."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]


class ArchiveSourceDistributionTests(unittest.TestCase):
    def load_builder(self):
        path = ROOT / "scripts/package-archive-source.py"
        spec = importlib.util.spec_from_file_location("archive_source", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_workspace_keeps_versions_and_lints_without_desktop_patches(self):
        builder = self.load_builder()
        original = ('[workspace]\nresolver = "2"\nmembers = [\n "desktop",\n "archive",\n]\n'
                    '[workspace.package]\nversion = "0.1.0"\n'
                    '[workspace.dependencies]\nserde = "=1.0.0"\n'
                    '[patch.crates-io]\ngpui = { path = "vendor/gpui" }\n'
                    '[workspace.lints.rust]\nunsafe_code = "deny"\n')
        result = builder.worker_workspace(original, ["rust/crates/nxtools-archive"])
        self.assertIn('"rust/crates/nxtools-archive"', result)
        self.assertNotIn('"desktop"', result)
        self.assertNotIn('[patch.crates-io]', result)
        self.assertIn('serde = "=1.0.0"', result)
        self.assertIn('unsafe_code = "deny"', result)

    def test_bundle_preserves_current_source_dependency_and_licenses(self):
        builder = self.load_builder()
        with tempfile.TemporaryDirectory(dir=os.environ.get("PI_SCRATCH_DIR")) as directory:
            root = Path(directory)
            stage = root / "stage"
            (stage / "rust/crates/nxtools-archive/src").mkdir(parents=True)
            (stage / "rust/crates/nxtools-archive/src/lib.rs").write_bytes(b"current source")
            (stage / "vendor/wim-format-0.1.1").mkdir(parents=True)
            (stage / "vendor/wim-format-0.1.1/LICENSE-LGPL-2.1").write_bytes(b"library license")
            os.utime(stage / "vendor/wim-format-0.1.1/LICENSE-LGPL-2.1", (0, 0))
            (stage / "Cargo.lock").write_bytes(b"exact versions")
            output = root / "bundle.zip"
            builder.write_archive(stage, output)
            with zipfile.ZipFile(output) as bundle:
                self.assertIsNone(bundle.testzip())
                self.assertEqual(bundle.read("rust/crates/nxtools-archive/src/lib.rs"), b"current source")
                self.assertEqual(bundle.read("vendor/wim-format-0.1.1/LICENSE-LGPL-2.1"), b"library license")
                self.assertEqual(bundle.read("Cargo.lock"), b"exact versions")

    def test_package_includes_source_before_signing(self):
        text = (ROOT / "scripts/package.ps1").read_text(encoding="utf-8-sig")
        self.assertIn("package-archive-source.py", text)
        self.assertNotIn("Join-Path $destination 'third_party", text)
        self.assertIn("--installer", text)
        self.assertIn("Complete release distribution failed", text)
        self.assertLess(text.index("package-archive-source.py"), text.index("write-trusted-release-manifest.ps1"))
        self.assertIn("Archive source distribution failed", text)
        notices = (ROOT / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
        self.assertIn("wim-format", notices)
        self.assertIn("ms-compress", notices)
        self.assertIn("archive-worker-source.zip", notices)
        instructions = (ROOT / "third_party/archive-worker/REBUILD.md").read_text(encoding="utf-8")
        self.assertIn("--offline", instructions)
        self.assertIn("--features isolated-codecs", instructions)
        self.assertIn("reverse engineering", instructions)

    def test_complete_distribution_is_explicit_opt_in(self):
        text = (ROOT / "scripts/package.ps1").read_text(encoding="utf-8-sig")
        self.assertIn("[switch]$CreateDistributionArchive", text)
        before_installer_bundle = text[:text.rindex("--installer")]
        condition = before_installer_bundle[before_installer_bundle.rindex("if ("):]
        self.assertIn("if ($CreateDistributionArchive -and -not $SkipInstaller", condition)

    def test_complete_distribution_keeps_installer_and_source_together(self):
        builder = self.load_builder()
        with tempfile.TemporaryDirectory(dir=os.environ.get("PI_SCRATCH_DIR")) as directory:
            root = Path(directory)
            setup = root / "NxTools.Next-setup-0.7.0.exe"
            setup.write_bytes(b"normal installer")
            materials = root / "third_party/archive-worker"
            materials.mkdir(parents=True)
            (materials / "archive-worker-source.zip").write_bytes(b"source kit")
            (materials / "REBUILD.md").write_bytes(b"relink instructions")
            for name in ["LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md"]:
                (root / name).write_bytes(name.encode())
            (root / "unrelated.pem").write_bytes(b"must not be included")
            builder.write_distribution(setup, materials, root)
            with zipfile.ZipFile(root / "NxTools.Next-release-0.7.0.zip") as archive:
                self.assertEqual(archive.read(setup.name), setup.read_bytes())
                self.assertEqual(archive.read("third_party/archive-worker/archive-worker-source.zip"), b"source kit")
                self.assertNotIn("unrelated.pem", archive.namelist())



if __name__ == "__main__":
    unittest.main()
