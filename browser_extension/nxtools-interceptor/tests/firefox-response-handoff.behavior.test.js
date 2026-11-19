"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground, createControlledHandoffResponder, browserHandoffReply } = require("./harness/load-background.cjs");
const request = { requestId: "frame-download", tabId: 7, frameId: 0, type: "main_frame", method: "GET",
  url: "https://files.test/export", statusCode: 200, incognito: false, cookieStoreId: "firefox-container-4",
  originUrl: "https://page.test/report", responseHeaders: [
    { name: "Content-Disposition", value: 'attachment; filename="report.zip"' },
    { name: "Content-Type", value: "application/zip" }, { name: "Content-Length", value: "4096" },
    { name: "Accept-Ranges", value: "bytes" }
  ] };
async function fixture(options = {}) {
  const responder = createControlledHandoffResponder({ prepareState: options.phase || "held", onRequest: options.respond });
  const bg = loadBackground({ browser: options.browser || "firefox", chrome: {
    tabs: [{ id: 7, active: true, url: request.originUrl, cookieStoreId: request.cookieStoreId, incognito: false }],
    storage: { captureResources: false, ...options.settings }, ...options.chrome
  }, desktop: { respond: responder.respond } });
  const cookieQueries = [];
  bg.chrome.api.cookies.getAll = async query => { cookieQueries.push(query); return [{ name: "session", value: "synthetic-cookie" }]; };
  await bg.settle();
  function headers(details = request) {
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ ...details, requestHeaders: [
      { name: "Referer", value: request.originUrl }, { name: "Authorization", value: "synthetic-auth" }
    ] });
  }
  function fire(details = request) {
    headers(details);
    const results = bg.chrome.api.webRequest.onHeadersReceived.fire(details);
    return results[0] || Promise.resolve({});
  }
  async function cancelled(details = request) {
    bg.chrome.api.webRequest.onErrorOccurred.fire({ ...details, error: "NS_BINDING_ABORTED" });
    await bg.settle();
  }
  return { bg, responder, cookieQueries, headers, fire, cancelled };
}
function plain(value) { return JSON.parse(JSON.stringify(value)); }
function noDownloadEffects(bg) {
  for (const method of ["pause", "resume", "cancel", "erase", "download"]) {
    assert.equal(bg.chrome.callsNamed(`downloads.${method}`).length, 0, `no downloads.${method}`);
  }
  assert.equal(bg.chrome.downloadItems.size, 0, "no download/history item is created by response takeover");
}
function browserItem(overrides = {}) {
  return { id: 91, tabId: 7, url: request.url, finalUrl: request.url, filename: "report.zip",
    cookieStoreId: request.cookieStoreId, incognito: false, referrer: request.originUrl,
    startTime: "2026-11-13T00:00:00Z", state: "in_progress", paused: false, ...overrides };
}

test("Firefox holds the attachment response through Pending, then verifies cancellation before Commit", async () => {
  const f = await fixture({ phase: "pending" });
  const event = f.bg.chrome.api.webRequest.onHeadersReceived;
  assert.equal(event.listeners.length, 1, "Firefox must intercept before downloads.onCreated");
  assert.deepEqual(plain(event.registrations[0].extra), ["blocking", "responseHeaders"]);
  assert.deepEqual(plain(event.registrations[0].filter.types), ["main_frame", "sub_frame"]);
  let settled = false;
  const decision = f.fire().then(value => { settled = true; return value; });
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 1);
  await f.bg.settle();
  assert.equal(settled, false, "the original response remains blocked during confirmation");
  noDownloadEffects(f.bg);
  const prepare = f.bg.desktop.requestsOfType("prepare_browser_handoff")[0].message;
  const id = prepare.payload.handoffId;
  assert.equal(prepare.requestId, id);
  assert.equal(prepare.payload.fileName, "report.zip");
  assert.equal(prepare.payload.cookie, "session=synthetic-cookie");
  assert.equal(prepare.payload.headers.authorization, "synthetic-auth");
  assert.equal(prepare.payload.headers.referer, request.originUrl);
  assert.ok(f.cookieQueries.every(query => query.storeId === request.cookieStoreId));
  const journal = JSON.stringify(f.bg.chrome.storage.session.nxtools_browser_handoffs);
  assert.ok(!/https:|synthetic|authorization|report\.zip|cookie/i.test(journal), "journal is secret-free");
  f.responder.records.get(id).state = "held";
  f.responder.records.get(id).taskId = "controlled-task";
  f.bg.clock.advance(1000);
  assert.deepEqual(plain(await decision), { cancel: true });
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0, "returning cancel is not observed cancellation");
  await f.cancelled({ ...request, requestId: "unrelated-request" });
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  await f.cancelled();
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 1);
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff")[0].message.payload.handoffId, id);
  assert.equal(f.bg.chrome.storage.session.nxtools_browser_handoffs.length, 0);
  noDownloadEffects(f.bg);
});

