"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");

function fixture(storage = {}) {
  return loadBackground({
    chrome: {
      storage,
      tabs: [{ id: 7, active: true, url: "https://example.test/watch", title: "fixture" }]
    },
    desktop: {
      respond(message) {
        return message.type === "get_snapshot" ? { ok: true, type: "snapshot", data: { tasks: [] } } : undefined;
      }
    }
  });
}

async function addResource(bg, url = "https://cdn.test/master.m3u8", frameId = 4) {
  await bg.sendRuntimeMessage({
    type: "content_media_resource",
    payload: {
      url,
      href: "https://example.test/watch",
      pageKey: "https://example.test/watch",
      mime: url.endsWith(".mpd") ? "application/dash+xml" : "application/vnd.apple.mpegurl",
      source: "content-media-source"
    }
  }, bg.senders.content({ id: 7, url: "https://example.test/watch" }, frameId));
  await bg.settle();
  return bg.evaluate("[...new Set(resourcesById.values())][0].id");
}

function callInspect(bg, resourceId) {
  return bg.evaluate(`inspectMediaResource(${JSON.stringify(resourceId)})`);
}

function installTabResponder(bg, handler) {
  const calls = [];
  bg.chrome.api.tabs.sendMessage = (tabId, message, optionsOrCallback, maybeCallback) => {
    const options = typeof optionsOrCallback === "function" ? undefined : optionsOrCallback;
    const callback = typeof optionsOrCallback === "function" ? optionsOrCallback : maybeCallback;
    calls.push({ tabId, message, options });
    Promise.resolve(handler({ tabId, message, options, calls })).then(callback);
  };
  return calls;
}

const readyHls = {
  ok: true,
  inspection: {
    state: "ready",
    kind: "hls",
    requiresMerge: false,
    encrypted: true,
    live: false,
    message: "untrusted content copy",
    body: "#EXTM3U",
    url: "https://secret.test/key"
  }
};

test("capture and Popup snapshots do not inspect or fetch media", async () => {
  const bg = fixture();
  await bg.settle();
  await addResource(bg);
  await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
  assert.equal(bg.chrome.tabMessages.filter(call => call.message.type === "content_inspect_media_resource").length, 0);
});

test("explicit inspection uses the original frame and the settings credential gate", async () => {
  for (const [storage, includeCredentials] of [
    [{ includeRequestHeaders: true, includeCookies: true }, true],
    [{ includeRequestHeaders: false, includeCookies: true }, false],
    [{ includeRequestHeaders: true, includeCookies: false }, false]
  ]) {
    const bg = fixture(storage);
    await bg.settle();
    const resourceId = await addResource(bg, "https://cdn.test/master.m3u8", 6);
    const calls = installTabResponder(bg, () => readyHls);
    const result = await callInspect(bg, resourceId);
    assert.equal(result.ok, true);
    assert.equal(calls.length, 1);
    assert.equal(calls[0].tabId, 7);
    assert.equal(calls[0].options.frameId, 6);
    assert.equal(calls[0].message.includeCredentials, includeCredentials);
    assert.equal(calls[0].message.url, "https://cdn.test/master.m3u8");
    assert.equal(result.inspection.message, "HLS 清单已检查");
    assert.equal("body" in result.inspection, false);
    assert.equal("url" in result.inspection, false);
  }
});

test("same-resource pending calls coalesce and expose only ephemeral inspecting state", async () => {
  const bg = fixture();
  await bg.settle();
  const resourceId = await addResource(bg);
  let release;
  const calls = installTabResponder(bg, () => new Promise(resolve => { release = resolve; }));
  const first = callInspect(bg, resourceId);
  const second = callInspect(bg, resourceId);
  await bg.waitFor(() => calls.length === 1);
  assert.equal(bg.evaluate(`resourcesById.get(${JSON.stringify(resourceId)}).mediaInspection.state`), "inspecting");
  release(readyHls);
  const [left, right] = await Promise.all([first, second]);
  assert.deepEqual(left, right);
  assert.equal(calls.length, 1);
  bg.clock.advance(600);
  await bg.settle();
  const stored = bg.chrome.storage.local.nxtools_resource_cache["7"][0];
  assert.equal("mediaInspection" in stored, false, "inspection is not persisted with resource cache");
});

test("failed inspection retries and untrusted states, reasons and copy are normalized", async () => {
  const bg = fixture();
  await bg.settle();
  const resourceId = await addResource(bg);
  let attempt = 0;
  const calls = installTabResponder(bg, () => ++attempt === 1
    ? { ok: false, inspection: { state: "failed", kind: "javascript", reason: "raw-url:https://secret", message: "secret" }, arbitrary: "secret" }
    : readyHls);
  const failed = await callInspect(bg, resourceId);
  assert.deepEqual(JSON.parse(JSON.stringify(failed.inspection)), {
    state: "failed", kind: "unknown", requiresMerge: false, encrypted: false, live: false,
    message: "页面返回的媒体信息无效", reason: "invalid_response"
  });
  const retried = await callInspect(bg, resourceId);
  assert.equal(retried.ok, true);
  assert.equal(retried.inspection.state, "ready");
  assert.equal(calls.length, 2);
});

test("navigation and replacement retire delayed inspection without publishing into a new resource", async () => {
  const bg = fixture();
  await bg.settle();
  const resourceId = await addResource(bg);
  let release;
  const calls = installTabResponder(bg, () => new Promise(resolve => { release = resolve; }));
  const pending = callInspect(bg, resourceId);
  await bg.waitFor(() => calls.length === 1);
  bg.chrome.api.webNavigation.onHistoryStateUpdated.fire({ tabId: 7, frameId: 0, url: "https://example.test/next" });
  await addResource(bg, "https://cdn.test/master.m3u8", 2);
  release(readyHls);
  const result = await pending;
  assert.equal(result.ok, false);
  assert.equal(result.inspection.reason, "page_changed");
  assert.equal(bg.evaluate(`resourcesById.get(${JSON.stringify(resourceId)}).frameId`), 2);
  assert.equal(bg.evaluate(`resourcesById.get(${JSON.stringify(resourceId)}).mediaInspection`), undefined);
});

test("inspection diagnostics use one correlation id and fixed terminal metadata", async () => {
  const bg = fixture();
  await bg.settle();
  const resourceId = await addResource(bg);
  installTabResponder(bg, () => readyHls);
  bg.evaluate("globalThis.__inspectionLogs = []; sendExtensionLog = (...args) => __inspectionLogs.push(args)");
  await callInspect(bg, resourceId);
  const logs = bg.evaluate("__inspectionLogs");
  assert.deepEqual(Array.from(logs, item => item[1]), ["media_inspection_started", "media_inspection_completed"]);
  assert.equal(logs[0][5], logs[1][5]);
  assert.equal(logs[0][3].reason, "started");
  assert.equal(logs[1][3].reason, "ready");
  assert.ok(logs[1][3].elapsedMs >= 0);
});
