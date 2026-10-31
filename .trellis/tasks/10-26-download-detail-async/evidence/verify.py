"""Read-only audit #12 evidence and exact dirty-delta verification."""
from pathlib import Path
import base64
import hashlib
import json

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]
EV = TASK / 'evidence'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(path):
    return json.loads((EV / path).read_text(encoding='utf-8-sig'))


def verify():
    manifest = load('manifest.json')
    for item in manifest['files']:
        assert sha((TASK / item['path']).read_bytes()) == item['sha256'], item['path']
    baseline = {item['path']: item for item in load('baseline-manifest.json')['files']}
    historical = load('historical-manifest.json')['files']
    protected = load('protected-manifest.json')
    for item in historical + protected:
        assert sha((ROOT / item['path']).read_bytes()) == item['sha256'], item['path']
    deltas = load('own-deltas.json')
    assert len(deltas) == 12 and sum(item.get('new_file', False) for item in deltas) == 3
    for item in deltas:
        after = (TASK / item['after_snapshot']).read_bytes()
        assert sha(after) == sha((ROOT / item['path']).read_bytes()) == item['after_sha256']
        if item.get('new_file'):
            assert item['path'] not in baseline
            continue
        before = (TASK / baseline[item['path']]['snapshot']).read_bytes()
        assert sha(before) == item['before_sha256']
        forward = before.splitlines(keepends=True)
        reverse = after.splitlines(keepends=True)
        for operation in reversed(item['ops']):
            old = base64.b64decode(operation['old'])
            new = base64.b64decode(operation['new'])
            assert b''.join(forward[operation['old_start']:operation['old_end']]) == old
            assert b''.join(reverse[operation['new_start']:operation['new_end']]) == new
            forward[operation['old_start']:operation['old_end']] = new.splitlines(keepends=True)
            reverse[operation['new_start']:operation['new_end']] = old.splitlines(keepends=True)
        assert b''.join(forward) == after and b''.join(reverse) == before
    owners = load('owner-checks.json')
    assert len(owners) == 19 and all(item['result'] == 'PASS' for item in owners)
    summary = load('verification-summary.json')
    for name, count in [('audit12-final-formatted', 9), ('final-download-page', 29),
                        ('download-main-related-01', 27), ('downloads-lifecycle-no-network', 48),
                        ('final-names-identity', 18), ('engine-task-detail-contract', 1)]:
        result = summary['results'][name]
        assert f'{count} passed; 0 failed; 0 ignored' in result
        assert result in (EV / 'commands' / (name + '.log')).read_text(encoding='utf-8')
    original = (EV / 'commands/audit12-store-lock-red-no-pdb.log').read_text(encoding='utf-8')
    final = (EV / 'commands/audit12-final-formatted.log').read_text(encoding='utf-8')
    assert '0 passed; 1 failed' in original
    assert 'next_ui_turn_before_watchdog=false watchdog_unlocked=true' in original
    assert 'next_ui_turn_before_watchdog=true watchdog_unlocked=false' in final
    assert '0 passed; 1 failed' in summary['results']['audit12-refresh-cache-red']
    for name in ['strict-desktop-tests-final', 'strict-download-tests',
                 'strict-download-production', 'workspace-check-final']:
        assert load('commands/' + name + '.json')['exit'] == 0, name
    for name in ['strict-desktop-production-final', 'workspace-fmt-final']:
        assert load('commands/' + name + '.json')['exit'] != 0, name
    scoped = load('commands/scoped-checks-final.json')
    assert scoped['fmt'] == scoped['diff'] == 0
    gates = load('commands/source-gate-results.json')
    assert len(gates) == 10 and all(item['exit'] == 0 for item in gates)
    # Original helper and other consumers were restored/never changed.
    for relative in ['rust/apps/nxtools-desktop/src/downloads/window_support.rs',
                     'rust/apps/nxtools-desktop/src/downloads/artifact_actions.rs',
                     'rust/apps/nxtools-desktop/src/downloads/task_windows_impl.rs',
                     'rust/apps/nxtools-desktop/src/downloads/completion_shutdown_impl.rs',
                     'rust/crates/nxtools-download/src/engine.rs', 'Cargo.lock']:
        assert sha((ROOT / relative).read_bytes()) == baseline[relative]['sha256']
    new_sources = ''.join((ROOT / item['path']).read_text(encoding='utf-8')
                          for item in deltas if item.get('new_file'))
    assert 'DEBUG-' not in new_sources and '#[ignore' not in new_sources and 'allow(clippy' not in new_sources
    assert summary['network_full_desktop_native_release'] == 'NOT RUN'
    print(f"PASS: {len(manifest['files'])} artifacts, 9 reversible live-dirty deltas + 3 new modules; "
          f"{len(protected)} protected files, {len(historical)} immutable history files, {len(owners)} owner checks.")
    print('PASS recorded real-store blocking red/green, lifecycle regressions, strict tests, Download strict, check, scoped fmt, source gates.')
    print('RETAINED FAIL: baseline Desktop production strict and four-file workspace fmt.')
    print('NOT RUN: network, full Desktop, native/physical/DPI/visual/installed/release; no global 13-finding closure.')


if __name__ == '__main__':
    verify()
