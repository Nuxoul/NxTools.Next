from pathlib import Path
import json,hashlib,difflib,shutil,datetime,re
R=Path.cwd(); S=Path(__file__).parent; T=R/'.trellis/tasks/10-25-rem-role-consistency'; E=T/'evidence'; sha=lambda b:hashlib.sha256(b).hexdigest()
if (E/'delivery-manifest.json').exists():raise SystemExit('refuse overwrite published delivery')
E.mkdir(parents=True,exist_ok=True)
rows=json.loads((S/'baseline-manifest.json').read_text()); deltas=[]; captured=[]
for row in rows:
 p=row['path']; b=Path(row['baseline']).read_bytes(); a=(R/p).read_bytes(); assert sha(b)==row['sha256']
 name=p.replace('/','__'); bp=E/'baselines'/(name+'.baseline'); ap=E/'current'/(name+'.current');bp.parent.mkdir(exist_ok=True);ap.parent.mkdir(exist_ok=True);bp.write_bytes(b);ap.write_bytes(a)
 captured.append(dict(path=p,before_sha256=sha(b),after_sha256=sha(a),baseline=bp.relative_to(T).as_posix(),current=ap.relative_to(T).as_posix(),changed=b!=a))
 if b==a:continue
 before=b.decode('utf8').splitlines(keepends=True);after=a.decode('utf8').splitlines(keepends=True);ops=[]
 for op,i,j,k,l in difflib.SequenceMatcher(None,before,after,autojunk=False).get_opcodes():
  if op!='equal':ops.append(dict(before_start=i,before_end=j,after_start=k,after_end=l,before_lines=before[i:j],after_lines=after[k:l]))
 forward=before.copy()
 for o in reversed(ops):assert forward[o['before_start']:o['before_end']]==o['before_lines'];forward[o['before_start']:o['before_end']]=o['after_lines']
 reverse=after.copy()
 for o in reversed(ops):assert reverse[o['after_start']:o['after_end']]==o['after_lines'];reverse[o['after_start']:o['after_end']]=o['before_lines']
 assert ''.join(forward).encode()==a and ''.join(reverse).encode()==b
 delta=dict(path=p,before_sha256=sha(b),after_sha256=sha(a),baseline=bp.relative_to(T).as_posix(),current=ap.relative_to(T).as_posix(),forward_rebuild=True,reverse_rebuild=True,operations=ops)
 deltas.append(delta)
 (E/(name+'.own-delta.txt')).write_text(''.join(difflib.unified_diff(before,after,fromfile='dirty-baseline/'+p,tofile='audit06/'+p)),encoding='utf8',newline='')
(E/'own-deltas.json').write_text(json.dumps(deltas,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
(E/'captured-sources.json').write_text(json.dumps(captured,indent=2)+'\n',encoding='utf8')
for p in sorted(S.iterdir()):
 if p.is_file() and (p.suffix in ['.log','.json','.ps1','.py']) and p.name!='publish-evidence.py':shutil.copyfile(p,E/p.name)
# Original scratch paths are provenance only; published baselines/current above are independently readable.
shutil.copyfile(Path(__file__),E/'publish-evidence.py')
newdocs=[]
for name in ['prd.md','outcomes.md']:
 b=(T/name).read_bytes();newdocs.append(dict(path=(T/name).relative_to(R).as_posix(),sha256=sha(b),bytes=len(b)))
reference_paths=[
 'vendor/gpui-component-080/src/root.rs','vendor/gpui-component-080/src/sizing.rs','vendor/gpui-component-080/src/form/form.rs','vendor/gpui-component-080/src/form/field.rs','vendor/gpui-component-080/src/switch.rs','vendor/gpui-component-080/src/time/time_field.rs','vendor/gpui-component-080/src/button/button.rs','vendor/gpui-component-080/src/progress/progress.rs','vendor/gpui-component-080/src/scroll/scrollable.rs',
 'rust/apps/nxtools-desktop/src/main_window/archive_page_tests.rs','rust/apps/nxtools-desktop/src/main_window/extensions_page.rs',
 'tests/architecture/ui-token-boundaries.ps1','tests/architecture/ui-consistency.ps1','tests/architecture/download-ui-lifecycle.ps1',
]
refs=[dict(path=p,sha256=sha((R/p).read_bytes())) for p in reference_paths]
(E/'reference-and-failure-sources.json').write_text(json.dumps(refs,indent=2)+'\n',encoding='utf8')
manifest=dict(task='Original audit #6, eighth serial slice',published_at=datetime.datetime.now().isoformat(),root=str(R),baseline_captured_at='2026-10-25T18:07:16.271938',changed_existing_count=len(deltas),captured_sources=captured,new_documents=newdocs,protected=json.loads((S/'protected-baseline.json').read_text()),frozen_history=json.loads((S/'frozen-history.json').read_text()),owner_checks='evidence/owner-invariants.json',own_delta='evidence/own-deltas.json',full_suite=dict(runs=1,passed=1237,failed=1,ignored=0,exit_code=101,log='evidence/desktop-full-once.log'),evidence=[])
for p in sorted(E.rglob('*')):
 if p.is_file():manifest['evidence'].append(dict(path=p.relative_to(T).as_posix(),sha256=sha(p.read_bytes()),bytes=p.stat().st_size))
(E/'delivery-manifest.json').write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
print('published existing deltas',len(deltas),'evidence',len(manifest['evidence']),'documents',len(newdocs));print('manifest SHA256',sha((E/'delivery-manifest.json').read_bytes()));print('own-deltas SHA256',sha((E/'own-deltas.json').read_bytes()))
