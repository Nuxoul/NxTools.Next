from pathlib import Path
import re,json,hashlib,collections,datetime
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-rem-archive-final-matrix';src=root/'rust/apps/nxtools-desktop/src'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
# Mask comments/string/character literals without changing offsets/newlines. This is a lexical
# navigation index, not a Rust parser or semantic proof. Unknown sites remain explicitly unfinished.
TOKEN=re.compile(r'//[^\n]*|/\*.*?\*/|r(?P<hash>\#*)".*?"(?P=hash)|b?"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'',re.S)
def mask(s):return TOKEN.sub(lambda m:''.join('\n' if c=='\n' else ' ' for c in m.group()),s)
def close(code,start,op,cl):
 depth=0
 for i in range(start,len(code)):
  if code[i]==op:depth+=1
  elif code[i]==cl:
   depth-=1
   if depth==0:return i
 return len(code)-1
def test_ranges(code):
 out=[]
 for m in re.finditer(r'#\[\s*cfg\s*\(\s*test\s*\)\s*\]',code):
  rest=code[m.end():]
  # Only attributes on an identifiable function/module create a body range. A cfg(test)
  # struct field must never swallow the next production impl block.
  item=re.match(r'\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|unsafe|const)\s+)*(?:fn\s+\w+|mod\s+\w+)',rest)
  if item is None:continue
  start=m.end()+item.end();brace=code.find('{',start);semi=code.find(';',start)
  end=semi if semi>=0 and (brace<0 or semi<brace) else close(code,brace,'{','}') if brace>=0 else len(code)
  out.append((m.start(),end))
 return out

def archive_class(rel,line,expr,test):
 if test:return 'TEST_ONLY','Not a production ordinary layout role; retained unchanged.'
 name=Path(rel).name
 if name in ['application.rs','jobs.rs']:
  return 'PROTECTED_NATIVE_CLIENT','Current user explicitly requires fixed Archive clients/minimum/placement. application960x640/min720x480; jobs560x300/min480x260 and forms560/680x600/min480x360. Current source, archive ui-spec9 and matching reference765-770. No ordinary role exemption implied.'
 if name=='virtual_drag_spike.rs':return 'UNFINISHED_REVIEW','Separate diagnostic/native spike, not evidence of ordinary product adoption; no launch or modification authorized.'
 if name=='browser.rs':
  if line in [2348,2349]:return 'APPROVED_FIXED_LAYOUT','Explicit tree200/clamp160..280: archive ui-spec.md:9. Public resizable owner unchanged.'
  if line in [1474,1556,1953] or line==1202:return 'UNFINISHED_PERSISTED_GEOMETRY','Current column widths/header/cells share restored/resized pixel state. Preserve state identity; no exact approved font-scaling exemption located. Requires current-Root role decision and resize/horizontal-scroll test; NOT CLOSED.'
  if line in [1954,2365]:return 'UNFINISHED_REVIEW','Column64..640 or details160 minimum: no exact approved fixed exception established; do not relabel as native geometry merely because stored in Pixels.'
  if line==2147:return 'UNFINISHED_VIEWPORT','Comment80 allocation is current implementation, not independently established approved fixed exception. Verify natural rich/plain text, bounded viewport/end within unchanged client before decision.'
  return 'UNFINISHED_ORDINARY_CANDIDATE','Actual production row/header/toolbar/vector/indent role. Requires effective Root16/14/18 red before change; original uniform_list, Tree, PageUp/Down, selection/drag and official controls unchanged.'
 if name=='forms.rs':
  if line==1653:return 'UNFINISHED_VIEWPORT','Error80 cap currently bounds original footer. Preserve full errors/scroll/footer; no exact approved permanent px exception found. Root/long text verification pending.'
  return 'UNFINISHED_ORDINARY_CANDIDATE','Actual Textarea48/160 allocation, not released vendor internal. No effective Root red yet; preserve InputState/draft/validation/default Enter/footer and original submit/cancel.'
 if name=='archive_page.rs':
  if line in [1383,1384]:return 'UNFINISHED_REVIEW','Settings sidebar160/range160..208 is current source. No exact approved fixed #6 exemption located; official Settings owns resize/selection, current main client protected.'
  return 'UNFINISHED_ORDINARY_CANDIDATE','Actual exclusion Textarea48 in official SettingField render. Needs production Root/long rules/save conflict/section merge tests, not helper-only source assertion.'
 return 'UNFINISHED_REVIEW','No per-site ordinary/fixed approval review completed; never counts as compliant by default.'

records=[];source_files=[]
for p in sorted(src.rglob('*.rs')):
 rel=p.relative_to(root).as_posix();s=p.read_text(encoding='utf-8-sig');code=mask(s);ranges=test_ranges(code)
 filetest=p.name=='tests.rs' or p.stem.endswith('_tests') or 'tests' in p.relative_to(src).parts
 rows=[]
 for m in re.finditer(r'\bpx\s*\(',code):
  begin=code.find('(',m.start());end=close(code,begin,'(',')');line=s.count('\n',0,m.start())+1
  is_test=filetest or any(a<=m.start()<=b for a,b in ranges)
  expr=s[m.start():end+1];is_archive='/archive/' in rel or rel.endswith('/main_window/archive_page.rs')
  if is_archive:category,reason=archive_class(rel,line,expr,is_test)
  elif is_test:category,reason='TEST_ONLY','Lexically inside cfg(test) item/module or dedicated test file. Navigation classification only; unchanged.'
  else:category,reason='UNFINISHED_REVIEW','Whole-project residual: not semantically classified at this exact site. Prior bounded slice evidence is not whole-file approval. Must inspect actual consumer/authority; cannot close #6.'
  rec={'path':rel,'line':line,'column':m.start()-s.rfind('\n',0,m.start()),'expression':expr,'test_only_lexical':is_test,'classification':category,'reason':reason,'evidence':'Task-start live snapshot; production/Root checks NOT RUN in Queue10'}
  rows.append(rec);records.append(rec)
 source_files.append({'path':rel,'sha256':sha(p),'px_calls':len(rows),'classification_counts':dict(collections.Counter(x['classification'] for x in rows)),'status':'TEST_ONLY' if filetest else 'NO_DIRECT_PX_CALLS_NOT_SEMANTIC_CLEARANCE' if not rows else 'UNFINISHED_REVIEW_OR_PRIOR_BOUNDED_EVIDENCE'})
archive=[r for r in records if '/archive/' in r['path'] or r['path'].endswith('/main_window/archive_page.rs')]
summary={'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'method':'Lexical source index, not Rust parser or semantic/behavior coverage. Strings/comments masked; exact cfg(test) item ranges and test filenames marked. Expressions can include nonlayout geometry. No zero-px or closure claim. Numeric roles outside px() and indirect consumers remain UNFINISHED_REVIEW.','source_files':len(source_files),'px_calls':len(records),'classifications':dict(collections.Counter(x['classification'] for x in records)),'archive_px_calls':len(archive),'archive_classifications':dict(collections.Counter(x['classification'] for x in archive)),'finding06_status':'OPEN_SUBSTANTIVE_ORDINARY_AND_UNREVIEWED_REMAINDERS'}
for name,obj in [('desktop-px-calls.json',records),('desktop-file-coverage.json',source_files),('archive-px-roles.json',archive),('role-inventory-summary.json',summary)]:
 dest=task/'evidence'/(name.removesuffix('.json')+'-v2.json');assert not dest.exists(),f'Never overwrite {dest}';dest.write_text(json.dumps(obj,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(json.dumps(summary,indent=2))
