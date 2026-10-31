from pathlib import Path
from collections import Counter
import hashlib,json,ast

TASK=Path(__file__).resolve().parents[1]
ROOT=TASK.parents[2]
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
manifest=load(TASK/'evidence/manifest.json')
assert manifest['status']=='INCOMPLETE_FINDING06_OPEN_BLOCKED_BEFORE_RED'
assert manifest['production_changes']==[] and manifest['rust_invocations']==0
for r in manifest['artifacts']:
 p=TASK/r['path'];assert p.is_file() and sha(p)==r['sha256'],f'Artifact changed: {p}'
baseline=load(TASK/'evidence/baseline-manifest.json')
for r in baseline['files']:
 p=ROOT/r['path'];snap=TASK/r['snapshot']
 assert snap.is_file() and sha(snap)==r['sha256'],f'Baseline changed: {snap}'
 assert p.is_file() and sha(p)==r['sha256'],f'Protected live path moved: {p}'
history=load(TASK/'evidence/historical-manifest.json')
for r in history['files']:
 p=ROOT/r['path'];assert p.is_file() and sha(p)==r['sha256'],f'History changed: {p}'
files=load(TASK/'evidence/desktop-file-coverage-v2.json')
records=load(TASK/'evidence/desktop-px-calls-v2.json')
archive=load(TASK/'evidence/archive-px-roles-v2.json')
assert len(files)==247 and len(records)==2882 and len(archive)==158
expected={r['path'] for r in files}
current={p.relative_to(ROOT).as_posix() for p in (ROOT/'rust/apps/nxtools-desktop/src').rglob('*.rs')}
assert expected==current,'Active Desktop source set changed'
for r in records:
 text=(ROOT/r['path']).read_text(encoding='utf-8-sig')
 lines=text.splitlines(keepends=True)
 start=sum(map(len,lines[:r['line']-1]))+r['column']-1
 assert text[start:start+len(r['expression'])]==r['expression'],f'Index source drift: {r}'
counts=Counter(r['classification'] for r in archive)
assert counts=={'TEST_ONLY':104,'PROTECTED_NATIVE_CLIENT':16,'APPROVED_FIXED_LAYOUT':3,'UNFINISHED_ORDINARY_CANDIDATE':23,'UNFINISHED_PERSISTED_GEOMETRY':4,'UNFINISHED_VIEWPORT':2,'UNFINISHED_REVIEW':6},counts
assert all(r['classification']!='TEST_ONLY' for r in archive if r['path'].endswith(('/archive/application.rs','/archive/browser.rs','/archive/forms.rs','/archive/jobs.rs','/main_window/archive_page.rs')))
for file in files:
 subset=[r for r in records if r['path']==file['path']]
 assert len(subset)==file['px_calls']
 assert dict(Counter(r['classification'] for r in subset))==file['classification_counts']

# Run only lexical helper definitions, never the inventory writer/module body.
source=(TASK/'evidence/tools/inventory-v2.py').read_text(encoding='utf-8')
tree=ast.parse(source)
helpers=[]
for node in tree.body:
 if isinstance(node,(ast.Import,ast.ImportFrom)):
  helpers.append(node)
 elif isinstance(node,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='TOKEN' for t in node.targets):helpers.append(node)
 elif isinstance(node,ast.FunctionDef) and node.name in ['mask','close','test_ranges']:helpers.append(node)
namespace={}
exec(compile(ast.Module(body=helpers,type_ignores=[]),'<lexical-helpers>','exec'),namespace)
def test_flag(text):
 code=namespace['mask'](text);pos=code.index('px(')
 return any(a<=pos<=b for a,b in namespace['test_ranges'](code))
assert not test_flag('struct S { #[cfg(test)] probe: bool, } impl S { fn draw() { px(32.); } }')
assert test_flag('#[cfg(test)] fn test() { px(32.); }')
assert test_flag('#[cfg(test)] mod tests { fn fixture() { px(32.); } }')
assert not test_flag('#[cfg(test)] fn test() { } fn draw() { px(32.); }')
assert 'px(' not in namespace['mask']('let s = "px(4.)"; // px(8.)')

results=load(TASK/'evidence/source-gate-results.json')
assert len(results)==12 and all(r['exit']==0 for r in results)
assert any('-SourceOnly' in r['command'] for r in results if 'archive-release-chain' in r['command'])
idle=load(TASK/'evidence/idle-observation.json')
assert not idle['natural_idle'] and len(idle['samples'])==21
assert all({83436,87140}.issubset({p['ProcessId'] for p in s['processes']}) for s in idle['samples'])
assert load(TASK/'evidence/task-checks.json')['exit']==0
print(f'PASS integrity: {len(manifest["artifacts"])} artifacts, {len(baseline["files"])} protected live files, {len(history["files"])} immutable earlier records; no source delta.')
print('PASS lexical-index consistency and cfg(test)-field regression; 247 files/2882 direct calls are navigation, not semantic/compiled coverage.')
print('PASS 12 source/document gates and task text whitespace only. Archive gate ran -SourceOnly, not packaging rejection.')
print('INCOMPLETE: original #6 OPEN; no effective Root red, ordinary implementation or Rust/interaction/native/visual acceptance.')
