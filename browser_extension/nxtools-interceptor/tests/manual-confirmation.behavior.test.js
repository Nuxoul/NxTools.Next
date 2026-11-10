"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");

const URL = "https://files.example.test/media.mp4";
const TAB = { id: 7, url: "https://files.example.test/page", active: true, cookieStoreId: "fixture-store" };
const PENDING = "确认窗口已打开，尚未创建任务";
function pending(message, overrides = {}) {
  return { ok: true, accepted: false, handoffId: message.payload.handoffId,
    data: { handoffState: "awaiting_confirmation", reason: "confirmation_pending", queued: false }, ...overrides };
}
async function fixture(browser, reply = pending) {
  const bg = loadBackground({ browser, chrome: { tabs: [TAB], storage: { openDialog: true } },
    desktop: { respond(message) {
      if (message.type === "create_task") return reply(message);
      return { ok: true, data: { tasks: [] } };
    } } });
  await bg.settle();
  const details = { requestId: "media-request", url: URL, tabId: TAB.id, cookieStoreId: TAB.cookieStoreId,
    type: "media", requestHeaders: [] };
  bg.chrome.api.webRequest.onBeforeSendHeaders.fire(details);
  bg.chrome.api.webRequest.onResponseStarted.fire({ ...details, statusCode: 200,
    responseHeaders: [{ name: "Content-Type", value: "video/mp4" }] });
  await bg.settle();
  return bg;
}
function assertPending(bg, result) {
  assert.equal(result?.ok, true, "valid confirmation admission is not a failed operation");
  assert.equal(result.accepted, false, "confirmation admission is not durable acceptance");
  assert.equal(result.taskId, undefined, "no task was created");
  assert.equal(result.data.handoffState, "awaiting_confirmation");
  assert.equal(result.data.reason, "confirmation_pending");
  assert.equal(result.message, PENDING);
  assert.equal(result.handoffId, bg.desktop.requestsOfType("create_task").at(-1).message.payload.handoffId);
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
  assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
}
const entries = [
  ["drop", { type: "drop_target_download", url: URL }],
  ["direct link", { type: "content_direct_download_request", url: URL }],
  ["active tab", { type: "popup_send_active_tab" }, true],
  ["resource", { type: "popup_send_resource", resourceId: `7:${URL}` }, true],
  ["latest resource", { type: "content_send_latest_resource" }],
  ["selected media", { type: "page_download_media", selectedResourceId: `7:${URL}` }],
  ["direct media", { type: "page_download_media", selectedResourceUrl: "https://files.example.test/other.mp4" }],
  ["Blob network source", { type: "content_blob_download_request", payload: { url: "blob:https://files.example.test/blob-id", sourceUrl: URL } }]
];
for (const browser of ["chrome", "firefox"]) {
  for (const [name, message, popup] of entries) test(`B1 ${browser} manual ${name} preserves confirmation admission without task acceptance`, async () => {
    const bg = await fixture(browser);
    const result = await bg.sendRuntimeMessage(message, popup ? bg.senders.popup() : bg.senders.content(TAB));
    assertPending(bg, result);
    const resource = bg.evaluate(`resourcesById.get(${JSON.stringify(`7:${URL}`)})`);
    assert.ok(!resource.sentToDesktopAt, "pending cannot mark the resource durably sent");
    if (["resource", "latest resource", "selected media", "Blob network source"].includes(name)) {
      const originalId = bg.desktop.requestsOfType("create_task").at(-1).message.payload.handoffId;
      assert.equal(resource.pendingBatchHandoffId, originalId);
      const retry = await bg.sendRuntimeMessage(message, popup ? bg.senders.popup() : bg.senders.content(TAB));
      assertPending(bg, retry);
      assert.equal(retry.handoffId, originalId, "pending retry keeps the existing resource handoff identity");
    }
  });

  test(`B1 ${browser} context menu confirmation is explanatory pending, not failure or task completion`, async () => {
    const bg = await fixture(browser);
    bg.chrome.api.contextMenus.onClicked.fire({ menuItemId: "nxtools-download-link", linkUrl: URL, pageUrl: TAB.url }, TAB);
    await bg.waitFor(() => bg.chrome.callsNamed("action.setBadgeText").some(call => call.args[0].text), "context menu feedback");
    assert.equal(bg.chrome.callsNamed("action.setBadgeText").at(-1).args[0].text, "…");
    const logs = bg.chrome.nativePorts.flatMap(port => port.sent).filter(message => message.type === "extension_log");
    assert.equal(logs.filter(message => message.payload.event === "context_menu_handoff_failed").length, 0);
    const terminal = logs.filter(message => message.payload.event === "handoff_state" && message.payload.handoffState === "awaiting_confirmation");
    assert.equal(terminal.length, 1);
    assert.equal(terminal[0].payload.reason, "confirmation_pending");
    assert.equal(typeof terminal[0].payload.elapsedMs, "number");
    assert.equal(terminal[0].payload.correlationId, bg.desktop.requestsOfType("create_task")[0].message.payload.handoffId);
    const state = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.equal(state.lastHandoff?.state, "awaiting_confirmation", "pending remains visible in the task workspace");
  });

  for (const [name, reply, reason] of [
    ["refusal", message => pending(message, { ok: false }), "desktop_refused"],
    ["missing identity", message => pending(message, { handoffId: undefined }), "identity_mismatch"],
    ["wrong identity", message => pending(message, { handoffId: "other" }), "identity_mismatch"],
    ["missing ok", message => pending(message, { ok: undefined }), "desktop_refused"],
    ["false acceptance", message => pending(message, { accepted: true }), "desktop_refused"],
    ["wrong reason", message => pending(message, { data: { handoffState: "awaiting_confirmation", reason: "desktop_refused" } }), "desktop_refused"],
    ["fabricated task", message => pending(message, { taskId: "not-persisted" }), "desktop_refused"]
  ]) test(`B1 ${browser} manual pending-shaped ${name} remains a failure`, async () => {
    const bg = await fixture(browser, reply);
    const result = await bg.sendRuntimeMessage({ type: "popup_send_active_tab" }, bg.senders.popup());
    assert.equal(result.ok, false);
    assert.notEqual(result.accepted, true);
    if (result.data) assert.equal(result.data.reason, reason);
    assert.notEqual(result.data?.handoffState, "awaiting_confirmation");
  });

  test(`B1 ${browser} manual durable receipt still preserves task identity`, async () => {
    const bg = await fixture(browser, message => ({ ok: true, accepted: true,
      handoffId: message.payload.handoffId, taskId: "persisted-task", data: { handoffState: "persisted", reason: "desktop_accepted" } }));
    const result = await bg.sendRuntimeMessage({ type: "drop_target_download", url: URL }, bg.senders.content(TAB));
    assert.equal(result.ok, true);
    assert.equal(result.accepted, true);
    assert.equal(result.taskId, "persisted-task");
    assert.equal(result.data.handoffState, "persisted");
  });
}

