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
  const bg = loadBackground({ browser: "firefox", chrome: { cancelFails: true }, desktop: { respond(message) {
    assert.ok(["create_task", "task_action"].includes(message.type));
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
  for (const [id, cookieStoreId, incognito] of [[61, "firefox-container-2", false], [62, "firefox-private", true]]) {
    const before = bg.desktop.requests.length;
    bg.chrome.api.downloads.onCreated.fire({ id, url, finalUrl: url, filename: "same.zip", totalBytes: 2048,
      startTime: "2026-11-09T00:00:00Z", cookieStoreId, incognito });
    await bg.waitFor(() => bg.desktop.requests.length === before + 1, "extension native request");
    await bg.waitFor(() => bg.evaluate("lastHandoff?.state !== 'submitting'"), "real host reply projection");
    const wire = bg.desktop.requests.at(-1).message;
    const reply = replies.at(-1);
    assert.equal(wire.payload.cookie, `session=${cookieStoreId}`);
    assert.equal("cookieStoreId" in wire.payload, false);
    assert.equal(reply.requestId, wire.requestId);
    assert.equal(reply.handoffId, wire.payload.handoffId);
    assert.equal(reply.accepted, false, "confirmation window is not a durable task receipt");
    assert.equal(reply.data.handoffState, "awaiting_confirmation");
    const status = JSON.parse(JSON.stringify(bg.evaluate("lastHandoff")));
    assert.equal(status.reason, "confirmation_pending");
    assert.equal(status.owner, "browser");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
    assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
  }
  const manual = await bg.sendRuntimeMessage({ type: "drop_target_download", url },
    bg.senders.content({ id: 7, url: "https://files.example.test/page", cookieStoreId: "firefox-container-3" }));
  assert.equal(manual.ok, true);
  assert.equal(manual.accepted, false);
  assert.equal(manual.taskId, undefined);
  assert.equal(manual.handoffId, bg.desktop.requests.at(-1).message.payload.handoffId);
  assert.equal(manual.data.handoffState, "awaiting_confirmation");
  assert.equal(manual.message, "确认窗口已打开，尚未创建任务");
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
  assert.deepEqual(queries, [
    { url, storeId: "firefox-container-2" },
    { url, storeId: "firefox-private" },
    { url, storeId: "firefox-container-3" },
    { url: "https://files.example.test/page", storeId: "firefox-container-3" },
  ]);
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
    cancelBrowserDownload: () => bg.chrome.api.downloads.cancel(63),
    eraseBrowserDownload: () => bg.chrome.api.downloads.erase({ id: 63 }),
    rollbackDesktopTask: taskId => bg.realm.sendDesktopRequest("task_action", { taskId, action: "cancel" }, 12000, handoffId)
  }), failure => failure.handoffReason === "browser_cancel_failed" && failure.compensation === "succeeded");
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1);
  assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0, "failed browser cancellation retains the browser item");
  const replay = await bg.realm.addDownloadToNxTools(payload, handoffId);
  assert.equal(replay.taskId, receipt.taskId);
  assert.equal(replay.duplicate, true, "compensation does not allocate a replacement handoff");
  await assert.rejects(bg.realm.addDownloadToNxTools({ ...payload, cookie: "conflicting=synthetic" }, handoffId),
    failure => failure.handoffReason === "desktop_refused");
  assert.equal(replies.at(-1).handoffId, handoffId);
  assert.equal(replies.at(-1).accepted, false);
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