test("Firefox response rejection releases the original once and onCreated does not offer it again", async () => {
  const f = await fixture({ phase: "aborted" });
  assert.deepEqual(plain(await f.fire()), {});
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 1);
  f.bg.chrome.api.downloads.onCreated.fire(browserItem());
  await f.bg.settle();
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 1);
  assert.equal(f.bg.chrome.downloadItems.get(91).paused, false);
  assert.equal(f.bg.chrome.callsNamed("downloads.pause").length, 0);
  assert.equal(f.bg.chrome.callsNamed("downloads.erase").length, 0);
});

test("unavailable desktop preserves the exact Firefox response without replacement download", async () => {
  const f = await fixture();
  f.bg.chrome.api.runtime.connectNative = () => { throw new Error("synthetic unavailable host"); };
  assert.deepEqual(plain(await f.fire()), {});
  noDownloadEffects(f.bg);
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 0);
  f.bg.chrome.api.downloads.onCreated.fire(browserItem());
  await f.bg.settle();
  assert.equal(f.bg.chrome.callsNamed("downloads.pause").length, 0);
});

for (const phase of ["aborted", "expired"]) test(`Firefox ${phase} Pending returns original response and never activates Held`, async () => {
  const f = await fixture({ phase: "pending" });
  const decision = f.fire();
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 1);
  const id = f.bg.desktop.requestsOfType("prepare_browser_handoff")[0].message.payload.handoffId;
  f.responder.records.get(id).state = phase;
  await f.bg.settle(); f.bg.clock.advance(1000);
  assert.deepEqual(plain(await decision), {});
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  noDownloadEffects(f.bg);
});

for (const changes of [ { method: "POST" }, { statusCode: 206 }, { statusCode: 302 }, { statusCode: 204 },
  { type: "xmlhttprequest" }, { responseHeaders: [{ name: "Content-Type", value: "text/html" }] },
  { responseHeaders: [{ name: "Content-Type", value: "application/pdf" }] }, { responseHeaders: [] }
]) test(`Firefox non-download response stays native: ${JSON.stringify(changes)}`, async () => {
  const f = await fixture();
  assert.deepEqual(plain(await f.fire({ ...request, ...changes })), {});
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 0);
  assert.equal(f.cookieQueries.length, 0);
  noDownloadEffects(f.bg);
});

for (const settings of [{ interceptDownloads: false }, { captureFileTypes: "pdf" }, { captureMinimumKiB: 8 },
  { captureExcludedSites: "files.test" }]) test(`Firefox response capture gates precede credentials: ${JSON.stringify(settings)}`, async () => {
  const f = await fixture({ settings });
  assert.deepEqual(plain(await f.fire()), {});
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 0);
  assert.equal(f.cookieQueries.length, 0);
  noDownloadEffects(f.bg);
});

for (const settings of [{ includeRequestHeaders: false }, { includeCookies: false }]) test(`Firefox response respects credential gate: ${JSON.stringify(settings)}`, async () => {
  const f = await fixture({ settings });
  assert.deepEqual(plain(await f.fire()), { cancel: true });
  const payload = f.bg.desktop.requestsOfType("prepare_browser_handoff")[0].message.payload;
  assert.equal(payload.cookie, undefined);
  assert.equal(f.cookieQueries.length, 0);
  if (settings.includeRequestHeaders === false) assert.deepEqual(plain(payload.headers), {});
  await f.cancelled();
});

test("Firefox duplicate response events share one handoff and one cancellation commit", async () => {
  const f = await fixture({ phase: "pending" });
  const first = f.fire(); const second = f.fire();
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 1);
  await f.bg.settle();
  const id = f.bg.desktop.requestsOfType("prepare_browser_handoff")[0].message.payload.handoffId;
  Object.assign(f.responder.records.get(id), { state: "held", taskId: "controlled-task" });
  f.bg.clock.advance(1000);
  assert.deepEqual(plain(await first), { cancel: true });
  assert.deepEqual(plain(await second), { cancel: true });
  await f.cancelled(); await f.cancelled();
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 1);
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 1);
});

test("Firefox cancellation without a terminal request event retains Held without Commit", async () => {
  const f = await fixture();
  assert.deepEqual(plain(await f.fire()), { cancel: true });
  f.bg.clock.advance(12000); await f.bg.settle();
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  assert.equal(f.bg.desktop.requestsOfType("abort_browser_handoff").length, 0);
  assert.equal(f.bg.chrome.storage.session.nxtools_browser_handoffs[0].phase, "cancelling");
});