for (const browser of ["chrome", "firefox"]) for (const [name, overrides, valid] of [
  ["confirmation", {}, true],
  ["contradictory acceptance", { accepted: true }, false],
  ["wrong identity", { handoffId: "other" }, false]
]) test(`B1 ${browser} uploaded Blob ${name} follows the shared handoff outcome`, async () => {
  const bg = await fixture(browser);
  bg.chrome.setTabMessageHandler((_tabId, message) => message.type === "content_blob_info"
    ? { ok: true, size: 12, filename: "fixture.bin", mime: "application/octet-stream" }
    : { ok: true });
  bg.desktop.respondWith(message => {
    if (message.type === "prepare_blob_upload") return { ok: true, data: {
      uploadId: "123e4567-e89b-42d3-a456-426614174000",
      uploadUrl: "http://127.0.0.1:49152/nxtools/blob/123e4567-e89b-42d3-a456-426614174000/" + "a".repeat(64),
      maxBytes: 1024, chunkBytes: 1024
    } };
    if (message.type === "commit_blob_upload") return pending(message, overrides);
    return { ok: true, data: { tasks: [] } };
  });
  const result = await bg.sendRuntimeMessage({ type: "content_blob_download_request", payload: {
    url: "blob:https://files.example.test/standalone", filename: "fixture.bin", size: 12
  } }, bg.senders.content(TAB));
  if (valid) {
    assert.equal(result.ok, true);
    assert.equal(result.accepted, false);
    assert.equal(result.taskId, undefined);
    assert.equal(result.data.handoffState, "awaiting_confirmation");
    assert.equal(result.message, PENDING);
    assert.notEqual(result.fallbackToBrowser, true);
  } else {
    assert.equal(result.ok, false);
    assert.notEqual(result.accepted, true);
    assert.equal(result.fallbackToBrowser, true);
  }
  assert.equal(bg.desktop.requestsOfType("prepare_blob_upload").length, 1);
  assert.equal(bg.desktop.requestsOfType("commit_blob_upload").length, 1);
});

for (const browser of ["chrome", "firefox"]) for (const transport of ["timeout", "disconnect"])
test(`B1 ${browser} manual ${transport} remains a failure retaining browser ownership`, async () => {
  const bg = await fixture(browser, () => undefined);
  const pendingResult = bg.sendRuntimeMessage({ type: "drop_target_download", url: URL }, bg.senders.content(TAB));
  await bg.waitFor(() => bg.desktop.requestsOfType("create_task").length === 1, "manual native request");
  if (transport === "timeout") bg.clock.advance(15000);
  else bg.chrome.nativePorts.at(-1).disconnect();
  const result = await pendingResult;
  assert.equal(result.ok, false);
  assert.notEqual(result.accepted, true);
  assert.equal(result.taskId, undefined);
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
  assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
});
