// Dependency-free Chrome CDP checks. Uses only local anonymous pipes, never a server.
// Run: node artifacts/ui-layout-quality/prototype/design-checks.cjs
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawn} = require('node:child_process');
const {pathToFileURL} = require('node:url');
const chrome = process.env.PROTOTYPE_CHROME || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const scratch = process.env.PI_SCRATCH_DIR || os.tmpdir();
const profile = fs.mkdtempSync(path.join(scratch,'nxtools-prototype-chrome-'));
const child = spawn(chrome,['--headless=new','--remote-debugging-pipe','--no-first-run','--no-default-browser-check','--disable-background-networking','--disable-component-update','--disable-sync','--disable-extensions','--disable-default-apps','--metrics-recording-only','--no-proxy-server',`--user-data-dir=${profile}`,'about:blank'],{stdio:['ignore','ignore','pipe','pipe','pipe']});
let serial = 0, buffer = '', session, failures = [], passes = 0;
const pending = new Map();
child.stdio[4].on('data',chunk => {
  buffer += chunk.toString();
  let index;
  while((index = buffer.indexOf('\0')) >= 0) {
    const text = buffer.slice(0,index); buffer = buffer.slice(index + 1);
    if(!text) continue;
    const message = JSON.parse(text);
    if(message.id && pending.has(message.id)) { const {resolve,reject,timer} = pending.get(message.id);clearTimeout(timer);pending.delete(message.id); message.error ? reject(new Error(JSON.stringify(message.error))) : resolve(message.result); }
  }
});
function send(method,params = {},sessionId = session) {
  return new Promise((resolve,reject) => {
    const id = ++serial; const timer = setTimeout(() => reject(new Error('CDP timeout: ' + method)),20000);
    pending.set(id,{resolve,reject,timer}); child.stdio[3].write(JSON.stringify({id,method,params,...(sessionId ? {sessionId} : {})}) + '\0');
  });
}
async function evaluate(expression) {
  const result = await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});
  if(result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
function check(condition,label,detail) { if(condition) passes++; else failures.push({label,detail}); }
async function action(code,label) { const result = await evaluate(`(() => {${code}})()`);check(result,label,result); }
async function run() {
  const target = await send('Target.createTarget',{url:'about:blank'},null);
  session = (await send('Target.attachToTarget',{targetId:target.targetId,flatten:true},null)).sessionId;
  await send('Page.enable');await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride',{width:1440,height:1000,deviceScaleFactor:1,mobile:false});
  await send('Page.navigate',{url:pathToFileURL(path.join(__dirname,'index.html')).href});
  for(let i=0;i<50;i++){ if(await evaluate('!!window.prototypeApp')) break; await new Promise(resolve => setTimeout(resolve,100)); }
  const audits = await evaluate(`(() => {
    const api = prototypeApp, audits = [];
    for(const [page,info] of Object.entries(api.pages)) {
      api.setPage(page);
      for(const size of (info.main ? ['default','minimum'] : ['default'])) {
        api.setSize(size);
        for(const state of (info.states || api.states)) {api.setState(state);audits.push(api.getAudit());}
      }
    }
    api.reset();return audits;
  })()`);
  for(const audit of audits) check(audit.pass,`${audit.page}/${audit.state}/${audit.size.join('x')}`,audit);
  const sections = await evaluate(`(() => {
    const output = [], api = prototypeApp;
    const groups = {quickSettings:['常规','索引','上下文','命令','诊断'],clipboardSettings:['触发与动作','保留','隐私','存储'],global:['常规','网络','应用黑名单'],supporting:['组件状态','菜单与子菜单','通知','被动预览','计划与删除','关机动作','浏览器扩展','媒体与拖拽']};
    for(const [page,sections] of Object.entries(groups)) {api.setPage(page);for(const size of (api.pages[page].main ? ['default','minimum'] : ['default'])){api.setSize(size);for(const section of sections){api.setSection(section);for(const state of ['normal','long','busy','error']){api.setState(state); output.push({...api.getAudit(),section});}}}}
    api.reset();return output;
  })()`);
  for(const audit of sections) check(audit.pass,`${audit.page}/${audit.section}/${audit.state}`,audit);
  await action(`prototypeApp.reset();document.querySelector('[data-task-check="1"]').click();document.querySelector('[data-task-check="2"]').click();document.querySelector('[data-action="batch-pause"]').click();return prototypeApp.getModel().selected.length===2 && prototypeApp.getModel().tasks.slice(0,2).every(t=>t.status==='已暂停');`,'download multiselect / batch pause');
  await action(`const e=document.getElementById('task-search');e.value='不存在';e.dispatchEvent(new Event('input',{bubbles:true}));return !document.querySelector('.task-row') && document.querySelector('.inline-state').textContent.includes('暂无');`,'download search empty');
  await action(`prototypeApp.reset();prototypeApp.setPage('global');const e=document.querySelector('[data-rule-toggle="0"][data-feature="quick"]');e.click();return prototypeApp.getModel().rules[0].clip && prototypeApp.getModel().rules[0].quick && !document.getElementById('global-apply').disabled;`,'independent per-app toggles');
  await action(`prototypeApp.setState('error');const before=JSON.stringify(prototypeApp.getModel().rules);document.getElementById('global-apply').click();return before===JSON.stringify(prototypeApp.getModel().rules) && prototypeApp.getModel().globalDirty && document.getElementById('global-message').textContent.includes('失败');`,'failed Apply retains draft');
  await action(`prototypeApp.setState('normal');document.getElementById('global-apply').click();return !prototypeApp.getModel().globalDirty && document.getElementById('global-message').textContent.includes('内存');`,'Apply success memory only');
  await action(`const e=document.getElementById('rule-name');e.value='合成进程';e.dispatchEvent(new Event('input',{bubbles:true}));document.getElementById('rule-add').click();const added=prototypeApp.getModel().rules.at(-1).name==='合成进程.exe';document.querySelector('[data-action="global-cancel"]').click();return added && prototypeApp.getModel().rules.length===4;`,'add normalization / Cancel');
  await action(`prototypeApp.setPage('quickSettings');prototypeApp.setSection('命令');const e=document.querySelector('#command-form [name="name"]');e.value='编辑后的演示命令';document.getElementById('command-form').requestSubmit();return prototypeApp.getModel().commands[0].name==='编辑后的演示命令';`,'command save');
  await action(`prototypeApp.setPage('clipboardSettings');const e=document.querySelector('[data-setting="capture"]');e.click();return prototypeApp.getModel().settings.capture===false && !document.querySelector('[data-action="global-apply"]');`,'clipboard immediate configuration');
  await action(`prototypeApp.setPage('clipboardPopup');document.querySelector('[data-action="clip-edit"]').click();const e=document.getElementById('clip-edit-text');e.value='修改后的合成历史';document.querySelector('[data-action="clip-edit-save"]').click();return prototypeApp.getModel().history[0].text==='修改后的合成历史' && !document.querySelector('.edit-sheet');`,'clipboard edit save');
  await action(`prototypeApp.setState('error');document.querySelector('[data-action="recover"]').click();document.querySelector('[data-action="clip-edit"]').click();return document.querySelector('[inert]')!==null;`,'edit sheet background inert');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
  check(await evaluate('!document.querySelector(".edit-sheet") && document.activeElement.dataset.action === "clip-edit"'),'Escape closes edit and returns focus');
  await evaluate('prototypeApp.setPage("quickPopup");document.getElementById("quick-query").focus()');
  for(const key of ['ArrowDown','ArrowRight']) {
    await send('Input.dispatchKeyEvent',{type:'keyDown',key,code:key,windowsVirtualKeyCode:key==='ArrowDown'?40:39});await send('Input.dispatchKeyEvent',{type:'keyUp',key,code:key,windowsVirtualKeyCode:key==='ArrowDown'?40:39});
  }
  check(await evaluate('prototypeApp.getModel().actionMode && document.activeElement.id === "quick-query"'),'keyboard arrows enter in-window action mode');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});
  check(await evaluate('document.getElementById("bench-notice").textContent.includes("模拟")'),'Enter simulation action');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
  check(await evaluate('!prototypeApp.getModel().actionMode'),'Escape exits action mode');
  await evaluate('prototypeApp.setPage("global");document.getElementById("rule-name").focus()');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Tab',code:'Tab',windowsVirtualKeyCode:9});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Tab',code:'Tab',windowsVirtualKeyCode:9});
  check(await evaluate('document.activeElement.id === "rule-clip" && getComputedStyle(document.activeElement).outlineWidth === "2px"'),'Tab logical order / visible focus');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:' ',code:'Space',windowsVirtualKeyCode:32});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:' ',code:'Space',windowsVirtualKeyCode:32});
  check(await evaluate('prototypeApp.getModel().ruleDraft.clip === false'),'Space checkbox activation');
  await action(`prototypeApp.setPage('supporting');prototypeApp.setSection('组件状态');const before=document.getElementById('bench-notice').textContent;document.querySelector('button[disabled][data-action="simulate"]').click();return before===document.getElementById('bench-notice').textContent;`,'disabled no dispatch');
  await send('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});
  await action(`prototypeApp.setSection('菜单与子菜单');document.querySelector('[data-action="submenu"]').click();return matchMedia('(prefers-reduced-motion:reduce)').matches && getComputedStyle(document.querySelector('.state-enter')).animationName==='none';`,'reduced motion');
  await send('Emulation.setDeviceMetricsOverride',{width:600,height:800,deviceScaleFactor:1,mobile:false});
  await action(`prototypeApp.reset();prototypeApp.setZoom('fit');const a=prototypeApp.getAudit();return a.pass && a.actual[0]===1060 && a.scale<1 && document.getElementById('geometry-readout').textContent.includes('已缩放');`,'600px workbench fit explicit / fixed actual pixels');
  await send('Emulation.setDeviceMetricsOverride',{width:1440,height:1000,deviceScaleFactor:1,mobile:false});
  await action(`prototypeApp.reset();prototypeApp.setPage('global');const center=e=>{const r=e.getBoundingClientRect();return r.left+r.width/2};const headings=[...document.querySelectorAll('.rules-heading span')];return Math.abs(center(headings[1])-center(document.querySelector('[data-rule-toggle="0"][data-feature="clip"]')))<1 && Math.abs(center(headings[2])-center(document.getElementById('rule-quick')))<1 && getComputedStyle(document.querySelector('.rule-name')).textAlign==='left' && document.getElementById('rule-name').offsetHeight===document.getElementById('rule-add').offsetHeight;`,'blacklist header / checkbox / editor alignment');
  await action(`const top=prototypeApp.getAudit().footer.top;prototypeApp.setState('busy');const busy=prototypeApp.getAudit().footer.top;prototypeApp.setState('error');return top===busy && busy===prototypeApp.getAudit().footer.top;`,'global footer stable across busy / error');
  await action(`prototypeApp.reset();prototypeApp.setPage('global');for(let i=0;i<20;i++){const e=document.getElementById('rule-name');e.value='合成程序_'+i;e.dispatchEvent(new Event('input',{bubbles:true}));document.getElementById('rule-add').click();}const a=prototypeApp.getAudit();const r=document.querySelector('[data-scroll-region="rules"]');const center=e=>{const b=e.getBoundingClientRect();return b.left+b.width/2};return a.pass && r.scrollHeight>r.clientHeight && Math.abs(center(document.querySelector('.rules-heading span:nth-child(2)'))-center(document.getElementById('rule-clip')))<1;`,'many rules bounded scroll / aligned editor');
  await action(`prototypeApp.reset();prototypeApp.setPage('quickSettings');prototypeApp.setSection('命令');document.querySelector('[name="name"]').value='未保存草稿';prototypeApp.setState('error');document.getElementById('command-form').requestSubmit();const kept=document.querySelector('[name="name"]').value==='未保存草稿';prototypeApp.setState('busy');prototypeApp.setState('normal');return kept && document.querySelector('[name="name"]').value==='未保存草稿';`,'command error / busy retain draft');
  await action(`document.querySelector('[data-action="command-select"][data-index="1"]').click();const switched=document.querySelector('[name="name"]').value==='打开文档';document.querySelector('[data-action="command-select"][data-index="0"]').click();return switched && document.querySelector('[name="name"]').value==='未保存草稿';`,'command selection preserves separate drafts');
  await action(`prototypeApp.reset();prototypeApp.setPage('confirm');document.getElementById('confirm-name').value='草稿文件.zip';prototypeApp.setState('error');document.querySelector('[data-action="confirm-start"]').click();prototypeApp.setState('normal');return document.getElementById('confirm-name').value==='草稿文件.zip';`,'confirmation error retry preserves fields');
  await action(`document.querySelector('[data-action="confirm-add"]').click();return prototypeApp.getModel().tasks[0].name==='草稿文件.zip' && prototypeApp.getModel().tasks[0].status==='已暂停';`,'only add creates paused synthetic task');
  await action(`prototypeApp.reset();prototypeApp.setPage('downloads');return document.querySelector('[data-region="chrome"]').offsetHeight===36 && document.querySelector('[data-region="work-header"]').offsetHeight===48 && document.querySelector('.rail').offsetWidth===176 && document.querySelector('.task-row').offsetHeight===48;`,'main chrome / header / rail / row budgets');
  await action(`document.querySelector('[data-task-check="1"]').focus();document.querySelector('[data-task-check="1"]').click();return document.activeElement.dataset.taskCheck==='1';`,'checkbox focus retained after rerender');
  await action(`prototypeApp.setPage('clipboardPopup');document.querySelector('[data-action="clip-edit"]').click();document.getElementById('clip-edit-text').value='未保存编辑';prototypeApp.setState('error');document.querySelector('[data-action="clip-edit-save"]').click();return document.getElementById('clip-edit-text').value==='未保存编辑' && prototypeApp.getModel().history[0].text!=='未保存编辑';`,'clipboard failed save retains text and last good history');
  await action(`prototypeApp.reset();prototypeApp.setPage('quickPopup');const a=prototypeApp.getAudit();return a.pass && a.actual[0]===620 && a.actual[1]===410 && document.querySelector('.popup-search').offsetHeight===42 && document.querySelector('.result-row').offsetHeight===40 && !document.getElementById('app-frame').textContent.includes('esc(task.type)');`,'verified Quick popup and compact row budgets');
  await action(`prototypeApp.reset();prototypeApp.setPage('quickCollapsed');const a=prototypeApp.getAudit();const e=document.getElementById('collapsed-query');e.value='项目';e.dispatchEvent(new Event('input',{bubbles:true}));return a.pass && a.actual[0]===620 && a.actual[1]===44 && prototypeApp.getAudit().actual[1]===410 && document.activeElement.id==='quick-query';`,'collapsed search budget and input expansion');
  await send('Emulation.setDeviceMetricsOverride',{width:1440,height:1000,deviceScaleFactor:1,mobile:false});
  await send('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});
  const screenshotDir=path.join(scratch,'prototype-browser-shots');fs.mkdirSync(screenshotDir,{recursive:true});
  for(const [page,section] of [['downloads',null],['global',null],['quickSettings','命令'],['clipboardSettings','触发与动作'],['quickPopup',null],['clipboardPopup',null],['confirm',null],['progress',null]]) {
    await evaluate(`prototypeApp.reset();prototypeApp.setPage(${JSON.stringify(page)});${section ? `prototypeApp.setSection(${JSON.stringify(section)});` : ''}`);
    const rect=await evaluate(`(() => {const r=document.getElementById('app-frame').getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,scale:1}})()`);
    const shot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:rect});fs.writeFileSync(path.join(screenshotDir,page+'.png'),Buffer.from(shot.data,'base64'));
  }
  console.log('Browser screenshots: '+screenshotDir);
  await send('Emulation.setDeviceMetricsOverride',{width:600,height:800,deviceScaleFactor:1,mobile:false});
  await action(`prototypeApp.reset();prototypeApp.setZoom('actual');return prototypeApp.getAudit().scale===1 && document.getElementById('stage').scrollWidth>document.getElementById('stage').clientWidth;`,'600px workbench actual mode scroll');
  const report = {passes,failures,auditCases:audits.length,sectionCases:sections.length,audits,sections};
  const reportPath = path.join(scratch,'prototype-design-checks.json');fs.writeFileSync(reportPath,JSON.stringify(report,null,2));
  console.log(JSON.stringify({passes,failures:failures.length,auditCases:audits.length,sectionCases:sections.length,reportPath},null,2));
  if(failures.length) console.log(JSON.stringify(failures.slice(0,12),null,2));
  process.exitCode = failures.length ? 1 : 0;
}
run().catch(error => {console.error(error);process.exitCode=1;}).finally(async () => {try{await send('Browser.close',{},null);}catch{}child.kill();for(const {timer} of pending.values())clearTimeout(timer);setTimeout(()=>{try{fs.rmSync(profile,{recursive:true,force:true});}catch{}},200);});
