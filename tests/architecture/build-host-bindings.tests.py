"""Exercise verification wrapper bindings and prerequisites with tool stubs."""
import json
import os
from pathlib import Path
import re
import shutil
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
        self.assertEqual(len(commands), 3, "wire exports, Release capability gate, and unfiltered Debug suite required")
        base = os.environ.get("PI_SCRATCH_DIR") or tempfile.gettempdir()
        with tempfile.TemporaryDirectory(prefix="fixture modes ", dir=base) as scratch:
            captured = []
            for index, command in enumerate(commands):
                script = Path(scratch) / f"mode-{index}.ps1"
                script.write_text("$ErrorActionPreference='Stop'\n$testsProject='Tests.csproj'\n"
                                  "function dotnet { $script:captured=@($args) }\n" + command
                                  + "\nConvertTo-Json -InputObject $script:captured -Compress\n", encoding="utf-8-sig")
                result = subprocess.run(["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script)],
                                        capture_output=True, text=True, encoding="utf-8", timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
                captured.append(json.loads(result.stdout))
            _, release, debug = captured
            self.assertEqual(release[release.index("-c") + 1], "Release")
            self.assertEqual(release[release.index("--fixtures") + 1:],
                             ["archive HZ4 process fixture", "archive HZ4 TERM01"])
            self.assertEqual(debug[debug.index("-c") + 1], "Debug")
            # PowerShell consumes -- for a function stub, unlike a native executable.
            self.assertEqual(debug[-2:], ["--no-build", "--fixtures"], "full suite must not be filtered")
        for variable in ("WORKER_FIXTURE", "CLI_LIFECYCLE_EVIDENCE_ROOT", "VOLUME_FIXTURE_ROOT"):
            self.assertIn("$env:NXTOOLS_ARCHIVE_" + variable + " = $original", source,
                          "caller fixture environment must be restored")


RUST_FIXTURE_ENV = (
    "PI_SCRATCH_DIR", "TEMP", "TMP", "STORY_MD", "NXTOOLS_TAR_STREAM_FIXTURES",
    "NXTOOLS_TAR_STREAM_EVIDENCE_ROOT", "NXTOOLS_TAR_STREAM_HOST",
    "NXTOOLS_TAR_STREAM_CHAIN_CASES", "NXTOOLS_ARCHIVE_WORKER_FIXTURE",
    "NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE", "NXTOOLS_OPT03_CREATE_WIRE",
)

RUST_FIXTURE_TOOLS = r"""
function Initialize-LowMemoryRustBuildEnvironment { }
function Initialize-GpuiReleaseEnvironment { }
function Assert-ReleaseBuildCommitHeadroom { param($MinimumGiB) }
function Assert-CargoBuildLaneIdle { }
function Record-Step([string]$Step) {
    [IO.File]::AppendAllText($env:STUB_LOG, "$Step`n")
    $global:LASTEXITCODE = if ($env:STUB_FAIL -eq $Step) { 17 } else { 0 }
}
function python {
    if ($args[0] -like '*build-host-bindings.tests.py') { Record-Step 'bindings'; return }
    if ($args[0] -notlike '*prepare-tar-stream-safety-fixtures.py' -or
        $args[1] -ne '--root' -or $args[3] -ne '--oracle' -or $args[4] -cne $env:STUB_ORACLE) {
        throw 'unexpected synthetic fixture producer arguments'
    }
    if (-not (Test-Path -LiteralPath $args[2] -PathType Container) -or
        (Split-Path -Parent $args[2]) -ne $env:PI_SCRATCH_DIR) { throw 'producer requires existing owned evidence root' }
    $directory = New-Item -ItemType Directory -Path (Join-Path $args[2] 'fixtures')
    [IO.File]::WriteAllText((Join-Path $directory.FullName 'plain.tar'), 'synthetic')
    Record-Step 'tar-inputs'
}
function cargo {
    $worker = Join-Path (Get-Location).Path 'target\debug\nxtools-archive-worker.exe'
    if (($args -join ' ') -eq 'build --jobs 1 -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs') {
        New-Item -ItemType Directory -Path (Split-Path -Parent $worker) -Force | Out-Null
        [IO.File]::WriteAllText($worker, 'isolated worker')
        Record-Step 'debug-worker'
    } elseif ($args[0] -eq 'test') {
        if (($args -join ' ') -cne 'test --workspace --jobs 1') { throw 'workspace test scope changed' }
        New-Item -ItemType Directory -Path (Split-Path -Parent $worker) -Force | Out-Null
        [IO.File]::WriteAllText($worker, 'workspace worker')
        if ($env:STORY_MD -or $env:NXTOOLS_TAR_STREAM_CHAIN_CASES) { throw 'caller override reached workspace tests' }
        if ($env:TEMP -cne $env:PI_SCRATCH_DIR -or $env:TMP -cne $env:PI_SCRATCH_DIR -or
            $env:NXTOOLS_OPT03_CREATE_WIRE -cne $env:PI_SCRATCH_DIR -or
            $env:NXTOOLS_TAR_STREAM_FIXTURES -cne (Join-Path $env:NXTOOLS_TAR_STREAM_EVIDENCE_ROOT 'fixtures') -or
            -not (Test-Path -LiteralPath $env:NXTOOLS_TAR_STREAM_HOST -PathType Leaf) -or
            [IO.File]::ReadAllText($env:NXTOOLS_ARCHIVE_WORKER_FIXTURE) -ne 'isolated worker') {
            throw 'missing or stale workspace test prerequisites'
        }
        foreach ($path in @($env:NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE,
            (Join-Path $env:NXTOOLS_OPT03_CREATE_WIRE 'ordinary.zip.json'),
            (Join-Path $env:NXTOOLS_OPT03_CREATE_WIRE 'split.zip.json'),
            (Join-Path $env:NXTOOLS_OPT03_CREATE_WIRE 'split.7z.json'),
            (Join-Path $env:NXTOOLS_TAR_STREAM_FIXTURES 'plain.tar'))) {
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing prerequisite: $path" }
        }
        Record-Step 'workspace-test'
    } else { Record-Step ($args[0] + ':' + $args[1]) }
}
function dotnet {
    $testProject = '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj'
    $apphost = Join-Path (Get-Location).Path 'csharp\NxTools.PlatformHost.Tests\bin\Debug\net8.0-windows\NxTools.PlatformHost.Tests.exe'
    if (($args -join '|') -ceq "build|$testProject|-c|Debug") {
        New-Item -ItemType Directory -Path (Split-Path -Parent $apphost) -Force | Out-Null
        [IO.File]::WriteAllText($apphost, 'synthetic apphost - never execute')
        Record-Step 'debug-apphost'
    } elseif ($args -contains 'archive OPT01 AC repair production progress wire') {
        $expected = "run|--project|$testProject|-c|Debug|--no-build|--fixtures|archive OPT01 AC repair production progress wire|OPT03 create actual native result locations"
        if (($args -join '|') -cne $expected -or -not (Test-Path -LiteralPath $apphost -PathType Leaf) -or
            $env:NXTOOLS_OPT03_CREATE_WIRE -cne $env:PI_SCRATCH_DIR -or
            (Split-Path -Parent $env:NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE) -ne $env:PI_SCRATCH_DIR -or
            [IO.File]::ReadAllText($env:NXTOOLS_ARCHIVE_WORKER_FIXTURE) -ne 'isolated worker') {
            throw 'unexpected C# fixture inputs or selection'
        }
        foreach ($file in @('ordinary.zip.json', 'split.zip.json', 'split.7z.json')) {
            [IO.File]::WriteAllText((Join-Path $env:NXTOOLS_OPT03_CREATE_WIRE $file), '{}')
        }
        [IO.File]::WriteAllText($env:NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE, '{}')
        Record-Step 'wire-exports'
    } elseif ($args[0] -eq 'build') { Record-Step 'bound-apphost' }
    else { Record-Step 'release-gate' }
}
"""


class RustWorkspaceFixturePrerequisites(unittest.TestCase):
    def test_workspace_test_receives_current_owned_prerequisites(self):
        cases = (
            ("Debug", "", "explicit"), ("Release", "", "conventional"),
            ("Debug", "wire-exports", "explicit"), ("Debug", "workspace-test", "explicit"),
            ("Debug", "", "missing"),
        )
        for configuration, failure, mode in cases:
            with self.subTest(configuration=configuration, failure=failure, oracle=mode):
                self.run_build(configuration, failure, mode)

    def run_build(self, configuration, failure, mode):
        base = os.environ.get("PI_SCRATCH_DIR") or tempfile.gettempdir()
        with tempfile.TemporaryDirectory(prefix="workspace fixture gate ", dir=base) as scratch:
            area = Path(scratch)
            scripts = area / "project" / "scripts"
            scripts.mkdir(parents=True)
            shutil.copy2(ROOT / "scripts/build.ps1", scripts / "build.ps1")
            (scripts / "windows-build-tools.ps1").write_text(RUST_FIXTURE_TOOLS, encoding="utf-8-sig")
            for name, step in (("publish-release-hosts", "publish"), ("test-platformhost-fixtures", "full-fixtures")):
                (scripts / f"{name}.ps1").write_text(f"Record-Step '{step}'\n", encoding="utf-8-sig")
            gates = scripts.parent / "tests/architecture"
            gates.mkdir(parents=True)
            for name in ("quick-launch-lifecycle", "indexer-lifecycle", "desktop-shell-lifecycle",
                         "platformhost-test-selection", "browser-signing-isolation", "trusted-release-sequence",
                         "installer-exact-file-rollback"):
                (gates / f"{name}.ps1").touch()
            os_temp = area / "system temp"
            os_temp.mkdir()
            caller_scratch = area / "caller scratch"
            caller_scratch.mkdir()
            programs = area / "programs"
            oracle = programs / "7-Zip/7z.exe"
            oracle.parent.mkdir(parents=True)
            if mode != "missing":
                oracle.touch()
            original = {name: f"caller-{name}" for name in RUST_FIXTURE_ENV}
            original.update(TEMP=str(os_temp), TMP=str(os_temp),
                            PI_SCRATCH_DIR=None if mode == "conventional" else str(caller_scratch))
            environment = os.environ.copy()
            for name, value in original.items():
                if value is None:
                    environment.pop(name, None)
                else:
                    environment[name] = value
            environment.update(STUB_FAIL=failure, STUB_LOG=str(area / "steps.log"),
                               STUB_ORACLE=str(oracle), STUB_BUILD=str(scripts / "build.ps1"),
                               STUB_CONFIGURATION=configuration, STUB_PROGRAMS=str(programs))
            environment.pop("NXTOOLS_TAR_STREAM_ORACLE", None)
            if mode != "conventional":
                environment["NXTOOLS_TAR_STREAM_ORACLE"] = str(oracle)
            # Use only the copied wrapper and PowerShell function stubs, not machine build tools.
            # An empty command-search PATH makes conventional oracle discovery deterministic.
            shell = shutil.which("powershell.exe")
            self.assertIsNotNone(shell)
            environment["PATH"] = str(area)
            script = area / "driver.ps1"
            variable_list = ",".join(f"'{name}'" for name in RUST_FIXTURE_ENV)
            script.write_text(
                "$ErrorActionPreference='Stop'\n"
                "$env:ProgramFiles=$env:STUB_PROGRAMS\n"
                "try { & $env:STUB_BUILD -Configuration $env:STUB_CONFIGURATION; $failure=$null }\n"
                "catch { $failure=$_.Exception.Message }\n"
                "finally {\n"
                "  $after=@{}\n"
                f"  foreach ($name in @({variable_list})) {{ $after[$name]=[Environment]::GetEnvironmentVariable($name) }}\n"
                "  @{failure=$failure;environment=$after} | ConvertTo-Json -Compress\n"
                "}\n",
                encoding="utf-8-sig",
            )
            result = subprocess.run([shell, "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script)],
                                    cwd=area, env=environment, capture_output=True, text=True, encoding="utf-8",
                                    errors="replace", timeout=30)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            report = json.loads(result.stdout.splitlines()[-1])
            steps = (area / "steps.log").read_text().splitlines()
            self.assertEqual(report["environment"], original, report)
            self.assertEqual(list(caller_scratch.iterdir()), [])
            self.assertEqual(list(os_temp.iterdir()), [])
            if mode == "missing":
                self.assertIn("NXTOOLS_TAR_STREAM_ORACLE", report["failure"] or "")
                self.assertNotIn("workspace-test", steps)
                return
            prerequisites = ["tar-inputs", "debug-worker", "debug-apphost", "wire-exports", "workspace-test"]
            expected = prerequisites[:prerequisites.index(failure) + 1] if failure else prerequisites
            if "tar-inputs" not in steps:
                self.fail(f"TAR fixture stage did not run: {report}, {steps}")
            start = steps.index("tar-inputs")
            self.assertEqual(steps[start:start + len(expected)], expected, report)
            if failure:
                self.assertIn("exit code 17", report["failure"] or "")
                self.assertEqual(steps[start:], expected, "failed prerequisites/tests must stop verification")
            else:
                self.assertIsNone(report["failure"])
                tail = steps[start + len(expected):]
                if configuration == "Debug":
                    self.assertEqual(tail, ["build:--jobs", "debug-worker", "publish", "bound-apphost", "full-fixtures"])
                else:
                    self.assertEqual(tail, ["build:--release", "build:--release", "publish", "bound-apphost",
                                            "release-gate", "full-fixtures"])


if __name__ == "__main__":
    unittest.main()
