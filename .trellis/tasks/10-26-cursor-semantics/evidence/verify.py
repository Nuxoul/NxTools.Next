from pathlib import Path
import json,hashlib,re
TASK=Path(__file__).resolve().parents[1]
ROOT=TASK.parents[2]
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
manifest=load(TASK/'evidence/manifest.json')
assert manifest['status']=='INCOMPLETE_AUDIT07_BLOCKED_BEFORE_PAINT_HIT_RED'
assert manifest['production_changes']==[] and manifest['rust_invocations']==0
for rec in manifest['artifacts']:
 p=TASK/rec['path'];assert p.is_file() and sha(p)==rec['sha256'],f'Artifact changed: {p}'
baseline=load(TASK/'evidence/baseline-manifest.json')
for rec in baseline['files']:
 current=ROOT/rec['path'];snapshot=TASK/rec['snapshot']
 assert snapshot.is_file() and sha(snapshot)==rec['sha256'],f'Baseline changed: {snapshot}'
 assert current.is_file() and sha(current)==rec['sha256'],f'Protected live source moved: {current}'
history=load(TASK/'evidence/historical-manifest.json')
for rec in history['files']:
 p=ROOT/rec['path'];assert p.is_file() and sha(p)==rec['sha256'],f'Historical evidence changed: {p}'
expected={r['path'] for r in baseline['files'] if r['path'].startswith('rust/apps/nxtools-desktop/src/')}
actual={p.relative_to(ROOT).as_posix() for p in (ROOT/'rust/apps/nxtools-desktop/src').rglob('*.rs')}
assert expected==actual,'Active Desktop source set changed'
cursor=load(TASK/'evidence/application-cursor-declarations.json')
assert len(cursor['pointer_declarations'])==12 and sum(r['test_only_file'] for r in cursor['pointer_declarations'])==1
for rec in cursor['all_declarations']:
 p=ROOT/rec['path'];line=p.read_text(encoding='utf-8-sig').splitlines()[rec['line']-1]
 assert line==rec['source'],f'Cursor declaration changed: {p}:{rec["line"]}'
assert (ROOT/'rust/apps/nxtools-desktop/src/ui/components/list.rs').read_text(encoding='utf-8').startswith('#![cfg(test)]')
guide=(TASK/'evidence/design-guide-original.txt').read_text(encoding='utf-8')
assert 'Use the default arrow cursor for buttons, checkboxes, menu items, tabs' in guide
assert 'Use a pointing hand for links' in guide
results=load(TASK/'evidence/source-gate-results.json')
assert len(results)==11 and all(r['exit']==0 for r in results)
idle=load(TASK/'evidence/idle-observation.json')
assert not idle['natural_idle'] and len(idle['samples'])==11
assert all({83436,87140}.issubset({p['ProcessId'] for p in s['processes']}) for s in idle['samples'])
assert not load(TASK/'evidence/final-process-observation.json')['natural_idle']
assert load(TASK/'evidence/task-checks.json')['exit']==0
print(f'PASS integrity: {len(manifest["artifacts"])} artifacts; {len(baseline["files"])} protected live files; {len(history["files"])} immutable previous records; zero production/source delta.')
print('PASS current source declaration inventory (12 hand declarations, one test-only), preserved true cursor exceptions and 11 source/document gates. These are not runtime cursor results.')
print('INCOMPLETE: #7 NOT REPAIRED; no effective production paint-hit red/green or Rust/interaction/native/visual acceptance.')
