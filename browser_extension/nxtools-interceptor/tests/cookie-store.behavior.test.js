"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground, createControlledHandoffResponder } = require("./harness/load-background.cjs");
const URL = "https://files.example.test/archive.zip";

function desktop() {
  const controlled = createControlledHandoffResponder({ fallback(message) {
    if (message.type === "create_task") return { ok: true, accepted: true, handoffId: message.payload.handoffId, taskId: "fixture-task" };
    if (message.type === "create_tasks") return { ok: true, data: { results: message.payload.items.map(item => ({ handoffId: item.handoffId, accepted: true, taskId: item.handoffId })) } };
    return { ok: true, data: { tasks: [] } };
  } });
  return { respond: controlled.respond };
}
function item(id, context = {}) {
  return { id, url: URL, finalUrl: URL, filename: "archive.zip", totalBytes: 2048,
    startTime: "2026-11-09T00:00:00Z", ...context };
}
function installCookies(bg) {
  const queries = [];
  bg.chrome.api.cookies.getAll = async query => {
    queries.push(JSON.parse(JSON.stringify(query)));
    return [{ name: "session", value: query.storeId || "wrong-default", path: "/", httpOnly: true, sameSite: "strict" }];
  };
  return queries;
}
async function download(bg, download) {
  const event = bg.chrome.api.downloads.onDeterminingFilename || bg.chrome.api.downloads.onCreated;
  let suggested = 0;
  const before = bg.chrome.callsNamed("downloads.cancel").length;
  event.fire(download, () => { suggested += 1; });
  await bg.waitFor(() => bg.chrome.callsNamed("downloads.cancel").length > before, "accepted browser download cancellation");
  return bg.desktop.requestsOfType("prepare_browser_handoff").at(-1)?.message.payload;
}

test("REF-02 Firefox same URL queries originating container and private stores at native handoff", async () => {
  const bg = loadBackground({ browser: "firefox", desktop: desktop() });
  await bg.settle();
  const queries = installCookies(bg);
  const first = await download(bg, item(11, { cookieStoreId: "firefox-container-2" }));
  const second = await download(bg, item(12, { cookieStoreId: "firefox-private", incognito: true }));
  assert.deepEqual(queries, [
    { url: URL, storeId: "firefox-container-2" },
    { url: URL, storeId: "firefox-private" }
  ]);
  assert.equal(first.cookie, "session=firefox-container-2");
  assert.equal(second.cookie, "session=firefox-private");
  assert.equal("cookieStoreId" in first, false, "store is consumed at Cookie query, not unused Rust IPC");
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 2);
});

module.exports = { desktop, item, installCookies, download, URL };

