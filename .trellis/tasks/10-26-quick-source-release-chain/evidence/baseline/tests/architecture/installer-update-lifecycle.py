"""Source-only installer ownership gate; never installs or operates a process/service."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
source = (root / 'installer/nxtools-next.iss').read_text(encoding='utf-8')
helper = (root / 'installer/close-owned-processes.ps1').read_text(encoding='utf-8')
executables = ['NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.Indexer.exe',
               'NxTools.Cli.exe', 'NxTools.BrowserHost.exe', 'NxTools.ArchiveWorker.exe',
               'NxTools.ShellSelectionHelper.exe']
new_members = ['NxTools.ArchiveWorker.exe', 'NxTools.ShellSelectionHelper.exe',
               'NxTools.ShellSelectionHelper.dll', 'NxTools.ShellSelectionHelper.deps.json',
               'NxTools.ShellSelectionHelper.runtimeconfig.json']
assert 'CloseApplications=yes' in source and 'RestartApplications=no' in source
assert re.search(r'^CloseApplicationsFilter=(.+)$', source, re.M)[1].split(',') == executables
for name in new_members:
    assert f'Source: "{{#PackageDir}}\\{name}"; DestDir: "{{app}}"; Flags: ignoreversion' in source
    assert name in helper
assert 'OwnedIndexerService()' in source and 'SameOwnedPath' in source
assert 'taskkill' not in source.lower() + helper.lower()
prepare = source.split('function PrepareToInstall', 1)[1].split('procedure FinalizeUpgradeFiles', 1)[0]
for name in ('NxTools.release.json', 'NxTools.release.sig'):
    assert f"ExtractTemporaryFile('{name}');" in prepare
    assert f'Source: "{{#PackageDir}}\\{name}"; DestDir: "{{app}}"; Flags: ignoreversion\n' in source
assert '-NewManifestPath' in prepare and '-NewSignaturePath' in prepare
assert "UpgradePreparationReady := False;" in prepare
assert prepare.index("Started := Exec(") < prepare.index("UpgradePreparationReady := True;")
assert "if UpgradePreparationReady and (not UpgradeFilesCommitted) and (not UpgradeFinalizationFailed)" in source
entry = helper.split("if ($LibraryOnly) { return }", 1)[1]
assert "Prepare-UpgradeBrowserHost $InstallDirectory $StatePath $CorrelationId -NewManifestPath $NewManifestPath -NewSignaturePath $NewSignaturePath" in entry
assert "Suspend-UpgradeBrowserHost $InstallDirectory" not in entry
preparation = helper.split("function Prepare-UpgradeBrowserHost", 1)[1].split("if ($LibraryOnly) { return }", 1)[0]
assert preparation.index("Assert-UpgradeSetRecord $installPath $state") < preparation.index("Complete-UpgradeOwnedSet $installPath $statePath $state -Committed")
assert preparation.index("Undo-UpgradeOwnedSet $installPath $statePath $state") < preparation.index("Suspend-UpgradeBrowserHost $installPath $statePath $OperationId")
assert "Previous complete old set not recovered; journal retained." in preparation
assert "Previous journal recovery incomplete; new transaction refused." in preparation
assert 'UpgradeNewBrowserHostHash' not in source, 'Never derive deletion authority after replacement/failure'
assert source.index('PrepareTrustedUpdate();', source.index('procedure CurStepChanged')) < source.index('FinalizeUpgradeFiles(True);')
assert 'NxToolsArchiveReleaseSha256' in helper
assert '$oldBrowserApproved -and (Test-UpgradeArchiveApproval' in helper
assert "'NxTools.exe', 'NxTools.PlatformHost.exe', 'NxTools.Indexer.exe'" in helper
assert '[NxToolsUpgradeProcess]::DeleteExactFile([IO.Path]::GetFullPath($Path), $ExpectedHash)' in helper
assert 'CreateNew' in helper and '$stream.Flush($true)' in helper and '[IO.File]::Replace' in helper
assert helper.index('if ($LibraryOnly) { return }') < helper.index("$serviceKey = 'HKLM:")
assert '$entry.attempted = $true\n            Write-UpgradeJournal' in helper
assert '$entry.restore_attempted = $true\n            Write-UpgradeJournal' in helper
assert '$entry.cleanup_attempted = $true\n            Write-UpgradeJournal' in helper
assert 'for ($index = $State.entries.Count - 1; $index -ge 0; $index--)' in helper
assert "catch { $failures.Add($_.Exception) }" in helper and '[AggregateException]::new' in helper
assert "if ($state.schema -eq 2)" in helper and '[string]$RollbackNewSha256' in helper
assert 'Rollback refused after durable commit point; new set and journal retained' in helper
cleanup = helper.split('function Complete-UpgradeSetCleanup', 1)[1].split('function Complete-UpgradeOwnedSet', 1)[0]
assert 'Undo-UpgradeOwnedSet' not in cleanup
assert 'Assert-UpgradeNewSet $InstallRoot $State' in cleanup
assert 'Remove-UpgradeExactFile $backup $entry.old_sha256' in cleanup
assert 'Incomplete backup cleanup evidence.' in cleanup
assert 'Complete-UpgradeSetCleanup $InstallRoot $StateFile $State' in helper
assert '# Deliberately outside the pre-commit compensation catch.' in helper
print('PASS installer complete file set, extracted predeclared hashes, durable attempted/reverse compensation, original owners; source-only, not native acceptance')
