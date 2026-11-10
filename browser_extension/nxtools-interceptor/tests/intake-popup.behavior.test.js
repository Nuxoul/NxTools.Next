"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

// Production script boundary with element doubles, not browser DOM acceptance.
function fixture() {
  const nodes = new Map();
  const node = id => {
    if (!nodes.has(id)) {
      const attrs = new Map(), listeners = new Map();
      nodes.set(id, { id, value: "", checked: false, disabled: false, textContent: "", dataset: {},
        classList: { toggle() {} }, contains: () => false,
        setAttribute: (key, value) => attrs.set(key, String(value)),
        getAttribute: key => attrs.get(key) ?? null,
        removeAttribute: key => attrs.delete(key), hasAttribute: key => attrs.has(key),
        addEventListener: (key, listener) => { const all = listeners.get(key) || []; all.push(listener); listeners.set(key, all); },
        fire: async key => { for (const listener of listeners.get(key) || []) await listener({}); }
      });
    }
    return nodes.get(id);
  };
  const state = { settings: { interceptDownloads: true, captureFileTypes: "zip", captureMinimumKiB: 128,
    captureExcludedSites: "", captureExcludedAddresses: "", dropTargetWidth: 188, dropTargetHeight: 86 },
    capturePaused: false, capturePauseAvailable: true, tasks: [], currentResources: [], otherResources: [] };
  const messages = [];
  let handler = async message => {
    if (message.type === "popup_save_settings") Object.assign(state.settings, message.settings);
    if (message.type === "popup_set_capture_paused") state.capturePaused = message.paused;
    return { ...structuredClone(state), ok: true };
  };
  const context = vm.createContext({ console, structuredClone,
    setTimeout: (...args) => { const timer = setTimeout(...args); timer.unref(); return timer; }, clearTimeout,
    document: { getElementById: node, querySelector: node, querySelectorAll: () => [], activeElement: null, addEventListener() {} },
    chrome: { runtime: { sendMessage: message => { messages.push(structuredClone(message)); return handler(message); } } }
  });
  for (const file of ["dom.js", "render.js", "actions.js"]) {
    vm.runInContext(fs.readFileSync(path.join(__dirname, "../popup", file), "utf8"), context, { filename: file });
  }
  vm.runInContext("renderConnection = renderHandoff = renderSiteToggle = renderCaptureSummary = renderTasks = renderResourceLists = renderAdvanced = () => {}; bindPopupEvents();", context);
  const run = source => vm.runInContext(source, context);
  const publish = () => { context.nextState = structuredClone(state); run("render(nextState)"); };
  publish();
  return { node, state, messages, run, publish, runtime: context.chrome.runtime, setHandler: value => { handler = value; },
    edit: async (id, value) => { node(id).value = value; await node(id).fire("input"); await node(id).fire("change"); } };
}

for (const invalid of ["", "abc", "-1", "1.5", "9007199254740992"]) {
  test(`invalid minimum ${JSON.stringify(invalid)} never saves or clears applied intent`, async () => {
    const f = fixture();
    await f.edit("captureMinimumKiB", invalid);
    assert.equal(f.messages.filter(message => message.type === "popup_save_settings").length, 0);
    assert.equal(f.state.settings.captureMinimumKiB, 128);
    f.publish();
    assert.equal(f.node("captureMinimumKiB").value, invalid);
    assert.equal(f.node("messageText").dataset.tone, "error");
    assert.equal(f.run("busyActionCount"), 0);
  });
}

test("complete minimum including zero uses the original settings request", async () => {
  const f = fixture();
  for (const value of ["256", "0", "00128", " 64 "]) {
    await f.edit("captureMinimumKiB", value);
    assert.equal(f.state.settings.captureMinimumKiB, Number(value));
    assert.equal(f.messages.at(-1).type, "popup_save_settings");
    assert.equal(f.node("captureMinimumKiB").value, String(Number(value)), "saved numeric draft is released and normalized");
  }
});

test("failed blurred rule draft survives refresh and explicit Save retries it", async () => {
  const f = fixture();
  f.setHandler(async message => message.type === "popup_save_settings" ? { ok: false, message: "保存失败" } : structuredClone(f.state));
  await f.edit("captureFileTypes", "mp4");
  await f.run("refresh({ force: true })");
  assert.equal(f.node("captureFileTypes").value, "mp4");
  assert.equal(f.state.settings.captureFileTypes, "zip");
  f.setHandler(async message => { Object.assign(f.state.settings, message.settings); return structuredClone(f.state); });
  await f.node("saveButton").fire("click");
  assert.equal(f.state.settings.captureFileTypes, "mp4");
  f.state.settings.captureFileTypes = "pdf";
  f.publish();
  assert.equal(f.node("captureFileTypes").value, "pdf", "successful save releases the draft");
});

test("save reply cannot replace a newer rule draft typed during persistence", async () => {
  const f = fixture();
  let release;
  f.setHandler(message => new Promise(resolve => { release = () => resolve({ ...structuredClone(f.state), settings: { ...f.state.settings, ...message.settings } }); }));
  f.node("captureFileTypes").value = "mp4";
  await f.node("captureFileTypes").fire("input");
  const save = f.node("captureFileTypes").fire("change");
  f.node("captureFileTypes").value = "pdf";
  await f.node("captureFileTypes").fire("input");
  release();
  await save;
  assert.equal(f.node("captureFileTypes").value, "pdf");
});

