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
async function trustedClick(rectExpression) {
  const rect = await evaluate(rectExpression);
  if (!rect || rect.width <= 0 || rect.height <= 0) {
    throw new Error(`CDP click target is not visible: ${JSON.stringify(rect)}`);
  }
  const x = rect.left + rect.width / 2;
  const y = rect.top + rect.height / 2;
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 });
}


const mockChrome = `
(() => {
  const listeners = [];
  window.__nx = { sent: [], listeners, storageChanged: [], responses: {} };
  Object.defineProperty(window, 'chrome', { configurable: true, value: {
    runtime: {
      id: 'fixture', lastError: null,
      getURL: () => 'data:text/javascript,',
      sendMessage(message, callback) {
        window.__nx.sent.push(message);
        const response = Object.hasOwn(window.__nx.responses, message.type)
          ? window.__nx.responses[message.type]
          : { ok: true };
        if (callback) setTimeout(() => callback(response), 0);
        return Promise.resolve(response);
      },
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
  await send('Page.setDocumentContent', { frameId, html: '<html><head><meta property="og:image" content="https://img.example.test/og.jpg"></head><body><a id="file" href="https://files.example.test/a.zip">a.zip</a><video id="v" width="320" height="180" poster="https://img.example.test/poster.jpg"></video></body></html>' });
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
  // Media entry dismiss: hides for this page view, survives the periodic
  // re-layout, and returns when the overlay setting is switched back on.
  const entryState = `(() => { const host = document.getElementById('nxToolsMediaDownload');
    const root = host?.shadowRoot; const trigger = root?.querySelector('.trigger')?.getBoundingClientRect();
    const dismiss = root?.querySelector('.dismiss')?.getBoundingClientRect();
    return { display: host?.style.display || '', trigger: trigger && { top: trigger.top, right: trigger.right },
      dismiss: dismiss && { top: dismiss.top, right: dismiss.right, width: dismiss.width } }; })()`;
  await evaluate('new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))');
  const shown = await evaluate(entryState);
  check(shown.display === 'block', 'media-entry-visible-on-video', shown);
  check(shown.dismiss && shown.trigger && shown.dismiss.right > shown.trigger.right && shown.dismiss.top < shown.trigger.top && shown.dismiss.width <= 18,
    'dismiss-sits-on-entry-top-right-corner', shown);
  await evaluate(`document.getElementById('nxToolsMediaDownload').shadowRoot.querySelector('.dismiss').click()`);
  check((await evaluate(entryState)).display === 'none', 'dismiss-hides-entry');
  await evaluate(`new Promise((r) => { document.getElementById('v').dispatchEvent(new Event('play')); setTimeout(r, 1300); })`);
  check((await evaluate(entryState)).display === 'none', 'dismissed-entry-stays-hidden-after-relayout');
  await evaluate(`window.__nx.storageChanged.forEach((fn) => fn({ mediaOverlayEnabled: { oldValue: false, newValue: true } }, 'local'))`);
  await evaluate('new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))');
  check((await evaluate(entryState)).display === 'block', 'reenabling-overlay-restores-entry');
  const thumbnailScan = await evaluate(`__nxDispatch({ type: 'content_scan_now' })`);
  check(thumbnailScan?.pageThumbnail === 'https://img.example.test/poster.jpg', 'scan-reports-page-thumbnail', thumbnailScan);
  const syntheticDirect = await evaluate(`(() => {
    const anchor = document.getElementById('file');
    anchor.setAttribute('download', 'a.zip');
    let preventedBeforeFixture = null;
    anchor.addEventListener('click', (event) => {
      preventedBeforeFixture = event.defaultPrevented;
      event.preventDefault();
    }, { once: true });
    window.__nx.sent.length = 0;
    anchor.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, button: 0 }));
    return { preventedBeforeFixture, types: window.__nx.sent.map((message) => message.type) };
  })()`);
  check(syntheticDirect.preventedBeforeFixture === false && syntheticDirect.types.length === 0,
    'synthetic-direct-click-preserves-browser-default', syntheticDirect);

  const syntheticDrop = await evaluate(`(() => {
    window.__nx.sent.length = 0;
    const dataTransfer = new DataTransfer();
    dataTransfer.setData('text/uri-list', 'https://files.example.test/drop.zip');
    const init = { bubbles: true, cancelable: true, dataTransfer, clientX: innerWidth - 30, clientY: innerHeight - 30 };
    const dragOver = new DragEvent('dragover', init);
    const dragDispatched = document.dispatchEvent(dragOver);
    const drop = new DragEvent('drop', init);
    const dropDispatched = document.dispatchEvent(drop);
    return {
      dragDispatched,
      dropDispatched,
      dragPrevented: dragOver.defaultPrevented,
      dropPrevented: drop.defaultPrevented,
      targetDisplay: document.getElementById('nxToolsDropTarget')?.style.display || '',
      types: window.__nx.sent.map((message) => message.type)
    };
  })()`);
  check(syntheticDrop.dragDispatched && syntheticDrop.dropDispatched &&
    !syntheticDrop.dragPrevented && !syntheticDrop.dropPrevented && syntheticDrop.types.length === 0,
    'synthetic-drop-is-passive', syntheticDrop);

  const syntheticOverlayMain = await evaluate(`(async () => {
    const root = document.getElementById('nxToolsMediaDownload').shadowRoot;
    const trigger = root.querySelector('.trigger');
    window.__nx.responses.content_get_resource_choices = { ok: true, choices: [] };
    window.__nx.sent.length = 0;
    const event = new MouseEvent('click', { bubbles: true, cancelable: true, button: 0, composed: true });
    const dispatched = trigger.dispatchEvent(event);
    await new Promise((resolve) => setTimeout(resolve, 60));
    return { dispatched, defaultPrevented: event.defaultPrevented, types: window.__nx.sent.map((message) => message.type) };
  })()`);
  check(syntheticOverlayMain.dispatched && !syntheticOverlayMain.defaultPrevented && syntheticOverlayMain.types.length === 0,
    'synthetic-overlay-main-click-is-passive', syntheticOverlayMain);

  await evaluate(`(() => {
    const root = document.getElementById('nxToolsMediaDownload').shadowRoot;
    const trigger = root.querySelector('.trigger');
    const panel = root.querySelector('.panel');
    panel.hidden = true;
    trigger.disabled = false;
    trigger.setAttribute('aria-expanded', 'false');
    window.__nx.responses.content_get_resource_choices = { ok: true, choices: [{
      url: 'https://media.example.test/clip.mp4', filename: 'clip.mp4', mime: 'video/mp4', ext: 'mp4',
      width: 1920, height: 1080, size: 4096, capturedAt: Date.now(), source: 'fixture'
    }] };
    window.__nx.sent.length = 0;
  })()`);
  await trustedClick(`(() => { const rect = document.getElementById('nxToolsMediaDownload').shadowRoot.querySelector('.trigger').getBoundingClientRect();
    return { left: rect.left, top: rect.top, width: rect.width, height: rect.height }; })()`);
  await evaluate('new Promise((resolve) => setTimeout(resolve, 60))');
  const trustedOverlayMain = await evaluate(`(() => {
    const root = document.getElementById('nxToolsMediaDownload').shadowRoot;
    return { choices: root.querySelectorAll('.choice').length, panelHidden: root.querySelector('.panel').hidden,
      types: window.__nx.sent.map((message) => message.type) };
  })()`);
  check(trustedOverlayMain.choices === 1 && trustedOverlayMain.panelHidden === false &&
    trustedOverlayMain.types.includes('content_get_resource_choices') && !trustedOverlayMain.types.includes('page_download_media'),
    'trusted-overlay-main-click-opens-choices', trustedOverlayMain);

  const syntheticOverlayChoice = await evaluate(`(async () => {
    const choice = document.getElementById('nxToolsMediaDownload').shadowRoot.querySelector('.choice');
    window.__nx.sent.length = 0;
    const event = new MouseEvent('click', { bubbles: true, cancelable: true, button: 0, composed: true });
    const dispatched = choice.dispatchEvent(event);
    await new Promise((resolve) => setTimeout(resolve, 20));
    return { dispatched, defaultPrevented: event.defaultPrevented, types: window.__nx.sent.map((message) => message.type) };
  })()`);
  check(syntheticOverlayChoice.dispatched && !syntheticOverlayChoice.defaultPrevented && syntheticOverlayChoice.types.length === 0,
    'synthetic-overlay-choice-click-is-passive', syntheticOverlayChoice);

  await evaluate(`(() => {
    const root = document.getElementById('nxToolsMediaDownload').shadowRoot;
    root.querySelector('.panel').hidden = true;
    root.querySelector('.trigger').disabled = false;
    root.querySelector('.trigger').setAttribute('aria-expanded', 'false');
  })()`);
  await trustedClick(`(() => { const rect = document.getElementById('nxToolsMediaDownload').shadowRoot.querySelector('.trigger').getBoundingClientRect();
    return { left: rect.left, top: rect.top, width: rect.width, height: rect.height }; })()`);
  await evaluate('new Promise((resolve) => setTimeout(resolve, 60))');


  await evaluate('window.__nx.sent.length = 0');
  await trustedClick(`(() => { const rect = document.getElementById('nxToolsMediaDownload').shadowRoot.querySelector('.choice').getBoundingClientRect();
    return { left: rect.left, top: rect.top, width: rect.width, height: rect.height }; })()`);
  await evaluate('new Promise((resolve) => setTimeout(resolve, 40))');
  const trustedOverlayChoice = await evaluate(`window.__nx.sent.map((message) => message.type)`);
  check(trustedOverlayChoice.filter((type) => type === 'page_download_media').length === 1,
    'trusted-overlay-choice-submits-download', trustedOverlayChoice);

  await evaluate(`(() => {
    window.__nx.responses.content_direct_download_request = { ok: true };
    window.__nx.sent.length = 0;
  })()`);
  await trustedClick(`(() => { const rect = document.getElementById('file').getBoundingClientRect();
    return { left: rect.left, top: rect.top, width: rect.width, height: rect.height }; })()`);
  await evaluate('new Promise((resolve) => setTimeout(resolve, 30))');
  const trustedDirect = await evaluate(`window.__nx.sent.map((message) => message.type)`);
  check(trustedDirect.filter((type) => type === 'content_direct_download_request').length === 1,
    'trusted-direct-click-submits-download', trustedDirect);

  await evaluate(`(() => {
    const anchor = document.getElementById('file');
    window.__nx.responses.content_direct_download_request = { ok: false, fallbackToBrowser: true };
    window.__nx.replayClicks = [];
    anchor.addEventListener('click', (event) => {
      window.__nx.replayClicks.push({
        trusted: event.isTrusted,
        passThrough: anchor.getAttribute('data-nxtools-pass-through')
      });
      event.preventDefault();
    });
    window.__nx.sent.length = 0;
  })()`);
  await trustedClick(`(() => { const rect = document.getElementById('file').getBoundingClientRect();
    return { left: rect.left, top: rect.top, width: rect.width, height: rect.height }; })()`);
  await evaluate('new Promise((resolve) => setTimeout(resolve, 60))');
  const replay = await evaluate(`({
    types: window.__nx.sent.map((message) => message.type),
    clicks: window.__nx.replayClicks
  })`);
  check(replay.types.filter((type) => type === 'content_direct_download_request').length === 1 &&
    replay.clicks.length === 1 && replay.clicks[0].trusted === false && replay.clicks[0].passThrough === '1',
    'browser-replay-synthetic-click-passes-through', replay);
  check(exceptions.length === 0, 'no-exceptions-after-input-trust-checks', exceptions.slice(0, 3));
  check(exceptions.length === 0, 'no-exceptions-after-dismiss', exceptions.slice(0, 3));
  const failed = rows.filter((row) => !row.pass);
  console.log(JSON.stringify({ files: contentFiles, passed: rows.length - failed.length, total: rows.length, failed }, null, 2));
  process.exitCode = failed.length ? 1 : 0;
}
run().catch((error) => { console.error(error); process.exitCode = 1; }).finally(() => {
  try { child.kill(); } catch {}
  setTimeout(() => { try { fs.rmSync(profile, { recursive: true, force: true }); } catch {} }, 500);
});