test("REF-02 same URL resource contexts stay isolated through single/batch and tab replacement", async () => {
  const tabs = [
    { id: 7, url: "https://files.example.test/page", cookieStoreId: "firefox-container-2", active: true },
    { id: 8, url: "https://files.example.test/page", cookieStoreId: "firefox-private", incognito: true }
  ];
  const bg = loadBackground({ browser: "firefox", desktop: desktop(), chrome: { tabs } });
  await bg.settle();
  const queries = installCookies(bg);
  for (const tab of tabs) {
    const details = { requestId: `r${tab.id}`, url: URL, tabId: tab.id, cookieStoreId: tab.cookieStoreId,
      type: "media", requestHeaders: [{ name: "Authorization", value: `auth-${tab.id}` }] };
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire(details);
    bg.chrome.api.webRequest.onResponseStarted.fire({ ...details, statusCode: 200,
      responseHeaders: [{ name: "Content-Type", value: "application/zip" }] });
    await bg.settle();
  }
  const original = await bg.sendRuntimeMessage({ type: "popup_send_resource", resourceId: `7:${URL}` }, bg.senders.popup());
  assert.equal(original.ok, true);
  const originalPayload = bg.desktop.requestsOfType("create_task").at(-1).message.payload;
  assert.equal(originalPayload.cookie, "session=firefox-container-2");
  assert.equal(originalPayload.headers.authorization, "auth-7", "later same-URL capture cannot overwrite original credentials");
  bg.chrome.tabs.delete(7);
  bg.chrome.api.tabs.onRemoved.fire(7);
  // Preserve the actual captured resource for a pending user dispatch while its tab is replaced.
  bg.chrome.tabs.set(8, { id: 8, url: "https://files.example.test/page", cookieStoreId: "firefox-container-9" });
  const result = await bg.sendRuntimeMessage({ type: "popup_send_resource", resourceId: `8:${URL}` }, bg.senders.popup());
  assert.equal(result.ok, true);
  let payload = bg.desktop.requestsOfType("create_task").at(-1)?.message.payload;
  assert.equal(payload.cookie, "session=firefox-private", "saved origin is not the replacement tab's store");
  assert.equal(payload.headers.authorization, "auth-8");
  await bg.realm.sendResourcesBatch([`8:${URL}`]);
  payload = bg.desktop.requestsOfType("create_tasks").at(-1)?.message.payload.items[0];
  assert.equal(payload.cookie, "session=firefox-private");
  assert.equal(payload.headers.authorization, "auth-8");
  assert.deepEqual(queries.map(query => query.storeId), [
    "firefox-container-2", "firefox-container-2",
    "firefox-private", "firefox-private", "firefox-private", "firefox-private"
  ]);
  const projected = JSON.stringify(bg.chrome.storage.local);
  assert.ok(!projected.includes("firefox-private") && !projected.includes("auth-8"), "privileged origin/context is not persisted");
});

test("REF-02 redirects retain request store after source tab closes", async () => {
  const bg = loadBackground({ browser: "firefox", desktop: desktop() });
  await bg.settle();
  const queries = installCookies(bg);
  const initial = "https://files.example.test/start";
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: "redirect", url: initial, tabId: 7,
    cookieStoreId: "firefox-container-3", type: "main_frame", requestHeaders: [{ name: "Accept", value: "*/*" }] });
  bg.chrome.api.webRequest.onBeforeRedirect.fire({ requestId: "redirect", url: initial, tabId: 7,
    redirectUrl: URL, responseHeaders: [{ name: "Location", value: URL }], statusCode: 302 });
  bg.chrome.api.tabs.onRemoved.fire(7);
  const payload = await download(bg, item(24, { url: initial, finalUrl: URL }));
  assert.equal(payload.cookie, "session=firefox-container-3");
  assert.deepEqual(queries, [{ url: URL, storeId: "firefox-container-3" }]);
});

test("REF-02 private Chromium download resolves existing private store, never normal", async () => {
  const bg = loadBackground({ desktop: desktop() });
  await bg.settle();
  const queries = installCookies(bg);
  bg.chrome.api.cookies.getAllCookieStores = async () => [{ id: "0", tabIds: [7] }, { id: "1", tabIds: [8] }];
  const payload = await download(bg, item(30, { incognito: true }));
  assert.equal(payload.cookie, "session=1");
  assert.deepEqual(queries, [{ url: URL, storeId: "1" }]);
});

test("REF-02 disabled gates do not enumerate or query stores", async () => {
  for (const settings of [{ includeRequestHeaders: false, includeCookies: true }, { includeRequestHeaders: true, includeCookies: false }]) {
    const bg = loadBackground({ desktop: desktop(), chrome: { storage: settings } });
    await bg.settle();
    const queries = installCookies(bg);
    let enumerated = 0;
    bg.chrome.api.cookies.getAllCookieStores = async () => { enumerated += 1; return [{ id: "1", tabIds: [] }]; };
    const payload = await download(bg, item(31, { incognito: true }));
    assert.equal(queries.length, 0);
    assert.equal(enumerated, 0);
    assert.ok(!payload.cookie);
  }
});

