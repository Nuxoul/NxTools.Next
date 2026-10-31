#!/usr/bin/env python3
"""Phase F P206/P207 source ownership guard; not Registry, COM registration or Explorer acceptance."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
host = root / "csharp/NxTools.PlatformHost/Windows/Archive"
helper = root / "csharp/NxTools.ShellSelectionHelper"
tests = root / "csharp/NxTools.PlatformHost.Tests"


def read(path: Path) -> str:
    return path.read_text("utf-8-sig")


service = read(host / "ArchiveIntegrationService.cs")
store = read(host / "WindowsArchiveIntegrationStore.cs")
plan = read(host / "ArchiveIntegrationPlan.cs")
future = read(host / "ArchiveSelectionHandlerPlan.cs")
core = read(helper / "ShellSelectionCore.cs")
helper_host = read(helper / "ShellSelectionHost.cs")
helper_program = read(helper / "Program.cs")
helper_project = read(helper / "NxTools.ShellSelectionHelper.csproj")
package = read(root / "scripts/package.ps1")
build = read(root / "scripts/build.ps1")
adapter = read(root / "csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs")

# P206: real apply stays gated; one transaction owner; no whole-tree deletes or UserChoice.
assert "internal const bool ProductApplyAuthorized = false;" in service
assert "ArchiveIntegrationTransaction.Apply(_store, plan)" in service
assert "ArchiveIntegrationTransaction.Uninstall(_store, facts)" in service
assert service.count("class ArchiveIntegrationTransaction") == 0, "no second transaction framework"
assert "DeleteSubKeyTree" not in store and "DeleteSubKeyTree" not in service
assert "key.ValueCount != 0 || key.SubKeyCount != 0" in store, "owned key removal only when empty"
assert "UserChoice" in store, "store validation rejects UserChoice targets"
assert "Registry.LocalMachine" not in store + service + future
for token in ("Registry.CurrentUser", "Microsoft.Win32", "SHChangeNotify", "LocalServer32"):
    assert token not in plan, "pure P206 plan must stay pure: " + token
assert "ArchiveIntegrationService.CreateProduct(_log)" in adapter

# Tests never construct the real Registry store or product apply.
for name in ("ArchiveIntegrationServiceTests.cs", "ArchiveIntegrationPlanTests.cs", "ArchiveShellSelectionTests.cs"):
    text = read(tests / name)
    assert "new WindowsArchiveIntegrationStore(" not in text, name
    assert "CreateProduct(" not in text, name
    assert "CoRegisterClassObject(" not in text and "Registry.CurrentUser" not in text and "Registry.LocalMachine" not in text, name
    assert "LocalServer.Run(" not in text and "ProcessDesktopLauncher" not in text, name

# P207: dedicated x64 out-of-process helper with both interfaces and no coalescing timer.
assert "<PlatformTarget>x64</PlatformTarget>" in helper_project and "<OutputType>WinExe</OutputType>" in helper_project
assert "IExecuteCommand, IObjectWithSelection" in core
for iid in ("7F9185B0-CB92-43c5-80A9-92277A4F7B54", "1c9cd5bb-98e9-4491-a60f-31aacc72b83c",
            "b63ea76d-1f85-456f-a19c-48159efa858b", "43826d1e-e718-42ee-bc55-a1e261c37bfe"):
    assert iid in core, iid
assert "SigdnFileSysPath = 0x80058000" in core
assert "Interlocked.Exchange(ref _executed, 1) != 0" in core, "Execute forwards once"
for token in ("System.Threading.Timer", "Task.Delay", "Debounce", "Coalesce"):
    assert token not in core + helper_host, "no timer/coalescing: " + token
assert "IsComActivation(args)" in helper_program and 'Generated.DesktopReleaseIdentity.Sha256' in helper_program
assert "CoRegisterClassObject" in helper_host and "RegclsSuspended" in helper_host

# Deployment belongs to the existing release owner; registration remains separately gated.
assert "ShellSelectionHelper" in package and "ShellSelectionHelper" in build
assert 'Sha256 = ""' in read(helper / "Generated/DesktopIdentity.g.cs"), "unpackaged fallback refuses"
assert "ShellSelectionIdentityPath" in helper_project
for token in ("ArchiveSelectionHandlerPlan.Create(",):
    for source in (service, store, adapter):
        assert token not in source, "future COM registration plan must not be reachable from product apply"

print("PASS Phase F P206/P207 source ownership: gated HKCU apply, exact revert, dedicated x64 helper, original release owner")
print("Scope: source only; real Registry/COM registration/Explorer/deployment acceptance are separate and NOT RUN.")
