from pathlib import Path
import base64,datetime,difflib,hashlib,json,re,shutil
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-download-virtual-projection';ev=task/'evidence';scratch=Path(__file__).parent
owned=['docs/reference-code/download.md','rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/downloads/floating_assistant.rs','rust/apps/nxtools-desktop/src/main_window.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/download_average_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs','rust/apps/nxtools-desktop/src/main_window/tests.rs']
new=['rust/apps/nxtools-desktop/src/main_window/download_page/projection_tests.rs']
assert not (ev/'own-deltas.json').exists(),'Write-once publication'
def sha(data):return hashlib.sha256(data).hexdigest()
def put(path,data):
 assert not path.exists(),str(path)
 path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
def dump(path,obj):put(path,(json.dumps(obj,indent=2)+'\n').encode())
base=json.loads((ev/'baseline-manifest.json').read_text())['files'];by={x['path']:x for x in base}
history=json.loads((ev/'historical-manifest.json').read_text())['files']
for x in history:assert sha((root/x['path']).read_bytes())==x['sha256'],x['path']
deltas=[]
for rel in owned:
 old=(task/by[rel]['snapshot']).read_bytes();cur=(root/rel).read_bytes();a=old.splitlines(keepends=True);b=cur.splitlines(keepends=True)
 ops=[]
 for tag,i,j,k,l in difflib.SequenceMatcher(a=a,b=b,autojunk=False).get_opcodes():
  if tag!='equal':ops.append({'old_start':i,'old_end':j,'new_start':k,'new_end':l,'old':base64.b64encode(b''.join(a[i:j])).decode(),'new':base64.b64encode(b''.join(b[k:l])).decode()})
 assert ops,rel
 forward=a.copy()
 for op in reversed(ops):forward[op['old_start']:op['old_end']]=base64.b64decode(op['new']).splitlines(keepends=True)
 reverse=b.copy()
 for op in reversed(ops):reverse[op['new_start']:op['new_end']]=base64.b64decode(op['old']).splitlines(keepends=True)
 assert b''.join(forward)==cur and b''.join(reverse)==old
 after='evidence/current/'+rel;put(task/after,cur)
 diff=''.join(difflib.unified_diff(old.decode('utf-8').splitlines(True),cur.decode('utf-8').splitlines(True),fromfile='live-dirty-baseline/'+rel,tofile='own-current/'+rel))
 put(ev/'deltas'/(rel.replace('/','__')+'.txt'),diff.encode())
 deltas.append({'path':rel,'before_sha256':sha(old),'after_sha256':sha(cur),'after_snapshot':after,'ops':ops})
for rel in new:
 cur=(root/rel).read_bytes();after='evidence/current/'+rel;put(task/after,cur);deltas.append({'path':rel,'new_file':True,'after_sha256':sha(cur),'after_snapshot':after})
dump(ev/'own-deltas.json',deltas)
protected=[];unowned=[]
for x in base:
 rel=x['path'];cur=(root/rel).read_bytes()
 if rel in owned:continue
 if sha(cur)==x['sha256']:protected.append(x)
 else:
  assert rel=='Cargo.lock',f'Unexpected source movement: {rel}'
  after='evidence/unowned/'+rel;put(task/after,cur)
  old=(task/x['snapshot']).read_bytes();diff=''.join(difflib.unified_diff(old.decode().splitlines(True),cur.decode().splitlines(True)))
  assert diff.count('+ "flate2",')==1 and len(diff.splitlines())==10,diff
  unowned.append({'path':rel,'before_sha256':x['sha256'],'observed_sha256':sha(cur),'snapshot':after,'attribution':'Toolchain/unowned lock reconciliation; existing manifest already declares flate2; initiating process unproven'})
