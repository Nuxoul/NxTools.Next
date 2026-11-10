from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
source_path = root / "csharp" / "NxTools.PlatformHost" / "Windows" / "TrustedUninstallCleanup.cs"
program = (root / "csharp" / "NxTools.PlatformHost" / "Program.cs").read_text(encoding="utf-8")
installer = (root / "installer" / "nxtools-next.iss").read_text(encoding="utf-8")

assert source_path.exists(), "missing production privileged uninstall owner"
source = source_path.read_text(encoding="utf-8")
for required in (
    "TrustedReleaseManifest.VerifyPackage",
    "TrustedBinary.Open",
    "TrustedDeploymentGate.Acquire",
    "transaction.json",
    "bootstrap-maintenance.json",
    "StopProtectedProcesses",
    "IndexerServiceInstaller.ServiceName",
    "TrustedUpdateBroker.ServiceName",
    "RootExists",
    "ValidateExactTree(serviceRoot, release.Files)",
    "ValidateUpdaterRegistration",
    "ValidateUpdaterRoot",
    "exact updater registration with the whole fixed root absent",
    "currentServiceRootExists",
):
    assert required in source, f"missing protected uninstall boundary: {required}"
tests = (root / "csharp" / "NxTools.PlatformHost.Tests" / "TrustedUninstallCleanupTests.cs").read_text(encoding="utf-8")
for required in (
    "second real Run removes signed exact-owned deployment remainders",
    "registration-free remainder with foreign file is retained",
    "registration-free remainder with recovery record is retained",
    "exact updater registration with whole root absent is removed",
    "foreign missing-root updater tuple is retained without effects",
):
    assert required in tests, f"missing protected uninstall fixture: {required}"
assert re.search(r'args\.Length > 0 && args\[0\] == "--uninstall-protected-components"', program)
assert "TrustedUninstallCleanup.Run" in program
assert "--uninstall-protected-components" in installer
assert installer.index("--uninstall-protected-components") < installer.index("RunUninstallCleanup();", installer.index("procedure CurUninstallStepChanged"))
print("protected uninstall source entry: passed")
