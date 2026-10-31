from pathlib import Path
import hashlib,json,re
R=Path.cwd(); T=R/'.trellis/tasks/10-25-rem-role-consistency'; E=T/'evidence'; O=Path(__file__).parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def must(ok,msg):
    if not ok:raise AssertionError(msg)
for name,want in [('delivery-manifest.json','0bb653bc732a72047a8227ad9e9c48b3316c198ee7789abf0dfc18ce94f5fb4f'),('own-deltas.json','bce01154fec5970e8464b41f51e7deddd6c61cd8ad705c6dbbb74f14ad5efc17')]:must(sha((E/name).read_bytes())==want,name)
m=load(E/'delivery-manifest.json');deltas=load(E/'own-deltas.json');sources={}
for x in m['evidence']:must(sha((T/x['path']).read_bytes())==x['sha256'],'evidence '+x['path'])
for x in m['new_documents']:must(sha((R/x['path']).read_bytes())==x['sha256'],'doc '+x['path'])
for x in m['captured_sources']:
    before=(T/x['baseline']).read_bytes();after=(T/x['current']).read_bytes()
    must(sha(before)==x['before_sha256'],'baseline '+x['path']);must(sha(after)==x['after_sha256'],'snapshot '+x['path']);must((R/x['path']).read_bytes()==after,'live '+x['path']);sources[x['path']]=(before.decode(),after.decode())
for d in deltas:
    b=(T/d['baseline']).read_bytes();a=(T/d['current']).read_bytes();bl=b.decode().splitlines(keepends=True);al=a.decode().splitlines(keepends=True)
    must(sha(b)==d['before_sha256'] and sha(a)==d['after_sha256'],'delta endpoints '+d['path'])
    forward=bl.copy();reverse=al.copy()
    for op in reversed(d['operations']):
        must(forward[op['before_start']:op['before_end']]==op['before_lines'],'before span '+d['path']);forward[op['before_start']:op['before_end']]=op['after_lines']
        must(reverse[op['after_start']:op['after_end']]==op['after_lines'],'after span '+d['path']);reverse[op['after_start']:op['after_end']]=op['before_lines']
    must(''.join(forward).encode()==a and ''.join(reverse).encode()==b,'bidirectional '+d['path'])
for group in ['protected','frozen_history']:
    for x in m[group]:must(sha((R/x['path']).read_bytes())==x['sha256'],group+' '+x['path'])
for x in load(E/'reference-and-failure-sources.json'):must(sha((R/x['path']).read_bytes())==x['sha256'],'reference/failure source '+x['path'])
invariants={x['label']:x for x in load(E/'owner-invariants.json')}; owner_checks=[]
def span(t,begin,end):
    start=t.index(begin);return t[start:t.index(end,start)]
def same(label,b,a):
    bn=re.sub(r'\s+','',b);an=re.sub(r'\s+','',a)
    must(bn==an,'owner changed '+label);must(sha(bn.encode())==invariants[label]['normalized_sha256'],'owner evidence '+label);owner_checks.append(label)
p='rust/apps/nxtools-desktop/src/downloads/schedule_view.rs';b,a=sources[p]
same('original struct, constructor, activation, default Enter, save/failure/defer',span(b,'pub(crate) struct DownloadScheduleView','impl Render for DownloadScheduleView'),span(a,'pub(crate) struct DownloadScheduleView','impl Render for DownloadScheduleView'))
same('parse/validation and existing helper tests',b[b.index('pub(crate) fn parse_clock_minute'):],a[a.index('pub(crate) fn parse_clock_minute'):])
for begin,end,label in [
('            move |_, _, cx| {','        let body =','titlebar close callback'),
('            .capture_key_down','            .child(dialog_shell','default Enter capture/release owner'),
('                            .on_click(cx.listener(move |this, requested: &bool, _, cx| {','        let start_control','weekday requested-value callback'),
('            cx.listener(|this, requested: &bool, _, cx| {','        let stop_control','start requested-value callback'),
('        let stop_control','        let close_application','stop control callback'),
('                        .on_click(move |_, _, cx| {','                .child(\n                    crate::ui::components::action::primary_action_button','Cancel callback'),
('                    .on_click(cx.listener(|this, _, window, cx| {','                .into_any_element(),','Apply callback')]:
    bb=span(b,begin,end);aa=span(a,begin,end)
    if label=='weekday requested-value callback':bb=bb[:bb.index('                    ),')];aa=aa[:aa.index('                    ),')]
    if label=='start requested-value callback':bb=bb[:bb.index('            }),')+15];aa=aa[:aa.index('            }),')+15]
    if label=='stop control callback':bb=span(bb,'            cx.listener','            }),')+'            }),';aa=span(aa,'            cx.listener','            }),')+'            }),'
    same(label,bb,aa)
