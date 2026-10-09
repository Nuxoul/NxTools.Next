'use strict';

// Design-only fixtures. There are deliberately no fetch, storage or platform calls.
const PAGES = {
  downloads: { label:'下载 · 工作区', size:[1060,650], main:true },
  quickSettings: { label:'速启 · 设置', size:[1060,650], main:true },
  clipboardSettings: { label:'剪贴板 · 设置', size:[1060,650], main:true },
  tools: { label:'工具 · 链接检测', size:[1060,650], main:true },
  global: { label:'全局设置 · 独立窗口', size:[720,480] },
  quickPopup: { label:'速启 · 搜索弹窗', size:[620,410] },
  quickCollapsed: { label:'速启 · 搜索-only 收起', size:[620,44], states:['normal','long'] },
  clipboardPopup: { label:'剪贴板 · 历史弹窗', size:[300,440] },
  probe: { label:'下载 · 链接探测', size:[560,176] },
  confirm: { label:'下载 · 新建确认', size:[560,278] },
  progress: { label:'下载 · 任务进度（纯内容）', size:[440,184], states:['normal','long','empty','loading','busy'] },
  completed: { label:'下载 · 完成（纯内容）', size:[440,184], states:['normal','long'] },
  expanded: { label:'下载 · 展开进度（纯内容）', size:[440,280] },
  failed: { label:'下载 · 失败（纯内容）', size:[440,196], states:['normal','long'] },
  dash: { label:'媒体 · DASH 确认', size:[560,353] },
  hls: { label:'媒体 · HLS 确认', size:[560,418] },
  mediaError: { label:'媒体 · 错误预算', size:[560,456] },
  supporting: { label:'配套界面 · 线框与组件状态', size:[1060,650], main:true }
};
const STATES = ['normal','long','empty','loading','error','busy'];
const ICONS = {
  download:'M8 2v8m-3-3 3 3 3-3M3 11v3h10v-3',
  search:'M10.5 10.5 14 14M11.5 6.5a5 5 0 1 1-10 0 5 5 0 0 1 10 0',
  clipboard:'M5 3H3v11h10V3h-2M5 2h6v3H5z',
  tools:'m10 2 1 3 3 1-3 3-2-2-6 6-2-2 6-6-2-2 3-1z',
  settings:'M6 2h4l1 3 3 1v4l-3 1-1 3H6l-1-3-3-1V6l3-1zM10 8a2 2 0 1 1-4 0 2 2 0 0 1 4 0',
  close:'m4 4 8 8m0-8-8 8', minimize:'M3 8h10', maximize:'M3 3h10v10H3z',
  plus:'M8 3v10M3 8h10', more:'M3 8h.1M8 8h.1M13 8h.1',
  file:'M4 2h5l3 3v9H4zM9 2v4h3', folder:'M2 4h5l2 2h5v7H2z',
  pause:'M5 3v10M11 3v10', play:'m5 3 8 5-8 5z', check:'m3 8 3 3 7-7',
  alert:'m8 2 7 12H1zM8 6v4M8 12h.1', arrow:'m6 3 5 5-5 5', back:'m10 3-5 5 5 5',
  link:'m6 10-1 1a3 3 0 0 1-4-4l3-3a3 3 0 0 1 4 0m2 2 1-1a3 3 0 0 1 4 4l-3 3a3 3 0 0 1-4 0m-2-4 4-4'
};
const esc = value => String(value ?? '').replace(/[&<>"']/g, char => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
const icon = name => `<svg aria-hidden="true" viewBox="0 0 16 16"><path d="${ICONS[name] || ICONS.file}"/></svg>`;
const btn = (label, action, extra = '') => `<button type="button" data-action="${action}" ${extra}>${label}</button>`;
const iconBtn = (name, label, action, extra = '') => {
  const classes = /class="([^"]+)"/.exec(extra)?.[1] || '';
  return btn(icon(name), action, `class="icon-button quiet ${classes}" aria-label="${esc(label)}" title="${esc(label)}" ${extra.replace(/class="[^"]+"/,'')}`);
};
const disabled = () => model.state === 'busy' || model.state === 'loading';
const checked = bool => bool ? 'checked' : '';
const clone = value => JSON.parse(JSON.stringify(value));

const INITIAL_TASKS = [
  { id:1, name:'项目资料包_秋季交付.zip', type:'压缩包', size:'1.24 GB', status:'下载中', speed:'8.6 MB/s', percent:64 },
  { id:2, name:'设计评审录屏_第二轮.mp4', type:'视频', size:'684 MB', status:'等待中', speed:'—', percent:0 },
  { id:3, name:'产品说明书_中文版.pdf', type:'文档', size:'12.8 MB', status:'已完成', speed:'—', percent:100 },
  { id:4, name:'开发环境离线安装包.zip', type:'压缩包', size:'2.10 GB', status:'已暂停', speed:'—', percent:38 },
  { id:5, name:'界面图标资源_定稿.zip', type:'压缩包', size:'42.6 MB', status:'失败', speed:'—', percent:21 }
];
const INITIAL_RULES = [
  { name:'KeePass.exe', clip:true, quick:false },
  { name:'1Password.exe', clip:true, quick:false },
  { name:'Bitwarden.exe', clip:true, quick:false },
  { name:'演示全屏应用.exe', clip:false, quick:true }
];
const INITIAL_COMMANDS = [
  { name:'打开项目目录', key:'proj', target:'D:\\Demo\\项目资料', args:'', enabled:true },
  { name:'打开文档', key:'docs', target:'D:\\Demo\\产品说明书.pdf', args:'', enabled:true },
  { name:'启动编辑器', key:'edit', target:'C:\\Demo\\Editor.exe', args:'{query}', enabled:false }
];
const INITIAL_HISTORY = [
  { id:1, text:'周四 14:00 进行界面评审，请准备下载窗口和设置页面。', kind:'文本', source:'演示文档', time:'刚刚' },
  { id:2, text:'D:\\Demo\\项目资料\\产品说明书.pdf', kind:'文件', source:'演示文件列表', time:'2 分钟前' },
  { id:3, text:'布局对齐检查截图.png', kind:'图片', source:'演示图片 · 无真实图像', time:'8 分钟前' },
  { id:4, text:'下载完成后检查文件名称与保存位置。', kind:'文本', source:'演示笔记', time:'12 分钟前' }
];
const FIXTURE_PATH = 'D:\\Demo\\下载';
let model;
function resetModel() {
  model = {
    page:'downloads', state:'normal', sizeMode:'default', customSize:null, zoom:'actual',
    tasks:clone(INITIAL_TASKS), selected:[], detail:1, filter:'全部', taskQuery:'',
    quickTab:'常规', clipTab:'触发与动作', commandIndex:0, commands:clone(INITIAL_COMMANDS),
    settings:{}, settingsMessage:'配置变化立即在内存中模拟，不写入系统。',
    globalTab:'应用黑名单', rules:clone(INITIAL_RULES), savedRules:clone(INITIAL_RULES),
    globalDraft:{startup:false,tray:true,route:'系统代理',proxy:'127.0.0.1:7890'}, savedGlobal:null,
    ruleDraft:{name:'',clip:true,quick:false}, editRule:-1, globalDirty:false, globalMessage:'',
    query:'', resultIndex:0, actionMode:false, actionTarget:null,
    clipQuery:'', clipCategory:'全部', history:clone(INITIAL_HISTORY), clipIndex:0, editClip:null,
    supportTab:'组件状态', submenu:false, noticeDismissed:false, date:18, paused:false, mediaSelected:0
  };
  model.savedGlobal = clone(model.globalDraft);
}
resetModel();
const frame = document.getElementById('app-frame');
const pageSelector = document.getElementById('page-selector');
pageSelector.innerHTML = Object.entries(PAGES).map(([id,page]) => `<option value="${id}">${page.label}</option>`).join('');

