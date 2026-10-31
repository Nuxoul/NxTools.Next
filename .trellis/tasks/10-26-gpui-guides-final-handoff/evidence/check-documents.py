from pathlib import Path
import datetime,hashlib,json,re,subprocess
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-gpui-guides-final-handoff';ev=task/'evidence';scratch=Path(__file__).parent
reports=[root/'docs/audits/2026-10-26-gpui-guides-final-review.md',root/'docs/audits/2026-10-26-gpui-guides-summary-zh.md']
assert not (ev/'checks.json').exists(),'Do not overwrite checks'
records=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(name,args,expected=0):
 log=ev/(name+'.log');assert not log.exists();started=datetime.datetime.now(datetime.timezone.utc).isoformat()
 p=subprocess.run(args,cwd=root,capture_output=True);log.write_bytes(p.stdout+p.stderr)
 rec={'name':name,'command':subprocess.list2cmdline(args),'exit':p.returncode,'expected_exit':expected,'passed':p.returncode==expected,'started_utc':started,'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'log':log.relative_to(root).as_posix(),'sha256':sha(log),'scope':'DOCUMENTATION_ONLY_NOT_PRODUCT_VERIFICATION'};records.append(rec)
 print(name,'exit',p.returncode,'expected',expected)
 return p
base=json.loads((ev/'workspace-baseline.json').read_text())['files'];moved=[]
for x in base:
 p=root/x['path']
 if not p.is_file() or sha(p)!=x['sha256']:moved.append(x['path'])
assert not moved,('Original workspace movement: stop/report, never revert',moved)
index=json.loads((ev/'source-index.json').read_text());indexed=0
for t in index['tasks']:
 for x in t['documents']+t['actual_changed_files_current_hashes']+([t['delta_index']]if t['delta_index']else[]):assert sha(root/x['path'])==x['sha256'];indexed+=1
for x in index['historical_command_logs']:assert sha(root/x['path'])==x['sha256'];indexed+=1
links=[]
for report in reports:
 text=report.read_text(encoding='utf-8');assert text.endswith('\n')
 assert not any(line.rstrip()!=line for line in text.splitlines()),report
 for target in re.findall(r'\]\(([^)]+)\)',text):
  if target.startswith(('https://','http://')):continue
  value=target.strip('<>');coordinate=re.search(r':(\d+)(?::(\d+))?$',value);plain=value[:coordinate.start()]if coordinate else value
  p=Path(plain);assert p.is_absolute() and p.resolve().is_relative_to(root.resolve()),target
  if p==ev/'checks.json':links.append({'report':report.relative_to(root).as_posix(),'target':target,'state':'GENERATED_AT_END_AND_THEN_VERIFIED'});continue
  assert p.is_file(),target
  if coordinate:assert 1<=int(coordinate.group(1))<=len(p.read_text(encoding='utf-8-sig').splitlines()),target
  links.append({'report':report.relative_to(root).as_posix(),'target':target,'state':'EXISTS'})
english=reports[0].read_text(encoding='utf-8');assert re.findall(r'(?m)^### #(\d+) —',english)==[str(i)for i in range(1,14)]
assert 'Final-stage complete Desktop attempts: **0**'in english and 'historical accepted count remains 7/13'in english
struct={'status':'PASS','reports':len(reports),'local_links':len(links),'original_git_visible_files_unchanged':len(base),'indexed_records_hash_checked':indexed,'original_findings':13,'no_old_manifest_or_product_changes':True,'links':links,'scope':'REPORT_LINK_SOURCE_HISTORY_INTEGRITY_ONLY'}
(ev/'document-structure.json').write_text(json.dumps(struct,indent=2)+'\n',encoding='utf-8')
run('spec-contracts',['python','tests/architecture/spec-contracts.py'])
run('document-scoped-git-diff-check',['git','diff','--check','--',*[p.relative_to(root).as_posix()for p in reports]])
# The reports are new/untracked, so also compare each one with a fresh empty scratch file.
empty=scratch/'empty-document.txt';assert not empty.exists();empty.write_bytes(b'')
for i,report in enumerate(reports,1):
 run(f'document-{i}-no-index-whitespace',['git','-c','core.autocrlf=false','diff','--no-index','--check','--',str(empty),str(report)])
 run(f'document-{i}-full-new-file-diff',['git','-c','core.autocrlf=false','diff','--no-index','--',str(empty),str(report)],expected=1)
(ev/'check-documents.py').write_bytes(Path(__file__).read_bytes())
output={'status':'PASS'if all(x['passed']for x in records)else'FAIL','purpose':'FINAL_REPORT_ONLY','product_Rust_native_full_Desktop_invocations':0,'structure':{k:v for k,v in struct.items()if k!='links'},'commands':records,'expected_diff_exit_note':'git diff --no-index returns1 for the intentional empty-to-new-report difference; whitespace checks separately require0. This is not a hidden test failure.'}
(ev/'checks.json').write_text(json.dumps(output,indent=2)+'\n',encoding='utf-8')
for link in links:
 value=re.sub(r':\d+(?::\d+)?$','',link['target'].strip('<>'));assert Path(value).is_file(),link
print(json.dumps({k:v for k,v in output.items()if k!='commands'},indent=2))
if output['status']!='PASS':raise SystemExit(1)
