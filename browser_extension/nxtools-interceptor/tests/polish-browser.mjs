// Supplied-Page checks for the production Popup and content DOM entry points.
import fs from "node:fs/promises";
import { loadPopup } from "./intake-control-browser.mjs";
const root = new URL("../", import.meta.url);
const wait = (page, fn) => page.waitForFunction(fn, undefined, { timeout: 10000 });

export async function runPolishPopup(page) {
  await loadPopup(page);
  const results = [];
  const record = async (label, fn) => {
    const pass = Boolean(await page.evaluate(fn));
    results.push({ label, pass });
    if (!pass) throw new Error(`Popup check failed: ${label}`);
  };
  await record("graphite-four-tabs-456px", () => document.querySelectorAll('[role="tab"]').length === 4 &&
    Math.round(document.querySelector('.shell').getBoundingClientRect().width) === 456);
  await page.focus('#homeCapturePauseButton');
  await page.press('#homeCapturePauseButton', 'Enter');
  await wait(page, () => busyActionCount === 0);
  await record("home-pause-keyboard-focus-and-synchronization", () => currentState.capturePaused &&
    document.activeElement.id === 'homeCapturePauseButton' &&
    document.querySelector('#capturePauseButton').getAttribute('aria-pressed') === 'true');
  await page.evaluate(() => {
    __fixture.state.lastHandoff = { state: 'skipped', reason: 'unsupported_request_method', owner: 'browser' };
    render(structuredClone(__fixture.state));
  });
  await record('non-get-retention-explanation', () => !document.querySelector('#handoffStatus').hidden &&
    document.querySelector('#handoffStatus').textContent.includes('POST'));
  await page.evaluate(() => {
    window.__heldView = NxToolsHeldRequests.holdNextRequests(chrome.runtime, ['popup_get_state']);
    window.__viewRequestCount = __fixture.messages.filter(message => message.type === 'popup_get_state').length;
    void refresh({ force: true });
  });
  await wait(page, () => __heldView.inspect().length === 1);
  await page.click('#resourcesTab', { label: 'open captured resource review' });
  await page.evaluate(() => __heldView.release(1));
  await wait(page, () => currentView === 'resources' && !refreshInFlight);
  await record("view-change-refreshes-after-earlier-request", () => __fixture.messages
    .filter(message => message.type === 'popup_get_state').slice(__viewRequestCount).at(-1)?.currentView === 'resources');
  await record("other-pages-start-collapsed", () => !document.querySelector('#otherResourcesDetails').open);
  await page.click('[data-resource-check="video-1"]', { label: 'select captured video resource' });
  await page.fill('#resourceSearchInput', '课程讲解');
  await record("local-search-keeps-hidden-selection", () => document.querySelectorAll('#currentResourceList [data-resource-row]').length === 1 &&
    selectedResourceIds.has('video-1') && document.activeElement.id === 'resourceSearchInput');
  await page.fill('#resourceSearchInput', 'synthetic.invalid');
  await record("domain-search", () => document.querySelectorAll('#currentResourceList [data-resource-row]').length === 3);
  await page.fill('#resourceSearchInput', '');
  await page.click('#extractLinksButton', { label: 'extract page links for review' });
  await wait(page, () => busyActionCount === 0 && !refreshInFlight);
  await record("extraction-is-review-only", () => !document.querySelector('#linkReviewStatus').hidden &&
    !__fixture.messages.some(message => ['popup_send_resources', 'popup_send_resource'].includes(message.type)));
  await page.click('[data-resource-preview="video-1"]', { label: 'open captured media details' });
  await page.evaluate(() => {
    window.__heldInspection = NxToolsHeldRequests.holdNextRequests(chrome.runtime, ['popup_inspect_resource']);
    __fixture.failure = 'popup_inspect_resource';
  });
  await page.click('[data-resource-inspect="video-1"]', { label: 'inspect media only on demand' });
  await wait(page, () => __heldInspection.inspect().length === 1);
  await record("inspection-pending-is-busy", () => document.querySelector('[data-resource-inspect="video-1"]').disabled &&
    document.querySelector('[data-resource-inspect="video-1"]').getAttribute('aria-busy') === 'true' &&
    document.querySelector('[data-resource-id="video-1"]').disabled && document.querySelector('#batchResourcesButton').disabled);
  await page.evaluate(() => __heldInspection.release(1));
  await wait(page, () => busyActionCount === 0);
  await record("inspection-failure-allows-retry", () => !document.querySelector('[data-resource-inspect="video-1"]').disabled &&
    document.querySelector('.inspection-summary').dataset.state === 'failed');
  await page.evaluate(() => { __fixture.failure = ''; });
  await page.click('[data-resource-inspect="video-1"]', { label: 'retry failed media inspection' });
  await wait(page, () => busyActionCount === 0 && !refreshInFlight);
  await record("inspection-retry-publishes-ready", () => document.querySelector('.inspection-summary').dataset.state === 'ready');
  await page.evaluate(() => {
    __fixture.state.currentResources[0].mediaInspection = { state: 'protected', kind: 'dash', encrypted: true, message: '受保护的媒体' };
    render(structuredClone(__fixture.state));
  });
  await record("protected-disables-send-and-selection", () => document.querySelector('[data-resource-check="video-1"]').disabled &&
    document.querySelector('[data-resource-id="video-1"]').disabled && !selectedResourceIds.has('video-1'));
  await page.press('[data-resource-inspect="video-1"]', 'Escape');
  await record("escape-restores-details-focus", () => !previewResourceId && document.activeElement.dataset.resourcePreview === 'video-1');
  await page.focus('#resourcesTab');
  await page.press('#resourcesTab', 'End');
  await wait(page, () => currentView === 'settings' && !refreshInFlight);
  await record("keyboard-view-preference", () => document.activeElement.id === 'settingsTab' &&
    __fixture.viewStore.nxtools_popup_view === 'settings');
  await page.evaluate(() => {
    __fixture.state.status = { ok: false, code: 'host_not_registered', message: '原生宿主未注册', guidance: '启动 NxTools 重新注册扩展。' };
    render(structuredClone(__fixture.state));
  });
  await page.click('#tasksTab', { label: 'show actionable connection guidance' });
  await wait(page, () => currentView === 'tasks' && !refreshInFlight);
  await record("home-fixed-diagnostic-guidance", () => !document.querySelector('#homeConnectionPanel').hidden &&
    document.querySelector('#homeConnectionGuidance').textContent.includes('重新注册'));
  for (const view of ['tasks', 'resources', 'advanced', 'settings']) {
    await page.click(`[data-view="${view}"]`, { label: 'check popup tab containment' });
    await wait(page, () => !refreshInFlight);
    await record(`no-horizontal-overflow-${view}`, () => {
      const shell = document.querySelector('.shell').getBoundingClientRect();
      return [...document.querySelectorAll('.view.active button,.view.active input,.view.active select,.view.active textarea')]
        .filter(node => node.getClientRects().length).every(node => {
          const rect = node.getBoundingClientRect(); return rect.left >= shell.left && rect.right <= shell.right + 1;
        }) && document.querySelector('.workspace').scrollWidth <= 456;
    });
  }
  await page.click('#resourcesTab', { label: 'review full extracted link list' });
  await wait(page, () => !refreshInFlight);
  await page.evaluate(() => {
    const template = __fixture.state.currentResources[1];
    __fixture.state.currentResources = Array.from({ length: 120 }, (_, index) => ({ ...template,
      id: `link-${index}`, filename: `资料-${index}.zip`, url: `https://synthetic.invalid/link-${index}`,
      description: { kind: 'download' }, mediaInspection: undefined }));
    render(structuredClone(__fixture.state));
  });
  await record('link-review-first-page-24', () => document.querySelectorAll('#currentResourceList [data-resource-row]').length === 24);
  for (let index = 0; index < 4; index++) await page.click('[data-resource-more="currentResourceList"]', { label: 'show next captured link group' });
  await page.click('[data-resource-check="link-119"]', { label: 'select final extracted page link' });
  await record('all-120-links-reviewable', () => document.querySelectorAll('#currentResourceList [data-resource-row]').length === 120 &&
    !document.querySelector('[data-resource-more="currentResourceList"]') && selectedResourceIds.has('link-119'));
  await page.cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
  await record('reduced-motion', () => matchMedia('(prefers-reduced-motion: reduce)').matches &&
    parseFloat(getComputedStyle(document.querySelector('.tab')).transitionDuration) <= .01);
  await record('no-unhandled-rejections', () => __fixture.unhandled.length === 0);
  return results;
}

