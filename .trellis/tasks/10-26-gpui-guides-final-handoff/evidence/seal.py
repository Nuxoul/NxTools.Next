from pathlib import Path
import subprocess,json,hashlib,datetime
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-gpui-guides-final-handoff';ev=task/'evidence';scratch=Path(__file__).parent
reports=['docs/audits/2026-10-26-gpui-guides-final-review.md','docs/audits/2026-10-26-gpui-guides-summary-zh.md'];taskdoc=task/'outcomes.md'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def command(name,args,expected=0,empty=False):
 p=subprocess.run(args,cwd=root,capture_output=True);data=p.stdout+p.stderr;log=ev/(name+'.log');assert not log.exists();log.write_bytes(data);assert p.returncode==expected and(not empty or not data),(args,p.returncode,data.decode('utf-8',errors='replace'));return {'command':subprocess.list2cmdline(args),'exit':p.returncode,'expected_exit':expected,'log':log.relative_to(root).as_posix(),'sha256':sha(log)}
base=json.loads((ev/'workspace-baseline.json').read_text())['files']
for x in base:assert(root/x['path']).is_file()and sha(root/x['path'])==x['sha256'],x['path']
spec=command('spec-contracts-handoff-final',['python','tests/architecture/spec-contracts.py'])
diff=command('document-scope-handoff-final',['git','diff','--check','--',*reports,taskdoc.relative_to(root).as_posix()],empty=True)
newdoc=command('handoff-outcome-whitespace',['git','-c','core.autocrlf=false','diff','--no-index','--check','--',str(scratch/'empty-document.txt'),str(taskdoc)],expected=1,empty=True)
status=subprocess.run(['git','status','--short'],cwd=root,capture_output=True,check=True);(ev/'git-status-after.txt').write_bytes(status.stdout)
visible=subprocess.run(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=root,capture_output=True,check=True).stdout.decode('utf-8').split('\0');old={x['path']for x in base};new=sorted(p for p in set(visible)-old if p and(root/p).is_file())
assert all(p in reports or p.startswith('.trellis/tasks/10-26-gpui-guides-final-handoff/')for p in new),new
checks={'status':'PASS_DOCUMENT_HANDOFF_ONLY','spec_contracts_exit':spec['exit'],'document_scope_exit':diff['exit'],'commands':[spec,diff,newdoc],'protected_existing_files':len(base),'new_files_only_reports_and_this_task':True,'new_files_at_observation':new,'full_Desktop_Rust_product_native_invocations':0}
(ev/'handoff-final-checks.json').write_text(json.dumps(checks,indent=2)+'\n',encoding='utf-8');(ev/'seal.py').write_bytes(Path(__file__).read_bytes())
manifest=ev/'delivery-manifest.json';assert not manifest.exists()
files=[root/p for p in reports]+[p for p in sorted(task.rglob('*'))if p.is_file()]
rows=[{'path':p.relative_to(root).as_posix(),'sha256':sha(p),'bytes':p.stat().st_size}for p in files]
manifest.write_text(json.dumps({'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'DOCUMENTATION_HANDOFF_COMPLETE_PROJECT_REMEDIATION_INCOMPLETE','files':rows},indent=2)+'\n',encoding='utf-8')
print('delivery_files',len(rows),'manifest_sha256',sha(manifest))
for p in [root/x for x in reports]:print(p.name,sha(p))