function chrome(title, main = false) {
  return `<header class="chrome" data-region="chrome"><span class="brand-symbol">${icon('download')}</span>
    <span class="chrome-title">${esc(title)}</span>${main ? iconBtn('settings','全局设置','open-global') : ''}
    <div class="caption-group">${main ? iconBtn('minimize','模拟最小化','caption') + iconBtn('maximize','模拟最大化','caption') : ''}${iconBtn('close','关闭演示窗口','close-window')}</div></header>`;
}
function moduleRail() {
  const modules = [['downloads','download','下载'],['quickSettings','search','速启'],['clipboardSettings','clipboard','剪贴板'],['tools','tools','工具']];
  return `<nav class="rail" aria-label="模块">${modules.map(([page,glyph,label]) => btn(`${icon(glyph)}<span>${label}</span>`,'page',`data-page="${page}" aria-selected="${model.page === page}"`)).join('')}
    <div class="rail-bottom">${btn(`${icon('settings')}<span>全局设置</span>`,'open-global')}</div></nav>`;
}
function workHeader(title, description, action = '') {
  return `<header class="work-header" data-region="work-header"><h1>${title}</h1><span class="support">${description}</span>${action}</header>`;
}
function mainShell(content) { return chrome('NxTools',true) + `<div class="shell">${moduleRail()}<section class="work">${content}</section></div>`; }
function status(text = model.settingsMessage) { return `<footer class="work-status" data-region="footer"><span>${esc(text)}</span><span>演示数据</span></footer>`; }
function tabs(items, current, action) {
  return `<nav class="subtabs" aria-label="设置分组">${items.map(name => btn(name,action,`data-value="${name}" aria-selected="${name === current}"`)).join('')}</nav>`;
}
function toggle(key, label, fallback = true, draft = false) {
  const source = draft ? model.globalDraft : model.settings;
  const value = source[key] ?? fallback;
  return `<label class="switch"><input type="checkbox" data-${draft ? 'global-setting' : 'setting'}="${key}" aria-label="${label}" ${checked(value)} ${disabled() ? 'disabled' : ''}><span class="switch-track"></span></label>`;
}
function field(key,label,value,type = 'text',draft = false) {
  const source = draft ? model.globalDraft : model.settings;
  return `<input id="field-${key}" type="${type}" data-${draft ? 'global-setting' : 'setting'}="${key}" aria-label="${label}" value="${esc(source[key] ?? value)}" ${disabled() ? 'disabled' : ''}>`;
}
function selectField(key,label,options,fallback,draft = false) {
  const source = draft ? model.globalDraft : model.settings;
  return `<select data-${draft ? 'global-setting' : 'setting'}="${key}" aria-label="${label}" ${disabled() ? 'disabled' : ''}>${options.map(value => `<option ${value === (source[key] ?? fallback) ? 'selected' : ''}>${esc(value)}</option>`).join('')}</select>`;
}
function settingRow(label,description,control) {
  return `<div class="setting-row"><div class="setting-label"><div>${label}</div><p class="description">${description}</p></div><div class="setting-control">${control}</div></div>`;
}
function group(title,rows) { return `<section class="group"><h2 class="group-title">${title}</h2>${rows.join('')}</section>`; }
function stateBlock(state,scope = '内容') {
  const details = {
    empty:['暂无演示内容',`可重置${scope}，或修改筛选条件。`,'重置演示'],
    loading:['正在加载演示内容','静态加载状态；不会发起网络请求。','取消加载'],
    error:['演示操作失败','未修改原内容。请重试；草稿仍保留。','重试'],
    busy:['正在处理演示操作','控件暂时禁用；可取消并保留草稿。','取消操作']
  }[state];
  if (!details) return '';
  return `<div class="inline-state ${state === 'error' ? 'error' : ''}" role="${state === 'error' ? 'alert' : 'status'}">${icon(state === 'error' ? 'alert' : 'file')}<div><h2>${details[0]}</h2><p class="description">${details[1]}</p><div class="actions">${btn(details[2],'recover', 'class="small"')}</div></div></div>`;
}
function longName(name) { return model.state === 'long' ? '跨部门联合项目_界面布局质量检查_中文文件名称与完整保存路径_最终交付版本_2026_秋季_' + name : name; }

// Downloads: one continuous table; a fixed detail/footer region and scoped batch actions.
function visibleTasks() {
  if (model.state === 'empty') return [];
  return model.tasks.filter(task => (model.filter === '全部' || (model.filter === '进行中' ? ['下载中','等待中','已暂停'].includes(task.status) : task.status === model.filter)) && longName(task.name).toLowerCase().includes(model.taskQuery.toLowerCase()));
}
function renderDownloads() {
  const tasks = visibleTasks();
  const selection = model.selected.length;
  const filters = ['全部','进行中','已完成','失败'];
  const toolbar = selection ? `<strong>${selection} 项已选</strong>${btn('暂停','batch-pause','class="small"')}${btn('继续','batch-resume','class="small"')}${btn('移除…','batch-remove','class="small danger"')}${btn('取消选择','clear-selection','class="small quiet"')}` :
    `<div class="segments">${filters.map(name => btn(name,'task-filter',`data-value="${name}" aria-pressed="${name === model.filter}"`)).join('')}</div>${iconBtn('more','切换排序','sort-tasks','class="small"')}`;
  const heading = `<div class="task-grid table-heading"><input type="checkbox" aria-label="选择当前任务" data-select-all ${checked(tasks.length > 0 && tasks.every(task => model.selected.includes(task.id)))} ${!tasks.length || disabled() ? 'disabled' : ''}><span>名称 / 保存位置</span><span>状态</span><span>大小</span><span>速度</span><span></span></div>`;
  const rows = tasks.map(task => `<div class="task-grid task-row ${model.selected.includes(task.id) || model.detail === task.id ? 'selected' : ''}" data-task="${task.id}">
    <input type="checkbox" aria-label="选择 ${esc(task.name)}" data-task-check="${task.id}" ${checked(model.selected.includes(task.id))} ${disabled() ? 'disabled' : ''}>
    <button class="task-identity" data-action="task-detail" data-id="${task.id}" title="${esc(longName(task.name))}"><span class="ellipsis">${esc(longName(task.name))}</span><span class="support mono ellipsis">${esc(FIXTURE_PATH)} / ${esc(task.type)}</span></button>
    <div><span class="${task.status === '失败' ? 'error-text' : task.status === '已完成' ? 'success-text' : ''}">${task.status}</span>${task.status === '下载中' ? `<div class="progress-track" aria-label="进度 ${task.percent}%"><div class="progress-fill" style="width:${task.percent}%"></div></div>` : ''}</div>
    <span class="mono">${task.size}</span><span class="mono">${task.speed}</span>${iconBtn('more','任务菜单','task-menu',`data-id="${task.id}"`)}</div>`).join('');
  const special = ['loading','error','busy'].includes(model.state);
  const detail = model.tasks.find(task => task.id === model.detail);
  return mainShell(workHeader('下载','管理任务与保存位置',btn(`${icon('plus')} 新建下载`,'new-download','class="primary small"')) +
    `<div class="toolbar">${toolbar}<input id="task-search" aria-label="搜索下载任务" placeholder="搜索任务" value="${esc(model.taskQuery)}"></div>` + heading +
    `<div class="tasks scroll" data-scroll-region="tasks">${special ? stateBlock(model.state,'任务') : rows || stateBlock('empty','任务')}</div>` +
    `<section class="detail-pane" data-region="detail">${detail ? `<div><strong>${esc(detail.name)}</strong><p class="support mono wrap" data-scroll-region="detail-path">${esc(FIXTURE_PATH)} / ${esc(longName(detail.name))}</p></div><p class="support">${detail.status === '失败' ? '模拟失败：目标位置不可写。原任务与进度保留。' : `任务详情 · ${detail.status} · 合成演示文件`}</p><div class="actions">${btn(detail.status === '下载中' ? '暂停' : '继续','task-toggle',`data-id="${detail.id}" ${disabled() ? 'disabled' : ''}`)}${btn('进度窗口','page','data-page="progress"')}</div>` : '<p class="support">选择任务查看详情</p>'}</section>` + status(`${tasks.length} 个显示任务 · 合成数据 · 文件操作仅模拟`));
}