test("Firefox response that starts instead of cancelling aborts Held and does not erase the browser", async () => {
  const f = await fixture();
  assert.deepEqual(plain(await f.fire()), { cancel: true });
  f.bg.chrome.api.webRequest.onResponseStarted.fire(request);
  await f.bg.settle();
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  assert.equal(f.bg.desktop.requestsOfType("abort_browser_handoff").length, 1);
  noDownloadEffects(f.bg);
});

test("Firefox wrong native handoff identity passes the response through and aborts the offered identity", async () => {
  const f = await fixture({ respond(message) {
    if (message.type === "prepare_browser_handoff") return browserHandoffReply("wrong-id", "held", "wrong-task");
  } });
  assert.deepEqual(plain(await f.fire()), {});
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  assert.equal(f.bg.desktop.requestsOfType("abort_browser_handoff").length, 1);
  noDownloadEffects(f.bg);
});

test("Chromium never registers a blocking response listener and preserves the controlled filename path", async () => {
  const f = await fixture({ browser: "chrome" });
  assert.equal(f.bg.chrome.api.webRequest.onHeadersReceived.listeners.length, 0);
  assert.equal(f.bg.manifest.permissions.includes("webRequestBlocking"), false);
  let suggestions = 0;
  assert.deepEqual(f.bg.chrome.api.downloads.onDeterminingFilename.fire(browserItem(), () => { suggestions += 1; }), [true]);
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("commit_browser_handoff").length === 1);
  await f.bg.settle();
  assert.equal(suggestions, 1);
  assert.equal(f.bg.chrome.callsNamed("downloads.pause").length, 1);
  assert.equal(f.bg.chrome.callsNamed("downloads.cancel").length, 1);
});

test("Firefox source manifest without exported blocking permission keeps the controlled downloads fallback", async () => {
  const f = await fixture({ chrome: { manifest: { permissions: ["webRequest", "downloads"] } } });
  assert.equal(f.bg.chrome.api.webRequest.onHeadersReceived.listeners.length, 0);
  f.bg.chrome.api.downloads.onCreated.fire(browserItem());
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("commit_browser_handoff").length === 1);
  assert.equal(f.bg.chrome.callsNamed("downloads.pause").length, 1);
});

test("Firefox long confirmation refusal does not offer the released response again after request-cache expiry", async () => {
  const f = await fixture({ phase: "pending" });
  const decision = f.fire();
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 1);
  await f.bg.settle();
  f.bg.clock.advance(180000); await f.bg.settle();
  const id = f.bg.desktop.requestsOfType("prepare_browser_handoff")[0].message.payload.handoffId;
  f.responder.records.get(id).state = "aborted";
  f.bg.clock.advance(1000);
  assert.deepEqual(plain(await decision), {});
  f.bg.chrome.api.downloads.onCreated.fire(browserItem());
  await f.bg.settle();
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 1);
  assert.equal(f.bg.chrome.callsNamed("downloads.pause").length, 0);
});

for (const phase of ["waiting_confirmation", "cancelling", "browser_cancelled"]) test(`Firefox response journal recovery preserves ${phase} authority`, async () => {
  const id = "12345678-1234-4abc-8def-1234567890ab";
  const bg = loadBackground({ browser: "firefox", chrome: { sessionStorage: { nxtools_browser_handoffs: [{
    kind: "response", requestId: "retired-request", downloadId: "response:retired-request", handoffId: id,
    phase, prepared: true, taskId: "held-task", startedAt: 1800000000000
  }] } }, desktop: { respond(message) {
    if (message.type === "get_browser_handoff") return browserHandoffReply(id, "held", "held-task");
    if (message.type === "commit_browser_handoff") return browserHandoffReply(id, "committed", "held-task");
    if (message.type === "abort_browser_handoff") return browserHandoffReply(id, "aborted");
  } } });
  await bg.settle();
  assert.equal(bg.desktop.requestsOfType("commit_browser_handoff").length, phase === "browser_cancelled" ? 1 : 0);
  assert.equal(bg.desktop.requestsOfType("abort_browser_handoff").length, phase === "waiting_confirmation" ? 1 : 0);
  assert.equal(bg.chrome.storage.session.nxtools_browser_handoffs.length, phase === "cancelling" ? 1 : 0);
  noDownloadEffects(bg);
});

test("Firefox native prepare timeout releases the response and prevents a second onCreated offer", async () => {
  const f = await fixture({ respond(message) { if (message.type === "prepare_browser_handoff") return null; } });
  const decision = f.fire();
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 1);
  f.bg.clock.advance(15000); await f.bg.settle();
  assert.deepEqual(plain(await decision), {});
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  f.bg.chrome.api.downloads.onCreated.fire(browserItem()); await f.bg.settle();
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 1);
  assert.equal(f.bg.chrome.callsNamed("downloads.pause").length, 0);
});