test("REF-02 unavailable source store or unsupported API never queries another store", async () => {
  for (const scenario of ["rejected-store", "unsupported-stores", "unsupported-cookies", "closed-tab"]) {
    const bg = loadBackground({ desktop: desktop() });
    await bg.settle();
    const queries = [];
    bg.chrome.api.cookies.getAll = async query => { queries.push(query); throw new Error("source unavailable"); };
    if (scenario === "unsupported-cookies") delete bg.chrome.api.cookies.getAll;
    const source = scenario === "rejected-store" || scenario === "unsupported-cookies"
      ? { cookieStoreId: "firefox-container-gone" }
      : scenario === "closed-tab" ? { tabId: 9 } : { incognito: true };
    const payload = await download(bg, item(32, source));
    assert.ok(!payload.cookie, scenario);
    assert.ok(queries.every(query => query.storeId === "firefox-container-gone"), scenario);
    if (scenario !== "rejected-store") assert.equal(queries.length, 0, scenario);
  }
});

test("REF-02 same URL request snapshots select source credentials, not latest URL cache", async () => {
  const bg = loadBackground({ browser: "firefox", desktop: desktop() }); await bg.settle();
  for (const [tabId, cookieStoreId] of [[7, "firefox-container-2"], [8, "firefox-private"]]) {
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: `snapshot-${tabId}`, url: URL, tabId, cookieStoreId,
      type: "main_frame", requestHeaders: [{ name: "Cookie", value: `captured=${tabId}` }, { name: "Authorization", value: `auth-${tabId}` }] });
  }
  const payload = await download(bg, item(33, { tabId: 7, cookieStoreId: "firefox-container-2" }));
  assert.equal(payload.cookie, "captured=7"); assert.equal(payload.headers.authorization, "auth-7");
});

test("REF-02 every context fallback URL queries the same originating store", async () => {
  const bg = loadBackground({ browser: "firefox", desktop: desktop() }); await bg.settle();
  const queries = installCookies(bg);
  const payload = await download(bg, item(34, { cookieStoreId: "firefox-container-4", referrer: "https://www.youtube.com/watch?v=fixture" }));
  assert.equal(queries.length, 5); assert.ok(queries.every(query => query.storeId === "firefox-container-4"));
  assert.equal(payload.cookie, "session=firefox-container-4");
});

test("REF-02 Chromium private download never borrows a normal tab's captured credentials", async () => {
  const bg = loadBackground({ desktop: desktop(), chrome: { tabs: [{ id: 7, url: "https://files.example.test/page", incognito: false }] } });
  await bg.settle(); const queries = installCookies(bg);
  bg.chrome.api.cookies.getAllCookieStores = async () => [{ id: "0", tabIds: [7] }, { id: "1", tabIds: [] }];
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: "normal", tabId: 7, url: URL, type: "main_frame",
    requestHeaders: [{ name: "Cookie", value: "normal=captured" }, { name: "Authorization", value: "normal-authorization" }] });
  await bg.settle();
  const privatePayload = await download(bg, item(35, { incognito: true }));
  assert.equal(privatePayload.cookie, "session=1"); assert.ok(!privatePayload.headers.authorization);
  assert.deepEqual(queries, [{ url: URL, storeId: "1" }]);
  bg.chrome.tabs.delete(7);
  const normalPayload = await download(bg, item(36, { incognito: false }));
  assert.equal(normalPayload.cookie, "normal=captured");
  assert.equal(normalPayload.headers.authorization, "normal-authorization", "observed tab privacy survives closure");
});

async function capturedTabs(sharedReferrer = false) {
  const tabs = [
    { id: 7, url: "https://files.example.test/source", cookieStoreId: "firefox-container-2", incognito: false },
    { id: 8, url: sharedReferrer ? "https://files.example.test/source" : "https://files.example.test/other", cookieStoreId: "firefox-container-9", incognito: false }
  ];
  const bg = loadBackground({ browser: "firefox", desktop: desktop(), chrome: { tabs } });
  await bg.settle();
  const queries = installCookies(bg);
  for (const tab of tabs) {
    bg.clock.advance(1);
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: `origin-${tab.id}`, url: URL, tabId: tab.id,
      cookieStoreId: tab.cookieStoreId, type: "main_frame",
      requestHeaders: [{ name: "Authorization", value: `auth-${tab.id}` }] });
    await bg.settle();
  }
  queries.length = 0;
  return { bg, queries };
}

