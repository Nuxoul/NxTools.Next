"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const capturePolicy = require("../capture-policy.js");
const batchPolicy = require("../batch-handoff-policy.js");
const { backgroundSource, extractDeclarations } = require("./harness/background-source.cjs");
const source = backgroundSource();

function fixture(limit = 2) {
  let now = 10000;
  let serial = 0;
  const timers = new Map();
  const writes = [];
  let sendHandoff = async () => { throw new Error("unexpected external send"); };
  let stored = {};
  const cx = vm.createContext({
    URL, Map, Set, Object, Number, String, Boolean,
    Date: { now: () => now },
    NxToolsCapturePolicy: capturePolicy,
    NxToolsBatchHandoffPolicy: batchPolicy,
    setTimeout(fn, ms) { const id = ++serial; timers.set(id, { fn, ms }); return id; },
    clearTimeout(id) { timers.delete(id); },
    chrome: { storage: { local: {
      async get(defaults) { return { ...defaults, ...stored }; },
      async set(value) { writes.push(value); stored = { ...stored, ...value }; },
    } } },
    sendDownloadPayload: (payload) => sendHandoff(payload),
    nextRequestId: () => "12345678-1234-4abc-8def-1234567890ab",
    buildResourceFileName: (value) => value.filename || "fixture.bin",
    sendExtensionLog() {},
    logResourceCaptured() {}, loadDisabledSites() {}, updateMobileUserAgentRule() {},
  });
  vm.runInContext(`
    const resourcesByTab = new Map(), resourcesById = new Map();
    let lastActiveTabId = 0, persistTimer = 0, linkReview = null;
    const mobileUserAgentTabs = new Set(), pageThumbnailByTab = new Map();
    const lastFrameScanByTab = new Map(), selectedMediaIndexByTab = new Map();
    const RESOURCE_LIMIT_PER_TAB = ${limit}, RESOURCE_TTL_MS = 1000;
    const STORAGE_RESOURCE_KEY = 'resources', STORAGE_LAST_TAB_KEY = 'lastTab';
    const STORAGE_MOBILE_UA_TABS_KEY = 'mobile', STORAGE_DISABLED_SITES_KEY = 'sites';
    const MAX_CAPTURED_URL_LENGTH = 16384, MAX_CAPTURED_TEXT_LENGTH = 1024;
    const MAX_CAPTURED_HEADER_NAME_LENGTH = 128, MAX_CAPTURED_HEADER_VALUE_LENGTH = 16384;
    const MAX_CAPTURED_HEADER_COUNT = 64;
    const USEFUL_HEADER_ALLOWLIST = new Set(['authorization','cookie','referer','range']);
    const GOOGLEVIDEO_MIN_REMAINING_MS = 100, GOOGLEVIDEO_MAX_FUTURE_MS = 100000;
    function parseUrlOrNull(value) { try { return new URL(value); } catch { return null; } }
    ${extractDeclarations(source, [
      "normalizeResourceUrlKey", "isHttpUrl", "isGoogleVideoUrl", "googleVideoInvalidReason", "isUsableGoogleVideoUrl",
      "boundedCapturedText", "boundedCapturedUrl", "boundedCapturedNumber", "sanitizeCapturedHeaderObject",
      "bucketForTab", "removeCachedResource", "indexCachedResource", "enforceResourceCapacity", "cacheResource",
      "pruneResources", "serializeResources", "schedulePersistResources", "persistResourcesNow", "loadPersistentResources",
      "clearTabResources", "sendResource"
    ])}
    globalThis.api = { cacheResource, pruneResources, serializeResources, loadPersistentResources,
      clearTabResources, persistResourcesNow, sendResource, resourcesByTab, resourcesById };
  `, cx);
  return { api: cx.api, writes, timers, time(value) { now = value; },
    send(callback) { sendHandoff = callback; },
    stored(value) { stored = value; }, data() { return stored; } };
}

function resource(id, url, capturedAt = 10000, extra = {}) {
  return { id, tabId: 7, url, capturedAt, ...extra };
}
const sharedUrl = "https://synthetic.invalid/shared.bin";
const alias = `7:${sharedUrl}`;

