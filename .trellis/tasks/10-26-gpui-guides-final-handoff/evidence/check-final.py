from pathlib import Path
import datetime,hashlib,json,re,subprocess
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-gpui-guides-final-handoff';ev=task/'evidence';scratch=Path(__file__).parent
reports=[root/'docs/audits/2026-10-26-gpui-guides-final-review.md',root/'docs/audits/2026-10-26-gpui-guides-summary-zh.md'];records=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(name,args,expected=0,empty=False):
 log=ev/(name+'.log');assert not log.exists();started=datetime.datetime.now(datetime.timezone.utc).isoformat();p=subprocess.run(args,cwd=root,capture_output=True);data=p.stdout+p.stderr;log.write_bytes(data)
 row={'name':name,'command':subprocess.list2cmdline(args),'exit':p.returncode,'expected_exit':expected,'output_empty_required':empty,'passed':p.returncode==expected and(not empty or not data),'started_utc':started,'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':log.relative_to(root).as_posix(),'sha256':sha(log),'scope':'FINAL_DOCUMENT_ONLY_NOT_PRODUCT_VALIDATION'};records.append(row);print(name,'exit',p.returncode,'PASS'if row['passed']else'FAIL')
semantics=json.loads((ev/'git-no-index-semantics.json').read_text());assert semantics[0]['exit']==1 and semantics[0]['output']=='' and semantics[1]['exit']==3 and 'trailing whitespace'in semantics[1]['output']
base=json.loads((ev/'workspace-baseline.json').read_text())['files'];moved=[x['path']for x in base if not(root/x['path']).is_file()or sha(root/x['path'])!=x['sha256']];assert not moved,moved
links=[]
for report in reports:
 text=report.read_text(encoding='utf-8');assert text.endswith('\n')and not any(line.rstrip()!=line for line in text.splitlines())
 for target in re.findall(r'\]\(([^)]+)\)',text):
  if target.startswith(('http://','https://')):continue
  v=target.strip('<>');m=re.search(r':(\d+)(?::(\d+))?$',v);plain=v[:m.start()]if m else v;p=Path(plain);assert p.is_absolute()and p.resolve().is_relative_to(root.resolve()),target
  if p!=ev/'checks-final.json':
   assert p.is_file(),target
   if m:assert 1<=int(m.group(1))<=len(p.read_text(encoding='utf-8-sig').splitlines())
  links.append({'report':report.relative_to(root).as_posix(),'target':target})
english=reports[0].read_text(encoding='utf-8');assert re.findall(r'(?m)^### #(\d+) —',english)==[str(i)for i in range(1,14)]
source_index=json.loads((ev/'source-index.json').read_text());indexed=0
for item in source_index['tasks']:
 for x in item['documents']+item['actual_changed_files_current_hashes']+([item['delta_index']]if item['delta_index']else[]):assert sha(root/x['path'])==x['sha256'];indexed+=1
for x in source_index['historical_command_logs']:assert sha(root/x['path'])==x['sha256'];indexed+=1
run('spec-contracts-final',['python','tests/architecture/spec-contracts.py'])
run('document-scoped-git-diff-check-final',['git','diff','--check','--',*[p.relative_to(root).as_posix()for p in reports]],empty=True)
for i,p in enumerate(reports,1):
 run(f'document-{i}-no-index-whitespace-final',['git','-c','core.autocrlf=false','diff','--no-index','--check','--',str(scratch/'empty-document.txt'),str(p)],expected=1,empty=True)
 run(f'document-{i}-full-new-file-diff-final',['git','-c','core.autocrlf=false','diff','--no-index','--',str(scratch/'empty-document.txt'),str(p)],expected=1)
assert all(x['passed']for x in records)
structure={'status':'PASS','reports':2,'local_links':len(links),'links':links,'original_findings':13,'indexed_records_hash_checked':indexed,'original_git_visible_files_unchanged':len(base),'excluded':'Git-ignored output/external repositories are not covered','reports_current':[{'path':p.relative_to(root).as_posix(),'sha256':sha(p),'bytes':p.stat().st_size}for p in reports]}
(ev/'document-structure-final.json').write_text(json.dumps(structure,indent=2)+'\n',encoding='utf-8')
checks={'status':'PASS_REPORT_ONLY','commands':records,'local_links':len(links),'protected_original_files':len(base),'indexed_records_hash_checked':indexed,'product_Rust_full_Desktop_native_invocations':0,'initial_attempt':'checks.json remains FAIL from erroneous expected no-index exit0; no log overwritten','controlled_semantics':'git-no-index-semantics.json proves clean intentional difference exit1/empty output, bad whitespace exit3/diagnostic; final no-index checks require1 AND empty output','full_project_acceptance':False}
assert not(ev/'checks-final.json').exists();(ev/'checks-final.json').write_text(json.dumps(checks,indent=2)+'\n',encoding='utf-8');(ev/'check-final.py').write_bytes(Path(__file__).read_bytes())
for x in links:assert Path(re.sub(r':\d+(?::\d+)?$','',x['target'].strip('<>'))).is_file()
print(json.dumps({k:v for k,v in checks.items()if k!='commands'},indent=2))
