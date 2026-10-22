'use strict';
const test=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../background.js'),'utf8');
function extract(start,end){const a=source.indexOf(start),b=source.indexOf(end,a+start.length);assert(a>=0&&b>a);return source.slice(a,b);}
function fixture(){
 let timerSerial=0;const timers=new Map(),ports=[];
 const cx=vm.createContext({TextEncoder,Date,console,
  setTimeout(fn,ms){const id=++timerSerial;timers.set(id,{fn,ms});return id;},clearTimeout(id){timers.delete(id);},
  chrome:{runtime:{lastError:null,getManifest:()=>({version:'fixture'}),connectNative(){
   const messages=[],disconnects=[];const port={sent:[],closed:false,onMessage:{addListener(fn){messages.push(fn);}},onDisconnect:{addListener(fn){disconnects.push(fn);}},
    postMessage(message){if(this.closed)throw new Error('closed');this.sent.push(message);},
    receive(message){messages.forEach(fn=>fn(message));},disconnect(){if(this.closed)return;this.closed=true;disconnects.forEach(fn=>fn());}};
   ports.push(port);return port;
  }}}});
 vm.runInContext(extract('let desktopPort = null;','const mobileUserAgentTabs')+
  `const NATIVE_HOST_NAME='fixture';const BROWSER_PROTOCOL_VERSION=2;const MAX_NATIVE_REQUEST_BYTES=256*1024;let serial=0;function nextRequestId(){return 'fixture-'+(++serial);}`+
  extract('function rejectPendingDesktopRequests(','function trimLogText(')+extract('function handleDesktopMessage(','function normalizeHeaders(')+
  `globalThis.api={sendDesktopRequest,ensureDesktopConnection,hold:()=>typeof retainDesktopPort==='function'?retainDesktopPort():()=>{},state:()=>connectionState,pending:()=>pendingDesktopRequests.size};`,cx);
 return {api:cx.api,ports,timers,async flush(){await Promise.resolve();await Promise.resolve();await Promise.resolve();},fireIdle(){for(const [id,t] of [...timers])if(t.ms<=1500){timers.delete(id);t.fn();}}};
}
test('native port releases when idle and never reconnects without a request',async()=>{
 const f=fixture();const promise=f.api.sendDesktopRequest('get_snapshot');await f.flush();assert.equal(f.ports.length,1);
 f.ports[0].receive({requestId:'native-hello',ok:true});await f.flush();const req=f.ports[0].sent.at(-1);
 f.ports[0].receive({requestId:req.requestId,ok:true,data:{}});await promise;await f.flush();f.fireIdle();await f.flush();
 assert.equal(f.ports[0].closed,true,'idle host must receive EOF by port disconnect');
 f.fireIdle();await f.flush();assert.equal(f.ports.length,1,'intentional/remote disconnect must not relaunch an idle host');
});
test('concurrent requests and a Blob lease keep the port until the last owner releases',async()=>{
 const f=fixture();const release=f.api.hold();const first=f.api.sendDesktopRequest('prepare_blob_upload');await f.flush();
 f.ports[0].receive({requestId:'native-hello',ok:true});await f.flush();const firstReq=f.ports[0].sent.at(-1);
 const second=f.api.sendDesktopRequest('get_snapshot');await f.flush();const secondReq=f.ports[0].sent.at(-1);
 f.ports[0].receive({requestId:firstReq.requestId,ok:true});await first;f.fireIdle();assert.equal(f.ports[0].closed,false);
 f.ports[0].receive({requestId:secondReq.requestId,ok:true});await second;f.fireIdle();assert.equal(f.ports[0].closed,false,'Blob capability/upload/commit owner retains native host');
 release();await f.flush();f.fireIdle();assert.equal(f.ports[0].closed,true);
});
test('disconnect rejects once, timeout releases, and stale old ports cannot invalidate a replacement',async()=>{
 const f=fixture();const pending=f.api.sendDesktopRequest('task_action');await f.flush();f.ports[0].receive({requestId:'native-hello',ok:true});await f.flush();
 f.ports[0].disconnect();await assert.rejects(pending,/断开/);f.fireIdle();await f.flush();assert.equal(f.ports.length,1);assert.equal(f.api.pending(),0);
 const next=f.api.sendDesktopRequest('get_snapshot',{},2200);await f.flush();assert.equal(f.ports.length,2);f.ports[1].receive({requestId:'native-hello',ok:true});await f.flush();
 f.ports[0].receive({requestId:'native-hello',ok:true});assert.equal(f.api.state(),'connected');
 const timeout=[...f.timers].find(([,t])=>t.ms===2200);assert(timeout);f.timers.delete(timeout[0]);timeout[1].fn();await assert.rejects(next,/超时/);await f.flush();f.fireIdle();assert.equal(f.ports[1].closed,true);
});
test('startup and diagnostics do not open native transport; Blob flow retains a lease',()=>{
 assert(!extract('chrome.runtime.onStartup.addListener(','chrome.tabs.onActivated').includes('ensureDesktopConnection()'),'browser startup must not create a resident host');
 assert(!extract('function sendExtensionLog(','function resourceSamples(').includes('ensureDesktopConnection()'),'best-effort logs do not launch a host');
 const blob=extract('async function sendBlobResourceViaContentScript(','async function addDownloadToNxTools(');
 assert(blob.includes('retainDesktopPort()')&&blob.includes('finally'),'multi-step Blob operation owns/retires an explicit lease');
});

test('failed handshake and send release ownership without reconnecting',async()=>{
 const f=fixture();const opening=f.api.sendDesktopRequest('get_snapshot');await f.flush();
 const timeout=[...f.timers].find(([,t])=>t.ms===1800);assert(timeout);f.timers.delete(timeout[0]);timeout[1].fn();
 await assert.rejects(opening);await f.flush();assert.equal(f.ports[0].closed,true);assert.equal(f.api.pending(),0);f.fireIdle();assert.equal(f.ports.length,1);
 const pending=f.api.sendDesktopRequest('get_snapshot');await f.flush();const port=f.ports[1];
 port.postMessage=()=>{throw new Error('synthetic post failure');};port.receive({requestId:'native-hello',ok:true});
 await assert.rejects(pending,/发送失败/);await f.flush();assert.equal(f.api.pending(),0);f.fireIdle();assert.equal(port.closed,true);
});
