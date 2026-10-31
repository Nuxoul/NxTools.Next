from pathlib import Path
import json,hashlib,datetime,re,ast,subprocess
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-cursor-semantics';evidence=task/'evidence'
scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\cursor-semantics')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,obj):
 p=evidence/name;assert not p.exists(),f'Never overwrite {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
def copy(source,rel):
 p=evidence/rel;assert not p.exists(),f'Never overwrite {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(source.read_bytes())
assert not (evidence/'manifest.json').exists()
baseline=load(evidence/'baseline-manifest.json');history=load(evidence/'historical-manifest.json')
for records in [baseline['files'],history['files']]:
 for rec in records:assert (root/rec['path']).is_file() and sha(root/rec['path'])==rec['sha256'],f'Independent movement: {rec["path"]}'
for p in scratch.glob('*.log'):copy(p,Path('logs')/p.name)
for name in ['observe-idle.ps1','source-gates.ps1','publish.py']:copy(scratch/name,Path('tools')/name)
copy(scratch.parent/'queue11-baseline.py',Path('tools/baseline.py'))
for name in ['idle-observation.json','final-process-observation.json']:copy(scratch/name,Path(name))
gates=load(scratch/'source-gate-results.json')
for rec in gates:rec['evidence_log']='evidence/logs/'+Path(rec['log']).name
save('source-gate-results.json',gates)
all_cursor=[]
for p in sorted((root/'rust/apps/nxtools-desktop/src').rglob('*.rs')):
 text=p.read_text(encoding='utf-8-sig');test_file=text.startswith('#![cfg(test)]')
 for number,line in enumerate(text.splitlines(),1):
  if re.search(r'\.cursor_[a-z_]+\s*\(',line):
   all_cursor.append({'path':p.relative_to(root).as_posix(),'line':number,'source':line,'test_only_file':test_file,'level':'Source declaration, not painted/hit cursor observation'})
pointers=[r for r in all_cursor if '.cursor_pointer(' in r['source']]
assert len(pointers)==12 and sum(r['test_only_file'] for r in pointers)==1
save('application-cursor-declarations.json',{'scope':'Direct application cursor style builder declarations; no aliases/indirect renderer completeness claim. No runtime result.','all_declarations':all_cursor,'pointer_declarations':pointers})
checks=[]
for rel in ['prd.md','inventory.md','outcomes.md','evidence/verify.py']:
 p=task/rel;text=p.read_text(encoding='utf-8');trailing=[i for i,line in enumerate(text.splitlines(),1) if line.rstrip(' \t')!=line]
 assert not trailing and text.endswith('\n') and not text.endswith('\n\n'),f'Task whitespace failure: {rel}'
 result=subprocess.run(['git','diff','--no-index','--check','--','NUL',str(p)],cwd=root,capture_output=True)
 diagnostic=result.stderr.decode('utf-8',errors='replace')
 assert result.returncode in [0,1] and not result.stdout
 assert not diagnostic or all('warning:' in x and ('CRLF will be replaced by LF' in x or 'LF will be replaced by CRLF' in x) for x in diagnostic.splitlines()),diagnostic
 checks.append({'file':rel,'trailing_whitespace_lines':trailing,'one_final_newline':True,'git_no_index_raw_exit':result.returncode,'git_stdout':'','git_stderr':diagnostic})
ast.parse((evidence/'verify.py').read_text(encoding='utf-8'))
save('task-checks.json',{'exit':0,'scope':'Four new task/verifier files only, not Rust fmt','explicit_whitespace':checks,'verify_python_ast':'PASS'})
save('own-deltas.json',{'basis':'Current live task-start dirty bytes, not HEAD','production_changes':[],'test_changes':[],'reference_changes':[],'reason':'Effective production cursor red not executed. No implementation; new task artifacts only.'})
pre=scratch.parent/'review-gpui-pre/gpui-pre-0.3.9/src'
base=Path(r'C:\Users\Administrator\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\gpui-base-0.8.0\src')
refs=[pre/p for p in ['window.rs','elements/div.rs','app/test_context.rs','platform/test/platform.rs']]+[base/p for p in ['button.rs','checkbox.rs','tabs.rs','link.rs']]
save('released-source-observations.json',{'scope':'Read-only exact released source observations. No public cursor observation recipe/compiled or native acceptance established. Registry untouched.','files':[{'path':str(p),'sha256':sha(p),'bytes':p.stat().st_size} for p in refs]})
save('verification-status.json',{'audit07':'OPEN_NOT_REPAIRED','production_paint_hit_cursor_red':'NOT_RUN','compliant_cursor_observation_seam':'UNFINISHED_INVESTIGATION_NOT_COMPILED','cursor_only_production_repair':'NOT_IMPLEMENTED','normal_hover_disabled_link_drag_exception_tests':'NOT_RUN','ui_quick_launch_downloads_tests':'NOT_RUN','first_seven_focus_gesture_protections':'NOT_RUN','strict_production_clippy':'NOT_RUN','strict_test_clippy':'NOT_RUN','workspace_check':'NOT_RUN','scoped_fmt':'NOT_RUN','workspace_fmt':'NOT_RUN','rust_invocations':0,'source_document_gates':'11/11 PASS','full_desktop_native_physical_user_data_release':'NOT_RUN / NOT_AUTHORIZED'})
artifacts=[{'path':p.relative_to(task).as_posix(),'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(task.rglob('*')) if p.is_file()]
save('manifest.json',{'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'INCOMPLETE_AUDIT07_BLOCKED_BEFORE_PAINT_HIT_RED','production_changes':[],'rust_invocations':0,'baseline_files':len(baseline['files']),'historical_files':len(history['files']),'artifacts':artifacts})
print(json.dumps({'manifest_sha256':sha(evidence/'manifest.json'),'artifacts':len(artifacts),'protected_live_files':len(baseline['files']),'history_files':len(history['files']),'status':'INCOMPLETE_AUDIT07_BLOCKED_BEFORE_PAINT_HIT_RED'},indent=2))
