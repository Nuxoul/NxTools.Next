import assert from 'node:assert/strict';
import { test } from 'node:test';
import { PassThrough, Writable } from 'node:stream';
import { Session, HostError } from './nxtools.mjs';
const run = '11111111-1111-4111-8111-111111111111';
const frame = value => `${JSON.stringify({ protocol_version: 1, run_id: run, ...value })}\n`;
function fixture() { const input = new PassThrough(); const sent = []; const output = new Writable({ write(chunk, _encoding, callback) { sent.push(JSON.parse(chunk)); callback(); } }); return { input, output, sent }; }
test('actual streams preserve call IDs, completion and one-shot exit ownership', async () => {
  const f = fixture(); f.input.write(frame({ op: 'invoke' })); const session = await Session.start(f.input, f.output);
  f.input.write(frame({ op: 'host_result', call_id: 'call-1', success: true, value: { items: [], has_more: false, next_cursor: null } }));
  const pages = []; for await (const page of session.pages('downloads.list_page')) pages.push(page);
  assert.equal(pages.length, 1); await session.complete('handled', { text: 'done' }); assert.equal(f.sent.at(-1).status, 'handled');
  f.input.destroy();
});
test('host failure never retries or exposes arbitrary private fields', async () => {
  const f = fixture(); f.input.write(frame({ op: 'invoke' })); const session = await Session.start(f.input, f.output);
  f.input.write(frame({ op: 'host_result', call_id: 'call-1', success: false, value: { code: 'native_failed', private: 'secret' } }));
  await assert.rejects(session.call('common.copy', { text: 'fixture' }), e => e instanceof HostError && e.message === 'native_failed'); assert.equal(f.sent.length, 1); session.close(); f.input.destroy();
});
test('input frame budget is enforced before a newline arrives', async () => {
  const f = fixture(); const pending = Session.start(f.input, f.output); f.input.write(Buffer.alloc(262145, 120));
  await assert.rejects(pending, /frame_too_large/); f.input.destroy();
});
test('explicit v2 once handshake and finish preserve session and run identity', async () => {
  const f = fixture(); const sid = '22222222-2222-4222-8222-222222222222';
  f.input.write(frame({ protocol_version: 2, op: 'initialize', session_id: sid, mode: 'once', limits: {} }));
  f.input.write(frame({ protocol_version: 2, op: 'invoke', session_id: sid }));
  const session = await Session.start(f.input, f.output, { protocolVersion: 2 });
  f.input.write(frame({ protocol_version: 2, op: 'host_result', session_id: sid, call_id: 'call-1', success: true, value: { ok: true } }));
  assert.deepEqual(await session.call('host.describe'), { ok: true }); await session.complete();
  assert.deepEqual(f.sent.map(f => f.op), ['ready', 'host_call', 'complete', 'request_finished']);
  assert.equal(f.sent[0].run_id, undefined); assert.ok(f.sent.every(f => f.session_id === sid && f.protocol_version === 2)); f.input.destroy();
});
test('v2 requires explicit opt-in and rejects a foreign session', async () => {
  const sid = '22222222-2222-4222-8222-222222222222';
  for (const explicit of [false, true]) {
    const f = fixture(); f.input.write(frame({ protocol_version: 2, op: 'initialize', session_id: sid, mode: 'once', limits: {} }));
    f.input.write(frame({ protocol_version: 2, op: 'invoke', session_id: run }));
    await assert.rejects(Session.start(f.input, f.output, { protocolVersion: explicit ? 2 : 1 })); f.input.destroy();
  }
});
test('v2 blob helper uses one receiver and preserves exact bytes/ACK/digest', async () => {
  const { createHash } = await import('node:crypto');
  const f=fixture(), sid='22222222-2222-4222-8222-222222222222';
  const data=Buffer.from('abc'), blob={blob_id:'33333333-3333-4333-8333-333333333333',length:3,media_type:'text/plain',sha256:createHash('sha256').update(data).digest('hex')};
  const v2=value=>frame({protocol_version:2,session_id:sid,...value});
  for (const value of [{op:'initialize',mode:'once',limits:{}},{op:'invoke'},
    {op:'host_result',call_id:'call-1',success:true,value:{...blob,transfer_id:'upload'}},
    {op:'data_ack',transfer_id:'upload',sequence:1,offset:3},
    {op:'host_result',call_id:'call-2',success:true,value:blob},
    {op:'host_result',call_id:'call-3',success:true,value:{transfer_id:'download',length:3}},
    {op:'data',transfer_id:'download',sequence:1,offset:0,data:data.toString('base64')},
    {op:'host_result',call_id:'call-4',success:true,value:{}}]) f.input.write(v2(value));
  const session=await Session.start(f.input,f.output,{protocolVersion:2});
  await session.progress({completed:1,total:2,message:'Working'}); await session.progress({message:'Unknown'});
  const sealed=await session.blobUpload(data,'text/plain'); assert.deepEqual(await session.blobRead(sealed),data);
  await session.blobRelease(sealed); await session.complete();
  assert.deepEqual(f.sent.map(value=>value.op),['ready','progress','progress','host_call','data','host_call','host_call','data_ack','host_call','complete','request_finished']);
});
test('v2 progress bounds and serial operation admission do not grow pending readers', async () => {
  const f=fixture(), sid='22222222-2222-4222-8222-222222222222';
  f.input.write(frame({protocol_version:2,op:'initialize',session_id:sid,mode:'once'}));
  f.input.write(frame({protocol_version:2,op:'invoke',session_id:sid}));
  const session=await Session.start(f.input,f.output,{protocolVersion:2});
  for (const value of [{completed:1},{completed:2,total:1},{completed:0,total:0},{completed:0,total:2**53},{message:'汉'.repeat(171)},{message:'\x1b'}]) await assert.rejects(session.progress(value));
  for (let i=0;i<256;i++) await session.progress();
  await assert.rejects(session.progress(),/progress_limit/); session.close();
});
test('serial helper queue is finite and concurrent calls share the original receiver', async () => {
  const f=fixture(); f.input.write(frame({op:'invoke'}));
  const session=await Session.start(f.input,f.output);
  const pending=Array.from({length:8},()=>session.call('fixture.echo'));
  await assert.rejects(session.call('fixture.echo'),/busy/);
  for (let n=1;n<=8;n++) f.input.write(frame({op:'host_result',call_id:`call-${n}`,success:true,value:{n}}));
  assert.deepEqual((await Promise.all(pending)).map(value=>value.n),[1,2,3,4,5,6,7,8]);
  assert.equal(f.sent.length,8); session.close();
});
test('storage helpers preserve null/CAS and never replay a conflict', async () => {
  const f=fixture(), sid='22222222-2222-4222-8222-222222222222';
  const v2=value=>frame({protocol_version:2,session_id:sid,...value});
  for(const value of [{op:'initialize',mode:'once'},{op:'invoke'},
    {op:'host_result',call_id:'call-1',success:true,value:{revision:1,cleanup_pending:false}},
    {op:'host_result',call_id:'call-2',success:true,value:{found:true,value:null,revision:1}},
    {op:'host_result',call_id:'call-3',success:true,value:{items:[],revision:1,has_more:false,next_cursor:null}},
    {op:'host_result',call_id:'call-4',success:false,value:{code:'storage_conflict'}}])f.input.write(v2(value));
  const session=await Session.start(f.input,f.output,{protocolVersion:2});
  assert.equal((await session.storageSet('key',null,{expectedRevision:0})).revision,1);
  assert.equal((await session.storageGet('key')).value,null);
  assert.equal((await session.storageList({prefix:'k',limit:1})).has_more,false);
  await assert.rejects(session.storageDelete('key',{expectedRevision:0}),e=>e instanceof HostError && e.code==='storage_conflict');
  const calls=f.sent.filter(value=>value.op==='host_call');
  assert.equal(calls.length,4);assert.deepEqual(calls[0].params,{key:'key',value:null,expected_revision:0});session.close();
  const legacy=fixture();legacy.input.write(frame({op:'invoke'}));
  const old=await Session.start(legacy.input,legacy.output);
  await assert.rejects(old.storageGet('key'),/unsupported_transport/);old.close();
});