function assertRemoved(f, removed) {
  assert(!f.api.resourcesById.has(removed.id), "canonical global key removed");
  for (const value of f.api.resourcesById.values()) {
    assert.notEqual(value, removed, "no global alias retains evicted resource");
  }
  for (const bucket of f.api.resourcesByTab.values()) {
    for (const value of bucket.values()) {
      assert.notEqual(value, removed, "no tab alias retains evicted resource");
    }
  }
}

test("capacity counts resources once and removes every alias of an evicted resource", () => {
  const f = fixture();
  f.api.cacheResource(resource("first", sharedUrl));
  const first = f.api.resourcesById.get("first");
  f.api.cacheResource(resource("second", "https://synthetic.invalid/second.bin", 10001));
  assert.equal(f.api.resourcesById.get("first"), first, "aliases do not consume resource capacity");
  f.api.cacheResource(resource("third", "https://synthetic.invalid/third.bin", 10002));
  assertRemoved(f, first);
  assert(!f.api.resourcesById.has(alias), "normalized alias removed with old resource");
  assert.equal(new Set(f.api.resourcesByTab.get(7).values()).size, 2);
});

test("evicting one of two same-URL IDs does not remove the surviving alias target", () => {
  const f = fixture();
  f.api.cacheResource(resource("first", sharedUrl));
  const first = f.api.resourcesById.get("first");
  f.api.cacheResource(resource("second", sharedUrl, 10001));
  const second = f.api.resourcesById.get("second");
  assert.equal(f.api.resourcesById.get(alias), second);
  f.api.cacheResource(resource("third", "https://synthetic.invalid/third.bin", 10002));
  assertRemoved(f, first);
  assert.equal(f.api.resourcesById.get(alias), second, "do not delete alias reassigned to a survivor");
  assert.equal(f.api.resourcesByTab.get(7).get(alias), second);
});

test("capacity eviction leaves no orphan normalized URL lookup", () => {
  const f = fixture();
  f.api.cacheResource(resource("first", sharedUrl));
  const first = f.api.resourcesById.get("first");
  f.api.cacheResource(resource("second", "https://synthetic.invalid/second.bin", 10001));
  f.api.cacheResource(resource("third", "https://synthetic.invalid/third.bin", 10002));
  assert(!f.api.resourcesById.has(alias), "evicted canonical resource must not remain through URL alias");
  assertRemoved(f, first);
});

test("same-URL response enrichment preserves each stable ID and pending handoff", () => {
  const f = fixture(8);
  const handoffId = "12345678-1234-4abc-8def-1234567890ab";
  f.api.cacheResource(resource("first", sharedUrl, 10000, { pendingBatchHandoffId: handoffId }));
  const first = f.api.resourcesById.get("first");
  f.api.cacheResource(resource("second", sharedUrl, 10001, {
    metadataSource: "response", mime: "application/zip", size: 4096,
  }));
  assert.equal(f.api.resourcesById.get("first"), first, "enrichment updates existing object in place");
  assert.equal(first.id, "first", "metadata enrichment is not an identity replacement");
  assert.equal(first.pendingBatchHandoffId, handoffId);
  assert.equal(first.mime, "application/zip");
  assert.equal(first.size, 4096);
});

test("TTL removes canonical and alias references but retains the exact cutoff", () => {
  const f = fixture(8);
  f.api.cacheResource(resource("old", sharedUrl));
  const old = f.api.resourcesById.get("old");
  f.time(11000);
  f.api.pruneResources();
  assert.equal(f.api.resourcesById.get("old"), old);
  f.time(11001);
  f.api.pruneResources();
  assertRemoved(f, old);
  assert.equal(f.api.resourcesById.size, 0);
  assert.equal(f.api.resourcesByTab.size, 0);
});

test("tab clearing removes all aliases without touching another tab or observed object", () => {
  const f = fixture(8);
  f.api.cacheResource(resource("first", sharedUrl));
  const observed = f.api.resourcesById.get("first");
  f.api.cacheResource(resource("second", sharedUrl, 10001));
  f.api.cacheResource(resource("other-tab", sharedUrl, 10002, { tabId: 8 }));
  const other = f.api.resourcesById.get("other-tab");
  f.api.clearTabResources(7);
  assertRemoved(f, observed);
  assert(!f.api.resourcesByTab.has(7));
  assert.equal(f.api.resourcesById.get("8:" + sharedUrl), other);
  assert.equal(observed.id, "first", "removal never mutates an in-flight object");
});

