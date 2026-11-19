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

# P206: explicit product Apply/Revert is enabled; native identity/ownership checks remain mandatory.
assert "internal const bool ProductApplyAuthorized = true;" in service
assert "ArchiveIntegrationTransaction.Apply(_store, plan, cancellation)" in service
assert "ProductApplyAuthorized, approveSelectionHelper" in service, "fixture and native composition share product policy"
assert "IndexerServiceInstaller.OpenArchiveRelease" in service, "original retained release approval remains"
assert "selection_helper_identity_unavailable" in service and "desktop_identity_unavailable" in service
assert "BoundedStaOperationRunner.TryRun" in service and "native_operation_unconfirmed" in service
assert "service.ExecuteBounded(command, correlationId, cancellation)" in adapter
dispatcher = read(root / "csharp/NxTools.PlatformHost/Protocol/PlatformCommandSequenceDispatcher.cs")
assert '"archive_shell_integration",' in dispatcher, "native work may not block the serial reader"
assert "ArchiveIntegrationTransaction.Uninstall(_store, facts)" in service
assert service.count("class ArchiveIntegrationTransaction") == 0, "no second transaction framework"
assert "DeleteSubKeyTree" not in store and "DeleteSubKeyTree" not in service
assert "key.ValueCount != 0 || key.SubKeyCount != 0" in store, "owned key removal only when empty"
assert "UserChoice" in store, "store validation rejects UserChoice targets"
# Apply replaces an earlier revision through the same exact uninstall; no manual revert step.
assert "ArchiveIntegrationTransaction.Uninstall(_store, existing)" in service
assert "ArchiveShellIntegrationStates.RevertRequired, command.DesiredRevision" not in service
# The Explorer choice is written only by the explicit default claim, after facts (including the
# original choices) persist, and is read back through the live association before success.
claim = read(host / "WindowsArchiveDefaultClaim.cs")
proof = read(host / "UserChoiceHash.cs")
assert "command.SetDefault && _defaultClaim is not null" in service
assert service.index("PriorUserChoices = priors") < service.index("_ledger.Save(facts)") < service.index("_defaultClaim.Set(extension, ArchiveIntegrationPlan.ProgId)")
assert "RestoreChoices(facts, [], correlationId)" in service, "Revert writes the original choice back"
assert "AssocQueryString" in claim and "RegQueryInfoKey" in claim and "DeleteSubKeyTree" not in claim
assert "Microsoft.Win32" not in proof and "File.Read" not in proof, "proof computation stays pure"
assert "Software\\Microsoft" not in plan, "pure value plan stays under Software\\Classes; the Explorer choice has its own owner"
assert "Registry.LocalMachine" not in store + service + future
for token in ("Registry.CurrentUser", "Microsoft.Win32", "SHChangeNotify", "LocalServer32"):
    assert token not in plan, "pure P206 plan must stay pure: " + token
assert "ArchiveIntegrationService.CreateProduct(_log)" in adapter

# Fixtures inject effects at the same product composition; never call its native one-argument factory.
for name in ("ArchiveIntegrationServiceTests.cs", "ArchiveIntegrationPlanTests.cs", "ArchiveShellSelectionTests.cs", "ArchiveAssociationSettingsTests.cs"):
    text = read(tests / name)
    assert "new WindowsArchiveIntegrationStore(" not in text, name
    assert "CreateProduct(NullPlatformLog.Instance)" not in text and "CreateProduct(log)" not in text, name
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

# Deployment stays with the original release owner; only explicit approved requests register values.
assert "ShellSelectionHelper" in package and "ShellSelectionHelper" in build
assert 'Sha256 = ""' in read(helper / "Generated/DesktopIdentity.g.cs"), "unpackaged fallback refuses"
assert "ShellSelectionIdentityPath" in helper_project
assert "ArchiveSelectionHandlerPlan.Create(selectionHelperPath, desiredRevision, normalized)" in plan
assert "ArchiveSelectionHandlerPlan.HelperExecutableName" in service
assert "ProductCompositionAppliesOnlyExplicitRequests" in read(tests / "ArchiveIntegrationServiceTests.cs")

print("PASS Phase F P206/P207 source ownership: explicit approved HKCU apply, bounded STA, exact revert, original release owner")
print("Scope: source only; real Registry/COM registration/Explorer/deployment acceptance are separate and NOT RUN.")
