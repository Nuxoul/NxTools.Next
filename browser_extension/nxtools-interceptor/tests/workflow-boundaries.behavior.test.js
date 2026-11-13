"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");
const tab = { id: 7, active: true, url: "https://example.test/watch", title: "fixture" };
function fixture() {
  return loadBackground({ chrome: { tabs: [tab] }, desktop: { respond: () => ({ ok: true, data: { tasks: [] } }) } });
}
function logs(bg, prefix) {
  return bg.chrome.nativePorts.flatMap(port => port.sent).filter(message =>
    message.type === "extension_log" && message.payload.event.startsWith(prefix));
}
function assertLifecycle(frames, events, reasons) {
  assert.deepEqual(frames.map(frame => frame.payload.event), events);
  assert.deepEqual(frames.map(frame => frame.payload.reason), reasons);
  assert.match(frames[0].payload.correlationId, /^[0-9a-f-]{36}$/);
  for (const frame of frames) {
    assert.equal(frame.payload.correlationId, frames[0].payload.correlationId);
    assert.notEqual(frame.requestId, frame.payload.correlationId);
    assert.ok(Number.isFinite(frame.payload.elapsedMs) && frame.payload.elapsedMs >= 0);
    assert.ok(!JSON.stringify(frame).includes("secret"));
  }
}

test("explicit connection checks emit correlated native frames while passive refresh stays quiet", async () => {
  const bg = fixture();
  await bg.settle();
  const result = await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  assert.equal(result.ok, true);
  const frames = logs(bg, "connection_check_");
  assertLifecycle(frames, ["connection_check_started", "connection_check_completed"], ["started", "ready"]);
  assert.equal(bg.desktop.requestsOfType("get_snapshot")[0].message.requestId, frames[0].payload.correlationId);
  await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  assert.equal(logs(bg, "connection_check_").length, 2);
});

test("link collection transports only bounded lifecycle metadata", async () => {
  const bg = fixture();
  await bg.settle();
  await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  bg.chrome.api.scripting.executeScript = (details, callback) => callback([{ result: {
    pageUrl: tab.url, links: [{ url: "https://cdn.test/secret.zip" }]
  } }]);
  const result = await bg.sendRuntimeMessage({ type: "popup_extract_links", mode: "page" }, bg.senders.popup());
  assert.equal(result.ok, true);
  assertLifecycle(logs(bg, "link_collection_"), ["link_collection_started", "link_collection_completed"], ["started", "ready"]);
});

test("protected inspection stays ephemeral and blocks single and batch sends at the runtime boundary", async () => {
  const bg = fixture();
  await bg.settle();
  await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  await bg.sendRuntimeMessage({ type: "content_media_resource", payload: {
    url: "https://cdn.test/master.mpd", href: tab.url, pageKey: tab.url,
    mime: "application/dash+xml", source: "content-media-source"
  } }, bg.senders.content(tab, 4));
  let state = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  const resourceId = state.currentResources[0].id;
  bg.chrome.api.tabs.sendMessage = (_tabId, message, options, callback) => {
    assert.equal(message.type, "content_inspect_media_resource");
    assert.equal(options.frameId, 4);
    callback({ ok: true, inspection: { state: "protected", kind: "dash", encrypted: true,
      message: "secret", body: "secret", keyUrl: "https://secret.test/key" } });
  };
  const inspected = await bg.sendRuntimeMessage({ type: "popup_inspect_resource", resourceId }, bg.senders.popup());
  assert.equal(inspected.inspection.state, "protected");
  state = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  assert.equal(state.currentResources[0].mediaInspection.state, "protected");
  assert.ok(!JSON.stringify(state.currentResources[0].mediaInspection).includes("secret"));
  for (const message of [{ type: "popup_send_resource", resourceId }, { type: "popup_send_resources", resourceIds: [resourceId] }]) {
    const result = await bg.sendRuntimeMessage(message, bg.senders.popup());
    assert.equal(result.ok, false);
    assert.match(result.message, /受保护/);
  }
  assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
  assert.equal(bg.desktop.requestsOfType("create_tasks").length, 0);
  assertLifecycle(logs(bg, "media_inspection_"), ["media_inspection_started", "media_inspection_completed"], ["started", "protected"]);
  bg.clock.advance(600);
  await bg.settle();
  assert.ok(!JSON.stringify(bg.chrome.storage.local.nxtools_resource_cache).includes("mediaInspection"));
});
