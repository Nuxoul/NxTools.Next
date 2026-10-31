from pathlib import Path
import json, hashlib, re, difflib
R=Path.cwd(); S=Path(__file__).parent; sha=lambda b:hashlib.sha256(b).hexdigest()
rows=json.loads((S/'baseline-manifest.json').read_text()); checks=[]
def texts(path):
 row=next(r for r in rows if r['path']==path)
 b=Path(row['baseline']).read_bytes(); a=(R/path).read_bytes(); assert sha(b)==row['sha256']
 return b.decode('utf8'),a.decode('utf8')
def normalized(s):return re.sub(r'\s+','',s)
def check(path, label, before, after):
 ok=normalized(before)==normalized(after)
 checks.append(dict(path=path,label=label,whitespace_normalized_equal=ok,before_sha256=sha(before.encode()),after_sha256=sha(after.encode()),normalized_sha256=sha(normalized(before).encode())))
 assert ok,(path,label)
def span(t,begin,end):
 st=t.index(begin);return t[st:t.index(end,st)]
schedule='rust/apps/nxtools-desktop/src/downloads/schedule_view.rs'; b,a=texts(schedule)
check(schedule,'original struct, constructor, activation, default Enter, save/failure/defer',span(b,'pub(crate) struct DownloadScheduleView','impl Render for DownloadScheduleView'),span(a,'pub(crate) struct DownloadScheduleView','impl Render for DownloadScheduleView'))
check(schedule,'parse/validation and existing helper tests',b[b.index('pub(crate) fn parse_clock_minute'):],a[a.index('pub(crate) fn parse_clock_minute'):])
for begin,end,label in [
 ('            move |_, _, cx| {','        let body =','titlebar close callback'),
 ('            .capture_key_down','            .child(dialog_shell','default Enter capture/release owner'),
 ('                            .on_click(cx.listener(move |this, requested: &bool, _, cx| {','        let start_control','weekday requested-value callback'),
 ('            cx.listener(|this, requested: &bool, _, cx| {','        let stop_control','start requested-value callback'),
 ('        let stop_control','        let close_application','stop control callback'),
 ('                        .on_click(move |_, _, cx| {','                .child(\n                    crate::ui::components::action::primary_action_button','Cancel callback'),
 ('                    .on_click(cx.listener(|this, _, window, cx| {','                .into_any_element(),','Apply callback'),
]:
 bb=span(b,begin,end); aa=span(a,begin,end)
 if label=='weekday requested-value callback':
  bb=bb[:bb.index('                    ),')];aa=aa[:aa.index('                    ),')]
 if label=='start requested-value callback':
  bb=bb[:bb.index('            }),')+15];aa=aa[:aa.index('            }),')+15]
 if label=='stop control callback':
  bb=span(bb,'            cx.listener','            }),')+'            }),';aa=span(aa,'            cx.listener','            }),')+'            }),'
 check(schedule,label,bb,aa)
page='rust/apps/nxtools-desktop/src/main_window/download_page.rs';b,a=texts(page)
check(page,'all original task/filter/sort/selection functions before render',b[:b.index('impl MainWindow')].replace('use crate::ui::metrics::PROGRESS_TRACK_HEIGHT;',''),a[:a.index('impl MainWindow')].replace('use crate::ui::metrics::{PROGRESS_TRACK_HEIGHT, ordinary_rem};',''))
check(page,'render_downloads original owner and actual uniform_list callback',span(b,'impl MainWindow','fn download_bounded_tooltip'),span(a,'impl MainWindow','fn download_bounded_tooltip').replace('                            .debug_selector(|| "download-task-list-viewport".into())\n',''))
check(page,'row immutable IDs and projection before layout',span(b,'pub(crate) fn download_task_row','    div()\n        .id(row_id.clone())'),span(a,'pub(crate) fn download_task_row','    div()\n        .id(row_id.clone())'))
check(page,'row pointer/context/selection/OLE callback boundary',span(b,'        .on_click(move |event, _, cx| {','        .child(\n            div()\n                .size_full()'),span(a,'        .on_click(move |event, _, cx| {','        .child(\n            div()\n                .size_full()'))
check(page,'all original progress/context/menu/batch/detail/format functions',span(b,'fn download_aggregate_progress','pub(crate) fn download_table_header'),span(a,'fn download_aggregate_progress','pub(crate) fn download_table_header'))
bb=span(b,'pub(crate) fn download_sort_header_cell','\n#[cfg(test)]');aa=span(a,'pub(crate) fn download_sort_header_cell','\n#[cfg(test)]')
check(page,'sort header actual Button and accessible naming before content style',bb[:bb.index('                .child(div().w_full()')],aa[:aa.index('                .child(\n                    div()')])
check(page,'sort header single click/scroll/persist/deferred owner',bb[bb.index('                .on_click(move |_event'):],aa[aa.index('                .on_click(move |_event'):])
for path in ['rust/apps/nxtools-desktop/src/downloads/polish_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs','docs/reference-code/download.md']:
 b,a=texts(path);ok=a.startswith(b);assert ok,path+' prefix changed';checks.append(dict(path=path,label='entire original dirty prefix is byte-identical',old_prefix_equal=True,baseline_sha256=sha(b.encode()),baseline_bytes=len(b.encode())))
for name in ['protected-baseline.json','frozen-history.json']:
 entries=json.loads((S/name).read_text());changed=[r['path'] for r in entries if not (R/r['path']).exists() or sha((R/r['path']).read_bytes())!=r['sha256']];assert not changed,(name,changed)
 checks.append(dict(label=name,count=len(entries),unchanged=True))
(S/'owner-invariants.json').write_text(json.dumps(checks,indent=2)+'\n',encoding='utf8')
print('PASS',len(checks),'owner/prefix/frozen checks')
