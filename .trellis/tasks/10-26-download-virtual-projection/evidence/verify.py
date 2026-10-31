"""Read-only audit #11 delivery verification; no build/native/user data operations."""
from pathlib import Path
import base64
import hashlib
import json

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]
EV = TASK / 'evidence'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def load(name):
    return json.loads((EV / name).read_text(encoding='utf-8-sig'))


def verify():
    manifest = load('manifest.json')
    for item in manifest['files']:
        path = TASK / item['path']
        assert digest(path.read_bytes()) == item['sha256'], item['path']
    baseline = {item['path']: item for item in load('baseline-manifest.json')['files']}
    for item in load('historical-manifest.json')['files']:
        assert digest((ROOT / item['path']).read_bytes()) == item['sha256'], item['path']
    for item in load('protected-manifest.json'):
        assert digest((ROOT / item['path']).read_bytes()) == item['sha256'], item['path']
    deltas = load('own-deltas.json')
    assert len(deltas) == 9
    for item in deltas:
        live = (ROOT / item['path']).read_bytes()
        after = (TASK / item['after_snapshot']).read_bytes()
        assert digest(live) == digest(after) == item['after_sha256'], item['path']
        if item.get('new_file'):
            assert item['path'] not in baseline
            continue
        before = (TASK / baseline[item['path']]['snapshot']).read_bytes()
        assert digest(before) == item['before_sha256']
        forward = before.splitlines(keepends=True)
        reverse = after.splitlines(keepends=True)
        for operation in reversed(item['ops']):
            prior = base64.b64decode(operation['old'])
            following = base64.b64decode(operation['new'])
            assert b''.join(forward[operation['old_start']:operation['old_end']]) == prior
            assert b''.join(reverse[operation['new_start']:operation['new_end']]) == following
            forward[operation['old_start']:operation['old_end']] = following.splitlines(keepends=True)
            reverse[operation['new_start']:operation['new_end']] = prior.splitlines(keepends=True)
        assert b''.join(forward) == after and b''.join(reverse) == before
    owner = load('owner-checks.json')
    assert len(owner) == 15 and all(item['result'] == 'PASS' for item in owner)
    summary = load('verification-summary.json')
    for name, count in [('audit11-five-final', 5), ('download-page-related', 26),
                        ('download-main-related', 27), ('downloads-related', 210),
                        ('protected-related', 69), ('final-alias-and-identity', 23)]:
        assert f'{count} passed; 0 failed; 0 ignored' in summary['results'][name]
        assert summary['results'][name] in (EV / 'commands' / (name + '.log')).read_text(encoding='utf-8')
    assert '0 passed; 2 failed' in summary['results']['audit11-red-10000']
    assert load('commands/strict-tests-final.json')['exit'] == 0
    assert load('commands/strict-production.json')['exit'] != 0
    assert load('commands/workspace-check.json')['exit'] == 0
    assert load('commands/workspace-fmt.json')['exit'] != 0
    scoped = load('commands/scoped-checks.json')
    assert scoped['fmt'] == scoped['diff'] == 0
    gates = load('commands/source-gate-results.json')
    assert len(gates) == 11 and all(item['exit'] == 0 for item in gates)
    page = (ROOT / 'rust/apps/nxtools-desktop/src/main_window/download_page.rs').read_text(encoding='utf-8')
    regression = (ROOT / 'rust/apps/nxtools-desktop/src/main_window/download_page/projection_tests.rs').read_text(encoding='utf-8')
    assert 'DEBUG-AUDIT11' not in page + regression
    assert '#[ignore' not in regression and 'allow(clippy' not in regression
    assert summary['full_desktop'] == summary['native_visual_release'] == 'NOT RUN'
    print(f"PASS: {len(manifest['files'])} artifacts; 8 reversible dirty deltas + 1 new test; "
          f"{summary['protected_files']} protected baseline files; {summary['history_files']} unchanged history files; "
          f"{len(owner)} owner checks; recorded behavioral/compiled/source results verified.")
    print('RETAINED FAIL: baseline production strict (SPACING_12 / DecodedMedia.animated) and five-file workspace fmt.')
    print('NOT RUN: full Desktop, real downloads/native/DPI/visual/installed/release. No global 13-finding closure.')


if __name__ == '__main__':
    verify()
