from pathlib import Path
import json,hashlib,re,difflib,base64,datetime
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-command-naming';scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\command-naming')
baseline=json.loads((task/'evidence/baseline-manifest.json').read_text())
history=json.loads((task/'evidence/historical-manifest.json').read_text())
by={r['path']:r for r in baseline['files']}
def sha(data):return hashlib.sha256(data).hexdigest()
def data(rel):return (root/rel).read_bytes()
def old(rel):return (task/by[rel]['snapshot']).read_bytes()
def norm(s):return re.sub(r'\s+','',s)
def text(b):return b.decode('utf-8-sig').replace('\r\n','\n')
base='rust/apps/nxtools-desktop/src/'
owned=[base+p for p in ['downloads.rs','downloads/new_download_view.rs','downloads/settings_view.rs','downloads/polish_tests.rs','main_window.rs','main_window/clipboard_page.rs','main_window/clipboard_page/kit_migration_tests.rs','main_window/download_page.rs','main_window/tools_page.rs','main_window/tools_page/currency_settings.rs','main_window/tools_page/floating_assistant_settings.rs','main_window/floating_assistant_tests.rs','ui/components/menu.rs']]
checks=[]
def check(name,actual,expected):
 assert actual==expected,f'Owner/source guard failed: {name}'
 checks.append({'name':name,'result':'PASS','sha256_actual':sha(actual.encode() if isinstance(actual,str) else actual)})
for rel in owned:
 before=text(old(rel));after=text(data(rel));restored=after
 p=rel.removeprefix(base)
 if p=='downloads.rs':
  fragment='\n    pub(crate) fn task_removal_deletes_file(&self) -> bool {\n        self.config.download.task_removal_deletes_file\n    }\n'
  assert restored.count(fragment)==1;restored=restored.replace(fragment,'',1)
 elif p=='main_window.rs':
  suffix='\n#[cfg(test)]\nmod command_naming_tests;\n';assert restored.endswith(suffix);restored=restored[:-len(suffix)]
 elif p=='downloads/new_download_view.rs':restored=restored.replace('选择下载目录…','选择下载目录')
 elif p=='downloads/settings_view.rs':
  for s in ['选择默认保存目录','选择 FFmpeg 可执行文件','导出 Chrome / Edge (.crx)','导出 Firefox (.xpi)','导出 Chrome 商店 (.zip)']:restored=restored.replace(s+'…',s)
 elif p=='downloads/polish_tests.rs':
  prefix=after.split('\n#[gpui_kit::test]\nfn audit08_settings_picker_and_export_labels_keep_objects_and_cancel')[0]
  for s in ['选择默认保存目录','选择 FFmpeg 可执行文件']:prefix=prefix.replace(s+'…',s)
  check('Original Download tests exact text prefix except two synchronized label ellipses',prefix.rstrip(),before.rstrip());continue
 elif p=='main_window/clipboard_page/kit_migration_tests.rs':
  check('Original Clipboard tests byte prefix',data(rel)[:len(old(rel))],old(rel));continue
 elif p=='main_window/clipboard_page.rs':
  for s in ['导出','导入','迁移']:restored=restored.replace('.label("'+s+'…")','.label("'+s+'")')
 elif p=='main_window/download_page.rs':
  fragment='''    let delete_file_label = if application
        .read_with(cx, |application, _| application.task_removal_deletes_file())
        .unwrap_or(false)
    {
        "删除文件…"
    } else {
        "从列表移除"
    };
'''
  assert restored.count(fragment)==1;restored=restored.replace(fragment,'',1)
  restored=restored.replace('                delete_file_label,','                "删除文件",',1)
  for s in ['下载设置','下载计划','新建下载','任务窗口']:restored=restored.replace(s+'…',s)
  restored=restored.replace('"取消任务",\n                DownloadBatchAction::Cancel','"取消",\n                DownloadBatchAction::Cancel')
  restored=restored.replace('取消所选可取消任务','取消可取消任务')
 elif p in ['main_window/tools_page.rs','main_window/tools_page/currency_settings.rs']:restored=restored.replace('.label("配置…")','.label("配置")')
 elif p=='main_window/tools_page/floating_assistant_settings.rs':
  restored=restored.replace('.label("配置…")','.label("配置")').replace('选择图片…','选择图片')
  check('Floating source only exact labels and local rustfmt',norm(restored),norm(before));continue
 elif p=='main_window/floating_assistant_tests.rs':restored=restored.replace('选择图片…','选择图片')
 elif p=='ui/components/menu.rs':
  restored=restored.replace('use gpui_kit::base::TestSupportExt as _;\n','',1)
  fragment='''            .id(selector)
            .role(gpui_kit::Role::Label)
            .aria_label(label.clone())
            .test_support()
'''
  assert restored.count(fragment)==1;restored=restored.replace(fragment,'',1)
 check('Exact inverse presentation delta: '+p,restored.rstrip(),before.rstrip())
references=['docs/reference-code/clipboard.md','docs/reference-code/download.md','docs/reference-code/gpui.md']
for rel in references:
 check('Append-only reference prefix: '+rel,data(rel)[:len(old(rel))],old(rel))
owned+=references
# Whole unchanged source verification covers original handlers and boundaries more strongly
# than selective callback hashes for all exact-inverse presentation-only source edits.
changed=[r['path'] for r in baseline['files'] if sha(data(r['path']))!=r['sha256']]
external=[p for p in changed if p not in owned]
for r in history['files']:assert (root/r['path']).is_file() and sha(data(r['path']))==r['sha256'],'Historical movement: '+r['path']
checks.append({'name':'All immutable earlier task records','result':'PASS','count':len(history['files'])})
protected=[r for r in baseline['files'] if r['path'] not in owned and r['path'] not in external]
for r in protected:assert sha(data(r['path']))==r['sha256']
new_test=base+'main_window/command_naming_tests.rs';assert (root/new_test).is_file()
result={'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'owned_existing':owned,'new_owned':[new_test],'checks':checks,'protected_unchanged':protected,'independent_changed_observations':[{'path':p,'baseline_sha256':by[p]['sha256'],'observed_sha256':sha(data(p))} for p in external],'history_files':len(history['files']),'no_worker_protocol_csharp_engine_policy_change':True,'scope':'Exact inverse text deltas/old prefixes/whole protected files and frozen history; not compiled acceptance'}
path=scratch/'owner-inspection-final.json';assert not path.exists();path.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'owner_checks':len(checks),'owned_existing':len(owned),'new_tests':1,'protected_unchanged':len(protected),'history_files':len(history['files']),'independent_changed':external},indent=2))