for (const privacyKnown of [true, false]) test(`REF-02 unique referrer recovers source context without a DownloadItem tab/store (privacy=${privacyKnown})`, async () => {
  const { bg, queries } = await capturedTabs();
  const payload = await download(bg, item(50, { referrer: "https://files.example.test/source", ...(privacyKnown ? { incognito: false } : {}) }));
  assert.equal(payload.headers.authorization, "auth-7");
  assert.equal(payload.cookie, "session=firefox-container-2");
  assert.deepEqual(queries, [
    { url: URL, storeId: "firefox-container-2" },
    { url: "https://files.example.test/source", storeId: "firefox-container-2" }
  ]);
});

test("REF-02 privacy alone cannot choose among same-URL containers with the same referrer", async () => {
  const { bg, queries } = await capturedTabs(true);
  const payload = await download(bg, item(51, { incognito: false, referrer: "https://files.example.test/source" }));
  assert.ok(!payload.headers.authorization, "ambiguous source cannot borrow another tab's Authorization");
  assert.ok(!payload.cookie, "ambiguous source cannot borrow another container's Cookie");
  assert.equal(queries.length, 0, "no originating store means no Cookie query");
});

for (const incognito of [true, false]) test(`B1 repair observed Chromium source survives closed tab (private=${incognito})`, async () => {
  const tabs = [
    { id: 7, url: "https://files.example.test/normal", incognito: false },
    { id: 8, url: "https://files.example.test/private", incognito: true }
  ];
  const bg = loadBackground({ desktop: desktop(), chrome: { tabs } });
  await bg.settle();
  const queries = installCookies(bg);
  const stores = [{ id: "0", tabIds: [7] }, { id: "1", tabIds: [8] }];
  bg.chrome.api.cookies.getAllCookieStores = async () => stores;
  for (const tab of tabs) {
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: `closed-${tab.id}`, tabId: tab.id, url: URL,
      type: "main_frame", requestHeaders: [{ name: "Authorization", value: `auth-${tab.id}` }] });
    await bg.settle();
  }
  for (const tab of tabs) {
    bg.chrome.tabs.delete(tab.id);
    bg.chrome.api.tabs.onRemoved.fire(tab.id);
  }
  for (const store of stores) store.tabIds = [];
  queries.length = 0;
  const payload = await download(bg, item(incognito ? 60 : 61, { incognito }));
  assert.equal(payload.cookie, incognito ? "session=1" : "session=0");
  assert.equal(payload.headers.authorization, incognito ? "auth-8" : "auth-7");
  assert.deepEqual(queries, [{ url: URL, storeId: incognito ? "1" : "0" }]);
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1);
});

for (const incognito of [true, false]) test(`B1 repair observed privacy wins over conflicting live Chromium tab (private=${incognito})`, async () => {
  const bg = loadBackground({ desktop: desktop(), chrome: { tabs: [
    { id: 7, url: "https://files.example.test/source", incognito }
  ] } });
  await bg.settle();
  const queries = installCookies(bg);
  const stores = [{ id: "0", tabIds: incognito ? [] : [7] }, { id: "1", tabIds: incognito ? [7] : [] }];
  bg.chrome.api.cookies.getAllCookieStores = async () => stores;
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: "observed", tabId: 7, url: URL,
    type: "main_frame", requestHeaders: [{ name: "Authorization", value: "source-auth" }] });
  await bg.settle();
  bg.chrome.tabs.set(7, { id: 7, url: "https://files.example.test/replacement", incognito: !incognito });
  stores[0].tabIds = incognito ? [7] : [];
  stores[1].tabIds = incognito ? [] : [7];
  queries.length = 0;
  const payload = await download(bg, item(incognito ? 62 : 63, { incognito }));
  assert.equal(payload.cookie, incognito ? "session=1" : "session=0");
  assert.equal(payload.headers.authorization, "source-auth");
  assert.deepEqual(queries, [
    { url: URL, storeId: incognito ? "1" : "0" },
    { url: "https://files.example.test/source", storeId: incognito ? "1" : "0" }
  ]);
});

