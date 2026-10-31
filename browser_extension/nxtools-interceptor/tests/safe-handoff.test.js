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

test("the worker has no obsolete response-header takeover or duplicate Blob reader", () => {
  const { backgroundSource, extractDeclaration } = require("./harness/background-source.cjs");
  const source = backgroundSource();
  const obsoleteOwners = [
    "preemptedBrowserDownloads",
    "PREEMPTED_DOWNLOAD_TTL_MS",
    "rememberPreemptedBrowserDownload",
    "wasPreemptedBrowserDownload",
    "isLikelyBrowserDownloadResponse",
    "sendPreemptedBrowserDownload",
    "handlePreemptiveBrowserDownload",
    "dispositionRequestsDownload",
    "BINARY_DOWNLOAD_EXTENSIONS",
    "FORCED_DOWNLOAD_MIME_TYPES",
    "readBlobResourceViaContentScript",
    "browser_download_already_preempted",
  ];
  for (const owner of obsoleteOwners) {
    assert.doesNotMatch(source, new RegExp(`\\b${owner}\\b`), `${owner} must not remain as a duplicate owner`);
  }

  const captureBody = extractDeclaration(source, "registerHeaderCapture");
  assert.match(captureBody, /rememberRequestHeaders\(details\);/);
  assert.match(captureBody, /captureRequestResource\(details\);/);
  assert.match(captureBody, /onResponseStarted\.addListener\([\s\S]*captureNetworkResource\(details\);/);
  assert.match(captureBody, /rememberRedirectHint\(details\);/);
  assert.doesNotMatch(captureBody, /onHeadersReceived|"blocking"|cancel|sendDownloadPayload/);
  assert.match(source, /chrome\.downloads\.onCreated\.addListener\(\(downloadItem\) => \{\s*void interceptBrowserDownload\(downloadItem, \{ eraseBrowserHistory: true \}\);/);
});

test("active Blob upload and content-script base64 fallback remain available", () => {
  const fs = require("node:fs");
  const path = require("node:path");
  const { backgroundSource, extractDeclaration } = require("./harness/background-source.cjs");
  const source = backgroundSource();
  const contentSource = fs.readFileSync(path.join(__dirname, "..", "content-script.js"), "utf8");
  assert.match(source, /return await sendBlobResourceViaContentScript\(resource\);/);
  const blobBody = extractDeclaration(source, "sendBlobResourceViaContentScript");
  for (const message of ["content_blob_info", "prepare_blob_upload", "content_upload_blob", "commit_blob_upload"]) {
    assert.ok(blobBody.includes(`"${message}"`), `${message} remains on the active bounded upload path`);
  }
  assert.match(blobBody, /finally\s*\{\s*releaseBlobPort\(\);/);
  assert.match(contentSource, /message\?\.type === "content_read_blob"[\s\S]*?readBlobInPage\(message\.url, message\.filename\)\.then\(sendResponse/);
});
