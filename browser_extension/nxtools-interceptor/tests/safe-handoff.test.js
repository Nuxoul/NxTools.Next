const assert = require("node:assert/strict");
const test = require("node:test");

const { completeAcceptedHandoff } = require("../handoff-policy.js");

test("a rejected desktop handoff leaves the browser download untouched", async () => {
  const calls = [];
  const result = await completeAcceptedHandoff({
    response: { ok: false, accepted: false, error: "desktop unavailable" },
    expectedHandoffId: "handoff-1",
    cancelBrowserDownload: async () => calls.push("cancel"),
    eraseBrowserDownload: async () => calls.push("erase"),
  });

  assert.deepEqual(calls, []);
  assert.equal(result.transferred, false);
});

test("an accepted desktop handoff cancels before erasing the browser item", async () => {
  const calls = [];
  const result = await completeAcceptedHandoff({
    response: { ok: true, accepted: true, handoffId: "handoff-2", taskId: "task-1" },
    expectedHandoffId: "handoff-2",
    cancelBrowserDownload: async () => calls.push("cancel"),
    eraseBrowserDownload: async () => calls.push("erase"),
  });

  assert.deepEqual(calls, ["cancel", "erase"]);
  assert.equal(result.transferred, true);
  assert.equal(result.taskId, "task-1");
});

test("an idempotent duplicate acceptance is still safe to remove from the browser", async () => {
  const calls = [];
  const result = await completeAcceptedHandoff({
    response: {
      ok: true,
      accepted: true,
      handoffId: "handoff-3",
      duplicate: true,
      taskId: "existing-task",
    },
    expectedHandoffId: "handoff-3",
    cancelBrowserDownload: async () => calls.push("cancel"),
    eraseBrowserDownload: async () => calls.push("erase"),
  });

  assert.deepEqual(calls, ["cancel", "erase"]);
  assert.equal(result.duplicate, true);
});

test("a cancellation failure does not erase the browser item", async () => {
  const calls = [];
  await assert.rejects(
    completeAcceptedHandoff({
      response: { ok: true, accepted: true, handoffId: "handoff-4", taskId: "task-2" },
      expectedHandoffId: "handoff-4",
      cancelBrowserDownload: async () => {
        calls.push("cancel");
        throw new Error("cancel failed");
      },
      eraseBrowserDownload: async () => calls.push("erase"),
      rollbackDesktopTask: async (taskId) => calls.push(`rollback:${taskId}`),
    }),
    /cancel failed/,
  );

  assert.deepEqual(calls, ["cancel", "rollback:task-2"]);
});

test("an acceptance for a different handoff never cancels the browser download", async () => {
  const calls = [];
  const result = await completeAcceptedHandoff({
    response: { ok: true, accepted: true, handoffId: "other", taskId: "task-3" },
    expectedHandoffId: "expected",
    cancelBrowserDownload: async () => calls.push("cancel"),
    eraseBrowserDownload: async () => calls.push("erase"),
  });

  assert.equal(result.transferred, false);
  assert.match(result.error, /identity mismatch/);
  assert.deepEqual(calls, []);
});
