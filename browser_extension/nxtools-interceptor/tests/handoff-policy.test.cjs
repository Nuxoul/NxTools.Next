const assert = require("node:assert/strict");
const test = require("node:test");

const { handoffBrowserDownload } = require("../handoff-policy.js");

test("browser download is never cancelled before durable NxTools acceptance", async () => {
  const calls = [];
  await assert.rejects(handoffBrowserDownload({
    async handoff() { calls.push("handoff"); throw new Error("host unavailable"); },
    async cancelBrowser() { calls.push("cancel"); },
    async compensate() { calls.push("compensate"); }
  }), /host unavailable/);
  assert.deepEqual(calls, ["handoff"]);
});

test("browser cancellation failure compensates the accepted NxTools task", async () => {
  const calls = [];
  const result = await handoffBrowserDownload({
    async handoff() { calls.push("handoff"); return { ok: true, data: { taskId: "task-1" } }; },
    async cancelBrowser() { calls.push("cancel"); throw new Error("browser kept download"); },
    async compensate(accepted) { calls.push(`compensate:${accepted.data.taskId}`); },
    async eraseBrowser() { calls.push("erase"); }
  });
  assert.equal(result.state, "browser_retained");
  assert.deepEqual(calls, ["handoff", "cancel", "compensate:task-1"]);
});

test("successful acceptance cancels and optionally erases the browser item in order", async () => {
  const calls = [];
  const result = await handoffBrowserDownload({
    async handoff() { calls.push("handoff"); return { ok: true }; },
    async cancelBrowser() { calls.push("cancel"); },
    async compensate() { calls.push("compensate"); },
    async eraseBrowser() { calls.push("erase"); }
  });
  assert.equal(result.state, "intercepted");
  assert.deepEqual(calls, ["handoff", "cancel", "erase"]);
});
