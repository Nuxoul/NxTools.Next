"""Release probes preserve the approved desktop's native service admission boundary."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]

class IndexerReleaseProbeTests(unittest.TestCase):
    def test_service_probes_do_not_impersonate_a_desktop(self):
        for name in ("indexer-idle-cpu.ps1", "indexer-service-smoke.ps1"):
            with self.subTest(script=name):
                source = (ROOT / "tests/performance" / name).read_text(encoding="utf-8-sig")
                self.assertNotIn("NamedPipeClientStream", source)
                self.assertIn("Invoke-IndexerDesktopSearch", source)

    def test_desktop_probe_uses_existing_driver_and_approved_identity(self):
        helper = ROOT / "tests/performance/indexer-desktop-probe.ps1"
        self.assertTrue(helper.is_file(), "shared authenticated desktop probe is missing")
        source = helper.read_text(encoding="utf-8-sig")
        for contract in ("QuickLaunchComparisonDriver.cs", "NxToolsDesktopSha256", "NxToolsPlatformHostSha256", "ParentProcessId", "RequireEmpty", "input_restored", "ExpectedPath"):
            self.assertIn(contract, source)
        self.assertNotIn("NamedPipeClientStream", source)

if __name__ == "__main__":
    unittest.main()
