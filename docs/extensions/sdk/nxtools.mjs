/** Dependency-free JSONL client: v1 default, explicit v2 once. No automatic retries. */
import { TextDecoder } from 'node:util';
import { createHash } from 'node:crypto';
export class HostError extends Error {
  constructor(value = {}) { super(typeof value.code === 'string' ? value.code : 'host_failed'); this.code = this.message; this.retryable = value.retryable === true; this.nativeErrorCode = value.native_error_code; }
}
export class Session {
  static async start(input = process.stdin, output = process.stdout, { protocolVersion = 1 } = {}) {
    if (![1, 2].includes(protocolVersion)) throw new Error('unsupported_transport');
    const session = new Session(input, output); session.protocolVersion = protocolVersion;
    try {
      let invoke = await session.read();
      if (protocolVersion === 2) {
        if (invoke.protocol_version !== 2 || invoke.op !== 'initialize' || invoke.mode !== 'once' || typeof invoke.session_id !== 'string' || !/^[a-f0-9-]{36}$/i.test(invoke.session_id)) throw new Error('invalid_initialize');
        session.sessionId = invoke.session_id; await session.send('ready', {}); invoke = await session.read();
      }
      if (invoke.protocol_version !== protocolVersion || invoke.op !== 'invoke' || typeof invoke.run_id !== 'string' || !/^[a-f0-9-]{36}$/i.test(invoke.run_id) || (protocolVersion === 2 && invoke.session_id !== session.sessionId)) throw new Error('invalid_invoke');
      session.invoke = invoke; session.runId = invoke.run_id; return session;
    } catch (error) { session.close(); throw error; }
  }
  constructor(input, output) {
    this.input = input; this.output = output; this.calls = 0; this.finished = false; this.protocolVersion = 1;
    this.operations = 0; this.serial = Promise.resolve(); this.progressSequence = 0;
    this.dataChunks = 0; this.dataSent = 0; this.dataReceived = 0;
    this.iterator = this.frames()[Symbol.asyncIterator]();
  }
  async *frames() {
    let pending = Buffer.alloc(0);
    for await (const chunk of this.input) {
      const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      let start = 0;
      for (let end = 0; end < bytes.length; end++) {
        if (bytes[end] !== 10) continue;
        if (pending.length + end - start + 1 > 256 * 1024) throw new Error('frame_too_large');
        const line = Buffer.concat([pending, bytes.subarray(start, end)]);
        pending = Buffer.alloc(0); start = end + 1;
        yield new TextDecoder('utf-8', { fatal: true }).decode(line);
      }
      if (pending.length + bytes.length - start > 256 * 1024) throw new Error('frame_too_large');
      pending = Buffer.concat([pending, bytes.subarray(start)]);
    }
    if (pending.length) throw new Error('incomplete_frame');
  }
  async read() {
    const { value, done } = await this.iterator.next();
    if (done) throw new Error('invalid_frame');
    const frame = JSON.parse(value);
    if (!frame || Array.isArray(frame) || typeof frame !== 'object') throw new Error('invalid_frame');
    return frame;
  }
  async send(op, fields) {
    if (this.finished) throw new Error('completed');
    const raw = `${JSON.stringify({ protocol_version: this.protocolVersion, ...(this.runId ? { run_id: this.runId } : {}), ...(this.protocolVersion === 2 ? { session_id: this.sessionId } : {}), op, ...fields })}\n`;
    if (Buffer.byteLength(raw, 'utf8') > 256 * 1024) throw new Error('frame_too_large');
    await new Promise((resolve, reject) => this.output.write(raw, error => error ? reject(error) : resolve()));
  }
  exclusive(operation) {
    if (this.operations >= 8) return Promise.reject(new Error('busy'));
    this.operations++;
    const pending = this.serial.then(operation);
    this.serial = pending.catch(() => {});
    return pending.finally(() => { this.operations--; });
  }
  v2() { if (this.protocolVersion !== 2) throw new Error('unsupported_transport'); }
  async receive(op, transferId) {
    const reply = await this.read();
    if (reply.protocol_version !== this.protocolVersion || reply.run_id !== this.runId || (this.protocolVersion === 2 && reply.session_id !== this.sessionId)) throw new Error('wrong_identity');
    if (reply.op === 'cancel') throw new HostError({ code: 'cancelled' });
    if (reply.op !== op || (transferId !== undefined && reply.transfer_id !== transferId)) throw new Error('invalid_reply');
    return reply;
  }
  chunkBudget(count, sending) {
    const total = sending ? this.dataSent : this.dataReceived;
    if (count <= 0 || count > 65536 || total + count > 16*1024*1024 || this.dataChunks >= 512) throw new Error('data_limit');
    this.dataChunks++; if (sending) this.dataSent += count; else this.dataReceived += count;
  }
  progress({ completed, total, message } = {}) {
    return this.exclusive(async () => {
      this.v2();
      if ((completed === undefined) !== (total === undefined) || (total !== undefined && (!Number.isSafeInteger(completed) || !Number.isSafeInteger(total) || total <= 0 || completed < 0 || completed > total))) throw new Error('invalid_progress');
      if (message !== undefined && (typeof message !== 'string' || Buffer.byteLength(message) > 512 || /[\x00-\x08\x0b-\x1f\x7f-\x9f]/u.test(message))) throw new Error('invalid_progress');
      if (this.progressSequence >= 256) throw new Error('progress_limit');
      await this.send('progress', { sequence: ++this.progressSequence, ...(total === undefined ? {} : { completed, total }), ...(message === undefined ? {} : { message }) });
    });
  }
  blobUpload(data, mediaType = 'application/octet-stream') {
    return this.exclusive(async () => {
      this.v2();
      if (!(data instanceof Uint8Array) || data.byteLength > 8*1024*1024) throw new Error('blob_limit');
      const bytes = Buffer.from(data); const sha256 = createHash('sha256').update(bytes).digest('hex');
      const blob = await this.callNow('blobs.begin', { length: bytes.length, media_type: mediaType, sha256 });
      for (let offset=0, sequence=1; offset<bytes.length; offset+=65536, sequence++) {
        const chunk = bytes.subarray(offset, offset+65536); this.chunkBudget(chunk.length, true);
        await this.send('data', { transfer_id: blob.transfer_id, sequence, offset, data: chunk.toString('base64') });
        const ack = await this.receive('data_ack', blob.transfer_id);
        if (ack.sequence !== sequence || ack.offset !== offset+chunk.length) throw new Error('invalid_ack');
      }
      return this.callNow('blobs.seal', { blob_id: blob.blob_id });
    });
  }
  blobRead(blob, { offset=0, length } = {}) {
    return this.exclusive(async () => {
      this.v2();
      const span = await this.callNow('blobs.read', { blob_id: blob.blob_id, offset, ...(length === undefined ? {} : { length }) });
      if (!Number.isSafeInteger(span.length) || span.length<0 || span.length>8*1024*1024) throw new Error('blob_limit');
      const bytes=Buffer.alloc(span.length); let received=0, sequence=0;
      while (received<span.length) {
        const part=await this.receive('data', span.transfer_id); sequence++;
        if (typeof part.data !== 'string' || part.data.length>87384) throw new Error('invalid_data');
        const chunk=Buffer.from(part.data,'base64');
        if (chunk.toString('base64')!==part.data || part.sequence!==sequence || part.offset!==offset+received || received+chunk.length>span.length) throw new Error('invalid_data');
        this.chunkBudget(chunk.length,false); chunk.copy(bytes,received); received+=chunk.length;
        await this.send('data_ack',{transfer_id:span.transfer_id,sequence,offset:offset+received});
      }
      if (offset===0 && span.length===blob.length && createHash('sha256').update(bytes).digest('hex')!==blob.sha256) throw new Error('digest_mismatch');
      return bytes;
    });
  }
  blobRelease(blob) { return this.exclusive(() => { this.v2(); return this.callNow('blobs.release',{blob_id:blob.blob_id}); }); }
  storageGet(key) { return this.exclusive(() => { this.v2(); return this.callNow('storage.get',{key}); }); }
  storageSet(key,value,{expectedRevision}={}) { return this.exclusive(() => { this.v2(); return this.callNow('storage.set',{key,value,...(expectedRevision===undefined?{}:{expected_revision:expectedRevision})}); }); }
  storageDelete(key,{expectedRevision}={}) { return this.exclusive(() => { this.v2(); return this.callNow('storage.delete',{key,...(expectedRevision===undefined?{}:{expected_revision:expectedRevision})}); }); }
  storageList({prefix='',limit=32,cursor=null}={}) { return this.exclusive(() => { this.v2(); return this.callNow('storage.list',{prefix,limit,cursor}); }); }
  /** @param {string} method @param {Record<string, unknown>} params */
  call(method, params = {}) { return this.exclusive(() => this.callNow(method, params)); }
  async callNow(method, params = {}) {
    if (++this.calls > 64) throw new Error('host_call_limit');
    const callId = `call-${this.calls}`;
    await this.send('host_call', { call_id: callId, method, params });
    const reply = await this.receive('host_result');
    if (reply.op !== 'host_result' || reply.call_id !== callId || typeof reply.success !== 'boolean' || !Object.hasOwn(reply, 'value')) throw new Error('invalid_reply');
    if (!reply.success) throw new HostError(reply.value);
    return reply.value;
  }
  async *pages(method, params = {}) {
    let cursor = null; const seen = new Set();
    do {
      const page = await this.call(method, { ...params, cursor });
      if (!Array.isArray(page.items) || typeof page.has_more !== 'boolean') throw new Error('invalid_page');
      yield page;
      if (!page.has_more) return;
      cursor = page.next_cursor;
      if (typeof cursor !== 'string' || seen.has(cursor)) throw new Error('invalid_cursor');
      seen.add(cursor);
    } while (true);
  }
  complete(status = 'success', value = {}) { return this.exclusive(() => this.completeNow(status, value)); }
  async completeNow(status, value) {
    if (!['success', 'handled', 'not_handled', 'error'].includes(status)) throw new Error('invalid_status');
    await this.send('complete', { status, value });
    if (this.protocolVersion === 2) await this.send('request_finished', {});
    this.finished = true; this.close();
  }
  close() { this.input.destroy(); }
}
