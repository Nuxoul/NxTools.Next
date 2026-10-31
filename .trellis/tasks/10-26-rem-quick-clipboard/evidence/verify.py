from pathlib import Path
import hashlib
import json

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

manifest = load(TASK / 'evidence/manifest.json')
assert manifest['status'] == 'INCOMPLETE_BLOCKED_BEFORE_EFFECTIVE_RED'
assert manifest['rust_invocations'] == 0
assert manifest['production_changes'] == []
for record in manifest['artifacts']:
    path = TASK / record['path']
    assert path.is_file(), f'Missing artifact: {path}'
    assert digest(path) == record['sha256'], f'Artifact changed: {path}'

baseline = load(TASK / 'evidence/baseline-manifest.json')
for record in baseline['files']:
    snapshot = TASK / record['snapshot']
    current = ROOT / record['path']
    assert digest(snapshot) == record['sha256'], f'Baseline changed: {snapshot}'
    assert current.is_file(), f'Protected source missing: {current}'
    assert digest(current) == record['sha256'], f'Protected live source moved: {current}'
history = load(TASK / 'evidence/historical-manifest.json')
for record in history['files']:
    current = ROOT / record['path']
    assert current.is_file(), f'Historical file missing: {current}'
    assert digest(current) == record['sha256'], f'Historical evidence changed: {current}'

gates = load(TASK / 'evidence/source-gate-results.json')
assert len(gates) == 11 and all(record['exit'] == 0 for record in gates)
for record in gates:
    assert (TASK / record['evidence_log']).is_file()
idle = load(TASK / 'evidence/idle-observation.json')
assert not idle['natural_idle'] and len(idle['samples']) == 11
for sample in idle['samples']:
    observed_ids = {p['ProcessId'] for p in sample['processes']}
    assert {83436, 87140}.issubset(observed_ids)
assert not load(TASK / 'evidence/final-process-observation.json')['natural_idle']
assert load(TASK / 'evidence/task-whitespace.json')['exit'] == 0

expected_rs = {r['path'] for r in baseline['files'] if r['path'].startswith('rust/apps/nxtools-desktop/src/') and r['path'].endswith('.rs')}
current_rs = {p.relative_to(ROOT).as_posix() for p in (ROOT / 'rust/apps/nxtools-desktop/src').rglob('*.rs')}
assert current_rs == expected_rs, 'Active Desktop source set changed'
print(f'PASS evidence integrity: {len(manifest["artifacts"])} artifacts; {len(baseline["files"])} live source/contract files and {len(history["files"])} historical files unchanged; 11 source/document gates')
print('INCOMPLETE: no effective Root red, no production implementation, no Rust/compiled/interaction/native/visual acceptance.')
