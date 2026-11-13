"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

function createNode(id = "") {
  const attrs = new Map();
  const listeners = new Map();
  const children = [];
  return {
    id,
    value: "",
    checked: false,
    disabled: false,
    hidden: false,
    textContent: "",
    title: "",
    dataset: {},
    className: "",
    tabIndex: 0,
    children,
    classList: { toggle() {}, add() {}, remove() {}, contains() { return false; } },
    setAttribute(key, value) { attrs.set(key, String(value)); },
    getAttribute(key) { return attrs.get(key) ?? null; },
    removeAttribute(key) { attrs.delete(key); },
    hasAttribute(key) { return attrs.has(key); },
    addEventListener(key, listener) { const list = listeners.get(key) || []; list.push(listener); listeners.set(key, list); },
    async fire(key, event = {}) { for (const listener of listeners.get(key) || []) await listener(event); },
    append(...values) { children.push(...values); },
    replaceChildren(...values) { children.splice(0, children.length, ...values); },
    querySelectorAll() { return []; },
    querySelector() { return null; },
    contains() { return false; },
    focus() {},
    remove() {},
    load() {},
    scrollIntoView() {}
  };
}

function fixture() {
  const nodes = new Map();
  const node = id => {
    if (!nodes.has(id)) nodes.set(id, createNode(id));
    return nodes.get(id);
  };
  const tabs = ["tasks", "resources", "advanced", "settings"].map(view => {
    const item = node(`${view}Tab`);
    item.dataset.view = view;
    return item;
  });
  const views = ["tasks", "resources", "advanced", "settings"].map(view => node(`${view}View`));
  const batchToolbar = node("batchToolbar");
  const statusFooter = node("statusFooter");
  const messages = [];
  const storageWrites = [];
  let storageGet = async () => ({});
  let runtimeHandler = async message => ({ ok: true, message: message.type });
  const document = {
    activeElement: null,
    getElementById: node,
    createElement: tag => { const item = createNode(tag); item.tagName = tag.toUpperCase(); return item; },
    createElementNS: (_namespace, tag) => { const item = createNode(tag); item.tagName = tag.toUpperCase(); return item; },
    createTextNode: text => ({ textContent: String(text) }),
    querySelector(selector) {
      if (selector === ".batch-toolbar") return batchToolbar;
      if (selector === ".status-footer") return statusFooter;
      return null;
    },
    querySelectorAll(selector) {
      if (selector === ".tab") return tabs;
      if (selector === ".view") return views;
      return [];
    },
    addEventListener() {}
  };
  const context = vm.createContext({
    console,
    URL,
    structuredClone,
    document,
    CSS: { escape: value => String(value) },
    IntersectionObserver: undefined,
    setInterval: () => 1,
    clearInterval() {},
    setTimeout: (...args) => { const timer = setTimeout(...args); timer.unref?.(); return timer; },
    clearTimeout,
    window: { close() {} },
    chrome: {
      storage: { local: {
        get: key => storageGet(key),
        set: async values => { storageWrites.push(structuredClone(values)); }
      } },
      runtime: { sendMessage: message => { messages.push(structuredClone(message)); return runtimeHandler(message); } }
    }
  });
  for (const file of ["format.js", "dom.js", "render.js", "actions.js"]) {
    vm.runInContext(fs.readFileSync(path.join(__dirname, "../popup", file), "utf8"), context, { filename: file });
  }
  const run = source => vm.runInContext(source, context);
  run("renderResourceLists = () => {}; renderTasks = () => {}; renderAdvanced = () => {}; renderCaptureSummary = () => {}; renderSiteToggle = () => {}; renderHandoff = () => {}; refresh = async () => currentState;");
  return {
    node,
    messages,
    storageWrites,
    run,
    context,
    setStorageGet(handler) { storageGet = handler; },
    setRuntimeHandler(handler) { runtimeHandler = handler; }
  };
}

