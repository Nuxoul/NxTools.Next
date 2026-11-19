"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground, createControlledHandoffResponder } = require("./harness/load-background.cjs");
const { createEvent } = require("./harness/chrome-api.cjs");

const url = "https://files.example.test/archive.zip";
function host() {
  return { respond(message) {
    if (message.type === "create_task") return { ok: true, accepted: true, handoffId: message.payload.handoffId, taskId: "accepted" };
    if (message.type === "create_tasks") return { ok: true, data: { results: message.payload.items.map(item => ({ handoffId: item.handoffId, accepted: true, taskId: item.handoffId })) } };
    return { ok: true, data: { tasks: [] } };
  } };
}
function popupPort(bg, sender = bg.senders.popup()) {
  const port = { name: "nxtools-popup", sender, onDisconnect: createEvent(), disconnect() { this.onDisconnect.fire(this); } };
  bg.chrome.api.runtime.onConnect.fire(port);
  return port;
}
async function send(bg, headers) {
  await bg.settle();
  await bg.realm.sendDownloadPayload({ url, fileName: "archive.zip", headers, contextUrl: "https://files.example.test/page", cookieSource: { cookieStoreId: "fixture-default" } });
  return bg.desktop.requestsOfType("create_task").at(-1).message.payload;
}

for (const browser of ["chrome", "firefox"]) {
  test(`L6 ${browser} disabled header master gate excludes credentials at the actual wire`, async () => {
    const bg = loadBackground({ browser, desktop: host(), chrome: { storage: { includeRequestHeaders: false, includeCookies: true } } });
    let reads = 0;
    bg.chrome.api.cookies.getAll = async () => { reads += 1; return [{ name: "fixture", value: "secret" }]; };
    const payload = await send(bg, { authorization: "fixture-secret", Cookie: "captured-secret", referer: "https://files.example.test/private" });
    assert.deepEqual(Object.keys(payload.headers || {}), [], "master gate must remove outbound headers");
    assert.ok(!payload.cookie, "master gate must remove dedicated Cookie");
    assert.ok(!payload.referrer, "master gate must not bypass through Referrer");
    assert.equal("headerText" in payload, false, "host-unused headerText is absent");
    assert.equal(reads, 0, "disabled master gate does not read Cookie API");
  });
}

test("L6 cookie gate removes captured Cookie while enabled headers survive once", async () => {
  const bg = loadBackground({ desktop: host(), chrome: { storage: { includeRequestHeaders: true, includeCookies: false } } });
  const payload = await send(bg, { authorization: "synthetic-authorization", Cookie: "synthetic-cookie", referer: "https://files.example.test/page" });
  assert.equal(payload.headers.authorization, "synthetic-authorization");
  assert.equal(Object.keys(payload.headers).some(name => name.toLowerCase() === "cookie"), false);
  assert.ok(!payload.cookie);
  assert.equal("headerText" in payload, false);
});

test("L6 enabled Cookie API retains HttpOnly and Strict synthetic cookies in one wire field", async () => {
  const bg = loadBackground({ desktop: host() });
  bg.chrome.api.cookies.getAll = async () => [{ name: "session", value: "synthetic", httpOnly: true, sameSite: "strict", path: "/" }];
  const payload = await send(bg, { authorization: "synthetic" });
  assert.equal(payload.cookie, "session=synthetic");
  assert.equal(Object.keys(payload.headers).some(name => name.toLowerCase() === "cookie"), false);
  assert.equal("headerText" in payload, false);
});

test("L6 actual 32 item batch carries one 4 KiB Cookie each below 256 KiB", async (t) => {
  const bg = loadBackground({ desktop: host() });
  await bg.settle();
  const cookie = "s=" + "x".repeat(4094);
  for (let index = 0; index < 32; index += 1) {
    const resourceUrl = `https://files.example.test/file-${index}.zip`;
    const details = { requestId: String(index), url: resourceUrl, tabId: 7, type: "xmlhttprequest", timeStamp: bg.clock.now() };
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ ...details, requestHeaders: [{ name: "Cookie", value: cookie }, { name: "Authorization", value: "synthetic" }] });
    bg.chrome.api.webRequest.onResponseStarted.fire({ ...details, statusCode: 200, responseHeaders: [{ name: "content-type", value: "application/zip" }, { name: "content-length", value: "2048" }] });
  }
  await bg.settle(30);
  const ids = bg.evaluate("[...resourcesById.keys()]");
  assert.equal(ids.length, 32);
  await bg.realm.sendResourcesBatch(ids);
  const request = bg.desktop.requestsOfType("create_tasks").at(-1)?.message;
  assert.ok(request, "batch reaches actual Native Messaging send");
  assert.ok(Buffer.byteLength(JSON.stringify(request)) < 256 * 1024);
  t.diagnostic(`batch_native_bytes=${Buffer.byteLength(JSON.stringify(request))}`);
  for (const item of request.payload.items) {
    assert.equal(item.cookie.length, 4096);
    assert.equal(Object.keys(item.headers).some(name => name.toLowerCase() === "cookie"), false, "Cookie is sent once");
    assert.equal("headerText" in item, false);
  }
});

