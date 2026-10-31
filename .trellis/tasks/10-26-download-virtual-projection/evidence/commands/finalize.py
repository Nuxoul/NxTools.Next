from pathlib import Path
import hashlib,json,re
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-download-virtual-projection';ev=task/'evidence';scratch=Path(__file__).parent

def sha(data):return hashlib.sha256(data).hexdigest()
def put(path,data):
 assert not path.exists(),str(path)
 path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
def dump(path,obj):put(path,(json.dumps(obj,indent=2)+'\n').encode())
base=json.loads((ev/'baseline-manifest.json').read_text())['files'];by={x['path']:x for x in base}
page='rust/apps/nxtools-desktop/src/main_window/download_page.rs';old=(task/by[page]['snapshot']).read_text(encoding='utf-8');cur=(root/page).read_text(encoding='utf-8');checks=[]
def same(name,a,b):assert a==b,name;checks.append({'name':name,'sha256':sha(a.encode()),'result':'PASS'})
def region(text,start,end):return text.split(start,1)[1].split(end,1)[0]
same('row owned display/callback/layout body',region(old,'    let DownloadRowSelection {','fn download_aggregate_progress(').replace('download_task_name(&task)','download_task_name(task)').replace('live_progress.as_ref(),','live_progress,'),region(cur,'    let DownloadRowSelection {','fn download_aggregate_progress('))
same('all context/batch/detail/header/persist functions',old.split('fn download_aggregate_progress(',1)[1].split('\n#[cfg(test)]\n#[path = "download_page/kit_migration_tests.rs"]',1)[0],cur.split('fn download_aggregate_progress(',1)[1].split('\n#[cfg(test)]\n#[path = "download_page/kit_migration_tests.rs"]',1)[0])
same('selected detail deferred lifecycle',region(old,'        if let Some(task_id) = selected_task_id {','        let detail_tab ='),region(cur,'        if let Some(task_id) = selected_task_id {','        let detail_tab ='))
same('production controls/list/footer geometry',region(old,'        let filter_width =','fn download_bounded_tooltip('),region(cur,'        let filter_width =','fn download_bounded_tooltip('))
for name in ['download_task_matches_search','download_status_sort_rank','reconcile_download_selection','update_download_selection','prepare_download_context_selection','download_context_capabilities','download_queue_placement_capabilities']:
 pat=rf'(?m)^pub\(crate\) fn {name}\b';m=re.search(pat,old);n=re.search(pat,cur);assert m and n,name
 a=re.search(r'(?m)^(?:pub\(crate\) )?fn |^impl ',old[m.end():]);b=re.search(r'(?m)^(?:pub\(crate\) )?fn |^impl ',cur[n.end():]);assert a and b
 same(name,old[m.start():m.end()+a.start()],cur[n.start():n.end()+b.start()])
ref='docs/reference-code/download.md';assert (root/ref).read_bytes().startswith((task/by[ref]['snapshot']).read_bytes());checks.append({'name':'reference append-only baseline prefix','result':'PASS'})
for rel in ['rust/apps/nxtools-desktop/src/main_window/tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_average_tests.rs']:
 a=(task/by[rel]['snapshot']).read_text(encoding='utf-8');b=(root/rel).read_text(encoding='utf-8');assert a.count('assert')==b.count('assert');checks.append({'name':'existing assertion count retained '+rel,'result':'PASS'})
dump(ev/'owner-checks.json',checks)
for p in sorted(scratch.iterdir()):
 if p.is_file():put(ev/'commands'/p.name,p.read_bytes())
results={}
for name in ['audit11-red-01','audit11-red-10000','audit11-green-02','audit11-five-final','download-page-related','download-main-related','downloads-related','protected-related','final-alias-and-identity']:
 text=(scratch/(name+'.log')).read_text(encoding='utf-8',errors='replace');s=re.findall(r'test result:.*',text);assert s,name;results[name]=s[-1]
assert '5 passed; 0 failed' in results['audit11-five-final'];assert '23 passed; 0 failed' in results['final-alias-and-identity']
history=json.loads((ev/'historical-manifest.json').read_text())['files'];protected=json.loads((ev/'protected-manifest.json').read_text())
for x in history+protected:assert sha((root/x['path']).read_bytes())==x['sha256'],x['path']
for x in json.loads((ev/'own-deltas.json').read_text()):assert sha((root/x['path']).read_bytes())==x['after_sha256']
scoped=json.loads((scratch/'scoped-checks.json').read_text(encoding='utf-8-sig'));assert scoped['fmt']==0 and scoped['diff']==0
quality=json.loads((scratch/'strict-tests-final.json').read_text(encoding='utf-8-sig'));assert quality['exit']==0
gates=json.loads((scratch/'source-gate-results.json').read_text(encoding='utf-8-sig'));assert len(gates)==11 and all(x['exit']==0 for x in gates)
dump(ev/'verification-summary.json',{'results':results,'test_strict_final':'PASS','production_strict':'FAIL baseline unused SPACING_12 / DecodedMedia.animated','workspace_check':'PASS','workspace_fmt':'FAIL five baseline-identical files','source_gates':'11 PASS','full_desktop':'NOT RUN','native_visual_release':'NOT RUN','history_files':len(history),'protected_files':len(protected),'owner_checks':len(checks)})
print(json.dumps({'owner_checks':len(checks),'protected':len(protected),'historical':len(history),'results':results},indent=2))
