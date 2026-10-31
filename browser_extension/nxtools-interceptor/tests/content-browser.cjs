'use strict';
// Real DOM content-script boundary: production page-hook + manifest content
// scripts in an isolated headless Chrome profile, mocked extension API, all
// network blocked. Proves message handling and load errors, not installed-extension
// isolated-world behavior.
const fs = require('node:fs'), path = require('node:path'), os = require('node:os');
const { spawn } = require('node:child_process');
const root = path.resolve(__dirname, '..');
const manifest = JSON.parse(fs.readFileSync(path.join(root, 'manifest.json'), 'utf8'));
const contentFiles = manifest.content_scripts[0].js;
const scratch = process.env.PI_SCRATCH_DIR || os.tmpdir();
const profile = fs.mkdtempSync(path.join(scratch, 'nxtools-content-'));
const chromePath = process.env.PROTOTYPE_CHROME || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const child = spawn(chromePath, ['--headless=new', '--remote-debugging-pipe', '--no-first-run', '--disable-background-networking',
  '--disable-component-update', '--disable-sync', '--disable-extensions', `--user-data-dir=${profile}`, 'about:blank'],
  { stdio: ['ignore', 'ignore', 'pipe', 'pipe', 'pipe'] });
let serial = 0, buffer = '', session;
const waiting = new Map();
const exceptions = [];
child.stdio[4].on('data', (data) => {
  buffer += data.toString();
  let end;
  while ((end = buffer.indexOf('\0')) >= 0) {
    const text = buffer.slice(0, end); buffer = buffer.slice(end + 1);
    if (!text) continue;
    const m = JSON.parse(text);
    if (m.method === 'Runtime.exceptionThrown') {
      const d = m.params.exceptionDetails;
      exceptions.push(d.exception?.description || d.text);
      continue;
    }
    const pending = waiting.get(m.id);
    if (pending) { clearTimeout(pending.timer); waiting.delete(m.id); m.error ? pending.reject(new Error(JSON.stringify(m.error))) : pending.resolve(m.result); }
  }
});
function send(method, params = {}, sid = session) {
  return new Promise((resolve, reject) => {
    const id = ++serial;
    const timer = setTimeout(() => { waiting.delete(id); reject(new Error('CDP timeout ' + method)); }, 20000);
    waiting.set(id, { resolve, reject, timer });
    child.stdio[3].write(JSON.stringify({ id, method, params, ...(sid ? { sessionId: sid } : {}) }) + '\0');
  });
}
async function evaluate(expression) {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails.exception?.description || r.exceptionDetails.text));
  return r.result.value;
}

const mockChrome = `
(() => {
  const listeners = [];
  window.__nx = { sent: [], listeners, storageChanged: [] };
  Object.defineProperty(window, 'chrome', { configurable: true, value: {
    runtime: {
      id: 'fixture', lastError: null,
      getURL: () => 'data:text/javascript,',
      sendMessage(message, callback) { window.__nx.sent.push(message); if (callback) setTimeout(() => callback({ ok: true }), 0); return Promise.resolve({ ok: true }); },
      onMessage: { addListener(fn) { listeners.push(fn); } }
    },
    storage: {
      local: { get(defaults, callback) { setTimeout(() => callback({ ...defaults }), 0); } },
      onChanged: { addListener(fn) { window.__nx.storageChanged.push(fn); } }
    }
  }});
  window.__nxDispatch = (message) => new Promise((resolve) => {
    let done = false;
    const results = listeners.map((fn) => fn(message, { id: 'fixture' }, (value) => { done = true; resolve(value); }));
    if (!done && !results.some((value) => value === true)) resolve('NO_RESPONSE');
    setTimeout(() => { if (!done) resolve('TIMEOUT'); }, 6000);
  });
})();`;

async function run() {
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
  await send('Network.setBlockedURLs', { urls: ['http://*', 'https://*', 'ws://*', 'wss://*'] });
  const frameId = (await send('Page.getFrameTree')).frameTree.frame.id;
  await send('Page.setDocumentContent', { frameId, html: '<html><body><a id="file" href="https://files.example.test/a.zip">a.zip</a><video id="v" width="320" height="180"></video></body></html>' });
  await evaluate(fs.readFileSync(path.join(root, 'page-hook.js'), 'utf8'));
  await evaluate(mockChrome);
  for (const file of contentFiles) {
    await evaluate(fs.readFileSync(path.join(root, file), 'utf8'));
  }
  await evaluate('new Promise((r) => setTimeout(r, 120))');
  const rows = [];
  const check = (pass, label, detail) => rows.push({ pass: Boolean(pass), label, detail });
  check(await evaluate('window.__nx.listeners.length') === 1, 'one-runtime-listener');
  check(exceptions.length === 0, 'no-load-or-settings-exceptions', exceptions.slice(0, 3));
  const overlay = await evaluate(`__nxDispatch({ type: 'media_overlay_set_enabled', enabled: true })`);
  check(overlay?.ok === true, 'overlay-toggle-responds', overlay);
  const scan = await evaluate(`__nxDispatch({ type: 'content_scan_now' })`);
  check(scan?.ok === true, 'scan-responds', scan);
  const info = await evaluate(`(() => { const url = URL.createObjectURL(new Blob(['abc'], { type: 'text/plain' })); return __nxDispatch({ type: 'content_blob_info', url, filename: 'a.txt' }); })()`);
  check(info?.ok === true && info.size === 3 && info.filename === 'a.txt', 'blob-info-handled-by-runtime-listener', info);
  const upload = await evaluate(`__nxDispatch({ type: 'content_upload_blob', url: 'blob:x', uploadUrl: 'http://evil.example/x', expectedSize: 3, chunkBytes: 1024 })`);
  check(upload?.ok === false && /参数无效/.test(upload.message || ''), 'blob-upload-rejects-invalid-capability', upload);
  const unknown = await evaluate(`__nxDispatch({ type: 'not_a_content_message' })`);
  check(unknown === 'NO_RESPONSE', 'unknown-message-not-claimed', unknown);
  check(exceptions.length === 0, 'no-exceptions-after-messages', exceptions.slice(0, 3));
  // Background re-injects the same list on tab activation/navigation; it must be a no-op.
  const before = await evaluate('window.__nx.listeners.length');
  for (const file of contentFiles) {
    await evaluate(fs.readFileSync(path.join(root, file), 'utf8'));
  }
  await evaluate('new Promise((r) => setTimeout(r, 60))');
  check(await evaluate('window.__nx.listeners.length') === before, 'reinjection-adds-no-listener');
  check(exceptions.length === 0, 'reinjection-no-redeclaration-errors', exceptions.slice(0, 3));
  const after = await evaluate(`__nxDispatch({ type: 'content_scan_now' })`);
  check(after?.ok === true, 'responds-after-reinjection', after);
  const failed = rows.filter((row) => !row.pass);
  console.log(JSON.stringify({ files: contentFiles, passed: rows.length - failed.length, total: rows.length, failed }, null, 2));
  process.exitCode = failed.length ? 1 : 0;
}
run().catch((error) => { console.error(error); process.exitCode = 1; }).finally(() => {
  try { child.kill(); } catch {}
  setTimeout(() => { try { fs.rmSync(profile, { recursive: true, force: true }); } catch {} }, 500);
});