test("captured page facts survive popup and independent batch with headers and cookies disabled", async () => {
  const pageUrl = "https://pages.example.test/article?chapter=3";
  const pageTitle = "章节".repeat(512);
  const bg = loadBackground({ desktop: host(), chrome: { tabs: [{ id: 7, url: pageUrl, title: pageTitle }],
    storage: { includeRequestHeaders: false, includeCookies: false } } });
  await bg.settle();
  const details = { requestId: "page-fact", url, tabId: 7, type: "xmlhttprequest",
    requestHeaders: [{ name: "Referer", value: "https://referrer.example.test/" }] };
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire(details);
  bg.chrome.api.webRequest.onResponseStarted.fire({ ...details, statusCode: 200,
    responseHeaders: [{ name: "content-type", value: "application/zip" }] });
  await bg.settle();
  const id = `7:${url}`;
  assert.equal((await bg.sendRuntimeMessage({ type: "popup_send_resource", resourceId: id }, bg.senders.popup())).ok, true);
  const single = bg.desktop.requestsOfType("create_task").at(-1).message.payload;
  assert.equal(single.sourcePageUrl, pageUrl);
  assert.equal(single.sourcePageTitle, pageTitle);
  assert.equal(single.referrer, undefined);
  assert.deepEqual(Object.keys(single.headers), []);
  assert.equal(single.cookie, undefined);
  await bg.realm.sendResourcesBatch([id]);
  const batch = bg.desktop.requestsOfType("create_tasks").at(-1).message.payload.items[0];
  assert.equal(batch.sourcePageUrl, pageUrl);
  assert.equal(batch.sourcePageTitle, pageTitle);
  assert.equal(batch.referrer, undefined);
  assert.equal(batch.cookie, undefined);
  assert.equal(JSON.stringify(bg.chrome.storage.local).includes(pageUrl), false);
  assert.equal(JSON.stringify(bg.chrome.storage.local).includes(pageTitle), false);
});

test("redirected automatic controlled prepare uses original captured page, never Referer inference", async () => {
  const original = "https://files.example.test/original.zip";
  const final = "https://cdn.example.test/final.zip";
  const pageUrl = "https://pages.example.test/article";
  const bg = loadBackground({ desktop: { respond: createControlledHandoffResponder({ fallback: host().respond }).respond },
    chrome: { tabs: [{ id: 7, url: pageUrl, title: "Captured title" }],
      storage: { includeRequestHeaders: false, includeCookies: false } } });
  await bg.settle();
  const details = { requestId: "page-redirect", url: original, tabId: 7, type: "xmlhttprequest" };
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ ...details, requestHeaders: [] });
  bg.chrome.api.webRequest.onResponseStarted.fire({ ...details, statusCode: 200,
    responseHeaders: [{ name: "content-type", value: "application/zip" }] });
  await bg.settle();
  bg.chrome.api.downloads.onDeterminingFilename.fire({ id: 19, url: original, finalUrl: final,
    tabId: 7, filename: "final.zip", referrer: "https://other.example.test/referer",
    startTime: "2026-11-13T00:00:00Z", totalBytes: 2048 }, () => {});
  await bg.waitFor(() => bg.desktop.requestsOfType("prepare_browser_handoff").length === 1, "controlled prepare");
  const payload = bg.desktop.requestsOfType("prepare_browser_handoff")[0].message.payload;
  assert.equal(payload.url, final);
  assert.equal(payload.sourcePageUrl, pageUrl);
  assert.equal(payload.sourcePageTitle, "Captured title");
  assert.equal(payload.cookie, undefined);
  const unknown = await bg.realm.sendDownloadPayload({ url: final, fileName: "final.zip",
    contextUrl: "https://other.example.test/referer", headers: { Referer: "https://other.example.test/referer" } });
  assert.equal(unknown.ok, true);
  const noPage = bg.desktop.requestsOfType("create_task").at(-1).message.payload;
  assert.equal("sourcePageUrl" in noPage, false);
  assert.equal("sourcePageTitle" in noPage, false);
});

test("L7 popup lifetime reuses one host for ten seconds then releases after one second", async () => {
  const bg = loadBackground({ desktop: host() });
  await bg.settle();
  assert.equal(bg.chrome.nativePorts.length, 0, "startup stays passive");
  const popup = popupPort(bg);
  await bg.settle();
  for (let elapsed = 0; elapsed <= 9000; elapsed += 1500) {
    const state = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.equal(state.status.ok, true);
    bg.clock.advance(elapsed === 9000 ? 1000 : 1500);
    await bg.settle();
  }
  assert.equal(bg.chrome.nativePorts.length, 1, "ten-second popup owns one native host");
  assert.equal(bg.chrome.nativePorts[0].closed, false);
  popup.disconnect();
  bg.clock.advance(999); await bg.settle();
  assert.equal(bg.chrome.nativePorts[0].closed, false);
  bg.clock.advance(1); await bg.settle();
  assert.equal(bg.chrome.nativePorts[0].closed, true);
});

test("L7 disconnected open popup does not reconnect on refresh and explicit test may retry", async () => {
  const bg = loadBackground({ desktop: host() });
  await bg.settle();
  const popup = popupPort(bg);
  await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
  bg.chrome.nativePorts[0].disconnect();
  for (let index = 0; index < 7; index += 1) {
    bg.clock.advance(1500); await bg.settle();
    const state = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.equal(state.status.ok, false, "disconnected status remains stable");
  }
  assert.equal(bg.chrome.nativePorts.length, 1, "refresh has no reconnect authority");
  const retry = await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  assert.equal(retry.ok, true);
  assert.equal(bg.chrome.nativePorts.length, 2, "explicit user connection test is demand");
  popup.disconnect();
});

test("L7 content and foreign ports cannot retain or start native transport", async () => {
  const bg = loadBackground({ desktop: host() });
  await bg.settle();
  popupPort(bg, bg.senders.content());
  popupPort(bg, bg.senders.foreign());
  await bg.settle();
  assert.equal(bg.chrome.nativePorts.length, 0);
  assert.equal(bg.evaluate("desktopPortOwners"), 0);
});
