from pathlib import Path
import json,hashlib,datetime,subprocess,ast
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-rem-archive-final-matrix';evidence=task/'evidence'
scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\rem-archive-final-matrix')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,obj):
 p=evidence/name;assert not p.exists(),f'Never overwrite {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
def copy(source,rel):
 p=evidence/rel;assert not p.exists(),f'Never overwrite {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(source.read_bytes())
assert not (evidence/'manifest.json').exists(),'Immutable manifest exists'
baseline=load(evidence/'baseline-manifest.json');history=load(evidence/'historical-manifest.json')
for records in [baseline['files'],history['files']]:
 for rec in records:assert (root/rec['path']).is_file() and sha(root/rec['path'])==rec['sha256'],f'Independent movement: {rec["path"]}; do not overwrite'
for p in scratch.glob('*.log'):copy(p,Path('logs')/p.name)
for name in ['observe-idle.ps1','source-gates.ps1','file-index.py','publish.py']:copy(scratch/name,Path('tools')/name)
copy(scratch/'inventory.py',Path('tools/inventory-v2.py'))
copy(scratch.parent/'queue10-baseline.py',Path('tools/baseline.py'))
idle=list(scratch.glob('idle-observation-*.json'));assert len(idle)==1
copy(idle[0],Path('idle-observation.json'));copy(scratch/'final-process-observation.json',Path('final-process-observation.json'))
gates=load(scratch/'source-gate-results.json')
for rec in gates:rec['evidence_log']='evidence/logs/'+Path(rec['log']).name
save('source-gate-results.json',gates)
checks=[]
for rel in ['prd.md','role-matrix.md','file-role-index.md','outcomes.md','evidence/verify.py']:
 p=task/rel;text=p.read_text(encoding='utf-8')
 trailing=[i for i,line in enumerate(text.splitlines(),1) if line.rstrip(' \t')!=line]
 assert not trailing and text.endswith('\n') and not text.endswith('\n\n'),f'Text whitespace failure: {rel}'
 proc=subprocess.run(['git','diff','--no-index','--check','--','NUL',str(p)],cwd=root,capture_output=True)
 assert proc.returncode in [0,1] and not proc.stdout and not proc.stderr, f'Unexpected diff diagnostic: {rel}'
 checks.append({'path':rel,'explicit_trailing_whitespace_lines':trailing,'one_final_newline':True,'git_no_index_raw_exit':proc.returncode,'git_stdout':'','git_stderr':''})
ast.parse((evidence/'verify.py').read_text(encoding='utf-8'))
save('task-checks.json',{'exit':0,'scope':'Five new task/verifier files only; not rustfmt; Git no-index raw difference exits retained','text_checks':checks,'verify_python_ast':'PASS'})
save('own-deltas.json',{'basis':'Task-start live dirty bytes, not HEAD','existing_production_deltas':[],'existing_test_deltas':[],'existing_reference_deltas':[],'reason':'No effective red; implementation blocked before source edits. Current task artifacts only.'})
refs=[Path(r'D:\Tool\Bandizip\langs\SimpChinese.lang'),Path(r'D:\Code\Archiver-main\gui\ui\app.slint'),Path(r'D:\Code\Archiver-main\gui\src\main.rs')]+[root/p for p in ['vendor/gpui-component-080/src/list/list_item.rs','vendor/gpui-component-080/src/form/field.rs','vendor/gpui-component-080/src/form/form.rs','vendor/gpui-component-080/src/input/textarea.rs','vendor/gpui-component-080/src/dialog/dialog.rs','vendor/gpui-component-080/src/switch.rs','vendor/gpui-component-080/src/progress/progress.rs']]
save('reference-source-observations.json',{'scope':'Read-only source observations, not native/visual acceptance or task-start hashes','files':[{'path':str(p),'sha256':sha(p),'bytes':p.stat().st_size} for p in refs]})
save('verification-status.json',{'finding06':'OPEN_SUBSTANTIVE_ORDINARY_AND_UNREVIEWED_REMAINDERS','root_red_green':'NOT_RUN','production_ordinary_remediation':'NOT_IMPLEMENTED','archive_and_affected_related_tests':'NOT_RUN','first_seven_and_accepted06_tests':'NOT_RUN','strict_production_clippy':'NOT_RUN','strict_test_clippy':'NOT_RUN','workspace_check':'NOT_RUN','scoped_rustfmt':'NOT_RUN','workspace_fmt':'NOT_RUN','source_document_gates':'12/12 PASS; Archive -SourceOnly only','rust_invocations':0,'full_desktop':'NOT_RUN / NOT_AUTHORIZED','native_physical_visual_user_release':'NOT_RUN / NOT_AUTHORIZED','inventory_v1':'SUPERSEDED_TOOLING_PREPARATION','inventory_v2':'Lexical navigation, not whole-product semantic coverage'})
artifacts=[{'path':p.relative_to(task).as_posix(),'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(task.rglob('*')) if p.is_file()]
save('manifest.json',{'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'INCOMPLETE_FINDING06_OPEN_BLOCKED_BEFORE_RED','rust_invocations':0,'production_changes':[],'baseline_files':len(baseline['files']),'historical_files':len(history['files']),'source_document_gates':12,'artifacts':artifacts})
print(json.dumps({'manifest_sha256':sha(evidence/'manifest.json'),'artifact_count':len(artifacts),'baseline_files':len(baseline['files']),'history_files':len(history['files']),'status':'INCOMPLETE_FINDING06_OPEN_BLOCKED_BEFORE_RED'},indent=2))