dump(ev/'protected-manifest.json',protected);dump(ev/'unowned-observations.json',unowned)
# Exact production ownership/callback regions, not string-only behavioral proof.
page='rust/apps/nxtools-desktop/src/main_window/download_page.rs';old=(task/by[page]['snapshot']).read_text(encoding='utf-8');cur=(root/page).read_text(encoding='utf-8')
checks=[]
def same(name,a,b):assert a==b,name;checks.append({'name':name,'sha256':sha(a.encode()),'result':'PASS'})
def region(text,start,end):return text.split(start,1)[1].split(end,1)[0]
same('row owned display/callback/layout body',region(old,'    let DownloadRowSelection {','fn download_aggregate_progress(').replace('download_task_name(&task)','download_task_name(task)').replace('live_progress.as_ref(),','live_progress,'),region(cur,'    let DownloadRowSelection {','fn download_aggregate_progress('))
same('all context/batch/detail/header/persist functions',old.split('fn download_aggregate_progress(',1)[1].split('\n#[cfg(test)]\n#[path = "download_page/kit_migration_tests.rs"]',1)[0],cur.split('fn download_aggregate_progress(',1)[1].split('\n#[cfg(test)]\n#[path = "download_page/kit_migration_tests.rs"]',1)[0])
same('selected detail deferred lifecycle',region(old,'        if let Some(task_id) = selected_task_id {','        let detail_tab ='),region(cur,'        if let Some(task_id) = selected_task_id {','        let detail_tab ='))
same('production controls/list/footer geometry',region(old,'        let filter_width =','fn download_bounded_tooltip('),region(cur,'        let filter_width =','fn download_bounded_tooltip('))
for name in ['download_task_matches_search','download_status_sort_rank','reconcile_download_selection','update_download_selection','prepare_download_context_selection','download_context_capabilities','download_queue_placement_capabilities']:
 pat=rf'(?m)^pub\(crate\) fn {name}\(';m=re.search(pat,old);n=re.search(pat,cur);assert m and n,name
 end_a=re.search(r'(?m)^(?:pub\(crate\) )?fn |^impl ',old[m.end():]);end_b=re.search(r'(?m)^(?:pub\(crate\) )?fn |^impl ',cur[n.end():]);assert end_a and end_b
 same(name,old[m.start():m.end()+end_a.start()],cur[n.start():n.end()+end_b.start()])
ref='docs/reference-code/download.md';assert (root/ref).read_bytes().startswith((task/by[ref]['snapshot']).read_bytes());checks.append({'name':'reference append-only baseline prefix','result':'PASS'})
for rel in ['rust/apps/nxtools-desktop/src/main_window/tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_average_tests.rs']:
 before=(task/by[rel]['snapshot']).read_text(encoding='utf-8');after=(root/rel).read_text(encoding='utf-8');assert before.count('assert')==after.count('assert');checks.append({'name':'existing assertion count retained '+rel,'result':'PASS'})
dump(ev/'owner-checks.json',checks)
# Preserve all command logs, attempted commands and scripts; no old log is overwritten.
for p in sorted(scratch.iterdir()):
 if p.is_file():put(ev/'commands'/p.name,p.read_bytes())
results={}
for name in ['audit11-red-01','audit11-red-10000','audit11-green-02','audit11-five-final','download-page-related','download-main-related','downloads-related','protected-related','final-alias-and-identity']:
 text=(scratch/(name+'.log')).read_text(encoding='utf-8',errors='replace');summary=re.findall(r'test result:.*',text);assert summary,name;results[name]=summary[-1]
assert '5 passed; 0 failed' in results['audit11-five-final']
assert '23 passed; 0 failed' in results['final-alias-and-identity']
dump(ev/'verification-summary.json',{'results':results,'test_strict_final':'PASS','production_strict':'FAIL baseline unused SPACING_12 / DecodedMedia.animated','workspace_check':'PASS','workspace_fmt':'FAIL five baseline-identical files','source_gates':'11 PASS','full_desktop':'NOT RUN','native_visual_release':'NOT RUN','history_files':len(history),'protected_files':len(protected),'owner_checks':len(checks)})
print(json.dumps({'deltas':len(deltas),'protected':len(protected),'history':len(history),'owner_checks':len(checks),'unowned':unowned},indent=2))
