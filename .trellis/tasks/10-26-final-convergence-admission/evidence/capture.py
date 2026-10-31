from pathlib import Path
import datetime,hashlib,json,re,subprocess
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-final-convergence-admission';ev=task/'evidence'
assert not task.exists(),'Do not overwrite an admission record'
ev.mkdir(parents=True)
def sha(data):return hashlib.sha256(data).hexdigest()
def dump(path,obj):
 assert not path.exists(),str(path)
 path.write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
def record(p):return {'path':p.relative_to(root).as_posix(),'sha256':sha(p.read_bytes()),'bytes':p.stat().st_size}
previous=root/'.trellis/tasks/10-26-quick-source-release-chain/evidence'
base=json.loads((previous/'baseline-manifest.json').read_text())['files']
source_paths=[root/x['path']for x in base]+[root/'tests/architecture/quick-launch-release-chain.ps1',root/'tests/architecture/quick-launch-release-chain-tests.ps1']
source=[record(p)for p in source_paths]
expected={x['path']:x['sha256']for x in base}
for x in json.loads((previous/'own-deltas.json').read_text()):expected[x['path']]=x['after_sha256']
movement=[x['path']for x in source if x['sha256']!=expected[x['path']]]
# Old evidence, including all accepted parent reviews and prior failure logs, is protected byte-for-byte.
history=[]
for folder in sorted((root/'.trellis/tasks').iterdir()):
 if folder==task or not folder.is_dir():continue
 if folder.name.startswith(('10-24-download-enter','10-25-archive-prompt','10-25-archive-settings','10-25-button-accessible','10-25-clipboard-stable','10-25-default-submit','10-25-download-picker','10-25-download-sort-keyboard','10-25-extensions-compile','10-25-rem-','10-26-extension-preview','10-26-rem-','10-26-cursor-semantics','10-26-command-naming','10-26-download-virtual-projection','10-26-download-detail-async','10-26-quick-retained-text','10-26-quick-source-release-chain')):
  history.extend(record(p)for p in sorted(folder.rglob('*'))if p.is_file())
logs=[
 '.trellis/tasks/10-25-rem-role-consistency/evidence/desktop-full-once.log',
 '.trellis/tasks/10-25-rem-role-consistency/evidence/desktop-full-once.json',
 '.trellis/tasks/10-25-rem-role-consistency/evidence/main-window-related.log',
 '.trellis/tasks/10-26-command-naming/evidence/logs/main-window-related.log',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/workspace-fmt-02.log',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/desktop-strict-production.log',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/desktop-strict-production.json',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/desktop-strict-tests-final.log',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/desktop-strict-tests-final.json',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/workspace-check-final.log',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/workspace-check-final.json',
 '.trellis/tasks/10-26-quick-retained-text/evidence/commands/baseline-blocker-attribution.json',
 '.trellis/tasks/10-26-quick-source-release-chain/evidence/commands/quick-full-source-final.log',
 '.trellis/tasks/10-26-quick-source-release-chain/evidence/commands/release-chain-final.log',
 '.trellis/tasks/10-26-quick-source-release-chain/evidence/commands/check-results.json']
records=[]
for rel in logs:
 p=root/rel;raw=p.read_bytes();encoding='utf-16'if raw.startswith((b'\xff\xfe',b'\xfe\xff'))else'utf-8-sig';text=raw.decode(encoding,errors='replace')
 signals=[line for line in text.splitlines()if any(term in line for term in ['test result:','panicked at','error:','warning:','QL-11 custom commands','100 assertions','Finished `','Diff in '])]
 records.append({**record(p),'encoding':encoding,'evidence_level':'HISTORICAL_NOT_RERUN','signals':signals})
now=datetime.datetime.now(datetime.timezone.utc).isoformat()
dump(ev/'source-baseline.json',{'created_utc':now,'basis':'live dirty bytes; hash-only baseline, no product edits','files':source,'movement_since_latest_queue16_delivery':movement})
dump(ev/'historical-protection.json',{'created_utc':now,'files':history})
dump(ev/'failure-log-hashes.json',records)
(ev/'git-status.txt').write_bytes(subprocess.run(['git','status','--short'],cwd=root,capture_output=True,check=True).stdout)
(ev/'capture.py').write_bytes(Path(__file__).read_bytes())
result={'status':'NOT_ADMITTED_INCOMPLETE_PREREQUISITE_REMEDIATION','created_utc':now,'blocking_implementation':['Queue08 partial; Extensions rem and consumer verification incomplete','Queue09 Quick/Clipboard ordinary rem not implemented','Queue10 Archive/residual ordinary rem not implemented; original #6 OPEN','Queue11 original #7 cursor not repaired'],'other_unpassed':['workspace fmt latest FAIL four files; no fresh invocation this phase','whole QuickLaunch source gate latest FAIL QL-11 line458','original #8 broad MainWindow206PASS6FAIL remains historical unresolved integration evidence'],'latest_compile_evidence':'prior #13 test-strict and workspace check PASS; no current compile failure asserted; no fresh build performed','final_stage_rust_invocations':0,'final_stage_full_desktop_attempts':0,'full_desktop_passed':None,'full_desktop_failed':None,'full_desktop_ignored':None,'full_desktop_exit':None,'full_desktop_state':'NOT RUN / allowance not consumed','live_source_files':len(source),'source_movement_since_queue16':movement,'historical_files':len(history),'evidence_log_records':len(records)}
dump(ev/'admission-result.json',result)
print(json.dumps(result,indent=2))
for x in records:
 print(x['path'],x['sha256'])
 for line in x['signals']:
  if 'test result:'in line or '100 assertions'in line:print('  '+line)
