"use strict";

const assert = require("node:assert/strict");
const { randomUUID } = require("node:crypto");
const { loadBackground } = require("./load-background.cjs");

// Invoked by the Rust BrowserHost fixture. Only framed messages use stdout.
const pending = new Map();
let buffered = Buffer.alloc(0);
const watchdog = setTimeout(() => {
  process.stderr.write("Native host roundtrip fixture timed out\n");
  process.exit(1);
}, 15000);
function writeFrame(message) {
  const body = Buffer.from(JSON.stringify(message));
  const prefix = Buffer.alloc(4);
  prefix.writeUInt32LE(body.length);
  process.stdout.write(Buffer.concat([prefix, body]));
}
process.stdin.on("data", chunk => {
  buffered = Buffer.concat([buffered, chunk]);
  while (buffered.length >= 4) {
    const size = buffered.readUInt32LE();
    assert.ok(size <= 256 * 1024, "bounded native reply");
    if (buffered.length < 4 + size) return;
    const reply = JSON.parse(buffered.subarray(4, 4 + size).toString("utf8"));
    buffered = buffered.subarray(4 + size);
    const resolve = pending.get(reply.requestId);
    assert.ok(resolve, "reply identity belongs to an outstanding extension request");
    pending.delete(reply.requestId);
    resolve(reply);
  }
});