test("restore recreates URL aliases, enforces unique capacity and rewrites private context", async () => {
  const f = fixture();
  const handoffId = "12345678-1234-4abc-8def-1234567890ab";
  f.stored({ resources: { 7: [
    resource("old", "https://synthetic.invalid/old.bin", 9000),
    resource("first", sharedUrl, 10000, { pendingBatchHandoffId: handoffId,
      requestHeaders: { authorization: "synthetic secret" }, referer: "https://synthetic.invalid/private",
      pageUrl: "https://synthetic.invalid/private", pageTitle: "private", pageKey: "private" }),
    resource("second", sharedUrl, 10001),
  ] } });
  await f.api.loadPersistentResources();
  assert.equal(new Set(f.api.resourcesByTab.get(7).values()).size, 2);
  assert(!f.api.resourcesById.has("old"));
  assert.equal(f.api.resourcesById.get(alias), f.api.resourcesById.get("second"));
  const persistent = f.data().resources["7"];
  assert.equal(persistent.length, 2);
  assert.equal(persistent.find((value) => value.id === "first").pendingBatchHandoffId, handoffId);
  for (const value of persistent) {
    for (const field of ["requestHeaders", "referer", "pageUrl", "pageTitle", "pageKey"]) {
      assert(!Object.hasOwn(value, field), `${field} is never persisted`);
    }
  }
  const reloaded = fixture();
  reloaded.stored(f.data());
  await reloaded.api.loadPersistentResources();
  assert.equal(reloaded.api.resourcesById.get(alias).id, "second");
  assert.equal(reloaded.api.resourcesById.get("first").pendingBatchHandoffId, handoffId);
});

test("recapture keeps the live object and late acceptance cannot clear a newer handoff", async () => {
  const f = fixture(8);
  let accept;
  const response = new Promise((resolve) => { accept = resolve; });
  let payload;
  let signalSent;
  const sent = new Promise((resolve) => { signalSent = resolve; });
  f.send((value) => { payload = value; signalSent(); return response; });
  f.api.cacheResource(resource("first", sharedUrl));
  const observed = f.api.resourcesById.get("first");
  const pending = f.api.sendResource("first");
  await sent;
  assert(payload, "real sendResource reached controlled outbound seam");
  assert.equal(f.data().resources["7"][0].pendingBatchHandoffId, payload.handoffId, "pending identity persisted before send");
  const newer = "87654321-4321-4abc-8def-1234567890ab";
  f.api.cacheResource(resource("first", sharedUrl, 10001));
  assert.equal(f.api.resourcesById.get("first"), observed);
  assert.equal(observed.pendingBatchHandoffId, payload.handoffId);
  observed.pendingBatchHandoffId = newer;
  accept({ ok: true, accepted: true, handoffId: payload.handoffId, taskId: "persisted-task" });
  await pending;
  assert.equal(observed.pendingBatchHandoffId, newer);
  assert.equal(observed.sentToDesktopAt, undefined);
  f.send(async (value) => ({ ok: true, accepted: true, handoffId: value.handoffId, taskId: "persisted-task" }));
  await f.api.sendResource("first");
  assert.equal(observed.pendingBatchHandoffId, undefined);
  assert(observed.sentToDesktopAt > 0);
});

test("URL-key capture remains distinct from the object previously reached by its alias", () => {
  const f = fixture(8);
  f.api.cacheResource(resource("first", sharedUrl));
  const first = f.api.resourcesById.get("first");
  f.api.cacheResource(resource(alias, sharedUrl, 10001));
  assert.equal(first.id, "first");
  assert.notEqual(f.api.resourcesById.get(alias), first);
});

test("URL aliases never overwrite another resource's real canonical ID", () => {
  const f = fixture(8);
  f.api.cacheResource(resource("first", sharedUrl));
  f.api.cacheResource(resource(alias, sharedUrl, 10001));
  const canonical = f.api.resourcesById.get(alias);
  f.api.cacheResource(resource("first", sharedUrl, 10002));
  assert.equal(f.api.resourcesById.get(alias), canonical, "real ID wins over URL alias replacement");
  assert.equal(f.api.resourcesByTab.get(7).get(alias), canonical);
  assert.equal(f.api.resourcesById.get("first").id, "first");
});

