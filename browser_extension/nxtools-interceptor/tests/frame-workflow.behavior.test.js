"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");
const tab = { id: 7, active: true, url: "https://example.test/watch" };
const frame = { frameId: 4, url: "https://embed.test/player", documentId: "original-document" };
function fixture() {
  const bg = loadBackground({ chrome: { tabs: [tab] }, desktop: { respond: () => ({ ok: true, data: { tasks: [] } }) } });
  let current = { ...frame };
  bg.chrome.api.webNavigation.getFrame = (_details, callback) => callback(current);
  return { bg, replaceFrame() { current = { ...frame, url: "https://embed.test/next", documentId: "new-document" }; } };
}
async function resource(bg) {
  bg.chrome.api.webRequest.onResponseStarted.fire({ requestId: "network-media", tabId: 7, frameId: 4,
    documentId: frame.documentId, documentUrl: frame.url, initiator: "https://embed.test", type: "media",
    url: "https://cdn.test/master.mpd", statusCode: 200, responseHeaders: [{ name: "Content-Type", value: "application/dash+xml" }] });
  await bg.settle();
  const state = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  return state.currentResources[0].id;
}
test("network media inspection stays in its source document and pending checks block sends", async () => {
  const { bg, replaceFrame } = fixture();
  await bg.settle();
  const resourceId = await resource(bg);
  let release, sentFrame;
  bg.chrome.api.tabs.sendMessage = (_tabId, message, options, callback) => {
    sentFrame = options.frameId; release = callback;
  };
  const pending = bg.sendRuntimeMessage({ type: "popup_inspect_resource", resourceId }, bg.senders.popup());
  await bg.waitFor(() => Boolean(release));
  assert.equal(sentFrame, 4);
  for (const message of [{ type: "popup_send_resource", resourceId }, { type: "popup_send_resources", resourceIds: [resourceId] }]) {
    const reply = await bg.sendRuntimeMessage(message, bg.senders.popup());
    assert.equal(reply.ok, false);
    assert.match(reply.message, /检查|识别/);
  }
  replaceFrame();
  release({ ok: true, inspection: { state: "protected", kind: "dash" } });
  const reply = await pending;
  assert.equal(reply.ok, false);
  assert.equal(reply.inspection.reason, "page_changed");
  assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
  assert.equal(bg.desktop.requestsOfType("create_tasks").length, 0);
});
test("a network resource cannot be inspected from a replacement subframe", async () => {
  const { bg, replaceFrame } = fixture();
  await bg.settle();
  const resourceId = await resource(bg);
  replaceFrame();
  const reply = await bg.sendRuntimeMessage({ type: "popup_inspect_resource", resourceId }, bg.senders.popup());
  assert.equal(reply.ok, false);
  assert.equal(reply.inspection.reason, "page_changed");
  assert.equal(bg.chrome.tabMessages.filter(call => call.message.type === "content_inspect_media_resource").length, 0);
});
test("selected links from a navigated subframe are discarded before review publication", async () => {
  const { bg, replaceFrame } = fixture();
  await bg.settle();
  let release;
  bg.chrome.api.scripting.executeScript = (details, callback) => {
    release = () => callback([{ frameId: 4, documentId: frame.documentId,
      result: { pageUrl: frame.url, links: [{ url: "https://cdn.test/old.zip" }] } }]);
  };
  const pending = bg.evaluate(`collectLinksForReview(${JSON.stringify(tab)}, { mode: 'selection', frameId: 4 })`);
  await bg.waitFor(() => Boolean(release));
  replaceFrame(); release();
  const reply = await pending;
  assert.equal(reply.ok, false);
  const state = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  assert.equal(state.currentResources.length, 0);
});
