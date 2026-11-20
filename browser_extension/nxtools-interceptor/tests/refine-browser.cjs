'use strict';
// Phase F measurable refinement targets (design.md R4, direction A), checked on
// the real production popup HTML/CSS/scripts with the mocked extension API in an
// isolated headless Chrome profile with all network blocked.
const fs = require('node:fs'), path = require('node:path'), os = require('node:os');
const { spawn } = require('node:child_process');
const root = path.resolve(__dirname, '..');
const scratch = process.env.PI_SCRATCH_DIR || os.tmpdir();
const profile = fs.mkdtempSync(path.join(scratch, 'nxtools-refine-'));
const chromePath = process.env.PROTOTYPE_CHROME || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const child = spawn(chromePath, ['--headless=new', '--remote-debugging-pipe', '--no-first-run', '--disable-background-networking',
  '--disable-component-update', '--disable-sync', '--disable-extensions', `--user-data-dir=${profile}`, 'about:blank'],
  { stdio: ['ignore', 'ignore', 'pipe', 'pipe', 'pipe'] });
let serial = 0, buffer = '', session;
const waiting = new Map();
child.stdio[4].on('data', (data) => {
  buffer += data.toString();
  let end;
  while ((end = buffer.indexOf('\0')) >= 0) {
    const text = buffer.slice(0, end); buffer = buffer.slice(end + 1);
    if (!text) continue;
    const m = JSON.parse(text), pending = waiting.get(m.id);
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
function popupDocument() {
  let html = fs.readFileSync(path.join(root, 'popup.html'), 'utf8');
  const css = [...html.matchAll(/<link[^>]+href="([^"]+\.css)"/g)].map((m) => fs.readFileSync(path.join(root, m[1]), 'utf8')).join('\n');
  const scripts = [...html.matchAll(/<script src="([^"]+)"><\/script>/g)].map((m) => m[1]);
  html = html.replace(/<script\b[\s\S]*?<\/script>/gi, '').replace(/<link\b[^>]*>/gi, '')
    .replace('icons/icon-32.png', 'data:image/png;base64,' + fs.readFileSync(path.join(root, 'icons/icon-32.png')).toString('base64'))
    .replace('</head>', `<style>${css}</style></head>`);
  const code = [fs.readFileSync(path.join(__dirname, 'popup-fixture.js'), 'utf8'),
    ...scripts.map((file) => fs.readFileSync(path.join(root, file), 'utf8'))].join('\n;\n');
  return { html, code };
}

async function run() {
  const target = await send('Target.createTarget', { url: 'about:blank' }, null);
  session = (await send('Target.attachToTarget', { targetId: target.targetId, flatten: true }, null)).sessionId;
  await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
  await send('Network.setBlockedURLs', { urls: ['http://*', 'https://*', 'ws://*', 'wss://*'] });
  await send('Emulation.setDeviceMetricsOverride', { width: 900, height: 900, deviceScaleFactor: 1, mobile: false });
  const { html, code } = popupDocument();
  const load = async (prepare = '') => {
    await send('Page.navigate', { url: 'about:blank' });
    await send('Page.setDocumentContent', { frameId: (await send('Page.getFrameTree')).frameTree.frame.id, html });
    // Let the initial passive refresh settle first; its snapshot predates the fixture edit.
    await evaluate(code);
    await evaluate('new Promise((r) => setTimeout(r, 40))');
    if (prepare) await evaluate(prepare);
  };
  const rows = [];
  const check = (pass, label, detail) => rows.push({ pass: Boolean(pass), label, detail });

  // Six current-page resources so the viewport, not the fixture, limits visibility.
  await load(`(() => { const s = __fixture.state; const base = s.currentResources;
    s.currentResources = [...base, ...base.map((r, i) => ({ ...r, id: r.id + '-b', filename: '补充 ' + r.filename }))];
    s.captureSummary.currentCount = s.currentResources.length; render(s); })()`);
  const header = await evaluate(`Math.round(document.querySelector('.tabs').getBoundingClientRect().top)`);
  check(header <= 120, 'header-above-tabs-at-most-120px', header);

  await evaluate(`setView('resources'); document.querySelector('.workspace').scrollTop = 0`);
  const visible = await evaluate(`(() => { const w = document.querySelector('.workspace').getBoundingClientRect();
    return [...document.querySelectorAll('#currentResourceList [data-resource-row]')].filter((row) => {
      const r = row.getBoundingClientRect(); return r.top >= w.top - 1 && r.bottom <= w.bottom + 1; }).length; })()`);
  check(visible >= 4, 'capture-shows-at-least-4-resources', visible);

  check(await evaluate(`!document.getElementById('mergeResourcesButton') && ![...document.querySelectorAll('button')].some((b) => /合并/.test(b.textContent))`),
    'no-disabled-merge-button');

  const idleFooter = await evaluate(`(() => { const f = document.querySelector('.status-footer'); return !f || f.hidden || f.getBoundingClientRect().height === 0; })()`);
  check(idleFooter, 'idle-status-bar-not-reserved');

  const batchIdle = await evaluate(`(() => { const b = document.querySelector('.batch-toolbar'); return !b || b.hidden || b.getBoundingClientRect().height === 0; })()`);
  check(batchIdle, 'batch-bar-hidden-without-selection');
  await evaluate(`(() => { const c = document.querySelector('[data-resource-check]'); c.checked = true; c.dispatchEvent(new Event('change', { bubbles: true })); })()`);
  check(await evaluate(`(() => { const b = document.querySelector('.batch-toolbar'); return Boolean(b && !b.hidden && b.getBoundingClientRect().height > 0 && /1/.test(b.textContent)); })()`),
    'batch-bar-appears-with-selection');

  check(await evaluate(`[...document.querySelectorAll('#currentResourceList .resource-thumb')].every((t) => t.querySelector('svg') && !/预览/.test(t.textContent))`),
    'type-icons-replace-text-placeholders');

  check(await evaluate(`(() => { const row = document.querySelector('#currentResourceList [data-resource-row]');
    const send = row.querySelector('[data-resource-id]').getBoundingClientRect();
    const name = row.querySelector('.resource-title strong').getBoundingClientRect();
    return Math.abs(send.top - name.top) < 24; })()`), 'row-actions-share-the-title-line');

  // Thumbnails: source order, real load, node reuse across refresh, fallback.
  const order = await evaluate(`(() => { const pick = (list) => list.map((c) => c.type + ':' + c.src.split('/').pop());
    return {
      video: pick(thumbnailCandidates({ url: 'https://a.test/v.mp4', poster: 'https://a.test/p.jpg', description: { kind: 'video' } }, 'https://a.test/page.jpg')),
      stream: pick(thumbnailCandidates({ url: 'https://a.test/s.m3u8', description: { kind: 'streaming' } }, 'https://a.test/page.jpg')),
      image: pick(thumbnailCandidates({ url: 'https://a.test/i.png', mime: 'image/png', description: { kind: 'image' } }, 'https://a.test/page.jpg')),
      file: pick(thumbnailCandidates({ url: 'https://a.test/f.zip', description: { kind: 'download' } }, 'https://a.test/page.jpg')) }; })()`);
  check(JSON.stringify(order) === JSON.stringify({ video: ['img:p.jpg', 'video:v.mp4', 'img:page.jpg'], stream: ['img:page.jpg'], image: ['img:i.png'], file: [] }),
    'thumbnail-source-order', order);
  await evaluate(`(() => { isHttpSource = (value) => /^(https?:|data:)/i.test(String(value || ''));
    const canvas = document.createElement('canvas'); canvas.width = 32; canvas.height = 18;
    const g = canvas.getContext('2d'); g.fillStyle = '#c33'; g.fillRect(0, 0, 32, 18);
    const s = __fixture.state; window.__thumbPng = canvas.toDataURL('image/png');
    s.currentResources[0].poster = window.__thumbPng;
    s.currentResources[1].poster = 'data:image/png;base64,AAAA';
    s.activeTab.thumbnail = window.__thumbPng.replace('data:image/png;', 'data:image/png;x=page;');
    document.querySelector('.workspace').scrollTop = 0; render(s); })()`);
  await evaluate('new Promise((r) => setTimeout(r, 400))');
  const loaded = await evaluate(`(() => { const thumbs = [...document.querySelectorAll('#currentResourceList .resource-thumb')];
    const media = (t) => t.querySelector('.resource-thumb-media');
    window.__thumbNode = media(thumbs[0]);
    return { first: thumbs[0].classList.contains('has-media') && media(thumbs[0])?.naturalWidth === 32,
      firstOpacity: getComputedStyle(media(thumbs[0])).opacity, iconHidden: getComputedStyle(thumbs[0].querySelector('svg')).visibility,
      fallback: thumbs[1].classList.contains('has-media') && /x=page/.test(media(thumbs[1])?.getAttribute('src') || ''),
      file: !media(thumbs[2]) && !thumbs[2].classList.contains('has-media'),
      size: [thumbs[0].getBoundingClientRect().width, thumbs[0].getBoundingClientRect().height] }; })()`);
  check(loaded.first && loaded.firstOpacity === '1' && loaded.iconHidden === 'hidden', 'thumbnail-poster-loads-over-icon', loaded);
  check(loaded.fallback, 'failed-thumbnail-falls-back-to-page-thumbnail', loaded);
  check(loaded.file, 'file-resource-keeps-type-icon', loaded);
  check(loaded.size[0] === 64 && loaded.size[1] === 36, 'thumbnail-slot-64x36', loaded.size);
  await evaluate('render(__fixture.state)');
  check(await evaluate(`document.querySelector('#currentResourceList .resource-thumb .resource-thumb-media') === window.__thumbNode &&
    document.querySelector('#currentResourceList .resource-thumb').classList.contains('has-media')`), 'thumbnail-node-reused-across-refresh');

  await evaluate(`setView('tasks')`);
  check(await evaluate(`document.getElementById('tasksTab').textContent === '概览' &&
    !document.querySelector('#taskList, #taskTabCount, #taskCountText, [data-task-id]')`), 'overview-without-desktop-tasks');
  check(await evaluate(`!elements.siteEnabledToggle.disabled && !elements.homeCapturePauseButton.disabled &&
    !elements.homeTestButton.disabled`), 'overview-retains-site-pause-and-connection-controls');

  check(await evaluate(`(() => { document.querySelector('.status-footer') && (document.getElementById('messageText').textContent = ''); setMessage('已发送到 NxTools'); const f = document.querySelector('.status-footer');
    return Boolean(f && !f.hidden && f.getBoundingClientRect().height > 0); })()`), 'feedback-status-bar-appears');
  check(await evaluate(`Boolean(getComputedStyle(document.documentElement).getPropertyValue('--nt-success').trim())`), 'success-token-defined');

  await evaluate(`setView('settings')`);
  check(await evaluate(`[...document.querySelectorAll('#settingsView .switch-row')].filter((r) => r.getClientRects().length).every((r) => r.getBoundingClientRect().height <= 50)`),
    'settings-rows-compact');
  check(await evaluate(`/不会自动启动桌面端|不会启动桌面端/.test(document.getElementById('settingsView').textContent)`),
    'desktop-autostart-note-in-settings');

  const failed = rows.filter((row) => !row.pass);
  console.log(JSON.stringify({ checks: rows.length, failed }, null, 2));
  process.exitCode = failed.length ? 1 : 0;
}
run().catch((error) => { console.error(error); process.exitCode = 1; }).finally(async () => {
  try { await send('Browser.close', {}, null); } catch {}
  child.kill();
  for (const pending of waiting.values()) clearTimeout(pending.timer);
  setTimeout(() => { try { fs.rmSync(profile, { recursive: true, force: true }); } catch {} }, 300);
});
