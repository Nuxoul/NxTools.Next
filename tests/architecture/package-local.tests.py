"""Windows-only local packaging handoff tests; fake publisher/files/packager, never Cargo."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.name == "nt", "Windows PowerShell entry")
class PackageLocalTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(
            prefix="package-local-fixture-", dir=os.environ.get("PI_SCRATCH_DIR")
        )
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "project with spaces"
        scripts = self.root / "scripts"
        scripts.mkdir(parents=True)
        shutil.copy2(ROOT / "scripts/package-local.ps1", scripts / "package-local.ps1")
        (scripts / "windows-build-tools.ps1").write_text(
            "function Initialize-LowMemoryRustBuildEnvironment {}\n"
            "function Initialize-GpuiReleaseEnvironment { $env:GPUI_FXC_PATH='fixture-fxc.exe' }\n"
            "function Get-ReleaseBuildCommitHeadroomGiB { return [double]$env:FIXTURE_HEADROOM }\n"
            "function Get-CimInstance { if ($env:FIXTURE_BUSY -eq '1') { [pscustomobject]@{Name='cargo.exe'} } }\n"
            "function Get-AuthenticodeSignature { [pscustomobject]@{Status='NotSigned'} }\n",
            encoding="ascii",
        )
        (scripts / "package.ps1").write_text(
            "param([string]$Configuration,[string]$SigningKeyPath,[long]$ReleaseSequence,[switch]$SkipVerification,[switch]$CreateDistributionArchive)\n"
            "$ErrorActionPreference='Stop'\n"
            "$r=Split-Path -Parent $PSScriptRoot\n"
            "@{configuration=$Configuration;sequence=$ReleaseSequence;skip=[bool]$SkipVerification;distribution=[bool]$CreateDistributionArchive;key=$SigningKeyPath}|ConvertTo-Json|Set-Content (Join-Path $r 'called.json')\n"
            "if ($env:FIXTURE_STDERR -eq '1') { [Console]::Error.WriteLine('Compiling fixture stderr - not a failure') }\n"
            "if ($env:FIXTURE_FAIL -eq '1') { exit 7 }\n"
            "$p=Join-Path $r 'artifacts\\win-x64';New-Item -ItemType Directory -Force $p|Out-Null\n"
            "$f=Join-Path $p 'owned.bin';[IO.File]::WriteAllText($f,'fixture')\n"
            "$digest=(Get-FileHash $f -Algorithm SHA256).Hash\n"
            "if ($env:FIXTURE_BAD_HASH -eq '1') { $digest='BAD' }\n"
            "@{release_sequence=$ReleaseSequence;files=@(@{path='owned.bin';size=7;sha256=$digest})}|ConvertTo-Json -Depth 4|Set-Content (Join-Path $p 'NxTools.release.json')\n"
            "$i=Join-Path $r 'artifacts\\installer';New-Item -ItemType Directory -Force $i|Out-Null\n"
            "[IO.File]::WriteAllText((Join-Path $i 'NxTools.Next-setup-0.7.3.exe'),'fake setup - never execute')\n"
            "exit 0\n",
            encoding="ascii",
        )
        self.key = Path(self.temp.name) / "publisher with spaces" / "existing.pem"
        self.key.parent.mkdir()
        self.key.write_text("NOT A REAL KEY; CHECK MUST NOT PARSE THIS", encoding="ascii")
        self.ledger = Path(str(self.key) + ".release-sequence.json")
        self.ledger.write_text('{"sequence":29}', encoding="ascii")
        package = self.root / "artifacts/win-x64"
        package.mkdir(parents=True)
        self.manifest = package / "NxTools.release.json"
        self.manifest.write_text('{"release_sequence":28}', encoding="ascii")
        self.programs = Path(self.temp.name) / "programs"
        inno = self.programs / "Inno Setup 6"
        inno.mkdir(parents=True)
        (inno / "ISCC.exe").write_text("not executed", encoding="ascii")
        self.env = os.environ.copy()
        self.env.update(
            NXTOOLS_RELEASE_SIGNING_KEY=str(self.key),
            FIXTURE_HEADROOM="10",
            FIXTURE_BUSY="0",
            FIXTURE_FAIL="0",
            FIXTURE_BAD_HASH="0",
        )
        self.env["ProgramFiles(x86)"] = str(self.programs)
        self.env["ProgramFiles"] = str(self.programs)

    def run_entry(self, *args):
        return subprocess.run(
            ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
             str(self.root / "scripts/package-local.ps1"), *args],
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            env=self.env, timeout=30,
        )

    def status(self):
        paths = list((self.root / ".diagnostics/packaging").glob("*/status.json"))
        self.assertEqual(len(paths), 1)
        return json.loads(paths[0].read_text("utf-8-sig"))

    def test_check_only_no_reservation_or_build(self):
        before = (self.ledger.read_bytes(), self.manifest.read_bytes())
        result = self.run_entry("-CheckOnly")
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads(result.stdout)
        self.assertEqual(value["NextSequence"], 30)
        self.assertTrue(value["Ready"])
        self.assertFalse(value["SequenceReserved"])
        self.assertFalse(value["PrivateKeyContentsReadByCheck"])
        self.assertEqual(before, (self.ledger.read_bytes(), self.manifest.read_bytes()))
        self.assertFalse((self.root / "called.json").exists())
        self.assertFalse((self.root / ".diagnostics").exists())

    def test_package_floor_and_explicit_key_override(self):
        self.manifest.write_text('{"release_sequence":35}', encoding="ascii")
        self.env["NXTOOLS_RELEASE_SIGNING_KEY"] = "Z:\\missing.pem"
        result = self.run_entry("-CheckOnly", "-SigningKeyPath", str(self.key))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["NextSequence"], 36)

    def test_missing_key_and_ledger_reject_without_build(self):
        self.key.unlink()
        self.assertNotEqual(self.run_entry("-CheckOnly").returncode, 0)
        self.key.write_text("fake", encoding="ascii")
        self.ledger.unlink()
        self.assertNotEqual(self.run_entry("-CheckOnly").returncode, 0)
        self.assertFalse((self.root / "called.json").exists())

    def test_invalid_sequence_rejects(self):
        for value in [0, -1, "29", 29.5, None]:
            self.ledger.write_text(json.dumps({"sequence": value}), encoding="ascii")
            self.assertNotEqual(self.run_entry("-CheckOnly").returncode, 0, value)
        self.assertFalse((self.root / "called.json").exists())

    def test_busy_and_low_memory_do_not_build_or_reserve(self):
        before = self.ledger.read_bytes()
        for name in ["FIXTURE_BUSY", "FIXTURE_HEADROOM"]:
            self.env[name] = "1"
            check = self.run_entry("-CheckOnly")
            self.assertEqual(check.returncode, 0, check.stderr)
            self.assertFalse(json.loads(check.stdout)["Ready"])
            self.assertNotEqual(self.run_entry().returncode, 0)
            self.env[name] = "0" if name == "FIXTURE_BUSY" else "10"
        self.assertEqual(before, self.ledger.read_bytes())
        self.assertFalse((self.root / "called.json").exists())

    def test_success_delegates_default_verification_and_writes_hash(self):
        result = self.run_entry()
        self.assertEqual(result.returncode, 0, result.stderr)
        called = json.loads((self.root / "called.json").read_text("utf-8-sig"))
        self.assertEqual(called["sequence"], 30)
        self.assertEqual(called["configuration"], "Release")
        self.assertFalse(called["skip"])
        self.assertFalse(called["distribution"])
        status = self.status()
        self.assertEqual(status["state"], "Completed")
        self.assertFalse(status["installation_executed"])
        setup = self.root / "artifacts/installer/NxTools.Next-setup-0.7.3.exe"
        digest = hashlib.sha256(setup.read_bytes()).hexdigest().upper()
        self.assertEqual(status["installer_sha256"], digest)
        self.assertIn(digest, Path(str(setup) + ".sha256").read_text())
        self.assertTrue(list((self.root / ".diagnostics/packaging").glob("*/previous-package/NxTools.release.json")))

    def test_explicit_skip_is_forwarded(self):
        result = self.run_entry("-SkipVerification")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads((self.root / "called.json").read_text("utf-8-sig"))["skip"])

    def test_distribution_archive_requires_explicit_request(self):
        result = self.run_entry("-SkipVerification", "-CreateDistributionArchive")
        self.assertEqual(result.returncode, 0, result.stderr)
        called = json.loads((self.root / "called.json").read_text("utf-8-sig"))
        self.assertTrue(called["distribution"])
        self.assertTrue(called["skip"])
        self.assertTrue(self.status()["create_distribution_archive"])

    def test_failed_packager_is_not_success(self):
        self.env["FIXTURE_FAIL"] = "1"
        result = self.run_entry()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.status()["state"], "Failed")
        self.assertIn("exit 7", self.status()["failure"])

    def test_successful_native_stderr_does_not_fail_packaging(self):
        self.env["FIXTURE_STDERR"] = "1"
        result = self.run_entry()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.status()["state"], "Completed")
        logs = list((self.root / ".diagnostics/packaging").glob("*/package.stderr.log"))
        self.assertEqual(len(logs), 1)
        self.assertIn("Compiling fixture stderr", logs[0].read_text(errors="replace"))
        self.assertFalse(json.loads((self.root / "called.json").read_text("utf-8-sig"))["skip"])

    def test_native_stderr_retains_nonzero_child_exit(self):
        self.env["FIXTURE_STDERR"] = "1"
        self.env["FIXTURE_FAIL"] = "1"
        result = self.run_entry()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.status()["state"], "Failed")
        self.assertIn("exit 7", self.status()["failure"])

    def test_mismatched_member_rejects(self):
        self.env["FIXTURE_BAD_HASH"] = "1"
        self.assertNotEqual(self.run_entry().returncode, 0)
        self.assertEqual(self.status()["state"], "Failed")
        self.assertIn("hash/size mismatch", self.status()["failure"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