test("Firefox native disconnect during Pending aborts without replacing the original response", async () => {
  const f = await fixture({ phase: "pending" });
  const decision = f.fire();
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 1);
  await f.bg.settle();
  f.bg.chrome.nativePorts[0].disconnect();
  f.bg.clock.advance(1000);
  assert.deepEqual(plain(await decision), {});
  assert.equal(f.bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  noDownloadEffects(f.bg);
});

test("Firefox final GET after POST redirect preserves captured referrer and container", async () => {
  const f = await fixture();
  const initial = { ...request, url: "https://files.test/form", method: "POST" };
  f.headers(initial);
  f.bg.chrome.api.webRequest.onBeforeRedirect.fire({ ...initial, statusCode: 303, responseHeaders: [
    { name: "Location", value: request.url }
  ] });
  assert.deepEqual(plain(await f.fire()), { cancel: true });
  const payload = f.bg.desktop.requestsOfType("prepare_browser_handoff")[0].message.payload;
  assert.equal(payload.url, request.url);
  assert.equal(payload.headers.referer, request.originUrl);
  assert.ok(f.cookieQueries.every(query => query.storeId === request.cookieStoreId));
  await f.cancelled();
});

test("Firefox response rules include the redirect's original address and source page", async () => {
  for (const settings of [{ captureExcludedAddresses: "/form" }, { captureExcludedSites: "page.test" }]) {
    const f = await fixture({ settings });
    const initial = { ...request, url: "https://files.test/form" };
    f.headers(initial);
    f.bg.chrome.api.webRequest.onBeforeRedirect.fire({ ...initial, statusCode: 302, responseHeaders: [
      { name: "Location", value: request.url }
    ] });
    assert.deepEqual(plain(await f.fire()), {});
    assert.equal(f.cookieQueries.length, 0);
    assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 0);
  }
});

test("Firefox response private store wins over incompatible live tab metadata", async () => {
  const f = await fixture();
  const details = { ...request, incognito: true, cookieStoreId: "firefox-private" };
  assert.deepEqual(plain(await f.fire(details)), { cancel: true });
  assert.ok(f.cookieQueries.length > 0);
  assert.ok(f.cookieQueries.every(query => query.storeId === "firefox-private"));
  await f.cancelled(details);
});

test("Firefox a later same-URL request from another container is not a released response fallback", async () => {
  const f = await fixture({ phase: "aborted" });
  assert.deepEqual(plain(await f.fire()), {});
  const other = { ...request, requestId: "other-container", tabId: 8, cookieStoreId: "firefox-container-5" };
  f.headers(other);
  f.bg.chrome.api.downloads.onCreated.fire(browserItem({ id: 92, tabId: 8, cookieStoreId: other.cookieStoreId }));
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 2);
  await f.bg.settle();
  assert.equal(f.bg.chrome.callsNamed("downloads.pause").length, 1);
});

test("Firefox missing session control never prepares a desktop task", async () => {
  const f = await fixture({ chrome: { sessionGetFails: true } });
  assert.deepEqual(plain(await f.fire()), {});
  assert.equal(f.bg.desktop.requestsOfType("prepare_browser_handoff").length, 0);
  noDownloadEffects(f.bg);
});

test("Firefox active response admission is capped at32 without evicting a held request", async () => {
  const f = await fixture({ phase: "pending" });
  const decisions = [];
  for (let index = 0; index < 33; index += 1) {
    decisions.push(f.fire({ ...request, requestId: `bounded-${index}`, url: `${request.url}/${index}` }));
  }
  await f.bg.waitFor(() => f.bg.desktop.requestsOfType("prepare_browser_handoff").length === 32);
  assert.deepEqual(plain(await decisions[32]), {});
  assert.equal(f.bg.chrome.storage.session.nxtools_browser_handoffs.length, 32);
  for (const record of f.responder.records.values()) record.state = "aborted";
  await f.bg.settle(); f.bg.clock.advance(1000);
  await Promise.all(decisions);
  assert.equal(f.bg.chrome.storage.session.nxtools_browser_handoffs.length, 0);
  noDownloadEffects(f.bg);
});

test("Firefox response diagnostics use one handoff UUID and fixed lifecycle fields without payload", async () => {
  const f = await fixture();
  assert.deepEqual(plain(await f.fire()), { cancel: true });
  await f.cancelled();
  const id = f.bg.desktop.requestsOfType("prepare_browser_handoff")[0].message.payload.handoffId;
  const logs = f.bg.chrome.nativePorts.flatMap(port => port.sent).filter(message => message.type === "extension_log" && message.payload?.event === "handoff_state");
  assert.ok(logs.length >= 2);
  const serialized = JSON.stringify(logs);
  assert.ok(serialized.includes(id));
  assert.ok(!/files\.test|page\.test|synthetic|report\.zip|authorization|referer/i.test(serialized));
  for (const log of logs) assert.equal(log.payload.correlationId, id);
});
