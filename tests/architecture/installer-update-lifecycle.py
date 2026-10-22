"""Source-only installer close/restart boundary; never installs or operates a service."""
from pathlib import Path
root = Path(__file__).resolve().parents[2]
source = (root / 'installer/nxtools-next.iss').read_text(encoding='utf-8')
required = ['CloseApplications=yes', 'RestartApplications=no', 'CloseApplicationsFilter=NxTools.exe,NxTools.PlatformHost.exe,NxTools.Indexer.exe,NxTools.Cli.exe,NxTools.BrowserHost.exe']
for entry in required:
    assert entry in source, f'Exact installed-file update coordination missing: {entry}'
assert "OwnedIndexerService()" in source
assert "SameOwnedPath" in source
assert 'taskkill' not in source.lower(), 'No name-wide process termination'
print('PASS installer explicit installed-file Restart Manager scope; not native lock-release/service acceptance')