function baseState() {
  return {
    settings: { interceptDownloads: true },
    capturePaused: false,
    capturePauseAvailable: true,
    status: { ok: true, code: "connected", message: "已连接", guidance: "桌面任务与发送功能可用。" },
    activeTab: { id: 7, url: "https://page.example/course", domain: "page.example" },
    siteControl: { supported: true, enabled: true, host: "page.example" },
    currentResources: [],
    otherResources: [],
    tasks: []
  };
}

test("home and Settings pause controls share one request without changing the master setting", async () => {
  const f = fixture();
  f.context.nextState = baseState();
  f.run("currentState = nextState; renderCapturePause(currentState)");
  f.setRuntimeHandler(async message => ({ ok: true, capturePaused: message.paused }));
  await f.run("toggleCapturePause()");
  assert.deepEqual(f.messages, [{ type: "popup_set_capture_paused", paused: true }]);
  assert.equal(f.run("currentState.settings.interceptDownloads"), true);
  assert.equal(f.node("capturePauseButton").getAttribute("aria-pressed"), "true");
  assert.equal(f.node("homeCapturePauseButton").getAttribute("aria-pressed"), "true");
  assert.equal(f.node("homeCaptureStatus").textContent, "本次浏览器会话已暂停");
});

test("async view restore never overrides a newer user selection or link review", async () => {
  const remembered = fixture();
  remembered.setStorageGet(async () => ({ nxtools_popup_view: "advanced" }));
  assert.equal(await remembered.run("restoreViewPreference()"), true);
  assert.equal(remembered.run("currentView"), "advanced");

  const f = fixture();
  let release;
  f.setStorageGet(() => new Promise(resolve => { release = resolve; }));
  const restoring = f.run("restoreViewPreference()");
  f.run("setView('settings', { trackIntent: true, persist: true })");
  release({ nxtools_popup_view: "advanced" });
  assert.equal(await restoring, false);
  assert.equal(f.run("currentView"), "settings");
  assert.deepEqual(f.storageWrites.at(-1), { nxtools_popup_view: "settings" });

  let releaseReview;
  f.setStorageGet(() => new Promise(resolve => { releaseReview = resolve; }));
  const secondRestore = f.run("restoreViewPreference()");
  f.context.reviewState = { activeTab: { id: 7 }, linkReview: { id: "review-1", tabId: 7, state: "ready", count: 2, message: "待复核" } };
  f.run("handleLinkReviewNavigation(reviewState)");
  releaseReview({ nxtools_popup_view: "tasks" });
  assert.equal(await secondRestore, false);
  assert.equal(f.run("currentView"), "resources");
  f.run("setView('advanced', { trackIntent: true, persist: true }); handleLinkReviewNavigation(reviewState)");
  assert.equal(f.run("currentView"), "advanced", "the same review never reclaims the tab after a manual choice");
  f.run("reviewState.linkReview.id = 'review-2'; handleLinkReviewNavigation(reviewState)");
  assert.equal(f.run("currentView"), "resources", "a new review ID switches to Capture once");
});

test("resource search is local and preserves selected IDs hidden by filtering", () => {
  const f = fixture();
  f.context.resources = [
    { id: "a", filename: "课程视频.mp4", url: "https://media.example/a.mp4", domain: "media.example", description: { kind: "video" } },
    { id: "b", filename: "资料包.zip", url: "https://files.example/b.zip", domain: "files.example", description: { kind: "download" } }
  ];
  f.run("selectedResourceIds.add('a'); currentResourceSearch = 'files.example'; currentResourceFilter = 'all'");
  assert.deepEqual(f.run("filterResourceList(resources).map(resource => resource.id)"), ["b"]);
  assert.equal(f.run("selectedResourceIds.has('a')"), true);
  assert.equal(f.messages.length, 0, "search does not send a scan or background request");
});