// Main Quick Find page is configuration, not an invented search dashboard.
function renderCommandEditor() {
  const commands = model.state === 'empty' ? [] : model.commands;
  const item = commands.length ? model.commandDrafts?.[model.commandIndex] || commands[model.commandIndex] : null;
  return `<div class="command-layout"><div class="command-list"><h2 class="group-title">自定义命令</h2>${commands.map((command,index) => btn(`<span class="mono">${esc(command.key)}</span><span class="ellipsis">${esc(command.name)}</span>`,'command-select',`data-index="${index}" aria-selected="${index === model.commandIndex}"`)).join('')}${btn(`${icon('plus')} 添加命令`,'command-add','class="small"')}</div><div>${item ? `<form id="command-form" data-command-index="${model.commandIndex}" class="form-stack"><h2>编辑命令</h2>
    <label>名称<input name="name" value="${esc(item.name)}" required ${disabled() ? 'disabled' : ''}></label>
    <label>关键词<input name="key" class="mono" value="${esc(item.key)}" required ${disabled() ? 'disabled' : ''}></label>
    <label>目标<input name="target" class="mono" value="${esc(item.target)}" required ${disabled() ? 'disabled' : ''}></label>
    <label>参数<input name="args" class="mono" value="${esc(item.args)}" placeholder="可选，例如 {query}" ${disabled() ? 'disabled' : ''}></label>
    <label class="checkbox-label"><input name="enabled" type="checkbox" ${checked(item.enabled)} ${disabled() ? 'disabled' : ''}>启用此命令</label>
    <div id="command-error" class="error-text support" role="alert">${model.state === 'error' ? '模拟保存失败；编辑内容仍保留。' : ''}</div>
    <div class="actions"><button type="submit" class="primary" ${disabled() ? 'disabled' : ''}>保存到演示</button>${btn('删除','command-delete','class="danger"')}</div><p class="description">编辑不会执行命令。所有目标均为合成路径。</p></form>` : stateBlock('empty','命令')}</div></div>`;
}
function renderQuickSettings() {
  const tab = model.quickTab;
  let content;
  if (tab === '常规') content = group('呼出与执行',[
    settingRow('全局呼出热键','只修改演示值，不注册 Windows 热键。',field('quickHotkey','速启热键','Alt + Space')),
    settingRow('全屏游戏中暂不呼出','仅独占全屏；不影响无边框窗口。',toggle('fullscreen','全屏游戏中暂不呼出')),
    settingRow('键盘动作','Enter 打开；→ 进入动作；Esc 返回。',btn('体验搜索弹窗','page','data-page="quickPopup"'))
  ]) + group('显示与预览',[
    settingRow('结果预览','独立被动窗口，不占用主搜索窗口。',btn('查看预览线框','show-preview'))
  ]);
  if (tab === '索引') content = group('索引范围',[
    settingRow('应用目录','开始菜单、桌面与系统工具；演示路径。',field('appPath','应用目录','D:\\Demo\\应用')),
    settingRow('本地文件索引','这里只展示配置；不会启动或连接索引服务。',toggle('indexFiles','本地文件索引')),
    settingRow('包含卷','原生卷枚举在此原型中不可用。',selectField('indexVolume','包含卷',['C: / D:（演示）','仅 D:（演示）'],'C: / D:（演示）')),
    settingRow('排除目录','此演示条目不会读取目录。',field('excludePath','排除目录','D:\\Demo\\临时文件'))
  ]);
  if (tab === '上下文') content = group('上下文联动',[
    settingRow('自动快速切换','从资源管理器返回打开 / 保存对话框时同步目录。',toggle('quickSwitch','自动快速切换')),
    settingRow('文件对话框搜索栏','受支持的对话框底部显示当前目录搜索。',toggle('dialogBar','文件对话框搜索栏')),
    settingRow('应用黑名单','按应用分别禁用速启或剪贴板。',btn('全局设置','open-global'))
  ]);
  if (tab === '命令') content = renderCommandEditor();
  if (tab === '诊断') content = group('历史与诊断',[
    settingRow('历史学习','演示设置，不读取历史或记录查询。',toggle('learn','历史学习')),
    settingRow('查询性能','没有真实索引样本；不能推导服务性能。','<span class="support">无演示测量</span>'),
    settingRow('运行日志','原型不读取产品日志。',btn('模拟查看日志','simulate','data-message="模拟查看日志；未访问文件系统。"'))
  ]);
  if (['loading','busy'].includes(model.state)) content = stateBlock(model.state) + content;
  if (model.state === 'error' && tab !== '命令') content = stateBlock('error') + content;
  if (model.state === 'empty' && tab !== '命令') content = stateBlock('empty') + content;
  return mainShell(workHeader('速启','设置',btn('搜索弹窗','page','class="small" data-page="quickPopup"')) + tabs(['常规','索引','上下文','命令','诊断'],tab,'quick-tab') + `<div class="work-content scroll" data-scroll-region="settings">${content}</div>` + status());
}
function renderClipboardSettings() {
  const tab = model.clipTab;
  const actions = ['原格式粘贴','仅复制','纯文本／路径粘贴','去换行粘贴','不执行'];
  let content;
  if (tab === '触发与动作') content = group('触发',[
    settingRow('全局呼出热键','仅演示修改，不接管系统快捷键。',field('clipHotkey','剪贴板热键','Alt + C')),
    settingRow('剪贴板捕获','不会监听或读取真实剪贴板。',toggle('capture','剪贴板捕获'))
  ]) + group('快捷动作',[
    settingRow('普通点击','保留原始格式的默认动作。','<span>原格式粘贴</span>'),
    ...['Ctrl','Alt','Shift'].map((modifier,index) => settingRow(`${modifier} + 点击`,'此处只模拟动作映射。',selectField(`modifier${modifier}`,`${modifier} 点击动作`,actions,actions[index + 1])))
  ]);
  if (tab === '保留') content = group('历史保留',[
    settingRow('最大普通记录','置顶与收藏记录不参与普通溢出清理。',field('maxHistory','最大普通记录','1000','number')),
    settingRow('自动清理天数','过期记录先进入回收状态，不立即物理删除。',field('retention','自动清理天数','30','number')),
    settingRow('文本大小上限','按 UTF-8 字节计算；超限跳过，不截断。',field('textLimit','文本大小上限','1048576','number')),
    settingRow('图片文件上限','跳过超限图片，保留其他适用格式。',field('imageLimit','图片文件上限','20971520','number')),
    settingRow('图片像素上限','不在原型中解码或处理真实图片。',field('pixelLimit','图片像素上限','40000000','number'))
  ]);
  if (tab === '隐私') content = group('隐私',[
    settingRow('应用黑名单','为每个应用分别禁用剪贴板或速启。',btn('全局设置','open-global')),
    settingRow('捕获状态','原型从不捕获数据，所有历史均为合成条目。','<span class="support">仅合成数据</span>')
  ]);
  if (tab === '存储') content = group('存储与维护',[
    settingRow('存储位置','合成路径；不会创建或移动任何文件。',field('clipStorage','存储位置','D:\\Demo\\剪贴板')),
    settingRow('历史归档','便携 .nxtclip；此处不执行真实导入 / 导出。',btn('模拟导出','simulate','data-message="已模拟导出；未生成归档文件。"')),
    settingRow('数据库维护','真实备份 / 压缩属于产品运行时。',btn('模拟备份','simulate','data-message="已模拟备份；未打开数据库。"')),
    settingRow('存储迁移','保持存储边界，不伪造文件选择器。',btn('模拟迁移','simulate','data-message="已模拟迁移；没有移动文件。"'))
  ]);
  if (!['normal','long'].includes(model.state)) content = stateBlock(model.state) + content;
  return mainShell(workHeader('剪贴板','设置',btn('历史弹窗','page','class="small" data-page="clipboardPopup"')) + tabs(['触发与动作','保留','隐私','存储'],tab,'clip-tab') + `<div class="work-content scroll" data-scroll-region="settings">${content}</div>` + status());
}
function renderTools() {
  return mainShell(workHeader('工具','当前仅链接检测') + `<div class="work-content scroll" data-scroll-region="tools">${!['normal','long'].includes(model.state) ? stateBlock(model.state) : ''}
    <div class="tools-row">${icon('link')}<div><h2>链接检测</h2><p class="description">检测链接、文件和目录；默认仅复制。不会读取真实剪贴板。</p></div>${toggle('linkEnabled','启用链接检测')}</div>
    ${group('链接检测设置',[
      settingRow('仅复制','默认策略；不自动启动外部应用。',toggle('copyOnly','链接仅复制')),
      settingRow('在浏览器打开','显式选择后才路由；这里只更改演示设置。',toggle('browserRoute','链接浏览器路由',false)),
      settingRow('自动隐藏','生成作用域内的隐藏策略；原型不运行后台定时器。',field('hideMs','自动隐藏毫秒','6000','number')),
      settingRow('浮窗位置','在显示器边界内放置；原型不操作显示器。',selectField('linkPosition','浮窗位置',['居中','左上','右上'],'居中'))
    ])}<p class="wireframe-label">链接面板线框 · 不调用系统 / 浏览器</p><div class="wireframe"><div class="actions">${icon('link')}<span class="mono">https://demo.invalid/资料</span>${btn('模拟复制','simulate','class="small" data-message="已模拟复制链接；真实剪贴板未改变。"')}</div></div></div>` + status());
}

// Global settings owns a separate local draft. Failed Apply never discards it.
function renderRules() {
  const rules = model.state === 'empty' ? [] : model.rules;
  return `<div class="rules-grid rules-heading"><span>应用进程</span><span>剪贴板</span><span>速启</span><span>操作</span></div>
    <div class="scroll" style="flex:1" data-scroll-region="rules">${rules.map((rule,index) => `<div class="rules-grid rule-row" data-rule="${index}">
      ${btn(esc(longName(rule.name)),'rule-edit',`class="rule-name ellipsis" title="${esc(longName(rule.name))}" data-index="${index}" ${disabled() ? 'disabled' : ''}`)}
      <span class="feature"><input type="checkbox" aria-label="禁用 ${esc(rule.name)} 的剪贴板" data-rule-toggle="${index}" data-feature="clip" ${checked(rule.clip)} ${disabled() ? 'disabled' : ''}></span>
      <span class="feature"><input type="checkbox" aria-label="禁用 ${esc(rule.name)} 的速启" data-rule-toggle="${index}" data-feature="quick" ${checked(rule.quick)} ${disabled() ? 'disabled' : ''}></span>
      ${iconBtn('close','移除 ' + rule.name,'rule-remove',`data-index="${index}" ${disabled() ? 'disabled' : ''}`)}</div>`).join('') || stateBlock('empty','规则')}${['loading','busy'].includes(model.state) ? stateBlock(model.state,'规则') : ''}</div>
    <div class="rule-editor rules-grid"><input id="rule-name" type="text" aria-label="应用进程名称" placeholder="输入应用进程名.exe" value="${esc(model.ruleDraft.name)}" ${disabled() ? 'disabled' : ''}>
      <label title="禁用剪贴板"><input id="rule-clip" type="checkbox" aria-label="新规则禁用剪贴板" ${checked(model.ruleDraft.clip)} ${disabled() ? 'disabled' : ''}></label>
      <label title="禁用速启"><input id="rule-quick" type="checkbox" aria-label="新规则禁用速启" ${checked(model.ruleDraft.quick)} ${disabled() ? 'disabled' : ''}></label>
      ${btn(model.editRule >= 0 ? '更新' : '添加','rule-add',`id="rule-add" ${disabled() ? 'disabled' : ''}`)}</div>`;
}
function renderGlobal() {
  const titles = { '常规':['常规','启动与窗口行为；修改仅保留在本次演示中。'], '网络':['网络','路由配置草稿；不会发起连接或读取系统代理。'], '应用黑名单':['应用黑名单','勾选后，在该应用中禁用对应功能。两项互相独立。'] };
  let content = renderRules();
  if (model.globalTab === '常规') content = `<div class="scroll" style="flex:1" data-scroll-region="global">${group('启动与窗口',[
    settingRow('开机启动','不修改注册表或计划任务。',toggle('startup','开机启动',false,true)),
    settingRow('关闭后留在托盘','不创建实际托盘图标。',toggle('tray','关闭后留在托盘',true,true))
  ])}${['loading','busy'].includes(model.state) ? stateBlock(model.state) : ''}</div>`;
  if (model.globalTab === '网络') content = `<div class="scroll" style="flex:1" data-scroll-region="global">${group('网络路由',[
    settingRow('路由策略','每次操作使用独立路由快照。',selectField('route','路由策略',['系统代理','直连','手动代理'],'系统代理',true)),
    settingRow('代理地址','仅验证演示格式，不测试连接。',field('proxy','代理地址','127.0.0.1:7890','text',true))
  ])}${['loading','busy'].includes(model.state) ? stateBlock(model.state) : ''}</div>`;
  const message = model.globalMessage || (model.state === 'error' ? '模拟应用失败；草稿保留。切换正常状态后重试。' : '');
  return chrome('全局设置') + `<div class="global-shell"><nav class="rail" aria-label="全局设置分类">${['常规','网络','应用黑名单'].map(tab => btn(tab,'global-tab',`data-value="${tab}" aria-selected="${tab === model.globalTab}"`)).join('')}</nav>
    <section class="global-content"><div class="global-heading"><h1>${titles[model.globalTab][0]}</h1><p class="description">${titles[model.globalTab][1]}</p></div>${content}<div class="global-message ${message.includes('失败') || message.includes('请输入') || message.includes('已存在') ? 'error-text' : 'support'}" id="global-message" role="status">${esc(message)}</div></section></div>
    <footer class="dialog-footer" data-region="footer"><span class="footer-message">${model.globalDirty ? '有未应用的演示更改' : '只在内存中模拟，不写入配置'}</span>${btn('取消更改','global-cancel')}${btn(model.state === 'busy' ? '正在应用…' : '应用','global-apply',`class="primary" id="global-apply" ${disabled() || !model.globalDirty ? 'disabled' : ''}`)}</footer>`;
}