test("earlier refresh preserves an active pause button busy state", async () => {
  const f = fixture();
  let release;
  f.setHandler(message => message.type === "popup_set_capture_paused" ? new Promise(resolve => { release = () => resolve({ ok: true, capturePaused: true }); }) : Promise.resolve(structuredClone(f.state)));
  const pause = f.node("capturePauseButton").fire("click");
  await f.run("refresh({ force: true })");
  assert.equal(f.node("capturePauseButton").disabled, true);
  assert.equal(f.node("capturePauseButton").textContent, "更新中");
  release();
  await pause;
  assert.equal(f.node("capturePauseButton").getAttribute("aria-pressed"), "true");
  assert.equal(f.node("capturePauseButton").disabled, false);
  assert.equal(f.state.settings.interceptDownloads, true);
});

test("unavailable session retries the original state read without toggling", async () => {
  const f = fixture();
  f.state.capturePauseAvailable = false;
  f.state.capturePaused = true;
  f.publish();
  assert.equal(f.node("capturePauseButton").disabled, false);
  assert.match(f.node("capturePauseButton").textContent, /重试/);
  f.setHandler(async () => ({ ...structuredClone(f.state), capturePauseAvailable: true }));
  await f.node("capturePauseButton").fire("click");
  assert.deepEqual(f.messages.map(message => message.type), ["popup_get_state"]);
  assert.equal(f.run("currentState.capturePaused"), true);
  assert.equal(f.run("currentState.capturePauseAvailable"), true);
});

test("failed pause retains applied state and a usable retry action", async () => {
  const f = fixture();
  f.setHandler(async () => ({ ok: false, message: "保存失败，请重试" }));
  await f.node("capturePauseButton").fire("click");
  assert.equal(f.run("currentState.capturePaused"), false);
  assert.equal(f.node("capturePauseButton").disabled, false);
  assert.equal(f.node("capturePauseButton").getAttribute("aria-busy"), null);
  assert.equal(f.node("messageText").dataset.tone, "error");
});

for (const operation of ["pause", "settings"]) {
  test(`older refresh cannot roll back successful ${operation}`, async () => {
    const f = fixture();
    let release;
    f.setHandler(message => {
      if (message.type === "popup_get_state") {
        const observed = structuredClone(f.state);
        return new Promise(resolve => { release = () => resolve(observed); });
      }
      if (message.type === "popup_set_capture_paused") return Promise.resolve({ ok: true, capturePaused: true });
      Object.assign(f.state.settings, message.settings);
      return Promise.resolve(structuredClone(f.state));
    });
    const refresh = f.run("refresh({ force: true })");
    if (operation === "pause") await f.node("capturePauseButton").fire("click");
    else await f.edit("captureFileTypes", "mp4");
    release();
    await refresh;
    if (operation === "pause") {
      assert.equal(f.run("currentState.capturePaused"), true);
      assert.equal(f.node("capturePauseButton").getAttribute("aria-pressed"), "true");
    } else {
      assert.equal(f.run("currentState.settings.captureFileTypes"), "mp4");
      assert.equal(f.node("captureFileTypes").value, "mp4");
    }
  });
}

test("browser fixture releases exact first refresh/pause requests once and passes later requests through", async () => {
  const { holdNextRequests } = require("./harness/held-requests.cjs");
  const f = fixture();
  const held = holdNextRequests(f.runtime, ["popup_get_state", "popup_set_capture_paused"]);
  const refresh = f.run("refresh({ force: true })");
  const refreshId = held.inspect().find(row => row.type === "popup_get_state").id;
  const later = await f.runtime.sendMessage({ type: "popup_get_state", fixtureRequestId: "later-refresh" });
  assert.equal(later.capturePaused, false);
  const pause = f.node("capturePauseButton").fire("click");
  const pauseId = held.inspect().find(row => row.type === "popup_set_capture_paused").id;
  assert.notEqual(refreshId, pauseId);
  assert.equal(held.inspect().length, 2, "later same-type requests never replace held owners");
  held.release(refreshId);
  await refresh;
  assert.equal(f.node("capturePauseButton").disabled, true);
  assert.equal(f.node("capturePauseButton").getAttribute("aria-busy"), "true");
  held.release(pauseId);
  await pause;
  assert.equal(f.run("currentState.capturePaused"), true);
  assert.equal(f.node("capturePauseButton").disabled, false);
  assert.throws(() => held.release(refreshId), /already released/);
  assert.throws(() => held.release(pauseId), /already released/);
  await f.runtime.sendMessage({ type: "popup_get_state", fixtureRequestId: "after-release" });
  assert.equal(f.messages.filter(message => message.type === "popup_set_capture_paused").length, 1);
  assert.equal(f.messages.filter(message => message.type === "popup_get_state").length, 3);
  assert.deepEqual(held.inspect().map(row => row.state), ["settled", "settled"]);
});