export async function runPolishContent(page) {
  await page.goto('about:blank');
  const linkSource = await fs.readFile(new URL('bg/link-collection.js', root), 'utf8');
  const manifest = JSON.parse(await fs.readFile(new URL('manifest.json', root), 'utf8'));
  const scripts = await Promise.all(manifest.content_scripts[0].js.map(file => fs.readFile(new URL(file, root), 'utf8')));
  await page.evaluate(source => {
    document.head.innerHTML = '<base href="https://fixture.test/watch">';
    document.body.innerHTML = '<section id="selected"><a href="/one.zip#one">one</a> https://cdn.test/two.pdf. <a href="javascript:void(0)">skip</a></section><a href="/outside.zip">outside</a><a href="/one.zip#two">duplicate</a>';
    const range = document.createRange(); range.selectNodeContents(document.querySelector('#selected'));
    getSelection().removeAllRanges(); getSelection().addRange(range);
    const script = document.createElement('script'); script.textContent = source; document.head.appendChild(script);
  }, linkSource);
  const selection = await page.evaluate(() => collectDocumentLinks('selection'));
  if (JSON.stringify(selection.links.map(link => link.url)) !== JSON.stringify(['https://fixture.test/one.zip', 'https://cdn.test/two.pdf'])) {
    throw new Error(`Actual DOM selection extraction failed: ${JSON.stringify(selection)}`);
  }
  const whole = await page.evaluate(() => collectDocumentLinks('page'));
  if (whole.links.length !== 2 || !whole.links.some(link => link.url.endsWith('/outside.zip'))) throw new Error('Actual page anchor extraction failed');
  await page.evaluate(sources => {
    window.__listeners = []; window.__fetches = []; window.__body = ''; window.__mime = 'application/dash+xml';
    window.chrome = { runtime: { id: 'fixture', lastError: null, getURL: () => 'data:text/javascript,',
      onMessage: { addListener(fn) { __listeners.push(fn); } }, sendMessage: async () => ({ ok: true }) },
      storage: { onChanged: { addListener() {} }, local: { get(defaults, callback) {
        const result = { ...defaults, captureResources: false, mediaOverlayEnabled: false, dropTargetEnabled: false };
        callback?.(result); return Promise.resolve(result);
      } } } };
    window.setInterval = () => 1;
    window.fetch = async (url, options) => { __fetches.push({ url, credentials: options.credentials }); return new Response(__body, { headers: { 'content-type': __mime } }); };
    for (const source of sources) { const script = document.createElement('script'); script.textContent = source; document.head.appendChild(script); }
    window.__inspect = () => new Promise(resolve => {
      const message = { type: 'content_inspect_media_resource', url: 'https://cdn.test/media', pageKey: location.href, includeCredentials: false };
      for (const listener of __listeners) if (listener(message, { id: 'fixture' }, resolve) === true) break;
    });
  }, scripts);
  const facts = await page.evaluate(async () => {
    __body = '<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="dynamic"><Period><AdaptationSet contentType="video"><Representation/></AdaptationSet><AdaptationSet mimeType="audio/mp4"><Representation/></AdaptationSet></Period></MPD>';
    const split = await __inspect();
    __body = '<MPD xmlns="urn:mpeg:dash:schema:mpd:2011"><Period><AdaptationSet><ContentProtection schemeIdUri="urn:uuid:fixture"/></AdaptationSet></Period></MPD>';
    const protectedDash = await __inspect();
    __mime = 'application/vnd.apple.mpegurl';
    __body = '#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI="https://secret.test/key"\n#EXTINF:4,\nsegment.ts\n#EXT-X-ENDLIST';
    const aes = await __inspect();
    return { split, protectedDash, aes, fetches: __fetches };
  });
  if (!facts.split.ok || !facts.split.inspection.requiresMerge || !facts.split.inspection.live) throw new Error('Native DOMParser DASH split/live failed');
  if (facts.protectedDash.inspection.state !== 'protected') throw new Error('Native DOMParser DASH protection failed');
  if (facts.aes.inspection.state !== 'ready' || !facts.aes.inspection.encrypted) throw new Error('AES-128 incorrectly treated as DRM');
  if (facts.fetches.length !== 3 || facts.fetches.some(fetch => fetch.credentials !== 'omit' || fetch.url !== 'https://cdn.test/media')) throw new Error('Recursive or credentialed media request');
  return { selection: selection.links, pageLinks: whole.links, nativeDomParser: true, inspections: 3, requests: facts.fetches.length };
}
