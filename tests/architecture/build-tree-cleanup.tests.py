"""Run unchanged Windows wrappers in owned fixture trees with all build/system tools stubbed."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]

TOOLS = r"""
function Initialize-LowMemoryRustBuildEnvironment {
    $env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
}
function Initialize-GpuiReleaseEnvironment { }
function Assert-CargoBuildLaneIdle { }
function Get-ReleaseBuildCommitHeadroomGiB { return 100.0 }
function Assert-ReleaseBuildCommitHeadroom { param($MinimumGiB) Fail-Step 'preflight' }
function Fail-Step([string]$Step) {
    [IO.File]::AppendAllText($env:CLEANUP_TOOL_LOG, $Step + "`n")
    if ($env:CLEANUP_FAIL -eq $Step) { throw "fixture-failure:$Step" }
}
function Native-Failed([string]$Step) {
    if ($env:CLEANUP_NATIVE_FAIL -eq $Step) { $global:LASTEXITCODE=17; return $true }
    return $false
}
function Assert-PackageTreeAlive([string]$Step) {
    if (-not (Test-Path -LiteralPath $env:CLEANUP_PACKAGE_TREE -PathType Container)) { throw "premature-build-tree-cleanup:$Step" }
    [IO.File]::AppendAllText($env:CLEANUP_TOOL_LOG, "alive:$Step`n")
}
if ($env:CLEANUP_INSTALLER -eq '1') {
    function global:powershell.exe {
        Assert-PackageTreeAlive 'installer-runtime'
        $global:LASTEXITCODE=0
    }
    foreach ($directory in @('SysWOW64','System32')) {
        $shell = Join-Path $env:SystemRoot ($directory + '\WindowsPowerShell\v1.0\powershell.exe')
        Set-Item -LiteralPath ('Function:\global:' + $shell) -Value {
            Assert-PackageTreeAlive 'native-shell-gate'
            $global:LASTEXITCODE=0
        }
    }
    Set-Item -LiteralPath ('Function:\global:' + $env:CLEANUP_ISCC_PATH) -Value {
        Assert-PackageTreeAlive 'iscc'
        Fail-Step 'iscc'
        [IO.Directory]::CreateDirectory((Split-Path -Parent $env:CLEANUP_INSTALLER_OUTPUT)) | Out-Null
        [IO.File]::WriteAllText($env:CLEANUP_INSTALLER_OUTPUT, 'synthetic installer - never execute')
        $global:LASTEXITCODE=0
    }
}
function cargo {
    Fail-Step 'cargo'
    if (Native-Failed 'cargo') { return }
    $global:LASTEXITCODE=0
}
function cmd.exe {
    $step = if (($args -join ' ') -match 'nxtools-archive-worker') { 'cargo-archive' } else { 'cargo-product' }
    Fail-Step $step
    if (Native-Failed $step) { return }
    $global:LASTEXITCODE=0
}
function python {
    if (($args -join ' ') -match 'package-archive-source.py') {
        if ($args -contains '--installer') {
            Assert-PackageTreeAlive 'distribution'
            Fail-Step 'distribution'
        } else { Fail-Step 'source' }
        $index = [Array]::IndexOf($args, '--output')
        $path = $args[$index + 1]
        [IO.Directory]::CreateDirectory((Split-Path -Parent $path)) | Out-Null
        [IO.File]::WriteAllText($path, 'synthetic source evidence')
        if (Native-Failed 'source') { return }
    } else { Fail-Step 'python-gate' }
    $global:LASTEXITCODE=0
}
function dotnet {
    $arguments = @($args)
    if ($arguments[0] -eq 'publish') {
        $artifacts = $arguments[[Array]::IndexOf($arguments, '--artifacts-path') + 1]
        $output = $arguments[[Array]::IndexOf($arguments, '-o') + 1]
        [IO.Directory]::CreateDirectory($artifacts) | Out-Null
        [IO.File]::WriteAllText((Join-Path $artifacts 'owned-obj.bin'), 'temporary compilation tree')
        [IO.Directory]::CreateDirectory($output) | Out-Null
        $helper = ($arguments -join ' ') -match 'ShellSelectionHelper.csproj'
        if ($helper) {
            foreach ($name in @('NxTools.ShellSelectionHelper.exe','NxTools.ShellSelectionHelper.dll','NxTools.ShellSelectionHelper.deps.json','NxTools.ShellSelectionHelper.runtimeconfig.json')) {
                [IO.File]::WriteAllText((Join-Path $output $name), "synthetic $name")
            }
        } else {
            [IO.File]::WriteAllText((Join-Path $output 'NxTools.PlatformHost.exe'), 'synthetic host')
            $configuration = $arguments[[Array]::IndexOf($arguments, '-c') + 1].ToLowerInvariant()
            $assembly = Join-Path $artifacts ('bin\NxTools.PlatformHost\' + $configuration + '_win-x64\NxTools.PlatformHost.dll')
            [IO.Directory]::CreateDirectory((Split-Path -Parent $assembly)) | Out-Null
            [IO.File]::WriteAllText($assembly, 'synthetic managed host metadata')
        }
        $step = if ($helper) { 'helper' } else { 'host' }
        if ($env:CLEANUP_FAIL -eq 'debug-host' -and $arguments -contains 'Debug' -and -not $helper) { Fail-Step 'debug-host' }
        [IO.File]::AppendAllText($env:CLEANUP_TOOL_LOG, "publish-root:$([IO.Path]::GetFullPath((Split-Path -Parent $output)))`n")
        Fail-Step $step
        if (Native-Failed $step) { return }
    } elseif ($arguments[0] -eq 'build') {
        # The generated inputs must still exist until the consuming test build returns.
        foreach ($argument in $arguments) {
            if ($argument -match '^-p:(ReleaseBinaryManifestPath|ArchiveReleaseManifestPath|ShellSelectionIdentityPath)=(.+)$') {
                if (-not (Test-Path -LiteralPath $Matches[2] -PathType Leaf)) { throw 'missing-live-generated-input' }
            }
        }
        Fail-Step 'test-build'
        if (Native-Failed 'test-build') { return }
    } elseif ($arguments -contains 'Release') {
        Fail-Step 'release-gate'
        if (Native-Failed 'release-gate') { return }
    } else {
        foreach ($variable in @('NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT','NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT')) {
            $path = [Environment]::GetEnvironmentVariable($variable)
            [IO.Directory]::CreateDirectory($path) | Out-Null
            [IO.File]::WriteAllText((Join-Path $path 'fixture-evidence.json'), '{"synthetic":true}')
        }
        Fail-Step 'fixtures'
        if (Native-Failed 'fixtures') { return }
    }
    $global:LASTEXITCODE=0
}
function Get-CimInstance { return $null }
function Start-Service { throw 'forbidden-real-service' }
function Stop-Service { throw 'forbidden-real-service' }
function Get-Service { throw 'forbidden-real-service' }
function Stop-Process { throw 'forbidden-real-process-operation' }
function Wait-Process { throw 'forbidden-real-process-operation' }
function New-Item {
    [CmdletBinding()]param([string]$ItemType, [string]$Path, [switch]$Force)
    $created = Microsoft.PowerShell.Management\New-Item @PSBoundParameters
    if ($Path -match 'package-build-[^\\]+\\generated$') {
        $env:CLEANUP_PACKAGE_TREE = Split-Path -Parent $Path
        Fail-Step 'directory'
    }
    $created
}
"""

SCRIPTS = {
    "write-shell-selection-identity.ps1": r"""
param([string]$DesktopPath,[string]$OutputPath)
[IO.File]::WriteAllText($OutputPath, '// synthetic desktop identity')
Fail-Step 'identity'
""",
    "write-release-binary-manifest.ps1": r"""
param([string]$Profile,[string]$TargetDirectory,[string]$ShellHelperDirectory,[string]$OutputPath,[string]$ArchiveOutputPath)
if (-not (Test-Path (Join-Path $ShellHelperDirectory 'NxTools.ShellSelectionHelper.dll'))) { throw 'missing-published-helper' }
[IO.File]::WriteAllText($OutputPath, '// synthetic binary approval')
[IO.File]::WriteAllText($ArchiveOutputPath, '// synthetic archive approval')
Fail-Step 'manifest'
""",
    "new-release-signing-key.ps1": r"""
param([string]$KeyPath)
[IO.File]::WriteAllText($KeyPath, 'synthetic-owned-key-not-a-private-key')
Fail-Step 'key'
""",
    "write-update-trust.ps1": r"""
param([string]$SigningKeyPath,[string]$OutputPath)
if (-not $OutputPath) { $OutputPath=Join-Path (Split-Path -Parent $PSScriptRoot) 'public-trust.g.cs' }
[IO.File]::WriteAllText($OutputPath, '// synthetic public trust')
Fail-Step 'trust'
""",
    "reserve-release-sequence.ps1": r"""
param([string]$SigningKeyPath,[long]$ReleaseSequence,[string]$PackageDirectory)
Fail-Step 'reserve'
""",
    "write-trusted-release-manifest.ps1": r"""
param([string]$PackageDirectory,[string]$SigningKeyPath,[long]$ReleaseSequence)
[IO.File]::WriteAllText((Join-Path $PackageDirectory 'NxTools.release.json'), '{"synthetic":true}')
[IO.File]::WriteAllText((Join-Path $PackageDirectory 'NxTools.release.sig'), 'synthetic signature')
Fail-Step 'sign'
""",
}


def ps_quote(value):
    return "'" + str(value).replace("'", "''") + "'"


class WrapperFixture:
    def __init__(self, mode, failure="", custom_evidence=False, installer=False):
        self.temp = tempfile.TemporaryDirectory(prefix="build-tree-fixture-", dir=os.environ.get("PI_SCRATCH_DIR"))
        self.area = Path(self.temp.name)
        self.root = self.area / "project with spaces"
        self.scripts = self.root / "scripts"
        self.scripts.mkdir(parents=True)
        for name in ("build.ps1", "test-platformhost-fixtures.ps1", "package.ps1", "publish-release-hosts.ps1"):
            shutil.copy2(ROOT / "scripts" / name, self.scripts / name)
        (self.scripts / "windows-build-tools.ps1").write_text(TOOLS, encoding="utf-8-sig")
        for name, content in SCRIPTS.items():
            (self.scripts / name).write_text(content, encoding="utf-8-sig")
        gates = self.root / "tests/architecture"
        gates.mkdir(parents=True)
        for name in ("quick-launch-lifecycle", "indexer-lifecycle", "desktop-shell-lifecycle",
                     "platformhost-test-selection", "browser-signing-isolation", "trusted-release-sequence",
                     "installer-exact-file-rollback", "desktop-release-subsystem", "desktop-product-icon", "bundled-extensions"):
            # Installer failure is after package.ps1's inner publication finally.
            (gates / (name + ".ps1")).write_text(
                "param([string]$DesktopExecutable)\n"
                + ("if ($env:CLEANUP_INSTALLER -eq '1') { Assert-PackageTreeAlive 'installer-gate' }; Fail-Step 'installer'\n" if name == "trusted-release-sequence" else ""), encoding="utf-8-sig")
        for profile in ("debug", "release"):
            target = self.root / "target" / profile
            target.mkdir(parents=True)
            for member in ("desktop", "indexer", "cli", "browser-host", "archive-worker"):
                (target / f"nxtools-{member}.exe").write_text("synthetic product - never execute", encoding="ascii")
        if failure == "copy":
            (self.root / "target/release/nxtools-desktop.exe").unlink()
        extension = self.root / "browser_extension/nxtools-interceptor"
        extension.mkdir(parents=True)
        (extension / "manifest.json").write_text("{}")
        for name in ("LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY_NOTICES.md"):
            (self.root / name).write_text("synthetic license evidence")
        self.scratch = self.area / "scratch"
        self.os_temp = self.area / "os temp"
        self.scratch.mkdir()
        self.os_temp.mkdir()
        self.owned_base = self.scratch if mode == "scratch" else self.os_temp
        self.foreign = []
        for prefix in ("vh-", "vf-", "package-build-"):
            directory = self.owned_base / (prefix + "foreign-owner")
            directory.mkdir()
            marker = directory / "keep.txt"
            marker.write_text("foreign owner")
            self.foreign.append(marker)
        self.caller_host = self.owned_base / "caller-host"
        self.caller_host.mkdir()
        (self.caller_host / "keep.txt").write_text("caller-owned input")
        self.key = self.area / "publisher-material" / "existing.pem"
        self.key.parent.mkdir()
        self.key.write_text("synthetic existing key - never parsed")
        self.ledger = Path(str(self.key) + ".release-sequence.json")
        self.ledger.write_text('{"sequence":29}')
        self.original_material = self.key.read_bytes(), self.ledger.read_bytes()
        self.env = os.environ.copy()
        self.env.update(TEMP=str(self.os_temp), TMP=str(self.os_temp), CLEANUP_FAIL=failure,
                        CLEANUP_TOOL_LOG=str(self.area / "tools.log"), NXTOOLS_RELEASE_SIGNING_KEY="")
        self.env["PI_SCRATCH_DIR"] = str(self.scratch) if mode == "scratch" else ""
        self.evidence_env = {}
        for variable in ("NXTOOLS_ARCHIVE_WORKER_FIXTURE", "NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT", "NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT"):
            value = str(self.area / variable) if custom_evidence else ""
            self.env[variable] = value
            self.evidence_env[variable] = value
        programs = self.area / "programs"
        programs.mkdir()
        self.env["ProgramFiles"] = self.env["ProgramFiles(x86)"] = str(programs)
        self.env["CLEANUP_NATIVE_FAIL"] = ""
        self.env["CLEANUP_PACKAGE_TREE"] = ""
        self.env["CLEANUP_INSTALLER"] = "0"
        if installer or failure == "installer":
            inno = programs / "Inno Setup 6"
            inno.mkdir()
            tool = inno / "ISCC.exe"
            tool.write_text("fixture marker - never execute")
            self.env["CLEANUP_INSTALLER"] = "1"
            self.env["CLEANUP_ISCC_PATH"] = str(tool)
            self.env["CLEANUP_INSTALLER_OUTPUT"] = str(self.root / "artifacts/installer/NxTools.Next-setup-0.7.2.exe")
            installer_script = self.root / "installer/nxtools-next.iss"
            installer_script.parent.mkdir()
            installer_script.write_text("synthetic compiler input - only tool stubs consume this")

    def close(self):
        self.temp.cleanup()

    def run(self, name, arguments):
        status = self.area / "result.json"
        script = self.area / "run.ps1"
        script.write_text(
            "$ErrorActionPreference='Stop'\n"
            f". {ps_quote(self.scripts / 'windows-build-tools.ps1')}\n"
            "$before=(Get-Location).Path; $succeeded=$false; $failure=''\n"
            "try { & " + ps_quote(self.scripts / name) + " " + arguments
            + "; $succeeded=$true } catch { $failure=$_.Exception.Message }\n"
            "$environment=@{}\n"
            "foreach ($name in @('NXTOOLS_ARCHIVE_WORKER_FIXTURE','NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT','NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT')) { $environment[$name]=[Environment]::GetEnvironmentVariable($name) }\n"
            f"@{{succeeded=$succeeded;failure=$failure;location_restored=((Get-Location).Path -eq $before);environment=$environment}} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath {ps_quote(status)} -Encoding UTF8\n",
            encoding="utf-8-sig",
        )
        process = subprocess.run(["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script)],
                                 env=self.env, cwd=self.root, capture_output=True, text=True,
                                 encoding="utf-8", errors="replace", timeout=30)
        if process.returncode != 0 or not status.exists():
            raise AssertionError(process.stdout + process.stderr)
        return json.loads(status.read_text("utf-8-sig"))

    def remaining(self):
        return sorted(str(path.name) for path in self.owned_base.iterdir()
                      if path.is_dir() and path.name.startswith(("vh-", "vf-", "package-build-"))
                      and not path.name.endswith("foreign-owner"))


@unittest.skipUnless(os.name == "nt", "Windows wrapper boundary")
class BuildTreeCleanupTests(unittest.TestCase):
    def fixture(self, mode, failure="", custom_evidence=False, installer=False):
        fixture = WrapperFixture(mode, failure, custom_evidence, installer)
        self.addCleanup(fixture.close)
        return fixture

    def assert_common(self, fixture, result, failure, expected_error=None):
        self.assertEqual(result["succeeded"], not bool(failure or expected_error), result["failure"])
        if expected_error:
            self.assertIn(expected_error, result["failure"])
        elif failure:
            self.assertIn("fixture-failure:" + failure if failure != "copy" else "nxtools-desktop.exe", result["failure"])
        self.assertTrue(result["location_restored"])
        self.assertEqual((fixture.key.read_bytes(), fixture.ledger.read_bytes()), fixture.original_material)
        for marker in fixture.foreign:
            self.assertEqual(marker.read_text(), "foreign owner")
        self.assertEqual((fixture.caller_host / "keep.txt").read_text(), "caller-owned input")
        self.assertEqual(fixture.remaining(), [], "only the exact invocation-owned build trees must be removed")

    def test_build_owner_success_and_downstream_failures(self):
        for mode in ("scratch", "temp"):
            for configuration, failure in [("Debug", ""), ("Release", ""), *[("Release", step) for step in (
                    "identity", "helper", "manifest", "host", "test-build", "release-gate", "debug-host", "fixtures")]]:
                with self.subTest(mode=mode, configuration=configuration, failure=failure):
                    fixture = self.fixture(mode, failure)
                    result = fixture.run("build.ps1", "-Configuration " + configuration)
                    self.assert_common(fixture, result, failure)
                    self.assertEqual({key: value or "" for key, value in result["environment"].items()}, fixture.evidence_env)
                    if not failure or failure == "fixtures":
                        evidence = list((fixture.root / ".diagnostics/verification").glob("*/fixture-evidence.json"))
                        self.assertTrue(evidence, "fixture evidence must survive owner tree cleanup")

    def test_fixture_owner_cleans_only_its_debug_tree_and_restores_environment(self):
        for mode in ("scratch", "temp"):
            for failure in ("", "identity", "helper", "manifest", "host", "test-build", "fixtures"):
                with self.subTest(mode=mode, failure=failure):
                    fixture = self.fixture(mode, failure, custom_evidence=True)
                    result = fixture.run("test-platformhost-fixtures.ps1", "-Configuration Release -HostBuildRoot " + ps_quote(fixture.caller_host))
                    self.assert_common(fixture, result, failure)
                    self.assertEqual(result["environment"], fixture.evidence_env)
                    if not failure or failure == "fixtures":
                        for variable in ("NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT", "NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT"):
                            self.assertTrue((Path(fixture.evidence_env[variable]) / "fixture-evidence.json").is_file())

    def test_package_owner_covers_setup_through_installer_failure(self):
        for mode in ("scratch", "temp"):
            for failure in ("", "directory", "reserve", "trust", "preflight", "cargo-product", "cargo-archive",
                            "identity", "helper", "manifest", "host", "copy", "source", "sign", "installer"):
                with self.subTest(mode=mode, failure=failure):
                    fixture = self.fixture(mode, failure)
                    destination = fixture.area / "retained-package"
                    arguments = "-Configuration Release -SkipVerification -OutputPath " + ps_quote(destination)
                    if failure != "installer":
                        arguments += " -SkipInstaller"
                    arguments += " -SigningKeyPath " + ps_quote(fixture.key) + " -ReleaseSequence 30"
                    result = fixture.run("package.ps1", arguments)
                    self.assert_common(fixture, result, failure)
                    if not failure or failure == "installer":
                        self.assertTrue((destination / "NxTools.PlatformHost.exe").is_file())
                        self.assertTrue((destination / "NxTools.ShellSelectionHelper.dll").is_file())
                        self.assertTrue((destination / "NxTools.release.sig").is_file())
                        self.assertTrue((fixture.root / "artifacts/installer/third_party/archive-worker/archive-worker-source.zip").is_file())

    def test_staging_public_input_references_survive_without_private_key_or_build_tree(self):
        for mode in ("scratch", "temp"):
            for failure in ("", "key", "trust", "preflight", "host", "source", "sign"):
                with self.subTest(mode=mode, failure=failure):
                    fixture = self.fixture(mode, failure)
                    destination = fixture.owned_base / "stage"
                    result = fixture.run("package.ps1", "-Configuration Release -SkipVerification -SkipInstaller -IsolatedStaging -OutputPath " + ps_quote(destination))
                    log = (fixture.area / "tools.log").read_text("utf-8-sig")
                    self.assertNotIn("reserve\n", log)
                    if not failure:
                        metadata = json.loads(Path(str(destination) + ".build-inputs.json").read_text("utf-8-sig"))
                        self.assertTrue(metadata["test_only"])
                        self.assertFalse(metadata["production_sequence_reserved"])
                        self.assertNotIn("build_directory", metadata, "temporary directory is not a delivered input")
                        generated = Path(metadata["generated_directory"])
                        self.assertTrue(generated.is_dir())
                        for name in ("DesktopIdentity.g.cs", "ReleaseBinaryManifest.g.cs", "ArchiveReleaseManifest.g.cs", "UpdateTrust.g.cs"):
                            self.assertTrue((generated / name).is_file(), name)
                        self.assertFalse(list(generated.rglob("*.pem")))
                        self.assertTrue(Path(str(destination) + ".TEST-public-trust.g.cs").is_file())
                        assembly = Path(metadata["host_assembly"])
                        self.assertEqual(assembly.read_text(), "synthetic managed host metadata")
                        self.assertNotIn("package-build-", str(assembly))
                    self.assert_common(fixture, result, failure)

    def test_native_failure_codes_and_nested_verification_unwind(self):
        cases = (
            ("build.ps1", "helper", "ShellSelectionHelper publish failed: 17", False),
            ("build.ps1", "test-build", "dotnet build failed with exit code 17", False),
            ("build.ps1", "release-gate", "C# Release fixture capability gates failed with exit code 17", False),
            ("test-platformhost-fixtures.ps1", "fixtures", "C# isolated fixture tests failed with exit code 17", False),
            ("package.ps1", "cargo-product", "cargo product build failed: 17", False),
            ("package.ps1", "cargo-archive", "cargo build failed with exit code 17", False),
            ("package.ps1", "source", "Archive source distribution failed", False),
            ("package.ps1", "fixtures", "C# isolated fixture tests failed with exit code 17", True),
        )
        for mode in ("scratch", "temp"):
            for name, step, expected, verify in cases:
                with self.subTest(mode=mode, script=name, step=step, verify=verify):
                    fixture = self.fixture(mode)
                    fixture.env["CLEANUP_NATIVE_FAIL"] = step
                    arguments = "-Configuration Release"
                    if name == "test-platformhost-fixtures.ps1":
                        arguments += " -HostBuildRoot " + ps_quote(fixture.caller_host)
                    elif name == "package.ps1":
                        arguments += " -SkipInstaller -OutputPath " + ps_quote(fixture.area / "package")
                        arguments += " -SigningKeyPath " + ps_quote(fixture.key) + " -ReleaseSequence 30"
                        if not verify:
                            arguments += " -SkipVerification"
                    result = fixture.run(name, arguments)
                    self.assert_common(fixture, result, "", expected_error=expected)
                    self.assertEqual({key: value or "" for key, value in result["environment"].items()}, fixture.evidence_env)

    def test_staging_consumer_uses_retained_managed_host(self):
        source = (ROOT / "csharp/NxTools.PlatformHost.Tests/ArchiveReleaseTests.cs").read_text("utf-8-sig")
        staged = source.split("internal static Task StagedActualIdentities()", 1)[1].split("private static string Hash", 1)[0]
        self.assertIn('GetProperty("host_assembly")', staged)
        self.assertNotIn('GetProperty("build_directory")', staged)
        self.assertNotIn('"host-build"', staged)
        self.assertIn('context.LoadFromAssemblyPath(hostDll)', staged)
        self.assertIn('TrustedReleaseManifest.VerifyPackage(full, publicKey, 0)', staged)
        self.assertIn('GetField("readHelper"', staged)

    def test_package_tree_lives_through_iscc_and_final_distribution(self):
        for mode in ("scratch", "temp"):
            for failure in ("", "iscc", "distribution"):
                with self.subTest(mode=mode, failure=failure):
                    fixture = self.fixture(mode, failure, installer=True)
                    destination = fixture.area / "package-with-installer"
                    result = fixture.run("package.ps1", "-Configuration Release -SkipVerification -OutputPath " + ps_quote(destination)
                                         + " -SigningKeyPath " + ps_quote(fixture.key) + " -ReleaseSequence 30 -CreateDistributionArchive")
                    self.assert_common(fixture, result, failure)
                    log = (fixture.area / "tools.log").read_text("utf-8-sig")
                    self.assertIn("alive:installer-gate", log)
                    self.assertIn("alive:native-shell-gate", log)
                    self.assertIn("alive:installer-runtime", log)
                    self.assertIn("alive:iscc", log)
                    if failure != "iscc":
                        self.assertIn("alive:distribution", log)
                        self.assertTrue((fixture.root / "artifacts/installer/NxTools.Next-setup-0.7.2.exe").is_file())
                    self.assertTrue((destination / "NxTools.release.sig").is_file())

    def test_publisher_borrows_outputs_until_its_caller_consumes_them(self):
        for failure in ("", "helper", "manifest", "host"):
            with self.subTest(failure=failure):
                fixture = self.fixture("scratch", failure)
                generated = fixture.caller_host / "generated"
                result = fixture.run("publish-release-hosts.ps1", "-Configuration Release -BuildDirectory " + ps_quote(fixture.caller_host)
                                     + " -GeneratedDirectory " + ps_quote(generated))
                self.assert_common(fixture, result, failure)
                self.assertTrue((generated / "DesktopIdentity.g.cs").is_file())
                self.assertTrue((fixture.caller_host / "shell-publish/NxTools.ShellSelectionHelper.dll").is_file())
                if not failure:
                    self.assertTrue((fixture.caller_host / "host-publish/NxTools.PlatformHost.exe").is_file())


if __name__ == "__main__":
    unittest.main(verbosity=2)