test("B1 repair Firefox observed store cannot acquire conflicting live privacy metadata", async () => {
  const bg = loadBackground({ browser: "firefox", desktop: desktop(), chrome: { tabs: [
    { id: 7, url: "https://files.example.test/replacement", cookieStoreId: "firefox-container-9", incognito: false }
  ] } });
  await bg.settle();
  const queries = installCookies(bg);
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: "private-source", tabId: 7, url: URL,
    cookieStoreId: "firefox-private", type: "main_frame",
    requestHeaders: [{ name: "Authorization", value: "private-source-auth" }] });
  await bg.settle();
  const payload = await download(bg, item(64, { cookieStoreId: "firefox-private", incognito: true }));
  assert.equal(payload.headers.authorization, "private-source-auth");
  assert.equal(payload.cookie, "session=firefox-private");
  assert.deepEqual(queries, [{ url: URL, storeId: "firefox-private" }]);
  assert.ok(!payload.referrer, "conflicting live page cannot supply the originating referrer");
});

for (const [gate, storage] of [
  ["headers", { includeRequestHeaders: false, includeCookies: true }],
  ["cookies", { includeRequestHeaders: true, includeCookies: false }]
]) test(`B1 repair closed Chromium source respects disabled ${gate} gate`, async () => {
  const bg = loadBackground({ desktop: desktop(), chrome: { storage, tabs: [
    { id: 7, url: "https://files.example.test/source", incognito: true }
  ] } });
  await bg.settle();
  const queries = installCookies(bg);
  let enumerated = 0;
  bg.chrome.api.cookies.getAllCookieStores = async () => { enumerated += 1; return [{ id: "1", tabIds: [] }]; };
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: "gated", tabId: 7, url: URL,
    type: "main_frame", requestHeaders: [{ name: "Authorization", value: "source-auth" }] });
  await bg.settle();
  bg.chrome.tabs.delete(7);
  bg.chrome.api.tabs.onRemoved.fire(7);
  const payload = await download(bg, item(65, { incognito: true }));
  assert.ok(!payload.cookie);
  assert.equal(queries.length, 0);
  assert.equal(enumerated, 0);
  if (gate === "headers") assert.deepEqual(Object.keys(payload.headers), []);
});

for (const browser of ["chrome", "firefox"]) test(`B1 repair ${browser} insufficient closed-source identity cannot select a store`, async () => {
  const bg = loadBackground({ browser, desktop: desktop() });
  await bg.settle();
  const queries = installCookies(bg);
  let enumerated = 0;
  bg.chrome.api.cookies.getAllCookieStores = async () => {
    enumerated += 1;
    return browser === "chrome" ? [{ id: "0", tabIds: [] }, { id: "1", tabIds: [] }]
      : [{ id: "firefox-default", tabIds: [] }, { id: "firefox-private", tabIds: [] }, { id: "firefox-container-2", tabIds: [] }];
  };
  if (browser === "chrome") {
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: "unresolved", tabId: 9, url: URL,
      type: "main_frame", requestHeaders: [{ name: "Accept", value: "*/*" }] });
    await bg.settle();
    bg.chrome.api.tabs.onRemoved.fire(9);
  }
  const payload = await download(bg, item(66, browser === "firefox" ? { incognito: true } : {}));
  assert.ok(!payload.cookie);
  assert.equal(queries.length, 0);
  assert.equal(enumerated, 0, "no source means no normal/private/container inference");
});
