import hashlib,json,pathlib
root=pathlib.Path.cwd(); ev=root/'.trellis/tasks/10-25-default-submit-keyboard/evidence'
def digest(b):return hashlib.sha256(b).hexdigest()
def raw(p):return p.read_bytes()
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def ensure(b,msg):
    if not b:raise AssertionError(msg)
m=load(ev/'delivery-manifest.json'); ds=load(ev/'own-deltas.json'); result={}
for name,want in [('delivery-manifest.json','52cb1160a4536aae54adb064ab2cdb0436ae77c6e44ec3a6502931f4ae4e2ab4'),('own-deltas.json','f6a8d678e638d21da6bf4c33f1261a4e2cc54e201ad25f844ec6fd9f0f7aed8c')]:ensure(digest(raw(ev/name))==want,name)
for x in m['existing']:ensure(digest(raw(root/x['path']))==x['current_sha256'],'current '+x['path'])
for x in m['new']+m['observed_unmodified']:ensure(digest(raw(root/x['path']))==x['sha256'],'new/observed '+x['path'])
for x in m['evidence']:ensure(digest(raw(ev/x['file']))==x['sha256'],'evidence '+x['file'])
result['manifest_evidence_hashes']=len(m['evidence']);result['live_existing']=len(m['existing']);result['live_new']=len(m['new'])
def replay(d,base,after):
    ensure(digest(base)==d['baseline_sha256'],'delta base '+d['path']);ensure(digest(after)==d['after_sha256'],'delta after '+d['path'])
    beforelines=base.decode('utf-8').splitlines(keepends=True); afterlines=after.decode('utf-8').splitlines(keepends=True)
    for o in d['operations']:
        ensure(''.join(beforelines[o['baseline_start']:o['baseline_end']])==o['baseline_text'],'base span '+d['path'])
        ensure(''.join(afterlines[o['after_start']:o['after_end']])==o['after_text'],'after span '+d['path'])
    forward=beforelines.copy();reverse=afterlines.copy()
    for o in reversed(d['operations']):forward[o['baseline_start']:o['baseline_end']]=o['after_text'].splitlines(keepends=True)
    for o in reversed(d['operations']):reverse[o['after_start']:o['after_end']]=o['baseline_text'].splitlines(keepends=True)
    ensure(''.join(forward).encode()==after,'forward '+d['path']);ensure(''.join(reverse).encode()==base,'reverse '+d['path'])
for d in ds:
    base=raw(ev/(d['path'].replace('/','__')+'.baseline'))
    after=raw(ev/'archive-reference.own-after-before-external.md.txt') if d['path']=='docs/reference-code/archive.md' else raw(root/d['path'])
    replay(d,base,after)
    if d['path'].endswith(('forms_tests.rs','job_tests.rs','polish_tests.rs')):ensure(after.startswith(base),'original tests prefix '+d['path'])
result['bidirectional_own_deltas']=len(ds);result['test_prefixes']=3
replay(load(ev/'archive-reference.external-append.json'),raw(ev/'archive-reference.own-after-before-external.md.txt'),raw(root/'docs/reference-code/archive.md'));result['external_append_separate']=True
for name in ['protected-manifest.json','item04-frozen.json']:
    entries=load(ev/name)
    for x in entries:ensure(digest(raw(root/x['path']))==x['sha256'],name+' '+x['path'])
    result[name]=len(entries)
probe=raw(ev/'original-restored-probe-evidence.rs.txt').decode('utf-8').replace('audit04_create_keyboard_release_and_missing_object_cancel_are_once_only','audit05_create_keyboard_release_and_missing_object_cancel_are_once_only')
ensure(probe.strip() in raw(root/'rust/apps/nxtools-desktop/src/archive/job_tests.rs').decode('utf-8'),'complete restored probe');result['complete_probe_only_name_changed']=True
print(json.dumps(result,indent=2));(pathlib.Path(__file__).parent/'integrity-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
import re
for inv in load(ev/'owner-invariants.json'):
    base=raw(ev/(inv['path'].replace('/','__')+'.baseline')).decode('utf-8')
    live=raw(root/inv['path']).decode('utf-8')
    def owner_part(text):return re.sub(r'\s+','',text[text.index(inv['begin']):text.index(inv['end'],text.index(inv['begin']))])
    ensure(owner_part(base)==owner_part(live),'unchanged original owner '+inv['path'])
result['unchanged_original_owner_sections']=3
logbytes=raw(ev/'desktop-full-once.log')
fulltext=logbytes.decode('utf-16') if logbytes.startswith((b'\xff\xfe',b'\xfe\xff')) else logbytes.decode('utf-8-sig')
ensure(re.search(r'test result: ok\. 1232 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 258\.96s',fulltext),'worker full log result')
result['worker_full_log_verified_not_parent_run']='1232/1232; zero ignored; 258.96s'
print(json.dumps(result,indent=2));(pathlib.Path(__file__).parent/'integrity-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