async function main() {
  const replies = [];
  const bg = loadBackground({ browser: "firefox", chrome: { tabs: [
    { id: 7, url: "https://files.example.test/page", cookieStoreId: "firefox-container-3", active: true }
  ] }, desktop: { respond(message) {
    assert.ok([
      "prepare_browser_handoff", "get_browser_handoff", "commit_browser_handoff", "abort_browser_handoff",
      "create_task", "create_tasks", "task_action"
    ].includes(message.type));
    return new Promise(resolve => {
      pending.set(message.requestId, reply => { replies.push(reply); resolve(reply); });
      writeFrame(message);
    });
  } } });
  await bg.settle();
  const queries = [];
  bg.chrome.api.cookies.getAll = async query => {
    queries.push(JSON.parse(JSON.stringify(query)));
    return [{ name: "session", value: query.storeId }];
  };
  const url = "https://files.example.test/same.zip";

  async function automatic(id, cookieStoreId, incognito) {
    const prepareBefore = bg.desktop.requestsOfType("prepare_browser_handoff").length;
    bg.chrome.api.downloads.onCreated.fire({ id, url, finalUrl: url, filename: "same.zip", totalBytes: 2048,
      startTime: `2026-11-09T00:00:0${id - 61}Z`, cookieStoreId, incognito });
    await bg.waitFor(() => bg.desktop.requestsOfType("prepare_browser_handoff").length === prepareBefore + 1,
      "controlled prepare request");
    const prepare = bg.desktop.requestsOfType("prepare_browser_handoff").at(-1).message;
    await bg.waitFor(() => replies.some(reply => reply.requestId === prepare.requestId), "controlled prepare reply");
    const prepared = replies.find(reply => reply.requestId === prepare.requestId);
    assert.equal(prepare.payload.cookie, `session=${cookieStoreId}`);
    assert.equal("cookieStoreId" in prepare.payload, false);
    assert.equal(prepared.handoffId, prepare.payload.handoffId);
    assert.equal(prepared.accepted, false, "prepare admission is not durable held acceptance");
    assert.equal(prepared.data.browserHandoffState, "pending");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, id - 61);
    for (let attempt = 0; attempt < 5 && bg.desktop.requestsOfType("commit_browser_handoff").length < id - 60; attempt += 1) {
      bg.clock.advance(1000);
      await bg.settle(20);
    }
    await bg.waitFor(() => bg.desktop.requestsOfType("commit_browser_handoff").length === id - 60,
      "controlled commit after browser cancellation");
    await bg.waitFor(() => bg.evaluate("lastHandoff?.state === 'transferred'"), "controlled transfer terminal");
    const status = JSON.parse(JSON.stringify(bg.evaluate("lastHandoff")));
    assert.equal(status.owner, "desktop");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, id - 60);
    const commit = bg.desktop.requestsOfType("commit_browser_handoff").at(-1).message;
    assert.equal(commit.payload.handoffId, prepare.payload.handoffId);
    assert.equal(typeof commit.payload.taskId, "string");
  }

  await automatic(61, "firefox-container-2", false);
  await automatic(62, "firefox-private", true);

  const manual = await bg.sendRuntimeMessage({ type: "drop_target_download", url },
    bg.senders.content({ id: 7, url: "https://files.example.test/page", cookieStoreId: "firefox-container-3" }));
  assert.equal(manual.ok, true);
  assert.equal(manual.accepted, false);
  assert.equal(manual.taskId, undefined);
  assert.equal(manual.handoffId, bg.desktop.requestsOfType("create_task").at(-1).message.payload.handoffId);
  assert.equal(manual.data.handoffState, "awaiting_confirmation");
  assert.equal(manual.message, "确认窗口已打开，尚未创建任务");

  const batchUrls = ["https://files.example.test/batch-a.zip", "https://files.example.test/batch-b.zip"];
  for (const [index, batchUrl] of batchUrls.entries()) {
    const details = { requestId: `batch-${index}`, url: batchUrl, tabId: 7, cookieStoreId: "firefox-container-3",
      type: "xmlhttprequest", requestHeaders: [{ name: "Authorization", value: `batch-${index}` }] };
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire(details);
    bg.chrome.api.webRequest.onResponseStarted.fire({ ...details, statusCode: 200,
      responseHeaders: [{ name: "Content-Type", value: "application/zip" }, { name: "Content-Length", value: "2048" }] });
  }
  await bg.settle(20);
  const batchIds = bg.evaluate("[...resourcesById.keys()].filter(id => id.includes('batch-'))");
  assert.equal(batchIds.length, 2);
  const batch = await bg.realm.sendResourcesBatch(batchIds);
  assert.equal(batch.ok, true);
  assert.equal(batch.accepted, true);
  const batchWire = bg.desktop.requestsOfType("create_tasks").at(-1).message;
  assert.equal(batchWire.payload.items.length, 2);
  assert.ok(batchWire.payload.items.every(value => value.openDialog === false && value.startImmediately === false));

  const handoffId = randomUUID();
  const payload = { handoffId, url, fileName: "rollback.zip", cookie: "rollback=synthetic",
    openDialog: false, startImmediately: false };
  const receipt = await bg.realm.addDownloadToNxTools(payload, handoffId);
  assert.equal(receipt.accepted, true);
  assert.equal(receipt.data.handoffState, "persisted");
  assert.equal(receipt.data.queued, true);
  await assert.rejects(bg.realm.NxToolsHandoffPolicy.completeAcceptedHandoff({
    response: receipt,
    expectedHandoffId: handoffId,
    cancelBrowserDownload: async () => { throw new Error("synthetic cancel failure"); },
    eraseBrowserDownload: () => bg.chrome.api.downloads.erase({ id: 63 }),
    rollbackDesktopTask: taskId => bg.realm.sendDesktopRequest("task_action", { taskId, action: "cancel" }, 12000, handoffId)
  }), failure => failure.handoffReason === "browser_cancel_failed" && failure.compensation === "succeeded");
  assert.equal(bg.chrome.callsNamed("downloads.erase").filter(call => call.args[0].id === 63).length, 0,
    "failed browser cancellation retains its original browser item");
  const replay = await bg.realm.addDownloadToNxTools(payload, handoffId);
  assert.equal(replay.taskId, receipt.taskId);
  assert.equal(replay.duplicate, true, "compensation does not allocate a replacement handoff");
  await assert.rejects(bg.realm.addDownloadToNxTools({ ...payload, cookie: "conflicting=synthetic" }, handoffId),
    failure => failure.handoffReason === "desktop_refused");
  assert.equal(replies.at(-1).handoffId, handoffId);
  assert.equal(replies.at(-1).accepted, false);

  assert.deepEqual(queries.slice(0, 4), [
    { url, storeId: "firefox-container-2" },
    { url, storeId: "firefox-private" },
    { url, storeId: "firefox-container-3" },
    { url: "https://files.example.test/page", storeId: "firefox-container-3" }
  ]);
  assert.equal(pending.size, 0);
  clearTimeout(watchdog);
  process.stdin.destroy();
  writeFrame({ type: "fixture_complete", scenarios: 5 });
}
main().catch(failure => {
  clearTimeout(watchdog);
  process.stderr.write(`${failure.stack}\n`);
  process.exitCode = 1;
  process.stdin.destroy();
});
