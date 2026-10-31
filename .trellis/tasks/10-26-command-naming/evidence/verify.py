from pathlib import Path
import json,hashlib,base64,ast,re
TASK=Path(__file__).resolve().parents[1]
ROOT=TASK.parents[2]
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(data):return hashlib.sha256(data).hexdigest()
manifest=load(TASK/'evidence/manifest.json')
assert manifest['status']=='IMPLEMENTED_BOUNDED_VERIFICATION_BLOCKED_NOT_GLOBAL_AUDIT08_CLOSED'
for r in manifest['artifacts']:
 p=TASK/r['path'];assert p.is_file() and sha(p.read_bytes())==r['sha256'],f'Artifact changed: {p}'
baseline=load(TASK/'evidence/baseline-manifest.json')
for r in baseline['files']:
 assert sha((TASK/r['snapshot']).read_bytes())==r['sha256'],f'Baseline changed: {r["path"]}'
deltas=load(TASK/'evidence/own-deltas.json')
for r in deltas['existing']:
 before=(TASK/r['before_snapshot']).read_bytes();after=(TASK/r['after_snapshot']).read_bytes()
 assert sha(before)==r['before_sha256'] and sha(after)==r['after_sha256']
 before_lines=before.splitlines(keepends=True);after_lines=after.splitlines(keepends=True)
 forward=[];reverse=[]
 for op in r['ops']:
  old=base64.b64decode(op['before_b64']);new=base64.b64decode(op['after_b64'])
  assert b''.join(before_lines[op['i1']:op['i2']])==old
  assert b''.join(after_lines[op['j1']:op['j2']])==new
  forward.append(new);reverse.append(old)
 assert b''.join(forward)==after and b''.join(reverse)==before,f'Delta replay failure: {r["path"]}'
 assert (ROOT/r['path']).read_bytes()==after,f'Own live source moved: {r["path"]}'
for r in deltas['new']:
 assert (ROOT/r['path']).read_bytes()==(TASK/r['snapshot']).read_bytes(),f'New source moved: {r["path"]}'
inspection=load(TASK/'evidence/owner-inspection-final.json')
for r in inspection['protected_unchanged']:
 assert (ROOT/r['path']).is_file() and sha((ROOT/r['path']).read_bytes())==r['sha256'],f'Protected source moved: {r["path"]}'
history=load(TASK/'evidence/historical-manifest.json')
for r in history['files']:
 assert (ROOT/r['path']).is_file() and sha((ROOT/r['path']).read_bytes())==r['sha256'],f'History changed: {r["path"]}'
# Recompute original callback/whole inverse presentation and old-prefix checks. This section
# of the frozen inspector is read-only; omit its write-once report publisher entirely.
source=(TASK/'evidence/tools/inspect.py').read_text(encoding='utf-8')
read_only=source.split('\npath=scratch/')[0]
namespace={};exec(compile(ast.parse(read_only),'<read-only-owner-inspection>','exec'),namespace)
assert len(namespace['checks'])==17
assert len(deltas['existing'])==16 and len(deltas['new'])==1
results=load(TASK/'evidence/compiled-results.json')
byname={r['record']:r for r in results}
for name,count in [('audit08-final-twelve.json',12),('downloads-final.json',210),('ui-related.json',237),('names-final.json',7),('history-identity-protection.json',11),('sort-protection.json',3),('audit04-protection.json',6),('audit05-protection.json',15),('audit06-protection.json',36),('quick-presentation-related.json',11)]:
 r=byname[name];assert r['exit']==0 and any(f'{count} passed; 0 failed; 0 ignored;' in s for s in r['summaries']),name
assert byname['main-window-related.json']['exit']==101 and any('206 passed; 6 failed;' in s for s in byname['main-window-related.json']['summaries'])
assert byname['strict-tests-final.json']['exit']==101 and byname['strict-production-final.json']['exit']==101
assert byname['workspace-check-final.json']['exit']==0 and byname['workspace-fmt-final.json']['exit']==1
last=load(TASK/'evidence/final-verification-results.json')
assert all(r['exit']==0 for r in last if r['name'] in ['scoped-fmt-final','scoped-diff-final'])
gates=load(TASK/'evidence/source-gate-results.json');assert len(gates)==11 and all(r['exit']==0 for r in gates)
assert load(TASK/'evidence/task-checks.json')['exit']==0
assert load(TASK/'evidence/protected-subsets.json')['keyboard8']==8 and load(TASK/'evidence/protected-subsets.json')['picker14']==14
print(f'PASS integrity: {len(manifest["artifacts"])} artifacts; 16 reversible existing dirty deltas +1 new test; 17 original-owner/prefix checks; {len(inspection["protected_unchanged"])} protected files; {len(history["files"])} historical records.')
print('PASS frozen actual results: audit08 12/12, Downloads210, UI237, names7, identity11, sort3, object6, audit0515, rem36, Quick11; source/document gates11; final scopedfmt/diff/check.')
print('RETAINED FAIL: MainWindow206/6, strict tests/production101, workspacefmt1. Global audit#8 NOT CLOSED; no full Desktop/native/physical/visual/user/release acceptance.')
