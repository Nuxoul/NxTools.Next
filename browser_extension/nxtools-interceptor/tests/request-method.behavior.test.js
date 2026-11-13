"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");
const url = "https://exports.example.test/report.zip";
const tab = { id: 7, url: "https://exports.example.test/reports", active: true, cookieStoreId: "origin-store" };

function fixture(browser) {
  return loadBackground({ browser, chrome: { tabs: [tab], storage: { captureResources: false, openDialog: false } },
    desktop: { respond(message) {
      return message.type === "create_task"
        ? { ok: true, accepted: true, handoffId: message.payload.handoffId, taskId: "durable-task" }
        : { ok: true, data: { tasks: [] } };
    } } });
}
function request(bg, method, target = url, requestId = "export", source = {}) {
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId, url: target, method,
    type: "main_frame", tabId: tab.id, cookieStoreId: tab.cookieStoreId,
    requestHeaders: [{ name: "Accept", value: "application/zip" }], ...source });
}
async function intercept(bg, browser, overrides = {}) {
  let suggestions = 0;
  const item = { id: 91, url, finalUrl: url, filename: "report.zip", totalBytes: 2048,
    referrer: tab.url, cookieStoreId: tab.cookieStoreId, ...overrides };
  const event = bg.chrome.api.downloads.onDeterminingFilename || bg.chrome.api.downloads.onCreated;
  event.fire(item, () => { suggestions += 1; });
  await bg.settle();
  await bg.waitFor(() => browser === "chrome" ? suggestions === 1 : Boolean(bg.evaluate("lastHandoff")));
  return bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "settings" }, bg.senders.popup());
}

for (const browser of ["chrome", "firefox"]) {
  test(`${browser} observed POST export stays in browser before credentials or native demand`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    let cookieReads = 0;
    bg.chrome.api.cookies.getAll = async () => { cookieReads += 1; return []; };
    request(bg, "POST");
    const state = await intercept(bg, browser);
    assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
    assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
    assert.equal(cookieReads, 0);
    assert.equal(state.lastHandoff.state, "skipped");
    assert.equal(state.lastHandoff.reason, "unsupported_request_method");
    assert.equal(state.lastHandoff.owner, "browser");
  });
}

for (const browser of ["chrome", "firefox"]) {
  test(`${browser} final GET after POST redirect retains ordinary durable handoff`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    const initial = "https://exports.example.test/prepare";
    request(bg, "POST", initial);
    bg.chrome.api.webRequest.onBeforeRedirect.fire({ requestId: "export", url: initial, tabId: 7,
      method: "POST", statusCode: 303, responseHeaders: [{ name: "Location", value: url }] });
    request(bg, "GET");
    const state = await intercept(bg, browser, { url: initial });
    assert.equal(state.lastHandoff.state, "transferred");
    assert.equal(bg.desktop.requestsOfType("create_task").length, 1);
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1);
  });

  test(`${browser} redirected POST remains in browser and another store cannot exclude a GET`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    const initial = "https://exports.example.test/prepare";
    request(bg, "POST", initial);
    bg.chrome.api.webRequest.onBeforeRedirect.fire({ requestId: "export", url: initial, tabId: 7,
      method: "POST", statusCode: 307, responseHeaders: [{ name: "Location", value: url }] });
    request(bg, "POST");
    let state = await intercept(bg, browser, { url: initial });
    assert.equal(state.lastHandoff.reason, "unsupported_request_method");
    const isolated = fixture(browser);
    await isolated.settle();
    request(isolated, "GET", url, "get");
    isolated.clock.advance(1);
    request(isolated, "POST", url, "other-container", { tabId: 8, cookieStoreId: "other-store" });
    state = await intercept(isolated, browser, { id: 92 });
    assert.equal(state.lastHandoff.state, "transferred");
    assert.equal(isolated.desktop.requestsOfType("create_task").length, 1);
  });

  test(`${browser} a newer unrelated GET cannot mask same-source POST evidence`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    request(bg, "POST", url, "export-post");
    bg.clock.advance(1);
    request(bg, "GET", url, "preview-get");
    const state = await intercept(bg, browser);
    assert.equal(state.lastHandoff.reason, "unsupported_request_method");
    assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
  });
}