// Search popup uses an in-window action mode, not a detached Shell menu.
function quickResults() {
  const items = model.actionMode ? [
    { name:'打开',detail:'模拟 Shell 打开，不访问目标',kind:'动作',glyph:'play' },
    { name:'打开所在文件夹',detail:'模拟定位，不启动资源管理器',kind:'动作',glyph:'folder' },
    { name:'复制路径',detail:'模拟复制，不修改剪贴板',kind:'动作',glyph:'clipboard' }
  ] : [
    { name:'产品说明书_中文版.pdf',detail:'D:\\Demo\\项目资料',kind:'文件',glyph:'file' },
    { name:'项目资料',detail:'D:\\Demo\\项目资料',kind:'文件夹',glyph:'folder' },
    { name:'演示编辑器',detail:'C:\\Demo\\Editor.exe',kind:'应用',glyph:'tools' },
    { name:'设计评审记录.txt',detail:'D:\\Demo\\评审',kind:'文件',glyph:'file' }
  ];
  if (model.state === 'empty') return [];
  return items.filter(item => `${item.name} ${item.detail}`.toLowerCase().includes(model.query.toLowerCase()));
}
function renderQuickCollapsed() {
  return `<section class="popup"><div class="popup-search">${icon('search')}<input id="collapsed-query" aria-label="速启搜索（输入后展示合成结果）" placeholder="搜索应用、文件与命令" value="">${iconBtn('close','关闭搜索演示','close-window')}</div></section>`;
}
function renderQuickPopup() {
  const results = quickResults();
  model.resultIndex = Math.min(model.resultIndex,Math.max(0,results.length - 1));
  return `<section class="popup">${model.actionMode ? `<div class="popup-breadcrumb">${iconBtn('back','返回搜索','exit-action')}<span class="ellipsis">${esc(model.actionTarget || '所选结果')} / 动作</span></div>` : ''}
    <div class="popup-search">${icon('search')}<input id="quick-query" role="combobox" aria-label="速启搜索" aria-expanded="true" aria-controls="quick-results" aria-activedescendant="quick-result-${model.resultIndex}" placeholder="${model.actionMode ? '搜索动作' : '搜索应用、文件与命令'}" value="${esc(model.query)}">${iconBtn('close','关闭搜索演示','close-window')}</div>
    <div id="quick-results" class="result-list scroll" role="listbox" aria-label="搜索结果" data-scroll-region="results">${['loading','error','busy'].includes(model.state) ? stateBlock(model.state,'搜索结果') : results.map((item,index) => `<button type="button" role="option" class="result-row ${model.animateList ? 'state-enter' : ''}" style="--row-index:${index}" id="quick-result-${index}" data-action="quick-result" data-index="${index}" aria-selected="${index === model.resultIndex}" title="${esc(longName(item.name))}">${icon(item.glyph)}<span style="min-width:0"><span class="result-name ellipsis">${esc(longName(item.name))}</span><span class="description mono ellipsis">${esc(item.detail)}</span></span><span class="support">${item.kind}</span></button>`).join('') || stateBlock('empty','搜索结果')}</div>
    <footer class="popup-footer" data-region="footer"><span>↑ ↓ 选择 · Enter 模拟打开 · → 动作</span>${btn('预览线框','show-preview','class="quiet small"')}</footer></section>`;
}
function visibleHistory() {
  if (model.state === 'empty') return [];
  return model.history.filter(item => (model.clipCategory === '全部' || model.clipCategory === item.kind) && item.text.toLowerCase().includes(model.clipQuery.toLowerCase()));
}
function renderClipboardPopup() {
  const items = visibleHistory();
  model.clipIndex = Math.min(model.clipIndex,Math.max(0,items.length - 1));
  const editing = model.editClip !== null;
  const item = model.history.find(entry => entry.id === model.editClip);
  return `<section class="popup compact"><div ${editing ? 'inert' : ''} style="display:contents"><header class="clip-header"><h2>剪贴板</h2>${iconBtn('settings','剪贴板设置','page','data-page="clipboardSettings"')}${iconBtn('close','关闭历史演示','close-window')}</header>
    <input id="clip-query" class="clip-search" aria-label="搜索剪贴板历史" placeholder="搜索演示历史" value="${esc(model.clipQuery)}">
    <nav class="clip-categories" aria-label="历史类别">${['全部','文本','图片','文件'].map(category => btn(category,'clip-category',`aria-pressed="${category === model.clipCategory}" data-value="${category}"`)).join('')}</nav>
    <div class="clip-list scroll" data-scroll-region="history">${['loading','error','busy'].includes(model.state) ? stateBlock(model.state,'历史') : items.map((entry,index) => `<div class="clip-row ${index === model.clipIndex ? 'selected' : ''}"><button type="button" class="clip-content" data-action="clip-activate" data-index="${index}" title="${esc(entry.text)}"><span class="ellipsis">${esc(longName(entry.text))}</span><span class="description">${entry.source}</span><span class="description">${entry.kind} · ${entry.time}</span></button>${iconBtn('more','编辑条目','clip-edit',`data-id="${entry.id}"`)}</div>`).join('') || stateBlock('empty','历史')}</div><footer class="clip-footer" data-region="footer">${items.length} 项演示记录 · 不读取真实剪贴板</footer></div>
    ${editing && item ? `<section class="edit-sheet" role="dialog" aria-modal="true" aria-label="编辑演示条目"><h2>编辑演示条目</h2><p class="description">只修改本次演示，不写入剪贴板。</p><textarea id="clip-edit-text" aria-label="条目文本">${esc(item.text)}</textarea><p id="clip-edit-error" class="description error-text" role="alert">${model.state === 'error' ? '模拟保存失败，文本仍保留。' : ''}</p><div class="actions">${btn('取消','clip-edit-cancel')}${btn('保存','clip-edit-save','class="primary"')}</div></section>` : ''}</section>`;
}