p='rust/apps/nxtools-desktop/src/main_window/download_page.rs';b,a=sources[p]
same('all original task/filter/sort/selection functions before render',b[:b.index('impl MainWindow')].replace('use crate::ui::metrics::PROGRESS_TRACK_HEIGHT;',''),a[:a.index('impl MainWindow')].replace('use crate::ui::metrics::{PROGRESS_TRACK_HEIGHT, ordinary_rem};',''))
same('render_downloads original owner and actual uniform_list callback',span(b,'impl MainWindow','fn download_bounded_tooltip'),span(a,'impl MainWindow','fn download_bounded_tooltip').replace('                            .debug_selector(|| "download-task-list-viewport".into())\n',''))
same('row immutable IDs and projection before layout',span(b,'pub(crate) fn download_task_row','    div()\n        .id(row_id.clone())'),span(a,'pub(crate) fn download_task_row','    div()\n        .id(row_id.clone())'))
same('row pointer/context/selection/OLE callback boundary',span(b,'        .on_click(move |event, _, cx| {','        .child(\n            div()\n                .size_full()'),span(a,'        .on_click(move |event, _, cx| {','        .child(\n            div()\n                .size_full()'))
same('all original progress/context/menu/batch/detail/format functions',span(b,'fn download_aggregate_progress','pub(crate) fn download_table_header'),span(a,'fn download_aggregate_progress','pub(crate) fn download_table_header'))
bb=span(b,'pub(crate) fn download_sort_header_cell','\n#[cfg(test)]');aa=span(a,'pub(crate) fn download_sort_header_cell','\n#[cfg(test)]')
same('sort header actual Button and accessible naming before content style',bb[:bb.index('                .child(div().w_full()')],aa[:aa.index('                .child(\n                    div()')])
same('sort header single click/scroll/persist/deferred owner',bb[bb.index('                .on_click(move |_event'):],aa[aa.index('                .on_click(move |_event'):])
for p in ['rust/apps/nxtools-desktop/src/downloads/polish_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs','docs/reference-code/download.md']:
    b,a=sources[p];must(a.startswith(b),'old prefix '+p)
def logtext(name):
    data=(E/name).read_bytes();return data.decode('utf-16') if data.startswith((b'\xff\xfe',b'\xfe\xff')) else data.decode('utf-8-sig')
full=logtext('desktop-full-once.log');must('1237 passed; 1 failed; 0 ignored' in full and '281.18s' in full,'full suite actual result');must('w6_manager_pointer_preview_confirm_and_toggle_commit_real_config' in full,'full failed name')
result={'own_deltas_bidirectional':len(deltas),'captured_live_sources':len(sources),'evidence_hashes':len(m['evidence']),'protected':len(m['protected']),'frozen_history':len(m['frozen_history']),'owner_sections_recomputed':len(owner_checks),'old_prefixes_verified':3,'reference_failure_hashes':len(load(E/'reference-and-failure-sources.json')),'worker_full_suite':'FAIL 1237 passed/1 failed/0 ignored;281.18s; not parent full run'}
print(json.dumps(result,indent=2));(O/'integrity-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
