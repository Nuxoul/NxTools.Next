"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");

for (const browser of ["chrome", "firefox"]) {
  test(`${browser} popup shows connection without projecting desktop tasks, including after disconnect`, async () => {
    const bg = loadBackground({ browser, desktop: { respond: () => ({
      ok: true, type: "snapshot", data: { tasks: [{ id: "desktop-only-task", displayFileName: "desktop-only.zip" }] }
    }) } });
    await bg.settle();
    const state = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.equal(state.status.ok, true);
    assert.equal(state.status.code, "connected");
    assert.equal(Object.hasOwn(state, "tasks"), false);
    assert.equal(Object.hasOwn(state.status, "data"), false);
    assert.equal(JSON.stringify(state).includes("desktop-only"), false);

    const connection = await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
    assert.equal(connection.ok, true);
    assert.equal(Object.hasOwn(connection, "data"), false);
    bg.chrome.api.runtime.connectNative = () => { throw new Error("Failed to start native messaging host."); };
    bg.chrome.nativePorts.at(-1).disconnect();
    const disconnected = await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
    assert.equal(disconnected.ok, false);
    assert.equal(Object.hasOwn(disconnected, "data"), false);
    assert.equal(JSON.stringify(disconnected).includes("desktop-only"), false);
  });

  test(`${browser} popup cannot pause, resume or cancel desktop tasks`, async () => {
    const bg = loadBackground({ browser, desktop: { respond: () => ({ ok: true }) } });
    await bg.settle();
    for (const action of ["pause", "resume", "cancel"]) {
      const reply = await bg.sendRuntimeMessage({ type: "popup_task_action", taskId: "desktop-only-task", action }, bg.senders.popup());
      assert.equal(reply, undefined);
    }
    assert.equal(bg.desktop.requestsOfType("task_action").length, 0);
    assert.equal(bg.chrome.nativePorts.length, 0, "removed controls have no native transport demand");
  });
}
