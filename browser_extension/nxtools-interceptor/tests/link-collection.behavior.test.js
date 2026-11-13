"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");
const tab = { id: 7, active: true, url: "https://links.example.test/list", title: "Downloads", cookieStoreId: "source-store" };
function fixture(links) {
  const bg = loadBackground({ chrome: { tabs: [tab] }, desktop: { respond(message) {
    if (message.type === "create_tasks") return { ok: true, data: { queued: true, items: message.payload.items.map(item => ({
      handoffId: item.handoffId, accepted: true, taskId: item.handoffId })) } };
    return { ok: true, data: { tasks: [] } };
  } } });
  const execute = bg.chrome.api.scripting.executeScript;
  bg.chrome.api.scripting.executeScript = (details, callback) => {
    if (!details.func) return execute(details, callback);
    callback([{ frameId: details.target.frameIds[0], result: { pageUrl: tab.url, links } }]);
  };
  return bg;
}

test("explicit page link extraction is bounded review-only and uses the original batch identities", async () => {
  const links = ["https://files.example.test/a.zip", "https://files.example.test/a.zip", "javascript:alert(1)",
    "mailto:person@example.test", ...Array.from({ length: 130 }, (_, index) => `https://files.example.test/${index}.pdf`)];
  const bg = fixture(links.map(url => ({ url })));
  await bg.settle();
  let cookieReads = 0;
  bg.chrome.api.cookies.getAll = async () => { cookieReads += 1; return []; };
  const result = await bg.sendRuntimeMessage({ type: "popup_extract_links", mode: "page" }, bg.senders.popup());
  assert.equal(result?.ok, true);
  assert.equal(result.linkReview.state, "ready");
  assert.equal(result.linkReview.count, 120);
  assert.equal(bg.desktop.requestsOfType("create_tasks").length, 0);
  assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
  assert.equal(cookieReads, 0, "collection reads no credentials");
  assert.equal(bg.chrome.nativePorts.length, 0, "collection does not demand a native host");
  const state = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  assert.equal(state.currentResources.length, 120);
  assert.ok(state.currentResources.every(resource => /^https?:/.test(resource.url)));
  const resourceIds = state.currentResources.slice(0, 2).map(resource => resource.id);
  const sent = await bg.sendRuntimeMessage({ type: "popup_send_resources", resourceIds }, bg.senders.popup());
  assert.equal(sent.acceptedResourceIds.length, 2);
  assert.equal(bg.desktop.requestsOfType("create_tasks").length, 1);
  const items = bg.desktop.requestsOfType("create_tasks")[0].message.payload.items;
  assert.ok(items.every(item => item.openDialog === false && item.startImmediately === false));
});

test("selected-link context menu keeps its originating frame and opens review without sending tasks", async () => {
  const bg = fixture([{ url: "https://files.example.test/selected.zip" }]);
  await bg.settle();
  bg.chrome.api.runtime.onInstalled.fire();
  assert.ok(bg.chrome.callsNamed("contextMenus.create").some(call => call.args[0].id === "nxtools-download-selection"));
  let injection;
  const execute = bg.chrome.api.scripting.executeScript;
  bg.chrome.api.scripting.executeScript = (details, callback) => {
    if (details.func) injection = details;
    return execute(details, callback);
  };
  bg.chrome.api.contextMenus.onClicked.fire({ menuItemId: "nxtools-download-selection", frameId: 3,
    selectionText: "https://files.example.test/selected.zip" }, tab);
  await bg.settle();
  assert.deepEqual(JSON.parse(JSON.stringify(injection.target)), { tabId: 7, frameIds: [3] });
  assert.equal(injection.args[0], "selection");
  assert.equal(bg.chrome.callsNamed("action.openPopup").length, 1);
  assert.equal(bg.desktop.requestsOfType("create_tasks").length, 0);
});

test("navigation during link extraction retires the old page instead of publishing its links", async () => {
  const bg = fixture([]);
  await bg.settle();
  let release;
  const execute = bg.chrome.api.scripting.executeScript;
  bg.chrome.api.scripting.executeScript = (details, callback) => {
    if (!details.func) return execute(details, callback);
    release = () => callback([{ result: { pageUrl: tab.url, links: [{ url: "https://files.example.test/old.zip" }] } }]);
  };
  const pending = bg.sendRuntimeMessage({ type: "popup_extract_links", mode: "page" }, bg.senders.popup());
  await bg.waitFor(() => Boolean(release));
  bg.chrome.tabs.set(7, { ...tab, url: "https://links.example.test/new" });
  bg.chrome.api.webNavigation.onCommitted.fire({ tabId: 7, frameId: 0, url: "https://links.example.test/new" });
  release();
  const result = await pending;
  assert.equal(result?.ok, false);
  const state = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  assert.equal(state.currentResources.length, 0);
});

test("content scripts cannot trigger privileged page extraction", async () => {
  const bg = fixture([]);
  const reply = await bg.sendRuntimeMessage({ type: "popup_extract_links", mode: "page" }, bg.senders.content(tab));
  assert.equal(reply.ok, false);
});
