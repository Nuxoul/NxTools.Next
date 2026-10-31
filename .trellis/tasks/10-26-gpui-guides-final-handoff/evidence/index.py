from pathlib import Path
import json,hashlib,datetime,re
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-gpui-guides-final-handoff';ev=task/'evidence'
def rec(p):
 assert p.is_file(),str(p)
 return {'path':p.relative_to(root).as_posix(),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size}
def delta_paths(v):
 out=[]
 if isinstance(v,list):
  for x in v:out+=delta_paths(x)
 elif isinstance(v,dict):
  p=v.get('path',v.get('file',v.get('source')))
  if isinstance(p,str) and (root/p).is_file():out.append(p)
  for k in ['existing','new','files','deltas','changes']:
   if k in v:out+=delta_paths(v[k])
 return out
mapping={1:'10-24-download-enter-popover',2:'10-25-download-sort-keyboard',3:'10-25-button-accessible-names',4:'10-25-archive-prompt-object',5:'10-25-default-submit-keyboard',6:'10-25-rem-role-consistency',7:'10-26-cursor-semantics',8:'10-26-command-naming',9:'10-25-download-picker-identity',10:'10-25-clipboard-stable-focus',11:'10-26-download-virtual-projection',12:'10-26-download-detail-async',13:'10-26-quick-retained-text'}
manual={1:['rust/apps/nxtools-desktop/src/downloads/new_download_view.rs','rust/apps/nxtools-desktop/src/downloads/new_download_keyboard_tests.rs','rust/apps/nxtools-desktop/src/downloads/tests.rs','docs/reference-code/download.md'],9:['rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/downloads/new_download_view.rs','rust/apps/nxtools-desktop/src/downloads/settings_view.rs','rust/apps/nxtools-desktop/src/downloads/tests.rs','rust/apps/nxtools-desktop/src/downloads/directory_picker.rs','rust/apps/nxtools-desktop/src/downloads/directory_picker_tests.rs','docs/reference-code/download.md']}
extra={3:['rust/apps/nxtools-desktop/src/ui/components/action_name_tests.rs'],4:['rust/crates/nxtools-archive/tests/prompt_display_contract.rs'],10:['rust/apps/nxtools-desktop/src/ui/history_identity_tests.rs']}
status={i:('PARENT_ACCEPTED_BOUNDED_S_C_Y'if i in [1,2,3,4,5,9,10]else'PARTIAL_OPEN'if i==6 else'NOT_REPAIRED'if i==7 else'IMPLEMENTED_BOUNDED_NOT_GLOBAL_ACCEPTED')for i in mapping}
entries=[]
all_tasks=list(mapping.values())+['10-25-extensions-compile-closeout','10-25-archive-settings-search-diagnosis','10-26-extension-preview-cancel-diagnosis','10-26-rem-new-download-basic','10-26-rem-new-download-overlays','10-26-rem-download-settings-task','10-26-rem-shared-ui','10-26-rem-pages-global-tools','10-26-rem-quick-clipboard','10-26-rem-archive-final-matrix','10-26-quick-source-release-chain','10-26-final-convergence-admission']
for name in all_tasks:
 t=root/'.trellis/tasks'/name;docs=[rec(t/n)for n in ['prd.md','outcomes.md','parent-review.md','parent-review-r1-r2.md','role-matrix.md','inventory.md','command-matrix.md']if(t/n).is_file()]
 candidates=[t/'evidence/own-deltas.json',t/'evidence/own-delta.json'];delta=next((p for p in candidates if p.is_file()),None)
 ids=[i for i,n in mapping.items()if n==name];files=delta_paths(json.loads(delta.read_text(encoding='utf-8-sig')))if delta else []
 for i in ids:files+=manual.get(i,[])+extra.get(i,[])
 entries.append({'task':name,'original_findings':ids or ([6]if name.startswith('10-26-rem-')else[]),'status':status[ids[0]]if ids else'READ_OWNING_OUTCOME_FOR_PARTIAL_OR_AUXILIARY_STATUS','documents':docs,'delta_index':rec(delta)if delta else None,'delta_availability_note':('Early #1/#9 raw deltas were communicated from the historical implementer scratch; project PRD/outcome/parent review contains file/hash evidence. No current local raw-delta link is invented.'if ids and ids[0]in[1,9]else None),'actual_changed_files_current_hashes':[rec(root/p)for p in sorted(set(files))],'current_hash_caution':'Current source hashes include subsequent authorized deltas and are not substituted into old manifests. Original delta index controls attribution.'})
logs=json.loads((root/'.trellis/tasks/10-26-final-convergence-admission/evidence/failure-log-hashes.json').read_text(encoding='utf-8-sig'))
for x in logs:
 assert rec(root/x['path'])['sha256']==x['sha256']
result={'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_of_initial_problem_descriptions':'Original per-item PRDs and accepted parent reviews; no standalone verbatim initial thirteen-item transcript found in inspected project docs','official_guide_urls':['https://gpui-kit.com/docs/design-guides/','https://gpui-kit.com/docs/coding-guides/'],'historical_parent_accepted_findings':[1,2,3,4,5,9,10],'later_bounded_implementations_not_new_global_acceptance':[8,11,12,13],'open_findings':[6,7],'final_integrated_attempts':0,'report_only_no_new_implementation':True,'tasks':entries,'historical_command_logs':logs}
dest=ev/'source-index.json';assert not dest.exists();dest.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');(ev/'index.py').write_bytes(Path(__file__).read_bytes())
print('indexed tasks',len(entries),'changed-file rows',sum(len(x['actual_changed_files_current_hashes'])for x in entries),'original logs',len(logs))
# Preliminary path check; report-owned checks.json is not generated until actual commands finish.
missing=[]
for name in ['2026-10-26-gpui-guides-final-review.md','2026-10-26-gpui-guides-summary-zh.md']:
 p=root/'docs/audits'/name
 for target in re.findall(r'\]\(([^)]+)\)',p.read_text(encoding='utf-8')):
  if target.startswith('https:'):continue
  resolved=re.sub(r':\d+(?::\d+)?$','',target.strip('<>'))
  q=Path(resolved)
  if not q.is_file() and not target.endswith('/evidence/checks.json'):missing.append({'report':name,'target':target})
print(json.dumps({'missing_local_links':missing},indent=2))