// Purpose-sized download windows. Chrome counts toward probe/confirmation only.
function compactFooter(message,actions) { return `<footer class="dialog-footer" data-region="footer"><span class="footer-message">${message}</span>${actions}</footer>`; }
function renderProbe() {
  const message = {normal:'先探测，再确认保存位置。',long:'输入框滚动显示完整链接，不改变窗口尺寸。',empty:'请输入演示链接。',loading:'正在模拟探测；没有网络请求。',error:'模拟探测失败，请检查链接后重试。',busy:'正在模拟探测；可取消。'}[model.state];
  return chrome('新建下载') + `<section class="compact-body"><label for="probe-url">下载链接</label><div class="probe-input"><input id="probe-url" aria-label="下载链接" class="mono" value="${model.state === 'empty' ? '' : 'https://demo.invalid/' + (model.state === 'long' ? 'synthetic/very-long-resource-name/'.repeat(5) : '') + '资料包.zip'}" ${disabled() ? 'disabled' : ''}></div><p id="probe-message" class="description ${model.state === 'error' ? 'error-text' : ''}" style="margin-top:4px">${message}</p></section>` + compactFooter('演示 · 不访问链接',btn('取消','probe-cancel') + btn(disabled() ? '探测中…' : '探测链接','probe-submit',`class="primary" ${disabled() ? 'disabled' : ''}`));
}
function renderConfirm() {
  const media = ['dash','hls','mediaError'].includes(model.page);
  const title = media ? '媒体下载确认' : '新建下载确认';
  const fields = `<div class="confirm-summary"><span>演示资源 · ${media ? '媒体' : '压缩包'}</span><span class="mono">${media ? '大小待确认' : '1.24 GB · 支持续传'}</span></div>
    <div class="compact-field"><label for="confirm-name">文件名</label><input id="confirm-name" value="${esc(longName(media ? '设计评审录屏.mp4' : '项目资料包.zip'))}" ${disabled() ? 'disabled' : ''}></div>
    <div class="compact-field"><label for="confirm-path">保存到</label><div class="field-plus"><input id="confirm-path" class="mono" value="${esc(FIXTURE_PATH)}" ${disabled() ? 'disabled' : ''}>${btn('模拟选择','simulate','data-message="原型不访问目录选择器；保存位置仍为合成路径。"')}</div></div>
    <div class="compact-field"><label for="confirm-category">分类</label><select id="confirm-category" ${disabled() ? 'disabled' : ''}><option>未分类</option><option>文档</option><option>视频</option><option>压缩包</option></select></div>`;
  let choices = '';
  if (media) choices = `<section class="media-group"><h3>${model.page === 'dash' ? 'DASH · 一组音视频' : 'HLS · 选择播放列表'}</h3>${(model.page === 'dash' ? ['1080p 视频 + AAC 音频'] : ['1080p · 合成播放列表','720p · 合成播放列表']).map((label,index) => btn(`${icon(model.mediaSelected === index ? 'check' : 'file')}${label}`,'media-select',`class="media-choice" aria-pressed="${index === model.mediaSelected}" data-index="${index}" ${disabled() ? 'disabled' : ''}`)).join('')}<p class="description">线框 · 未解析真实流；不保证任何原生媒体提供方能力。</p></section>`;
  const error = model.page === 'mediaError' || model.state === 'error';
  return chrome(title) + `<section class="compact-body scroll" data-scroll-region="confirmation">${fields}${choices}<p class="description ${error ? 'error-text' : ''}" id="confirm-message" role="status" style="margin-top:8px">${error ? '模拟确认失败；文件名和位置保留，可返回后重试。' : disabled() ? '正在模拟确认，禁止重复提交。' : model.state === 'empty' ? '当前没有可用资源；返回并重新探测。' : '仅添加会保留暂停状态；立即下载只模拟入队。'}</p></section>` + compactFooter('所有文件操作仅模拟',btn('返回','page','data-page="probe"') + btn('仅添加','confirm-add',`${disabled() || model.state === 'empty' ? 'disabled' : ''}`) + btn(disabled() ? '处理中…' : '立即下载','confirm-start',`class="primary" ${disabled() || model.state === 'empty' ? 'disabled' : ''}`));
}
function renderTaskWindow() {
  const completed = model.page === 'completed';
  const failure = model.page === 'failed' || model.state === 'error';
  const expanded = model.page === 'expanded';
  const title = model.state === 'empty' ? '暂无演示任务' : model.state === 'loading' ? '正在加载演示任务' : completed ? '下载完成' : failure ? '下载失败' : model.paused ? '已暂停' : '正在下载';
  return `<section class="task-window compact"><header class="task-window-title">${icon(completed ? 'check' : failure ? 'alert' : 'download')}<h2>${title}</h2>${iconBtn('close','关闭进度演示，不停止任务','close-window')}</header>
    <p class="file-name wrap" data-scroll-region="task-name">${esc(longName('项目资料包_秋季交付.zip'))}</p>
    ${failure ? `<p class="error-box" role="alert">模拟失败：目标位置不可写。${model.state === 'long' ? '这是用于验证长错误换行和固定底部动作的合成失败信息。'.repeat(4) : '任务进度保留，可重试。'}</p>` : `<div class="metrics support"><span class="mono">${['empty','loading'].includes(model.state) ? '— / —' : completed ? '1.24 / 1.24 GB' : '812 MB / 1.24 GB'}</span><span>${model.state === 'empty' ? '暂无演示任务' : completed ? '已完成 · 演示' : disabled() ? '正在处理…' : model.paused ? '已暂停' : '8.6 MB/s · 剩余 52 秒'}</span></div>${!completed ? `<div class="progress-track"><div class="progress-fill" style="width:${['empty','loading'].includes(model.state) ? 0 : 64}%"></div></div>` : ''}`}
    ${expanded ? `<div class="task-expanded" data-scroll-region="task-details"><div class="metrics support"><span>连接</span><span class="mono">4 / 8（演示）</span></div><div class="metrics support"><span>保存位置</span><span class="mono">${esc(FIXTURE_PATH)}</span></div><div class="metrics support"><span>校验</span><span>下载后检查 · 演示</span></div></div>` : ''}
    <footer class="actions" data-region="footer">${completed ? btn('模拟打开','simulate','data-message="已模拟打开文件；未访问文件系统。"') + btn('模拟定位','simulate','data-message="已模拟定位；未启动资源管理器。"') : failure ? btn('重试','task-retry','class="primary"') : btn(expanded ? '收起' : '详情','page',`data-page="${expanded ? 'progress' : 'expanded'}"`) + btn(model.paused ? '继续' : '暂停','progress-toggle',`${disabled() ? 'disabled' : ''}`)}${btn('关闭','close-window')}</footer></section>`;
}

// Additional surfaces are explicitly wireframes where a native/provider boundary is needed.
const SUPPORT_TABS = ['组件状态','菜单与子菜单','通知','被动预览','计划与删除','关机动作','浏览器扩展','媒体与拖拽'];
function renderSupportContent() {
  switch (model.supportTab) {
    case '组件状态': return `<h1>组件状态</h1><p class="description">同一语义角色共享高度、基线和焦点边界。</p>
      <div class="component-line">${btn('主要动作','simulate','class="primary" data-message="演示动作已完成。"')}${btn('次要动作','simulate','data-message="次要演示动作已完成。"')}${btn('禁用动作','simulate','disabled title="当前操作不可用" data-message="不应触发"')}${btn('已选择','simulate','aria-pressed="true" data-message="已选择状态示例。"')}</div>
      <div class="component-line"><input aria-label="组件输入示例" placeholder="32px 输入框"><label class="actions"><input type="checkbox" checked>启用规则</label>${toggle('componentToggle','组件开关')}</div>
      <div class="component-line"><div class="segments">${['全部','进行中','完成'].map(name => btn(name,'component-segment',`data-value="${name}" aria-pressed="${(model.settings.segment || '全部') === name}"`)).join('')}</div></div>${stateBlock(model.state === 'normal' || model.state === 'long' ? 'error' : model.state)}<p class="note">Hover / pressed / keyboard focus 可直接交互；无持续动画。禁用控件不派发。</p>`;
    case '菜单与子菜单': return `<h1>菜单与子菜单</h1><p class="description">28px 行 · 固定图标槽 · Esc 逐层返回 · ↑ ↓ 移动</p><div class="menu-columns"><div class="menu-spec" role="menu" aria-label="任务菜单">
      ${btn('模拟打开 <span>Enter</span>','simulate','role="menuitem" data-message="已模拟打开；无系统操作。"')}${btn('复制 <span>›</span>','submenu','role="menuitem" aria-haspopup="menu" aria-expanded="' + model.submenu + '"')}
      <hr>${btn('计划开始…','schedule','role="menuitem"')}${btn('移除…','delete-wire','role="menuitem" class="danger"')}</div>${model.submenu ? `<div class="menu-spec state-enter" role="menu" aria-label="复制子菜单">${btn('模拟复制路径','simulate','role="menuitem" data-message="已模拟复制路径；未修改剪贴板。"')}${btn('模拟复制链接','simulate','role="menuitem" data-message="已模拟复制链接；未修改剪贴板。"')}</div>` : ''}</div><p class="wireframe-label">Shell 能力与原生子菜单提供方不在浏览器中执行。</p>`;
    case '通知': return `<h1>通知</h1><p class="description">紧凑状态表面 · 原应用中锚定右下角 · 此处为布局样本</p>${model.noticeDismissed ? stateBlock('empty','通知') : `<div class="notification"><strong>演示下载已完成</strong><p class="description">项目资料包_秋季交付.zip</p><div class="actions">${btn('模拟定位','simulate','class="small" data-message="已模拟定位；未启动资源管理器。"')}${btn('关闭','dismiss-notice','class="small"')}</div></div>`}<div class="notification error"><strong>演示操作失败</strong><p class="description wrap">目标位置不可写。草稿与任务进度保留。</p><div class="actions">${btn('重试','recover','class="small"')}</div></div>`;
    case '被动预览': return `<h1>独立被动预览</h1><p class="description">单一预览，选中项更新在原窗口中；这里不创建 HWND。</p><div class="preview-wire wireframe"><div><h2>主搜索 / 历史窗口</h2><p class="description">始终拥有键盘焦点。预览不接管输入、不成为第二个搜索状态所有者。</p><div class="component-line">${btn('说明书.pdf','preview-file','data-value="说明书.pdf"')}${btn('评审记录.txt','preview-file','data-value="评审记录.txt"')}</div></div><aside class="passive-preview" aria-label="被动预览线框"><h2>${esc(model.settings.previewFile || '说明书.pdf')}</h2><div class="preview-content"><p>被动预览线框</p><p class="description">真实文件预览 / 解析器未运行。</p></div><p class="description" style="margin-top:8px">最终设备像素边界先应用，再显示；WS_EX_NOACTIVATE 需 C# 实现。</p></aside></div>`;
    case '计划与删除': return `<h1>计划开始 / 移除</h1><p class="description">仅模拟用户决策，不调度后台任务。</p><div class="wireframe"><h2>选择开始日期（演示月）</h2><div class="schedule-grid">${Array.from({length:28},(_,i) => btn(i + 1,'select-date',`data-date="${i + 1}" aria-pressed="${model.date === i + 1}"`)).join('')}</div><div class="actions"><input type="time" aria-label="开始时间" value="09:00">${btn('确认计划','simulate',`class="primary" data-message="已模拟计划：演示月 ${model.date} 日；无后台调度。"`)}</div></div><div class="wireframe"><h2>移除演示任务？</h2><p class="description">只移除内存中的所选任务，不删除文件。</p><div class="actions" style="margin-top:12px">${btn('取消','page','data-page="downloads"')}${btn('移除演示任务','delete-confirm','class="danger"')}</div></div>`;
    case '关机动作': return `<h1>完成后的关机动作</h1><p class="wireframe-label">原生边界线框 · 不运行系统命令 / 计时器</p><div class="wireframe"><h2>所有任务完成后（演示）</h2>${settingRow('完成后动作','明确决策；不把原生操作伪装成已执行。',selectField('shutdownMode','完成后动作',['不执行','关机（线框）','睡眠（线框）'],'不执行'))}<div class="actions" style="margin-top:16px">${btn('模拟确认','simulate','class="primary" data-message="已模拟确认完成后动作；不会关闭或挂起电脑。"')}${btn('取消','simulate','data-message="演示动作已取消。"')}</div></div>`;
    case '浏览器扩展': return `<h1>浏览器扩展 Popup</h1><p class="wireframe-label">浏览器 / Native Messaging 边界线框，无扩展、无连接。</p><div class="wireframe extension-wire" style="width:340px"><h2>NxTools 下载</h2><p class="description" style="margin-top:8px">桌面连接：原型未连接</p>${settingRow('下载接管','仅演示开关，不安装扩展。',toggle('extensionCapture','演示下载接管',false))}${btn('当前页媒体','extension-media','class="small"')}<p class="description" style="margin-top:8px">页面捕获、Blob、请求上下文与浏览器权限未运行。</p></div>`;
    case '媒体与拖拽': return `<h1>媒体与原生拖拽</h1><p class="description">各状态保留独立预算，不把媒体行放进普通确认框。</p><div class="component-line">${btn('DASH · 560 × 353','page','data-page="dash"')}${btn('HLS · 560 × 418','page','data-page="hls"')}${btn('媒体错误 · 560 × 456','page','data-page="mediaError"')}</div><div class="wireframe"><h2>最终文件拖拽线框</h2><p class="description">完成后的文件表面 → Windows Shell；仅示意，浏览器不调用 OLE。</p><p class="mono" style="margin:12px 0">D:\Demo\下载\项目资料包.zip</p>${btn('说明原生边界','simulate','data-message="真实拖拽由仍按下的 GPUI 指针回调携带 source HWND，经 C# 专用 STA 进入 OLE；原型不执行。"')}<p class="description" style="margin-top:8px">不阻塞串行平台命令、热键或打开 / 定位操作；不使用浏览器拖拽伪造原生验收。</p></div>`;
  }
}
function renderSupporting() {
  return chrome('配套界面 · 设计线框',true) + `<div class="support-layout"><nav class="rail" aria-label="配套界面">${SUPPORT_TABS.map(tab => btn(tab,'support-tab',`data-value="${tab}" aria-selected="${tab === model.supportTab}"`)).join('')}<div class="rail-bottom">${btn('返回下载','page','data-page="downloads"')}</div></nav><section class="support-body scroll" data-scroll-region="supporting">${renderSupportContent()}</section></div>` + status('配套设计线框 · 不代表原生能力已实现');
}

