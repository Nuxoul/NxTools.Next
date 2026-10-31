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

  await evaluate(`setView('tasks')`);
  check(await evaluate(`(() => { const item = document.querySelector('#taskList .item'); const bar = item.querySelector('progress').getBoundingClientRect();
    const act = item.querySelector('[data-action]').getBoundingClientRect(); return Math.abs((bar.top + bar.bottom) / 2 - (act.top + act.bottom) / 2) < 14; })()`),
    'task-progress-and-actions-on-one-line');
  check(await evaluate(`Boolean(document.querySelector('#taskList [data-status-tone]'))`), 'task-status-semantic-tone');

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
