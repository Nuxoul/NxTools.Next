const assert = require("node:assert/strict");
const test = require("node:test");

const {
  acceptedHandoffResponse,
  buildExtensionLogRequest,
  completeAcceptedHandoff,
} = require("../handoff-policy.js");

test("extension diagnostics keep transport identity separate from feature correlation", () => {
  const allocated = ["feature-correlation", "already-pending", "log-transport-id"];
  const request = buildExtensionLogRequest({
    protocolVersion: 2,
    level: "info",
    event: "batch_handoff_started",
    message: "started",
    correlationId: "feature-correlation",
    allocateRequestId: () => allocated.shift(),
    isRequestIdPending: (requestId) => requestId === "already-pending",
  });

  assert.equal(request.requestId, "log-transport-id");
  assert.equal(request.payload.correlationId, "feature-correlation");
});

test("manual and context-menu handoffs require explicit identity-bound acceptance", () => {
  assert.equal(acceptedHandoffResponse({
    ok: true,
    accepted: true,
    handoffId: "handoff-1",
  }, "handoff-1"), true);
  assert.equal(acceptedHandoffResponse({ ok: true }, "handoff-1"), false);
  assert.equal(acceptedHandoffResponse({
    ok: true,
    accepted: true,
    handoffId: "foreign",
  }, "handoff-1"), false);
});

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

test("interception holds the browser save dialog until the handoff resolves", () => {
  // Resolving onDeterminingFilename eagerly lets Chrome open its own Save As
  // dialog before the intercepted download is canceled. The listener must
  // forward suggest into the handoff and only resolve it on terminal paths.
  const source = require("node:fs").readFileSync(
    require("node:path").join(__dirname, "..", "background.js"),
    "utf8",
  );

  assert.match(
    source,
    /determiningFilenameEvent\.addListener\(\(downloadItem, suggest\) => \{\s*void interceptBrowserDownload\(downloadItem, \{ suggest \}\);/,
    "the listener forwards suggest instead of resolving it eagerly",
  );
  assert.match(
    source,
    /const yieldToBrowser = \(\) => \{/,
    "every hand-back path resolves the pending suggestion",
  );
  const handoffBody = source.slice(
    source.indexOf("async function interceptBrowserDownload"),
    source.indexOf("function createContextMenus"),
  );
  const handoffResolves = [...handoffBody.matchAll(/yieldToBrowser\(\);/g)].length;
  assert.equal(
    handoffResolves,
    5,
    "disabled, site-disabled, preempted, accepted and rejected paths all yield once",
  );
});
