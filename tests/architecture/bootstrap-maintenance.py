#!/usr/bin/env python3
"""Source wiring only; not administrator/service/installed acceptance."""
from pathlib import Path
root=Path(__file__).resolve().parents[2]
read=lambda p:(root/p).read_text('utf-8-sig')
core=read('csharp/NxTools.PlatformHost/Windows/TrustedBootstrapMaintenance.cs')
win=read('csharp/NxTools.PlatformHost/Windows/TrustedBootstrapMaintenanceWindows.cs')
broker=read('csharp/NxTools.PlatformHost/Windows/TrustedUpdateBroker.cs')
manifest=read('csharp/NxTools.PlatformHost/Windows/TrustedReleaseManifest.cs')
setup=read('installer/nxtools-maintenance.iss')
normal=read('installer/nxtools-next.iss')
program=read('csharp/NxTools.PlatformHost/Program.cs')
assert '--maintain-trusted-bootstrap' in program and '--maintain-trusted-bootstrap' in setup
assert 'EnsureBootstrapMigration' in normal and '-BootstrapMigrationPackage' in normal
assert 'PrivilegesRequired=admin' in setup and 'CreateAppDir=no' in setup and 'Uninstallable=no' in setup
assert 'WindowsBuiltInRole.Administrator' in win and 'NxToolsUpdateKeySha256' in win
assert 'TrustedDeploymentGate.Acquire()' in win and 'maintenance_ready' in win+broker
assert 'bootstrap-maintenance.lock' in core+broker and 'bootstrap_maintenance_busy' in broker
assert 'next.Sequence <= Math.Max(state.Sequence, previous?.Sequence ?? 0)' in core
assert 'VerifyRecoverySnapshot' in core and 'RequireStateUnchanged' in core
assert 'RecoveryOnly' in manifest and 'ValidateApplicationRelease' in manifest
assert 'MaintenanceProbe' in broker and 'RemoveSupersededArchiveFiles' in broker
assert 'RemoveSupersededArchiveFiles' in manifest and 'SetFileInformationByHandle' in manifest
assert 'TimeSpan.FromMinutes(4) + TimeSpan.FromSeconds(15)' in win
repair=read('csharp/NxTools.PlatformHost/Windows/TrustedMissingBootstrapRepair.cs')
assert '--repair-missing-trusted-bootstrap' in program
assert 'TrustedUpdateBroker.EnrollMissingRoot(package, sink, id)' in repair
repair_enrollment=broker.split('internal static void EnrollMissingRoot(',1)[1].split('private static void ProvisionEnrollment(',1)[0]
assert 'IsEnrolled()' not in repair_enrollment
assert 'RequireMissingRoot(Root)' in repair_enrollment and 'VerifyPackage(package, publicKey, 0)' in repair_enrollment
assert 'ProvisionEnrollment(package, publicKey, verified, log, correlationId, requireInitialDeployment: true)' in repair_enrollment
assert 'RequireMissingRoot(root)' in repair and 'operations.Stop()' in repair and 'operations.EnterDeploymentGate()' in repair
assert 'operations.CompleteDeployment(package, log, correlation)' in repair and 'TrustedUpdateBroker.RequestAsync(package, id' in repair
transaction=read('csharp/NxTools.PlatformHost/Windows/TrustedUpdateTransaction.cs')
assert 'previous.DeploymentPending' in transaction and 'initial_deployment_committed' in transaction
print('PASS explicit same-key bootstrap maintenance wiring / strict new admission / exact legacy recovery; source only')
