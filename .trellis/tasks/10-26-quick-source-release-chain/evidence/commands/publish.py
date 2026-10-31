from pathlib import Path
import base64,datetime,difflib,hashlib,json,re
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-quick-source-release-chain';ev=task/'evidence';scratch=Path(__file__).parent
owned='tests/architecture/quick-launch-lifecycle.ps1';new=['tests/architecture/quick-launch-release-chain.ps1','tests/architecture/quick-launch-release-chain-tests.ps1']
assert not (ev/'own-deltas.json').exists()
def sha(data):return hashlib.sha256(data).hexdigest()
def put(path,data):
 assert not path.exists(),str(path)
 path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
def dump(path,data):put(path,(json.dumps(data,indent=2)+'\n').encode())
base=json.loads((ev/'baseline-manifest.json').read_text())['files'];by={x['path']:x for x in base};history=json.loads((ev/'historical-manifest.json').read_text())['files']
protected=[]
for x in base:
 assert sha((task/x['snapshot']).read_bytes())==x['sha256'],x['path']
 if x['path']!=owned:
  assert sha((root/x['path']).read_bytes())==x['sha256'],'Unexpected movement '+x['path'];protected.append(x)
for x in history:assert sha((root/x['path']).read_bytes())==x['sha256'],'Historical movement '+x['path']
before=(task/by[owned]['snapshot']).read_bytes();after=(root/owned).read_bytes();a=before.decode();b=after.decode()
loader="$publisherPath = Join-Path $root 'scripts\\publish-release-hosts.ps1'\n$publisherSource = Get-Content -Raw -LiteralPath $publisherPath\n. (Join-Path $root 'tests\\architecture\\quick-launch-release-chain.ps1')\n"
old_condition="    $buildScriptSource -notmatch 'write-release-binary-manifest\\.ps1' -or"
new_condition='    -not (Test-QuickLaunchReleaseChain $buildScriptSource $publisherSource $manifestWriterSource) -or'
assert b.replace(loader,'').replace(new_condition,old_condition)==a,'Only exact minimal existing-gate delta is allowed'
start="if ($protocolSource -notmatch 'ElevatedDirectoryHelperRequested'"
def predicates(text):
 region=start+text.split(start,1)[1].split("    throw @'",1)[0]
 return re.findall(r"\$(\w+)\s+(-notmatch|-match)\s+'([^']*)'",region)
previous=predicates(a);current=predicates(b);previous.remove(('buildScriptSource','-notmatch',r'write-release-binary-manifest\.ps1'));assert previous==current and len(current)==73
matrix=json.loads((scratch/'original-compound-matrix.json').read_text(encoding='utf-8-sig'));assert len(matrix)==74 and sum(x['violated']for x in matrix)==1
assert 'Quick-launch elevated-navigation regression' in (scratch/'release-chain-red.log').read_text()
assert '100 assertions; 26 broken chains + 73 retained security predicates' in (scratch/'release-chain-final.log').read_text()
results=json.loads((scratch/'check-results.json').read_text(encoding='utf-8-sig'));assert len(results)==11
assert [x['name']for x in results if x['exit']!=0]==['quick-full-source-final']
assert 'QL-11 custom commands' in (scratch/'quick-full-source-final.log').read_text()
source_results=json.loads((scratch/'final-source-results.json').read_text(encoding='utf-8-sig'));assert all(x.get('parse')=='PASS'for x in source_results[:3]);assert source_results[3]['expected_rejection']=='PASS'
for p in new:
 text=(root/p).read_text();assert not any(line.rstrip()!=line for line in text.splitlines()),p
checks=[{'name':'exact minimal existing gate inverse restores dirty baseline','result':'PASS'}, {'name':'all73original safety predicate tuples/order unchanged','result':'PASS'}, {'name':'original compound throw and all unrelated gate statements preserved','result':'PASS'}, {'name':'629protected files including product/build/publish/writer/security/generated sources unchanged','result':'PASS'}, {'name':'4778historical evidence files unchanged','result':'PASS'}]
left=before.splitlines(keepends=True);right=after.splitlines(keepends=True);ops=[]
for kind,i,j,k,l in difflib.SequenceMatcher(a=left,b=right,autojunk=False).get_opcodes():
 if kind!='equal':ops.append({'old_start':i,'old_end':j,'new_start':k,'new_end':l,'old':base64.b64encode(b''.join(left[i:j])).decode(),'new':base64.b64encode(b''.join(right[k:l])).decode()})
f=left.copy();v=right.copy()
for op in reversed(ops):
 f[op['old_start']:op['old_end']]=base64.b64decode(op['new']).splitlines(keepends=True)
 v[op['new_start']:op['new_end']]=base64.b64decode(op['old']).splitlines(keepends=True)
assert b''.join(f)==after and b''.join(v)==before
dest='evidence/current/'+owned;put(task/dest,after)
deltas=[{'path':owned,'before_sha256':sha(before),'after_sha256':sha(after),'after_snapshot':dest,'ops':ops}]
put(ev/'deltas/quick-launch-lifecycle.txt',''.join(difflib.unified_diff(a.splitlines(True),b.splitlines(True),fromfile='live-dirty-baseline/'+owned,tofile='current/'+owned)).encode())
for p in new:
 data=(root/p).read_bytes();dest='evidence/current/'+p;put(task/dest,data);deltas.append({'path':p,'new_file':True,'after_sha256':sha(data),'after_snapshot':dest})
dump(ev/'own-deltas.json',deltas);dump(ev/'protected-manifest.json',protected);dump(ev/'owner-checks.json',checks)
for p in sorted(scratch.iterdir()):
 if p.is_file():put(ev/'commands'/p.name,p.read_bytes())
put(ev/'commands/queue16-baseline.py',(scratch.parent/'queue16-baseline.py').read_bytes())
summary={'status':'TARGET_SOURCE_GATE_REPAIRED_WHOLE_GATE_BLOCKED_BY_PREEXISTING_QL11','original_compound_predicates':74,'original_violations':1,'retained_security_predicates':73,'source_only_assertions':100,'chain_mutations_rejected':26,'protected_files':len(protected),'historical_files':len(history),'existing_deltas':1,'new_test_files':2,'related_checks':results,'PSScriptAnalyzer':'NOT INSTALLED','Rust_CSharp_full_desktop_native_package':'NOT RUN'}
dump(ev/'verification-summary.json',summary);print(json.dumps(summary,indent=2))
