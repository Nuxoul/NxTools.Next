'use strict';
// Real production DOM/scripts, mocked Chrome API, isolated profile, all network blocked.
const fs = require('node:fs'), path = require('node:path'), os = require('node:os');
const { spawn } = require('node:child_process');
const root = path.resolve(__dirname, '..');
const scratch = process.env.PI_SCRATCH_DIR || os.tmpdir();
const output = path.join(scratch, 'nxtools-extension-redesign');
const profile = fs.mkdtempSync(path.join(scratch, 'nxtools-redesign-'));
const chrome = process.env.PROTOTYPE_CHROME || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const child = spawn(chrome, ['--headless=new', '--remote-debugging-pipe', '--no-first-run', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--disable-extensions', `--user-data-dir=${profile}`, 'about:blank'], { stdio: ['ignore','ignore','pipe','pipe','pipe'] });
let serial = 0, buffer = '', session;
const waiting = new Map();
child.stdio[4].on('data', data => { buffer += data.toString(); let end; while ((end = buffer.indexOf('\0')) >= 0) { const text = buffer.slice(0,end); buffer = buffer.slice(end+1); if (!text) continue; const m = JSON.parse(text), pending = waiting.get(m.id); if (pending) { clearTimeout(pending.timer); waiting.delete(m.id); m.error ? pending.reject(new Error(JSON.stringify(m.error))) : pending.resolve(m.result); } } });
function send(method, params = {}, sid = session) { return new Promise((resolve,reject) => { const id = ++serial; const timer = setTimeout(() => { waiting.delete(id); reject(new Error('CDP timeout '+method)); },20000); waiting.set(id,{resolve,reject,timer}); child.stdio[3].write(JSON.stringify({id,method,params,...(sid ? {sessionId:sid}: {})})+'\0'); }); }
async function evaluate(expression) { const r = await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true}); if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails)); return r.result.value; }
const css = ['popup-tokens.css','popup.css'].map(f => fs.readFileSync(path.join(root,f),'utf8')).join('\n');
const html = fs.readFileSync(path.join(root,'popup.html'),'utf8').replace(/<script\b[\s\S]*?<\/script>/gi,'').replace(/<link\b[^>]*>/gi,'').replace('icons/icon-32.png','data:image/png;base64,'+fs.readFileSync(path.join(root,'icons/icon-32.png')).toString('base64')).replace('</head>',`<style>${css}</style></head>`);
const fixture = fs.readFileSync(path.join(__dirname,'popup-fixture.js'),'utf8');
const popupScripts = [...fs.readFileSync(path.join(root,'popup.html'),'utf8').matchAll(/<script src="([^"]+)"><\/script>/g)].map(m => m[1]);
const production = popupScripts.map(f => fs.readFileSync(path.join(root,f),'utf8')).join('\n;\n');
async function load() { await send('Page.navigate',{url:'about:blank'}); await send('Page.setDocumentContent',{frameId:(await send('Page.getFrameTree')).frameTree.frame.id,html}); await evaluate(fixture+'\n'+production); await evaluate('new Promise(r=>setTimeout(r,20))'); }
async function run() {
 const target = await send('Target.createTarget',{url:'about:blank'},null); session = (await send('Target.attachToTarget',{targetId:target.targetId,flatten:true},null)).sessionId;
 await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable'); await send('Network.setBlockedURLs',{urls:['http://*','https://*','ws://*','wss://*']}); await send('Emulation.setDeviceMetricsOverride',{width:900,height:900,deviceScaleFactor:1,mobile:false});
 await load();
 const results = await evaluate(`(async()=>{
  const rows=[]; const check=(pass,label,detail)=>rows.push({pass:Boolean(pass),label,detail}); const settle=()=>new Promise(r=>setTimeout(r,30));
  check(__fixture.messages.length===1&&__fixture.messages[0].type==='popup_get_state','initialization-passive-snapshot-only');
  check(__fixture.ports.length===1&&__fixture.ports[0].name==='nxtools-popup'&&!__fixture.ports[0].closed,'popup-entry-retains-one-port');
  check(statusText.textContent==='已连接','stable-connected-label');
  renderConnection({ok:false,message:'合成传输状态变化'});check(statusText.textContent==='NxTools 未运行','stable-not-running-label');renderConnection(__fixture.state.status);
  const taskProgress=document.querySelector('progress.bar');check(taskProgress&&Math.abs(taskProgress.value-43.75)<.01&&!taskProgress.hasAttribute('style'),'csp-safe-task-progress-real-value');
  tasksTab.focus();tasksTab.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true}));await settle();
  check(resourcesTab.getAttribute('aria-selected')==='true'&&document.activeElement===resourcesTab,'actual-tab-arrow-focus');
  const filter=document.querySelector('[data-resource-filter="audio"]');filter.click();
  check(filter.getAttribute('aria-pressed')==='true'&&document.querySelector('[data-resource-filter="all"]').getAttribute('aria-pressed')==='false','filter-selected-semantics');
  check(currentResourceList.querySelectorAll('[data-resource-row]').length===1,'filter-real-content');
  document.querySelector('[data-resource-filter="all"]').click();
  const preview=document.querySelector('[data-resource-preview="audio-1"]');
  if(preview){preview.focus();preview.click();const panel=document.querySelector('.resource-preview-panel');check(Boolean(panel),'preview-button-opens');const pb=panel?.getBoundingClientRect(),wb=document.querySelector('.workspace').getBoundingClientRect();check(pb&&pb.top>=wb.top&&pb.bottom<=wb.bottom,'expanded-preview-fully-visible');document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));check(!document.querySelector('.resource-preview-panel')&&document.activeElement?.dataset.resourcePreview==='audio-1','preview-escape-focus-return');}else check(false,'preview-keyboard-control');
  const focusTarget=document.querySelector('[data-resource-preview="audio-1"]');if(focusTarget){focusTarget.focus();render(__fixture.state);check(document.activeElement?.dataset.resourcePreview==='audio-1','refresh-retains-resource-focus');}
  setView('advanced');mediaSpeedInput.focus();mediaSpeedInput.value='1.75';render(__fixture.state);check(mediaSpeedInput.value==='1.75','refresh-retains-media-speed-draft');setView('resources');
  const checks=[...document.querySelectorAll('[data-resource-check]')];checks.slice(0,2).forEach(c=>{c.checked=true;c.dispatchEvent(new Event('change',{bubbles:true}));});
  check(batchResourcesButton.disabled===false&&!document.getElementById('mergeResourcesButton')&&!document.querySelector('.batch-toolbar').hidden,'batch-enabled-merge-not-offered');batchResourcesButton.click();await settle();
  check(__fixture.messages.filter(m=>m.type==='popup_send_resources').length===1,'one-batch-message');
  check(document.querySelector('[data-resource-check="audio-1"]').checked&&!document.querySelector('[data-resource-check="video-1"]').checked,'partial-failure-selection-retained');
  settingsTab.click();await settle();__fixture.failure='popup_save_settings';includeCookies.checked=true;includeCookies.dispatchEvent(new Event('change',{bubbles:true}));await settle();
  check(messageText.textContent.includes('失败')&&__fixture.unhandled.length===0,'autosave-failure-visible-no-unhandled');
  __fixture.failure='';saveButton.click();await settle();check(__fixture.state.settings.includeCookies&&messageText.textContent.includes('保存'),'save-retry-original-policy');
  advancedTab.click();await settle();check(featureOverlayButton.getAttribute('aria-pressed')==='true','feature-pressed-semantics');
  __fixture.failure='popup_toggle_feature';featureDeepScanButton.click();await settle();check(messageText.textContent.includes('失败')&&__fixture.unhandled.length===0,'feature-failure-visible');
  __fixture.failure='popup_media_action';mediaPlayButton.click();await settle();check(messageText.textContent.includes('失败')&&__fixture.unhandled.length===0,'media-failure-visible');
  __fixture.failure='';__fixture.delay=50;const before=__fixture.messages.filter(m=>m.type==='popup_media_action').length;mediaPlayButton.click();mediaPlayButton.click();await new Promise(r=>setTimeout(r,160));check(__fixture.messages.filter(m=>m.type==='popup_media_action').length===before+1,'media-busy-single-dispatch');__fixture.delay=0;
  check(__fixture.unhandled.length===0,'no-unhandled-promises');
  for(const page of ['tasks','resources','advanced','settings']){setView(page);document.querySelector('.workspace').scrollTop=0;const work=document.querySelector('.workspace');check(work.clientHeight>=420&&work.clientHeight<=445,page+'-bounded-workspace');const bad=[...document.querySelectorAll('.view.active *')].filter(e=>{const s=getComputedStyle(e);return e.getClientRects().length&&s.display!=='inline'&&e.scrollWidth>e.clientWidth+1&&!['INPUT','SELECT'].includes(e.tagName)&&!['hidden','auto','scroll'].includes(s.overflowX)}).map(e=>e.className);check(!bad.length,page+'-no-horizontal-overflow',bad);}
  setView('advanced');document.querySelector('.workspace').scrollTop=0;const mediaViewport=document.querySelector('.workspace').getBoundingClientRect();check(featureMobileUaButton.getBoundingClientRect().bottom<=mediaViewport.bottom-4,'all-media-tools-default-visible',{last:featureMobileUaButton.getBoundingClientRect().toJSON(),viewport:mediaViewport.toJSON()});
  setView('settings');const work=document.querySelector('.workspace');work.scrollTop=work.scrollHeight;const s=saveButton.getBoundingClientRect(),w=work.getBoundingClientRect();check(s.top>=w.top&&s.bottom<=w.bottom+1,'settings-save-reachable');
  window.dispatchEvent(new Event('pagehide'));check(__fixture.ports[0].closed,'popup-pagehide-releases-port');
  return rows;
 })()`);
 fs.mkdirSync(output,{recursive:true});
 await load(); await evaluate(`setView('resources');const target=document.querySelector('[data-resource-preview="audio-1"]');target.focus();if(document.activeElement!==target)throw new Error('Preview button focus missing')`);
 await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Enter',code:'Enter',windowsVirtualKeyCode:13,text:'\r',unmodifiedText:'\r'});
 await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});
 results.push({pass:await evaluate(`Boolean(document.querySelector('.resource-preview-panel'))`),label:'real-enter-preview-activation'});
 await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
 await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
 results.push({pass:await evaluate(`!document.querySelector('.resource-preview-panel')&&document.activeElement?.dataset.resourcePreview==='audio-1'`),label:'real-escape-preview-focus'});
 await send('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});
 results.push({pass:await evaluate(`parseFloat(getComputedStyle(document.querySelector('button')).transitionDuration)<=0.001`),label:'actual-reduced-motion'});
 await send('Emulation.setEmulatedMedia',{features:[]});
 // Reload a deterministic fixture for each real production rendering state.
 for(const [name,page,mode] of [['tasks','tasks','normal'],['capture','resources','normal'],['media','advanced','normal'],['settings','settings','normal'],['settings-bottom','settings','bottom'],['empty','resources','empty'],['offline','tasks','offline'],['long','resources','long'],['media-unavailable','advanced','unavailable'],['error','tasks','error'],['busy','resources','busy']]){
  await load(); await evaluate(`(()=>{const s=__fixture.state;${mode==='empty'?'s.currentResources=[];s.otherResources=[];s.captureSummary.currentCount=0;':''}${mode==='offline'?'s.status={ok:false,message:"桌面未运行，不会自动启动"};s.tasks=[];':''}${mode==='unavailable'?'s.mediaItems=[];s.mediaPlaybackState.available=false;':''}${mode==='long'?'s.activeTab.title="合成长页面标题".repeat(20);s.currentResources.forEach(r=>r.filename="完整中文多语言长文件名".repeat(18));':''}render(s);setView('${page}');${mode==='bottom'?'document.querySelector(".workspace").scrollTop=9999;':''}${mode==='error'?'setMessage("合成请求失败，原浏览器下载保持不变。".repeat(4));':''}${mode==='busy'?'selectedResourceIds.add(currentState.currentResources[0].id);renderResourceLists();beginBusy(elements.batchResourcesButton,"提交中…");':''}})()`);
  const shot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:{x:0,y:0,width:456,height:await evaluate('document.body.scrollHeight'),scale:1}});fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(shot.data,'base64'));
 }
 for (const name of ['selected','preview','media-bottom']) {
  await load();
  await evaluate(`setView('${name==='media-bottom'?'advanced':'resources'}');${name==='selected'?"const c=document.querySelector('[data-resource-check]');c.checked=true;c.dispatchEvent(new Event('change',{bubbles:true}));":''}${name==='preview'?"toggleResourcePreview('audio-1');":''}${name==='media-bottom'?"document.querySelector('.workspace').scrollTop=9999;":''}`);
  const shot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:{x:0,y:0,width:456,height:await evaluate('document.body.scrollHeight'),scale:1}});
  fs.writeFileSync(path.join(output,name+'.png'),Buffer.from(shot.data,'base64'));
 }
 const styles=require(path.join(root,'shadow-styles.js'));
 await send('Page.setDocumentContent',{frameId:(await send('Page.getFrameTree')).frameTree.frame.id,html:'<html><body style="margin:0;background:#e4e4e7;font:14px system-ui"><h2 style="margin:24px">合成网页 / 注入界面样式</h2><div id="media" style="position:fixed;left:450px;top:100px"></div><div id="drop" style="position:fixed;left:550px;top:100px;width:188px;height:86px"></div><div id="small" style="position:fixed;left:550px;top:250px;width:96px;height:48px"></div></body></html>'});
 const shadow=await evaluate(`(()=>{const m=document.getElementById('media').attachShadow({mode:'open'});m.innerHTML='<style>'+${JSON.stringify(styles.mediaEntry)}+'</style><button class="trigger" aria-expanded="true"><span class="icon">↓</span><span class="status">已发送</span></button><div class="panel"><button class="choice"><span class="choice-kind"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6.5h11a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 16V8A1.5 1.5 0 0 1 4 6.5z"/><path d="M16.5 10.5l5-3v9l-5-3"/></svg></span><span class="choice-text"><span class="choice-file">课程演示 · 第一章.mp4</span><span class="choice-meta">1080p · 视频 · 64 MB</span></span></button><button class="choice"><span class="choice-kind"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6.5h11a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 16V8A1.5 1.5 0 0 1 4 6.5z"/><path d="M16.5 10.5l5-3v9l-5-3"/></svg></span><span class="choice-text"><span class="choice-file">合成中文长媒体文件名'.repeat(1)+'用于检查完整提示.mp4</span><span class="choice-meta">720p · 视频 · 32 MB</span></span></button></div>';for(const id of ['drop','small']){const s=document.getElementById(id).attachShadow({mode:'open'});s.innerHTML='<style>'+${JSON.stringify(styles.dropTarget)}+'</style><div class="target"><span class="icon">↓</span><strong>拖到这里下载</strong><small>发送到 NxTools 队列</small></div>';}const t=document.getElementById('small').shadowRoot.querySelector('.target');return {fits:t.scrollWidth<=t.clientWidth&&t.scrollHeight<=t.clientHeight};})()`);results.push({pass:shadow.fits,label:'minimum-drop-96x48-contained',detail:shadow});
 await evaluate(`(()=>{const icon='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/></svg>';for(const id of ['media','drop','small'])document.getElementById(id).shadowRoot.querySelector('.icon').innerHTML=icon;})()`);
 const shot=await send('Page.captureScreenshot',{format:'png'});fs.writeFileSync(path.join(output,'injected.png'),Buffer.from(shot.data,'base64'));
 const failures=results.filter(r=>!r.pass);fs.writeFileSync(path.join(output,'checks.json'),JSON.stringify({checks:results.length,failures,results},null,2));console.log(JSON.stringify({checks:results.length,failures,output},null,2));process.exitCode=failures.length?1:0;
}
run().catch(e=>{console.error(e);process.exitCode=1;}).finally(async()=>{try{await send('Browser.close',{},null);}catch{}child.kill();for(const p of waiting.values())clearTimeout(p.timer);setTimeout(()=>{try{fs.rmSync(profile,{recursive:true,force:true});}catch{}},250);});