test("connection presentation uses backend message and guidance with sensible legacy fallback", () => {
  const f = fixture();
  f.context.status = { ok: false, code: "host_not_registered", message: "浏览器接管未注册", guidance: "在下载设置中重新注册后重试。" };
  f.run("renderConnection(status)");
  assert.equal(f.node("statusText").textContent, "浏览器接管未注册");
  assert.equal(f.node("homeConnectionGuidance").textContent, "在下载设置中重新注册后重试。");
  assert.equal(f.node("statusText").getAttribute("title"), null);
  f.run("renderConnection({ ok: false })");
  assert.equal(f.node("statusText").textContent, "NxTools 未运行");
  assert.match(f.node("homeConnectionGuidance").textContent, /启动 NxTools/);
});

test("page extraction exposes review state and never submits a batch", async () => {
  const f = fixture();
  f.context.nextState = baseState();
  f.run("currentState = nextState; elements.extractLinksButton.disabled = false");
  f.setRuntimeHandler(async message => {
    assert.equal(message.type, "popup_extract_links"); assert.equal(message.mode, "page");
    return { ok: true, message: "已提取 3 个链接", linkReview: { id: "review-page", tabId: 7, state: "ready", count: 3, message: "请复核后发送" } };
  });
  await f.run("extractPageLinks()");
  assert.deepEqual(f.messages.map(message => message.type), ["popup_extract_links"]);
  assert.equal(f.run("localLinkReviewStatus.state"), "ready");
  assert.match(f.node("linkReviewStatus").textContent, /3 项/);
  assert.equal(f.messages.some(message => message.type === "popup_send_resources"), false);
});

test("media inspection is explicit, guarded, retryable and protected resources cannot remain selected", async () => {
  const f = fixture();
  const state = baseState();
  state.currentResources = [{ id: "media-1", filename: "stream.m3u8", url: "https://media.example/stream.m3u8", domain: "media.example", description: { kind: "streaming" } }];
  f.context.nextState = state;
  f.run("currentState = nextState; previewResourceId = 'media-1'; selectedResourceIds.add('media-1')");
  assert.equal(f.messages.length, 0, "rendering details never inspects automatically");

  let release;
  f.setRuntimeHandler(() => new Promise(resolve => { release = resolve; }));
  const first = f.run("inspectResourceFromButton(elements.extractLinksButton, 'media-1')");
  const duplicate = f.run("inspectResourceFromButton(elements.extractLinksButton, 'media-1')");
  assert.equal(f.messages.length, 1, "pending guard suppresses duplicate inspection");
  await duplicate;
  release({ ok: false, resourceId: "media-1", inspection: { state: "failed", kind: "unknown", requiresMerge: false, encrypted: false, live: false, message: "无法识别，可重试" }, message: "无法识别，可重试" });
  await first;
  assert.equal(f.run("previewResourceId"), "media-1");
  assert.equal(f.run("findResourceById('media-1').mediaInspection.state"), "failed");

  f.setRuntimeHandler(async () => { throw new Error("inspection unavailable"); });
  await f.run("inspectResourceFromButton(elements.extractLinksButton, 'media-1')");
  assert.equal(f.run("findResourceById('media-1').mediaInspection.state"), "failed");
  assert.equal(f.run("inspectingResourceIds.has('media-1')"), false);

  f.setRuntimeHandler(async () => ({ ok: false, resourceId: "media-1", inspection: { state: "protected", kind: "dash", requiresMerge: false, encrypted: true, live: false, message: "检测到受保护媒体" }, message: "检测到受保护媒体" }));
  await f.run("inspectResourceFromButton(elements.extractLinksButton, 'media-1')");
  assert.equal(f.run("isProtectedResource(findResourceById('media-1'))"), true);
  assert.equal(f.run("selectedResourceIds.has('media-1')"), false);
  assert.deepEqual(f.messages.map(message => message.type), ["popup_inspect_resource", "popup_inspect_resource", "popup_inspect_resource"]);
});