test("production-size capacity retains 120 distinct resources despite URL aliases", () => {
  const f = fixture(120);
  for (let index = 0; index <= 120; index++) {
    f.api.cacheResource(resource(`resource-${index}`, `https://synthetic.invalid/${index}.bin`, 10000 + index));
  }
  assert(!f.api.resourcesById.has("resource-0"));
  assert(!f.api.resourcesById.has("7:https://synthetic.invalid/0.bin"));
  assert.equal(new Set(f.api.resourcesByTab.get(7).values()).size, 120);
  assert.equal(f.api.serializeResources()["7"].length, 120);
});

test("recapture changing URL removes the old alias while preserving the live object", () => {
  const f = fixture(8);
  f.api.cacheResource(resource("first", sharedUrl));
  const first = f.api.resourcesById.get("first");
  const changedUrl = "https://synthetic.invalid/changed.bin";
  f.api.cacheResource(resource("first", changedUrl, 10001));
  assert.equal(f.api.resourcesById.get("first"), first);
  assert(!f.api.resourcesById.has(alias));
  assert(!f.api.resourcesByTab.get(7).has(alias));
  assert.equal(f.api.resourcesById.get(`7:${changedUrl}`), first);
});

test("same-URL enrichment retains source priority and never transfers handoff or acceptance", () => {
  const f = fixture(8);
  const firstHandoff = "12345678-1234-4abc-8def-1234567890ab";
  const secondHandoff = "87654321-4321-4abc-8def-1234567890ab";
  f.api.cacheResource(resource("first", sharedUrl, 10000, {
    filename: "release.zip", filenameSource: "content-disposition", metadataSource: "response",
    mime: "application/zip", size: 8192, headerSource: "browser",
    requestHeaders: { authorization: "synthetic browser context" }, pendingBatchHandoffId: firstHandoff,
  }));
  const first = f.api.resourcesById.get("first");
  f.api.cacheResource(resource("second", sharedUrl, 10001, {
    filename: "resource.bin", filenameSource: "page", metadataSource: "page", mime: "video/mp4",
    size: 16384, headerSource: "page", pendingBatchHandoffId: secondHandoff, sentToDesktopAt: 999,
  }));
  assert.equal(first.id, "first");
  assert.equal(first.capturedAt, 10000);
  assert.equal(first.filename, "release.zip");
  assert.equal(first.mime, "application/zip");
  assert.equal(first.size, 8192);
  assert.equal(first.requestHeaders.authorization, "synthetic browser context");
  assert.equal(first.pendingBatchHandoffId, firstHandoff);
  assert.equal(first.sentToDesktopAt, undefined);
});

test("acceptance after capacity eviction settles the observed handoff without resurrecting cache", async () => {
  const f = fixture(1);
  let accept, signalSent;
  const response = new Promise((resolve) => { accept = resolve; });
  const sent = new Promise((resolve) => { signalSent = resolve; });
  f.send(() => { signalSent(); return response; });
  f.api.cacheResource(resource("first", sharedUrl));
  const observed = f.api.resourcesById.get("first");
  const pending = f.api.sendResource("first");
  await sent;
  f.api.cacheResource(resource("second", "https://synthetic.invalid/second.bin", 10001));
  assertRemoved(f, observed);
  accept({ ok: true, accepted: true, handoffId: observed.pendingBatchHandoffId, taskId: "persisted-task" });
  await pending;
  assertRemoved(f, observed);
  assert.equal(observed.pendingBatchHandoffId, undefined);
  assert(observed.sentToDesktopAt > 0);
});

test("expired signed media URL is removed from both lookup indexes", () => {
  const f = fixture(8);
  const url = "https://media.googlevideo.com/video?expire=11";
  f.api.cacheResource(resource("signed", url));
  const signed = f.api.resourcesById.get("signed");
  assert(signed);
  f.time(10901);
  f.api.pruneResources();
  assertRemoved(f, signed);
  assert.equal(f.api.resourcesById.size, 0);
});

test("restore repeated canonical ID never leaves the earlier URL alias alive", async () => {
  const f = fixture(8);
  const changedUrl = "https://synthetic.invalid/changed.bin";
  f.stored({ resources: { 7: [
    resource("first", sharedUrl), resource("first", changedUrl, 10001),
  ] } });
  await f.api.loadPersistentResources();
  assert(!f.api.resourcesById.has(alias));
  assert.equal(f.api.resourcesById.get(`7:${changedUrl}`), f.api.resourcesById.get("first"));
  assert.equal(new Set(f.api.resourcesByTab.get(7).values()).size, 1);
});
