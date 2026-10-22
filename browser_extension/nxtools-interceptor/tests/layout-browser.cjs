'use strict';
// Product HTML/CSS layout only. Never loads popup.js or real Chrome extension APIs.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os');
const {spawn}=require('node:child_process');
const root=path.resolve(__dirname,'..');
const scratch=process.env.PI_SCRATCH_DIR||os.tmpdir();
const profile=fs.mkdtempSync(path.join(scratch,'nxtools-extension-layout-'));
const chrome=process.env.PROTOTYPE_CHROME||'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const child=spawn(chrome,['--headless=new','--remote-debugging-pipe','--no-first-run','--disable-background-networking','--disable-component-update','--disable-sync','--disable-extensions',`--user-data-dir=${profile}`,'about:blank'],{stdio:['ignore','ignore','pipe','pipe','pipe']});
let serial=0,buffer='',session; const waiting=new Map();
child.stdio[4].on('data',data=>{buffer+=data.toString();let end;while((end=buffer.indexOf('\0'))>=0){const text=buffer.slice(0,end);buffer=buffer.slice(end+1);if(!text)continue;const m=JSON.parse(text);const p=waiting.get(m.id);if(p){clearTimeout(p.timer);waiting.delete(m.id);m.error?p.reject(new Error(JSON.stringify(m.error))):p.resolve(m.result);}}});
function send(method,params={},sessionId=session){return new Promise((resolve,reject)=>{const id=++serial;const timer=setTimeout(()=>{waiting.delete(id);reject(new Error('CDP timeout '+method));},20000);waiting.set(id,{resolve,reject,timer});child.stdio[3].write(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})})+'\0');});}
async function evaluate(expression){const r=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value;}
async function run(){
 const target=await send('Target.createTarget',{url:'about:blank'},null); session=(await send('Target.attachToTarget',{targetId:target.targetId,flatten:true},null)).sessionId;
 await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable'); await send('Network.setBlockedURLs',{urls:['http://*','https://*','ws://*','wss://*']});
 await send('Emulation.setDeviceMetricsOverride',{width:900,height:900,deviceScaleFactor:1,mobile:false});
 const css=fs.readFileSync(path.join(root,'popup-tokens.css'),'utf8')+'\n'+fs.readFileSync(path.join(root,'popup.css'),'utf8');
 const icon=fs.readFileSync(path.join(root,'icons/icon-32.png')).toString('base64');
 const html=fs.readFileSync(path.join(root,'popup.html'),'utf8').replace(/<script\b[\s\S]*?<\/script>/gi,'').replace(/<link\b[^>]*>/gi,'').replace('icons/icon-32.png','data:image/png;base64,'+icon).replace('</head>',`<style>${css}</style></head>`);
 await send('Page.setDocumentContent',{frameId:(await send('Page.getFrameTree')).frameTree.frame.id,html});
 const results=await evaluate(`(()=>{
   const rows=[];const check=(pass,label,detail)=>rows.push({pass,label,detail});
   check(document.documentElement.getBoundingClientRect().width===456&&document.body.getBoundingClientRect().width===456,'intrinsic456',document.documentElement.getBoundingClientRect().width);
   check(!document.querySelector('script'),'business-script-not-mounted');
   for(const page of ['tasks','resources','advanced','settings']) for(const mode of ['normal','long','empty']){
     document.querySelectorAll('.view').forEach(e=>{const active=e.id===page+'View';e.classList.toggle('active',active);e.setAttribute('aria-hidden',String(!active));});
     document.querySelectorAll('.tab').forEach(e=>e.classList.toggle('active',e.dataset.view===page));
     document.getElementById('activeTabTitle').textContent=mode==='long'?'完整合成页面标题用于检查长内容与控件对齐'.repeat(8):'合成测试页面';
     document.getElementById('activeTabDomain').textContent='demo.invalid';
     document.getElementById('siteToggleHint').textContent='设置仅保留当前演示状态，不请求网页';
     document.getElementById('captureSummaryText').textContent='已捕获合成资源';
     document.getElementById('statusText').textContent='合成连接状态';
     document.getElementById('mediaSelect').innerHTML='<option>合成视频名称</option>';
     const name=mode==='long'?'合成中文文件名称与多语言长内容-'.repeat(12):'测试资料包.zip';
     document.getElementById('taskList').innerHTML=mode==='empty'?'<div class="empty">暂无任务</div>':Array.from({length:mode==='long'?8:3},()=>'<article class="item"><div class="item-main"><strong>'+name+'</strong><small>下载中 · 12 MB / 48 MB</small></div><span class="badge">42%</span><div class="task-actions"><button class="mini-send">暂停</button><button class="mini-send danger">取消</button></div><div class="bar"><i style="width:42%"></i></div></article>').join('');
     document.getElementById('currentResourceList').innerHTML=mode==='empty'?'<div class="empty">暂无资源</div>':Array.from({length:4},()=>'<article class="item resource"><span class="resource-thumb symbol">MP4</span><div class="resource-main"><strong>'+name+'</strong><div class="resource-name">合成文件名</div><div class="resource-url">demo.invalid/'+'synthetic/'.repeat(mode==='long'?30:1)+'</div><div class="resource-meta"><span class="meta-chip">1080p</span><span class="meta-chip">视频</span><span class="meta-chip">48 MB</span></div></div><div class="resource-actions"><button class="mini-send">解析</button></div></article>').join('');
     document.getElementById('messageText').textContent=mode==='long'?'模拟失败信息保留全部文本，可阅读而不截断。'.repeat(5):'全部合成数据，未连接产品';
     const view=document.getElementById(page+'View');const bad=[view,...view.querySelectorAll('*')].filter(e=>{const s=getComputedStyle(e);return e.getClientRects().length&&s.display!=='inline'&&e.scrollWidth>e.clientWidth+1&&!['INPUT','SELECT'].includes(e.tagName)&&!['hidden','auto'].includes(s.overflowX);}).map(e=>({tag:e.tagName,class:e.className,extra:e.scrollWidth-e.clientWidth}));
     check(!bad.length,page+'/'+mode+'/horizontal',bad);
     check(getComputedStyle(document.querySelector('.workspace')).maxHeight==='445px',page+'/'+mode+'/bounded-workspace');
   }
   document.querySelectorAll('.view').forEach(e=>e.classList.toggle('active',e.id==='advancedView'));
   const input=document.getElementById('mediaSpeedInput').getBoundingClientRect(),button=document.getElementById('mediaSpeedButton').getBoundingClientRect();
   check(input.height===32&&button.height===32&&Math.abs(input.y-button.y)<1,'media-form-same-baseline',{input,button});
   document.querySelectorAll('.view').forEach(e=>e.classList.toggle('active',e.id==='settingsView'));
   const work=document.querySelector('.workspace');work.scrollTop=work.scrollHeight;
   const save=document.getElementById('saveButton').getBoundingClientRect(),viewport=work.getBoundingClientRect();
   check(save.top>=viewport.top&&save.bottom<=viewport.bottom+1,'scroll-to-original-save-action');
   for(const row of document.querySelectorAll('.switch-row')){if(!row.getClientRects().length)continue;const toggle=row.querySelector('input');check(toggle.getBoundingClientRect().right<=row.getBoundingClientRect().right+1,'original-toggle-contained');}
   return rows;
 })()`);
 const styles=require(path.join(root,'shadow-styles.js'));
 const shadow=await evaluate(`(()=>{const host=document.createElement('div');host.style.cssText='position:fixed;left:480px;top:100px';document.body.append(host);const shadow=host.attachShadow({mode:'open'});shadow.innerHTML='<style>'+${JSON.stringify(styles.mediaEntry)}+'</style><button class="trigger">↓</button><div class="panel"><button class="choice"><span class="choice-file">'+('合成媒体长名称'.repeat(12))+'</span><span class="choice-meta">1080p · 视频 · 48MB</span></button></div>';const row=shadow.querySelector('.choice').getBoundingClientRect(),title=shadow.querySelector('.choice-file').getBoundingClientRect(),meta=shadow.querySelector('.choice-meta').getBoundingClientRect();return {pass:title.bottom<=meta.top+1&&row.width<=414,rowHeight:row.height,title:getComputedStyle(shadow.querySelector('.choice-file')).fontSize,meta:getComputedStyle(shadow.querySelector('.choice-meta')).fontSize,cssParsed:shadow.querySelector('style').sheet.cssRules.length};})()`);
 results.push({pass:shadow.pass&&shadow.title==='13px'&&shadow.meta==='12px',label:'shadow-resource-two-level',detail:shadow});
 await send('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});
 const motion=await evaluate(`parseFloat(getComputedStyle(document.querySelector('.view.active')).animationDuration)<=0.001`);results.push({pass:motion,label:'popup-reduced-motion'});
 await evaluate(`document.querySelectorAll('body>div').forEach(e=>{if(e.shadowRoot)e.remove();});`);
 const outputDir=path.join(root,'..','..','artifacts/ui-layout-quality/remaining-browser');fs.mkdirSync(outputDir,{recursive:true});
 for(const page of ['tasks','resources','advanced','settings']){await evaluate(`document.querySelectorAll('.view').forEach(e=>e.classList.toggle('active',e.id==='${page}View'));document.querySelector('.workspace').scrollTop=0;`);const shot=await send('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:{x:0,y:0,width:456,height:Math.min(900,await evaluate('document.body.scrollHeight')),scale:1}});fs.writeFileSync(path.join(outputDir,page+'.png'),Buffer.from(shot.data,'base64'));}
 const failures=results.filter(r=>!r.pass);fs.writeFileSync(path.join(scratch,'remaining-extension-layout.json'),JSON.stringify({checks:results.length,failures,results},null,2));console.log(JSON.stringify({checks:results.length,failures,outputDir},null,2));process.exitCode=failures.length?1:0;
}
run().catch(e=>{console.error(e);process.exitCode=1;}).finally(async()=>{try{await send('Browser.close',{},null);}catch{}child.kill();for(const p of waiting.values())clearTimeout(p.timer);setTimeout(()=>{try{fs.rmSync(profile,{recursive:true,force:true});}catch{}},250);});
