#!/usr/bin/env python3
"""Source boundary gate only; not compiled, native or release acceptance."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
cli = (root / 'rust/apps/nxtools-cli/src/archive/execution.rs').read_text(encoding='utf-8')
host = (root / 'csharp/NxTools.PlatformHost/Protocol/ArchiveCliHost.cs').read_text(encoding='utf-8')
program = (root / 'csharp/NxTools.PlatformHost/Program.cs').read_text(encoding='utf-8')
checks = {
    'archive mode before normal native runtime': program.index('if (ArchiveCliHost.IsMode(args))') < program.index('IndexerServiceInstaller.PinCurrentProcessImage()'),
    'invalid archive mode rejects early': 'if (!ArchiveCliHost.TryParse(args, out var archiveCliOptions)) return 2;' in program,
    'original read and job owners': 'new ArchiveReadSession(log)' in host and 'new ArchiveJobSession(log)' in host,
    'no ordinary adapter or UI runtime': all(value not in host for value in ('new WindowsPlatformAdapter', 'Application.Run(', 'DoDragDrop(', 'Process.Start(', 'InstallElevated(')),
    'exact held parent and pipe peer': 'ExtensionHostBroker.ValidateParent' in host and '"NxTools.Cli.exe"' in host and 'serverPid != options.ParentPid' in host,
    'fixed unique pipe syntax': 'Guid.TryParseExact(suffix, "D"' in host and 'nxtools-archive-cli-' in host,
    'no injected worker selection': all(value not in host for value in ('ForOwnedTestPackage(', 'ForOwnedWorkerFixture(', 'GetEnvironmentVariable(', 'worker_path')),
    'only extract without readmission': 'Kind: "extract", Readmission: null' in host,
    'bounded event transport': 'BoundedChannelOptions(32)' in host and 'outgoing.Writer.TryWrite' in host,
    'retirement before source completion': host.index('stop.Cancel();\n            reads?.Stop();') < host.index('await reads.DisposeAsync()'),
    'original product read client': 'ArchiveReadClient::new(bridge.archive_client_bridge())' in cli,
    'original typed job route': '.send_archive_job(' in cli and 'ArchiveJobAction::Cancel' in cli,
    'no direct extraction or codec': all(value not in cli for value in ('ArchiveEngine::', 'ZipArchive::', 'extract_all(', 'unpack(', 'remove_dir_all(', 'std::fs::write(')),
    'child does not inherit console control termination': 'creation_flags(CREATE_NO_WINDOW)' in cli,
    'actual read cleanup required': 'terminal.status == ArchiveReadStatus::Succeeded' in cli and 'terminal.cleanup_complete' in cli,
    'both native sessions drained': 'await reads.DisposeAsync()' in host and 'await jobs.DisposeAsync()' in host,
}
failed = [name for name, passed in checks.items() if not passed]
for name in failed:
    print('FAIL ' + name)
if failed:
    raise SystemExit(1)
print(f'PASS archive CLI source boundaries: {len(checks)} assertions; not product/native acceptance')
