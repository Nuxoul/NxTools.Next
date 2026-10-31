"""Exercise the actual test-build command with a fake dotnet, without building/publishing."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class VerifiedHostTestBindings(unittest.TestCase):
    def test_current_generated_identities_reach_both_test_configurations(self):
        source = (ROOT / "scripts/build.ps1").read_text(encoding="utf-8-sig")
        match = re.search(r"(?m)^    dotnet build [\s\S]*?(?=^    Assert-NativeSuccess 'dotnet build')", source)
        self.assertIsNotNone(match, "actual Tests build command must remain discoverable")
        base = os.environ.get("PI_SCRATCH_DIR") or tempfile.gettempdir()
        with tempfile.TemporaryDirectory(prefix="build bindings ", dir=base) as scratch:
            directory = Path(scratch)
            for configuration in ("Debug", "Release"):
                with self.subTest(configuration=configuration):
                    generated_root = directory / (configuration + " verified hosts")
                    generated_root.mkdir()
                    script = directory / (configuration + ".ps1")
                    escaped_root = str(generated_root).replace("'", "''")
                    script.write_text(
                        "$ErrorActionPreference='Stop'\n"
                        f"$hostBuildRoot='{escaped_root}'\n"
                        f"$Configuration='{configuration}'\n"
                        "function dotnet { $script:captured=@($args) }\n"
                        + match.group(0)
                        + "\nConvertTo-Json -InputObject $script:captured -Compress\n",
                        encoding="utf-8-sig",
                    )
                    result = subprocess.run(
                        ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script)],
                        capture_output=True, text=True, encoding="utf-8", timeout=30,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    arguments = json.loads(result.stdout)
                    self.assertEqual(arguments[0], "build")
                    self.assertIn(configuration, arguments)
                    for property_name, filename in (
                        ("ReleaseBinaryManifestPath", "ReleaseBinaryManifest.g.cs"),
                        ("ArchiveReleaseManifestPath", "ArchiveReleaseManifest.g.cs"),
                        ("ShellSelectionIdentityPath", "DesktopIdentity.g.cs"),
                    ):
                        expected = f"-p:{property_name}={generated_root / 'generated' / filename}"
                        self.assertIn(expected, arguments, "test Host must use this build's exact approval, not empty source defaults")

    def test_release_capability_gates_and_full_debug_suite_are_distinct(self):
        build_source = (ROOT / "scripts/build.ps1").read_text(encoding="utf-8-sig")
        fixture_source = (ROOT / "scripts/test-platformhost-fixtures.ps1").read_text(encoding="utf-8-sig")
        self.assertIn("-Configuration $Configuration -HostBuildRoot $hostBuildRoot", build_source)
        source = build_source + "\n" + fixture_source
        commands = re.findall(r"(?m)^ +dotnet run [\s\S]*?(?=^ +Assert-NativeSuccess)", source)
        self.assertEqual(len(commands), 2, "Release capability gate and unfiltered Debug suite required")
        base = os.environ.get("PI_SCRATCH_DIR") or tempfile.gettempdir()
        with tempfile.TemporaryDirectory(prefix="fixture modes ", dir=base) as scratch:
            captured = []
            for index, command in enumerate(commands):
                script = Path(scratch) / f"mode-{index}.ps1"
                script.write_text("$ErrorActionPreference='Stop'\nfunction dotnet { $script:captured=@($args) }\n"
                                  + command + "\nConvertTo-Json -InputObject $script:captured -Compress\n",
                                  encoding="utf-8-sig")
                result = subprocess.run(["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script)],
                                        capture_output=True, text=True, encoding="utf-8", timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
                captured.append(json.loads(result.stdout))
            release, debug = captured
            self.assertEqual(release[release.index("-c") + 1], "Release")
            self.assertEqual(release[release.index("--fixtures") + 1:],
                             ["archive HZ4 process fixture", "archive HZ4 TERM01"])
            self.assertEqual(debug[debug.index("-c") + 1], "Debug")
            # PowerShell consumes -- for a function stub, unlike a native executable.
            self.assertEqual(debug[-2:], ["--no-build", "--fixtures"], "full suite must not be filtered")
        for variable in ("WORKER_FIXTURE", "CLI_LIFECYCLE_EVIDENCE_ROOT", "VOLUME_FIXTURE_ROOT"):
            self.assertIn("$env:NXTOOLS_ARCHIVE_" + variable + " = $original", source,
                          "caller fixture environment must be restored")


if __name__ == "__main__":
    unittest.main()