const RENDERERS = { downloads:renderDownloads,quickSettings:renderQuickSettings,clipboardSettings:renderClipboardSettings,tools:renderTools,global:renderGlobal,quickPopup:renderQuickPopup,quickCollapsed:renderQuickCollapsed,clipboardPopup:renderClipboardPopup,probe:renderProbe,confirm:renderConfirm,progress:renderTaskWindow,completed:renderTaskWindow,expanded:renderTaskWindow,failed:renderTaskWindow,dash:renderConfirm,hls:renderConfirm,mediaError:renderConfirm,supporting:renderSupporting };
function render(focusId = null) {
  const active = frame.contains(document.activeElement) ? document.activeElement : null;
  const focusAttrs = active ? Object.fromEntries([...active.attributes].filter(attr => attr.name === 'id' || attr.name === 'name' || attr.name.startsWith('data-')).map(attr => [attr.name,attr.value])) : null;
  const editingText = document.getElementById('clip-edit-text')?.value;
  captureDrafts();
  const entering = frame.dataset.page !== model.page;
  const previousScroll = Object.fromEntries([...frame.querySelectorAll('[data-scroll-region]')].map(element => [element.dataset.scrollRegion,element.scrollTop]));
  frame.dataset.page = model.page; frame.dataset.state = model.state;
  frame.classList.toggle('compact',['probe','confirm','dash','hls','mediaError','global'].includes(model.page));
  frame.innerHTML = RENDERERS[model.page]();
  if(disabled()) frame.querySelectorAll('[data-action="batch-pause"],[data-action="batch-resume"],[data-action="batch-remove"],[data-action="command-delete"],[data-action="command-add"],[data-action="clip-edit-save"]').forEach(element => {element.disabled = true;});
  model.animateList = false;
  pageSelector.value = model.page;
  document.getElementById('state-selector').value = model.state;
  document.getElementById('size-selector').value = model.sizeMode;
  document.getElementById('size-selector').disabled = !PAGES[model.page].main;
  for(const option of document.getElementById('state-selector').options) option.disabled = !applicableStates().includes(option.value);
  document.getElementById('zoom-selector').value = model.zoom;
  applyGeometry();
  const ruleScroll = frame.querySelector('[data-scroll-region="rules"]');
  if(ruleScroll) frame.querySelector('.global-content').style.setProperty('--scroll-gutter',(ruleScroll.offsetWidth - ruleScroll.clientWidth) + 'px');
  restoreDrafts();
  if(editingText !== undefined && document.getElementById('clip-edit-text')) document.getElementById('clip-edit-text').value = editingText;
  frame.querySelectorAll('[data-scroll-region]').forEach(element => { element.scrollTop = previousScroll[element.dataset.scrollRegion] || 0; });
  if (focusId) document.getElementById(focusId)?.focus({preventScroll:true});
  else if(focusAttrs && Object.keys(focusAttrs).length) {
    const selector = Object.entries(focusAttrs).map(([key,value]) => `[${key}="${CSS.escape(value)}"]`).join('');
    const target = frame.querySelector(selector);
    if(target && !target.disabled && !target.closest('[inert]')) target.focus({preventScroll:true});
    else if(entering) frame.querySelector('input:not(:disabled),button:not(:disabled)')?.focus({preventScroll:true});
  }
  else if(entering) frame.querySelector('input:not(:disabled),button:not(:disabled)')?.focus({preventScroll:true});
}
function captureDrafts() {
  const command = document.getElementById('command-form');
  if(command && !command.querySelector('[name="name"]')?.disabled) {
    const data = new FormData(command); const index = Number(command.dataset.commandIndex);
    model.commandDrafts ||= {};
    model.commandDrafts[index] = {...Object.fromEntries(['name','key','target','args'].map(key => [key,String(data.get(key) || '')])),enabled:data.get('enabled') === 'on'};
  }
  model.downloadDraft ||= {};
  for(const id of ['probe-url','confirm-name','confirm-path','confirm-category']) {
    const element = document.getElementById(id); if(element) model.downloadDraft[id] = element.value;
  }
}
function restoreDrafts() {
  for(const [id,value] of Object.entries(model.downloadDraft || {})) {
    const element = document.getElementById(id); if(element) element.value = value;
  }
}
function dimensions() {
  if (model.customSize && PAGES[model.page].main) return model.customSize;
  return PAGES[model.page].main && model.sizeMode === 'minimum' ? [920,480] : PAGES[model.page].size;
}
function applyGeometry() {
  const [width,height] = dimensions();
  const stage = document.getElementById('stage');
  const padding = parseFloat(getComputedStyle(stage).paddingLeft) * 2;
  const scale = model.zoom === 'fit' ? Math.min(1,(stage.clientWidth - padding - 2)/width,(stage.clientHeight - padding - 2)/height) : 1;
  frame.style.width = width + 'px'; frame.style.height = height + 'px'; frame.style.transform = `scale(${scale})`;
  document.getElementById('frame-space').style.width = width * scale + 'px';
  document.getElementById('frame-space').style.height = height * scale + 'px';
  document.getElementById('geometry-readout').textContent = `${width} × ${height} CSS px · ${model.zoom === 'actual' ? '100% 实际像素' : `适应视口 ${Math.round(scale * 100)}% · 已缩放`}`;
}
function notice(message) { document.getElementById('bench-notice').textContent = message; }
function applicableStates() { return PAGES[model.page].states || STATES; }
function setPage(page) {
  if (!PAGES[page]) throw new Error('Unknown prototype page: ' + page);
  captureDrafts();
  model.page = page; model.customSize = null; model.editClip = null;
  if(!applicableStates().includes(model.state)) model.state = 'normal';
  model.animateList = true;
  render();
}
function setState(state) {
  if (!STATES.includes(state)) throw new Error('Unknown prototype state: ' + state);
  if(model.page === 'progress' && state === 'error') { setPage('failed'); notice('失败使用独立 440 × 196 预算，不挤入 184px 进度窗口。'); return; }
  if (!applicableStates().includes(state)) throw new Error('State not applicable to this prototype page: ' + state);
  captureDrafts(); model.state = state; model.globalMessage = '';
  if(state === 'long') {
    if(['probe','confirm','dash','hls','mediaError'].includes(model.page)) {
      model.downloadDraft = {};
      for(const id of ['probe-url','confirm-name','confirm-path','confirm-category']) document.getElementById(id)?.remove();
    }
    if(model.page === 'quickSettings' && model.quickTab === '命令') {
      const draft = model.commandDrafts?.[model.commandIndex] || model.commands[model.commandIndex];
      model.commandDrafts ||= {};
      model.commandDrafts[model.commandIndex] = {...draft,target:'D:\\Demo\\' + '跨部门项目资料\\'.repeat(8) + '演示文件.pdf'};
      frame.querySelector('#command-form')?.remove();
    }
  }
  render();
}
function setSize(width,height) {
  if (width === 'minimum' || width === 'default') { model.sizeMode = width; model.customSize = null; }
  else {
    if (!PAGES[model.page].main) throw new Error('Fixed-size surfaces cannot be enlarged.');
    if (!Number.isFinite(width) || !Number.isFinite(height) || width < 920 || height < 480) throw new Error('Main minimum is 920 × 480.');
    model.customSize = [width,height];
  }
  applyGeometry();
}
function markGlobalDirty() { model.globalDirty = JSON.stringify(model.rules) !== JSON.stringify(model.savedRules) || JSON.stringify(model.globalDraft) !== JSON.stringify(model.savedGlobal); }
function recover() {
  captureDrafts(); model.state = 'normal';
  if(model.page === 'downloads' && !visibleTasks().length) {model.taskQuery = '';model.filter = '全部';if(!model.tasks.length)model.tasks = clone(INITIAL_TASKS);}
  if(model.page === 'quickPopup') model.query = '';
  if(model.page === 'clipboardPopup') {model.clipQuery = '';model.clipCategory = '全部';}
  render(); notice('演示状态已恢复；原内容和草稿保留。');
}
function support(tab) { model.supportTab = tab; setPage('supporting'); }
function executeQuick() {
  const result = quickResults()[model.resultIndex];
  if (!result || disabled() || model.state === 'error') return;
  notice(`已模拟${model.actionMode ? result.name : '打开'}：${result.name}；未调用 Shell / 剪贴板。`);
}
function enterAction() {
  const result = quickResults()[model.resultIndex];
  if (!result || model.actionMode || disabled()) return;
  model.actionTarget = result.name; model.actionMode = true; model.query = ''; model.resultIndex = 0; render('quick-query');
}
function exitAction() { model.actionMode = false; model.query = ''; model.resultIndex = 0; render('quick-query'); }

const ACTIONS = {
  page: element => setPage(element.dataset.page),
  'open-global': () => setPage('global'), caption: () => notice('已模拟窗口动作；未操作 Windows 窗口。'),
  'close-window': () => { if (model.page === 'global' && model.globalDirty) { model.globalMessage = '草稿尚未应用；请应用或取消更改后关闭。'; render(); return; } setPage('downloads'); notice('演示窗口已关闭；后台任务没有真实运行。'); },
  'new-download': () => setPage('probe'), recover,
  simulate: element => notice(element.dataset.message || '已完成演示动作；没有系统操作。'),
  'task-filter': element => { model.filter = element.dataset.value; model.selected = []; render(); },
  'task-detail': element => { model.detail = Number(element.dataset.id); render(); },
  'task-menu': element => { model.detail = Number(element.dataset.id); support('菜单与子菜单'); },
  'task-toggle': element => { const task = model.tasks.find(item => item.id === Number(element.dataset.id)); if (!task || disabled()) return; task.status = task.status === '下载中' ? '已暂停' : '下载中'; task.speed = task.status === '下载中' ? '8.6 MB/s' : '—'; render(); notice('任务状态已在内存中模拟更改。'); },
  'batch-pause': () => { model.tasks.forEach(task => { if(model.selected.includes(task.id)){task.status = '已暂停';task.speed = '—';} }); render(); },
  'batch-resume': () => { model.tasks.forEach(task => { if(model.selected.includes(task.id)){task.status = '下载中';task.speed = '8.6 MB/s';} }); render(); },
  'batch-remove': () => support('计划与删除'), 'clear-selection': () => { model.selected = []; render(); },
  'sort-tasks': () => { model.tasks.reverse(); render(); notice('已切换演示任务顺序。'); },
  'quick-tab': element => { model.quickTab = element.dataset.value; render(); },
  'clip-tab': element => { model.clipTab = element.dataset.value; render(); },
  'command-select': element => { model.commandIndex = Number(element.dataset.index); render(); },
  'command-add': () => { model.commands.push({name:'新命令',key:'new' + model.commands.length,target:'D:\\Demo\\新目标',args:'',enabled:true}); model.commandIndex = model.commands.length - 1; model.state = 'normal'; render(); },
  'command-delete': () => { frame.querySelector('#command-form')?.remove(); model.commands.splice(model.commandIndex,1); model.commandDrafts = {}; model.commandIndex = Math.max(0,model.commandIndex - 1); render(); },
  'global-tab': element => { model.globalTab = element.dataset.value; render(); },
  'rule-edit': element => { const index = Number(element.dataset.index); model.editRule = index; model.ruleDraft = clone(model.rules[index]); render('rule-name'); },
  'rule-remove': element => { model.rules.splice(Number(element.dataset.index),1); model.editRule = -1; model.ruleDraft = {name:'',clip:true,quick:false}; markGlobalDirty(); render(); },
  'rule-add': () => {
    let name = model.ruleDraft.name.trim().replace(/^.*[\\/]/,'');
    if (!name || /[<>:"|?*]/.test(name)) { model.globalMessage = '请输入有效的应用进程名称，例如 Demo.exe。'; render('rule-name'); return; }
    if (!/\.exe$/i.test(name)) name += '.exe';
    if(model.rules.some((rule,index) => index !== model.editRule && rule.name.toLowerCase() === name.toLowerCase())) { model.globalMessage = '该进程已存在，请编辑现有规则。'; render('rule-name'); return; }
    const rule = {...model.ruleDraft,name};
    if (model.editRule >= 0) model.rules[model.editRule] = rule; else model.rules.push(rule);
    model.ruleDraft = {name:'',clip:true,quick:false}; model.editRule = -1; model.globalMessage = '规则已加入演示草稿；尚未应用。'; model.state = 'normal'; markGlobalDirty(); render('rule-name');
  },
  'global-apply': () => {
    if (!model.globalDirty || disabled()) return;
    if (model.state === 'error') { model.globalMessage = '模拟应用失败；草稿保留。切换正常状态后重试。'; render(); return; }
    if (model.globalDraft.route === '手动代理' && !/^[^\s:]+:\d{1,5}$/.test(model.globalDraft.proxy)) { model.globalMessage = '请输入有效的代理地址（主机:端口）；草稿保留。'; render(); return; }
    model.savedRules = clone(model.rules); model.savedGlobal = clone(model.globalDraft); model.globalDirty = false; model.globalMessage = '已应用到本次演示内存；没有写入真实配置。'; render();
  },
  'global-cancel': () => { model.rules = clone(model.savedRules); model.globalDraft = clone(model.savedGlobal); model.globalDirty = false; model.ruleDraft = {name:'',clip:true,quick:false}; model.editRule = -1; model.globalMessage = '已撤销未应用的演示更改。'; model.state = 'normal'; render(); },
  'show-preview': () => support('被动预览'),
  'quick-result': element => { model.resultIndex = Number(element.dataset.index); executeQuick(); render(); },
  'exit-action': exitAction,
  'clip-category': element => { model.clipCategory = element.dataset.value; model.clipIndex = 0; render(); },
  'clip-activate': element => { model.clipIndex = Number(element.dataset.index); render(); const item = visibleHistory()[model.clipIndex]; if(item) notice(`已模拟原格式粘贴：${item.kind}；没有读取或写入剪贴板。`); },
  'clip-edit': element => { model.editClip = Number(element.dataset.id); render('clip-edit-text'); },
  'clip-edit-cancel': () => { const id = model.editClip; model.editClip = null; render(); frame.querySelector(`[data-action="clip-edit"][data-id="${id}"]`)?.focus(); },
  'clip-edit-save': () => {
    const text = document.getElementById('clip-edit-text').value;
    if (model.state === 'error' || !text.trim()) { document.getElementById('clip-edit-error').textContent = text.trim() ? '模拟保存失败，文本仍保留。' : '请输入条目文本。'; return; }
    const item = model.history.find(entry => entry.id === model.editClip); item.text = text; model.editClip = null; render(); notice('已保存演示条目；真实剪贴板未改变。');
  },
  'probe-cancel': () => { model.state = 'normal'; setPage('downloads'); },
  'probe-submit': () => {
    const url = document.getElementById('probe-url').value;
    if (!/^https?:\/\/.+/i.test(url)) { document.getElementById('probe-message').textContent = '请输入有效的 HTTP / HTTPS 演示链接。'; return; }
    if (model.state === 'error') { document.getElementById('probe-message').textContent = '模拟探测失败，链接保留。切换正常状态后重试。'; return; }
    setPage('confirm'); notice('已模拟探测成功；没有网络请求。');
  },
  'confirm-add': () => confirmTask(false), 'confirm-start': () => confirmTask(true),
  'progress-toggle': () => { model.paused = !model.paused; render(); notice('已模拟暂停 / 继续；没有真实下载。'); },
  'task-retry': () => { model.state = 'normal'; setPage('progress'); notice('已模拟重试；没有网络请求。'); },
  'media-select': element => { model.mediaSelected = Number(element.dataset.index); render(); },
  'support-tab': element => { model.supportTab = element.dataset.value; model.submenu = false; render(); },
  'component-segment': element => { model.settings.segment = element.dataset.value; render(); },
  submenu: () => { model.submenu = !model.submenu; render(); if(model.submenu) frame.querySelector('[aria-label="复制子菜单"] button')?.focus(); },
  'dismiss-notice': () => { model.noticeDismissed = true; render(); },
  'preview-file': element => { model.settings.previewFile = element.dataset.value; render(); },
  'select-date': element => { model.date = Number(element.dataset.date); render(); },
  schedule: () => support('计划与删除'), 'delete-wire': () => support('计划与删除'),
  'delete-confirm': () => { const ids = model.selected.length ? model.selected : [model.detail]; model.tasks = model.tasks.filter(task => !ids.includes(task.id)); model.selected = []; model.detail = model.tasks[0]?.id; setPage('downloads'); notice('已移除演示任务；未删除文件。'); },
  'extension-media': () => support('媒体与拖拽')
};
function confirmTask(start) {
  const name = document.getElementById('confirm-name').value.trim();
  if(!name || model.state === 'error' || model.page === 'mediaError') { document.getElementById('confirm-message').textContent = !name ? '请输入文件名；草稿保留。' : '模拟确认失败；文件名和保存位置保留。'; return; }
  const path = document.getElementById('confirm-path').value.trim();
  model.tasks.unshift({id:Date.now(),name,path,type:'演示文件',size:'1.24 GB',status:start ? '下载中' : '已暂停',speed:start ? '8.6 MB/s' : '—',percent:0});
  model.detail = model.tasks[0].id; model.state = 'normal'; setPage(start ? 'progress' : 'downloads'); model.downloadDraft = {}; notice('已添加到演示内存；没有创建文件或下载。');
}

frame.addEventListener('click', event => {
  const element = event.target.closest('[data-action]');
  if(!element || element.disabled || element.closest('[inert]')) return;
  if(disabled() && ['batch-pause','batch-resume','batch-remove','command-delete','command-add','clip-edit-save'].includes(element.dataset.action)) return;
  ACTIONS[element.dataset.action]?.(element);
});
frame.addEventListener('input', event => {
  const element = event.target;
  if(element.id === 'collapsed-query' && element.value.trim()) { model.query = element.value; model.actionMode = false; setPage('quickPopup'); document.getElementById('quick-query').focus(); return; }
  if(element.id === 'task-search') { model.taskQuery = element.value; rerenderInput(element); }
  if(element.id === 'quick-query') { model.query = element.value; model.resultIndex = 0; rerenderInput(element); }
  if(element.id === 'clip-query') { model.clipQuery = element.value; model.clipIndex = 0; rerenderInput(element); }
  if(element.id === 'rule-name') model.ruleDraft.name = element.value;
});
function rerenderInput(element) { const start = element.selectionStart; const end = element.selectionEnd; const id = element.id; render(id); document.getElementById(id)?.setSelectionRange(start,end); }
frame.addEventListener('change', event => {
  const element = event.target;
  if(element.disabled) return;
  if(element.dataset.setting) { model.settings[element.dataset.setting] = element.type === 'checkbox' ? element.checked : element.value; model.settingsMessage = '已即时模拟更改；仅保留在内存，不写入系统。'; frame.querySelector('.work-status span')?.replaceChildren(model.settingsMessage); notice(model.settingsMessage); }
  if(element.dataset.globalSetting) { model.globalDraft[element.dataset.globalSetting] = element.type === 'checkbox' ? element.checked : element.value; markGlobalDirty(); render(); }
  if(element.dataset.taskCheck) { const id = Number(element.dataset.taskCheck); model.selected = element.checked ? [...new Set([...model.selected,id])] : model.selected.filter(value => value !== id); render(); }
  if(element.hasAttribute('data-select-all')) { model.selected = element.checked ? visibleTasks().map(task => task.id) : []; render(); }
  if(element.dataset.ruleToggle !== undefined) { model.rules[Number(element.dataset.ruleToggle)][element.dataset.feature] = element.checked; markGlobalDirty(); render(); }
  if(element.id === 'rule-clip') model.ruleDraft.clip = element.checked;
  if(element.id === 'rule-quick') model.ruleDraft.quick = element.checked;
});
frame.addEventListener('submit', event => {
  if(event.target.id !== 'command-form') return;
  event.preventDefault(); if(disabled()) return;
  const data = new FormData(event.target);
  const item = Object.fromEntries(['name','key','target','args'].map(key => [key,String(data.get(key) || '').trim()]));
  item.enabled = data.get('enabled') === 'on';
  const failure = model.state === 'error' ? '模拟保存失败；草稿仍保留。' : model.commands.some((command,index) => index !== model.commandIndex && command.key === item.key) ? '关键词已存在，请使用其他关键词。' : '';
  if(failure) { document.getElementById('command-error').textContent = failure; return; }
  model.commands[model.commandIndex] = item; model.settingsMessage = '命令已保存到演示内存；未执行目标。'; render(); notice(model.settingsMessage);
});
frame.addEventListener('keydown', event => {
  const quickInput = event.target.id === 'quick-query';
  if(model.page === 'quickPopup') {
    if(['ArrowDown','ArrowUp'].includes(event.key)) { event.preventDefault(); const length = quickResults().length; model.resultIndex = length ? (model.resultIndex + (event.key === 'ArrowDown' ? 1 : -1) + length) % length : 0; render('quick-query'); frame.querySelector(`[data-index="${model.resultIndex}"]`)?.scrollIntoView({block:'nearest'}); }
    if(event.key === 'ArrowRight' && quickInput && event.target.selectionStart === event.target.value.length) { event.preventDefault(); enterAction(); }
    if(event.key === 'Enter' && quickInput) { event.preventDefault(); executeQuick(); }
    if(event.key === 'Escape') { event.preventDefault(); if(model.actionMode) exitAction(); else ACTIONS['close-window'](); }
  }
  if(model.page === 'clipboardPopup') {
    if(model.editClip !== null) {
      if(event.key === 'Escape') { event.preventDefault(); ACTIONS['clip-edit-cancel'](); }
      if(event.key === 'Tab') { const controls = [...frame.querySelectorAll('.edit-sheet textarea,.edit-sheet button')]; const first = controls[0],last = controls.at(-1); if(event.shiftKey && event.target === first){event.preventDefault();last.focus();} else if(!event.shiftKey && event.target === last){event.preventDefault();first.focus();} }
    } else {
      if(['ArrowDown','ArrowUp'].includes(event.key)) { event.preventDefault(); const length = visibleHistory().length; model.clipIndex = length ? (model.clipIndex + (event.key === 'ArrowDown' ? 1 : -1) + length) % length : 0; render('clip-query'); frame.querySelector('.clip-row.selected')?.scrollIntoView({block:'nearest'}); }
      if(event.key === 'Enter' && event.target.id === 'clip-query') { event.preventDefault(); ACTIONS['clip-activate']({dataset:{index:model.clipIndex}}); }
      if(event.key === 'Escape') ACTIONS['close-window']();
    }
  }
  const menu = event.target.closest('[role="menu"]');
  if(menu && ['ArrowDown','ArrowUp'].includes(event.key)) { event.preventDefault(); const items = [...menu.querySelectorAll('button:not(:disabled)')]; const index = items.indexOf(event.target); items[(index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length]?.focus(); }
  if(model.page === 'supporting' && event.key === 'Escape' && model.submenu) { model.submenu = false; render(); frame.querySelector('[data-action="submenu"]')?.focus(); }
});

// Audit treats intentional scroll and ellipsis as named content policies, not layout defects.
function getAudit() {
  const root = frame.getBoundingClientRect(); const scale = root.width / frame.offsetWidth;
  const outside = [],overflow = [],scrollRegions = [];
  for(const element of frame.querySelectorAll('*')) {
    if(!element.getClientRects().length || element.tagName === 'SVG' || element.closest('svg')) continue;
    const rect = element.getBoundingClientRect(); const style = getComputedStyle(element);
    const owner = element.closest('[data-scroll-region]');
    if((!owner || owner === element) && (rect.left < root.left - 1 || rect.top < root.top - 1 || rect.right > root.right + 1 || rect.bottom > root.bottom + 1)) outside.push({tag:element.tagName,id:element.id,class:element.className,rect:[rect.x,rect.y,rect.width,rect.height]});
    if(element.hasAttribute('data-scroll-region')) scrollRegions.push({name:element.dataset.scrollRegion,client:[element.clientWidth,element.clientHeight],scroll:[element.scrollWidth,element.scrollHeight]});
    if(element.scrollWidth > element.clientWidth + 1 && !['auto','scroll'].includes(style.overflowX) && !element.classList.contains('ellipsis') && !['INPUT','SELECT','TEXTAREA'].includes(element.tagName) && !element.closest('.ellipsis') && style.display !== 'inline') overflow.push({tag:element.tagName,id:element.id,class:element.className,excess:element.scrollWidth-element.clientWidth});
    if(style.overflowX === 'auto' && element.scrollWidth > element.clientWidth + 1) overflow.push({tag:element.tagName,id:element.id,class:element.className,excess:element.scrollWidth-element.clientWidth,kind:'scroll-region-horizontal'});
  }
  const footer = frame.querySelector('[data-region="footer"]')?.getBoundingClientRect();
  return {page:model.page,state:model.state,size:dimensions(),actual:[frame.offsetWidth,frame.offsetHeight],scale,pass:outside.length === 0 && overflow.length === 0,outside,overflow,scrollRegions,footer:footer ? {top:(footer.top-root.top)/scale,bottom:(footer.bottom-root.top)/scale,height:footer.height/scale} : null,focus:document.activeElement?.id || document.activeElement?.dataset.action || document.activeElement?.tagName};
}
window.prototypeApp = Object.freeze({
  pages:clone(PAGES),states:[...STATES],setPage,setState,setSize,getAudit,
  setZoom: value => { if(!['actual','fit'].includes(value)) throw new Error('Unknown zoom'); model.zoom = value; applyGeometry(); document.getElementById('zoom-selector').value = value; },
  getModel: () => clone(model),
  setSection: value => { if(model.page === 'quickSettings') model.quickTab = value; else if(model.page === 'clipboardSettings') model.clipTab = value; else if(model.page === 'global') model.globalTab = value; else if(model.page === 'supporting') model.supportTab = value; else throw new Error('Page has no sections'); render(); },
  reset: () => { frame.replaceChildren(); resetModel(); render(); notice('演示已重置。'); }
});
pageSelector.addEventListener('change', event => setPage(event.target.value));
document.getElementById('state-selector').addEventListener('change', event => setState(event.target.value));
document.getElementById('size-selector').addEventListener('change', event => setSize(event.target.value));
document.getElementById('zoom-selector').addEventListener('change', event => window.prototypeApp.setZoom(event.target.value));
document.getElementById('reset-prototype').addEventListener('click', window.prototypeApp.reset);
document.getElementById('audit-button').addEventListener('click', () => { const audit = getAudit(); notice(audit.pass ? `布局检查通过：${audit.size.join(' × ')}，窗口内无越界 / 非预期水平溢出。` : `发现布局问题：${audit.outside.length} 项越界，${audit.overflow.length} 项水平溢出。详见 prototypeApp.getAudit()。`); });
window.addEventListener('resize',applyGeometry);
render();
