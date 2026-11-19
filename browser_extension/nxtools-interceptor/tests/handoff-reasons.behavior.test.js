"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const {
  loadBackground,
  browserHandoffReply,
  createControlledHandoffResponder
} = require("./harness/load-background.cjs");

const item = {
  id: 41, url: "https://files.example.test/private.zip?credential=synthetic",
  finalUrl: "https://files.example.test/private.zip?credential=synthetic",
  referrer: "https://files.example.test/private", filename: "private.zip",
  startTime: "2026-11-09T00:00:00Z", totalBytes: 2048, cookieStoreId: "fixture-store"
};
function fixture(browser, scenario = {}) {
  const controlled = createControlledHandoffResponder({ queued: scenario.queued === true, onRequest(message, records) {
    return scenario.onRequest?.(message, records);
  }, fallback(message) {
    if (message.type === "task_action") return { ok: true };
    return { ok: true, data: { tasks: [] } };
  } });
  return loadBackground({ browser, chrome: scenario.chrome,
    desktop: { helloFails: scenario.helloFails, respond: controlled.respond }
  });
}
function fire(bg, browser) {
  let suggestions = 0;
  if (browser === "chrome") {
    const [result] = bg.chrome.api.downloads.onDeterminingFilename.fire(item, () => { suggestions += 1; });
    assert.equal(result, true);
  } else bg.chrome.api.downloads.onCreated.fire(item);
  return () => suggestions;
}
async function finished(bg, browser, suggestions) {
  await bg.waitFor(() => Boolean(bg.evaluate(
    "typeof lastHandoff !== 'undefined' && lastHandoff && !['submitting','persisted'].includes(lastHandoff.state)"
  )), "handoff terminal state");
  const status = JSON.parse(JSON.stringify(bg.evaluate("typeof lastHandoff === 'undefined' ? null : lastHandoff")));
  if (browser === "chrome") assert.equal(suggestions(), status.state === "awaiting_confirmation" ? 0 : 1,
    "filename suggestion remains held only while confirmation is pending");
  return status;
}
function retained(bg) {
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
  assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
}

for (const browser of ["chrome", "firefox"]) {
  for (const [name, reason, settings] of [
    ["pause", "temporarily_paused", { interceptDownloads: false }],
    ["site rule", "rule_excluded", { nxtools_disabled_sites: ["files.example.test"] }]
  ]) test(`B1 ${browser} ${name} is a policy decision retaining browser ownership`, async () => {
    const bg = fixture(browser, { chrome: { storage: settings } }); await bg.settle();
    const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.reason, reason); assert.equal(status.state, "skipped"); assert.equal(status.owner, "browser");
    retained(bg); assert.equal(bg.chrome.nativePorts.length, 0, "policy skip never launches the host");
  });

  test(`B1 ${browser} pending confirmation keeps the controlled original paused without cancellation`, async () => {
    const bg = fixture(browser, { onRequest(message) {
      if (message.type === "prepare_browser_handoff") return browserHandoffReply(message.payload.handoffId, "pending");
    } });
    await bg.settle();
    const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.reason, "confirmation_pending");
    assert.equal(status.state, "awaiting_confirmation");
    assert.equal(status.owner, "browser");
    retained(bg);
    assert.equal(bg.chrome.callsNamed("downloads.pause").length, 1);
    assert.equal(bg.chrome.callsNamed("downloads.resume").length, 0);
    assert.equal(bg.chrome.downloadItems.get(item.id).paused, true);
  });

  for (const [name, response, reason, state] of [
    ["desktop abort", message => browserHandoffReply(message.payload.handoffId, "aborted"), "confirmation_cancelled", "failed"],
    ["expiry", message => browserHandoffReply(message.payload.handoffId, "expired"), "confirmation_timeout", "failed"],
    ["missing handoff", message => browserHandoffReply(message.payload.handoffId, "missing"), "confirmation_cancelled", "failed"],
    ["held without task identity", message => {
      const reply = browserHandoffReply(message.payload.handoffId, "held", "persisted-task");
      delete reply.taskId;
      return reply;
    }, "identity_mismatch", "failed"],
    ["wrong handoff identity", () => browserHandoffReply("other-handoff", "pending"), "identity_mismatch", "failed"]
  ]) test(`B1 ${browser} ${name} restores the browser without cancellation`, async () => {
    const bg = fixture(browser, { onRequest(message) {
      if (message.type === "prepare_browser_handoff") return response(message);
    } });
    await bg.settle();
    const status = await finished(bg, browser, fire(bg, browser));
    retained(bg);
    assert.equal(status.reason, reason); assert.equal(status.state, state); assert.equal(status.owner, "browser");
    assert.equal(bg.chrome.callsNamed("downloads.resume").length, 1, "terminal rejection restores the paused original");
    const wire = bg.desktop.requestsOfType("prepare_browser_handoff")[0].message;
    assert.equal(status.correlationId, wire.requestId); assert.equal(wire.requestId, wire.payload.handoffId);
    assert.ok(!JSON.stringify(status).includes("synthetic-secret"));
  });

  test(`B1 ${browser} held queued acceptance transfers only after browser cancellation and commit`, async () => {
    const bg = fixture(browser, { queued: true });
    await bg.settle(); const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.state, "transferred"); assert.equal(status.reason, "browser_transferred");
    assert.equal(status.owner, "desktop"); assert.equal(status.queued, false, "queued belongs to the held receipt, not final download success");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1);
    assert.equal(bg.desktop.requestsOfType("commit_browser_handoff").length, 1);
    const logs = bg.chrome.nativePorts.flatMap(port => port.sent).filter(message => message.type === "extension_log");
    const phases = logs.filter(message => message.payload.event === "handoff_state").map(message => message.payload.handoffState);
    assert.deepEqual(phases, ["persisted", "transferred"]);
    for (const log of logs.filter(message => message.payload.event === "handoff_state")) {
      assert.equal(log.payload.correlationId, status.correlationId);
      assert.equal(typeof log.payload.elapsedMs, "number");
      assert.ok(!/private|credential|synthetic|cookie|headers/i.test(JSON.stringify(log.payload)), "diagnostics carry no payload");
    }
    const popup = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.deepEqual(JSON.parse(JSON.stringify(popup.lastHandoff)), status, "actual popup projection retains only safe handoff status");
    assert.ok(!JSON.stringify(bg.chrome.storage).includes("browser_transferred"), "status is transient, not stored");
  });

  for (const abortReplyValid of [true, false]) test(`B1 ${browser} browser cancellation failure aborts held desktop ownership (abort reply valid=${abortReplyValid})`, async () => {
    const bg = fixture(browser, { chrome: { cancelFails: true }, onRequest(message) {
      if (!abortReplyValid && message.type === "abort_browser_handoff") {
        return browserHandoffReply("other-handoff", "aborted");
      }
    } });
    await bg.settle();
    const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.reason, "browser_cancel_failed"); assert.equal(status.state, "failed"); assert.equal(status.owner, "browser");
    assert.equal(status.compensation, abortReplyValid ? "succeeded" : "failed");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1); assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
    assert.equal(bg.desktop.requestsOfType("abort_browser_handoff").length, 1);
    assert.equal(bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
    assert.equal(bg.chrome.callsNamed("downloads.resume").length, 1);
    assert.equal(bg.desktop.requestsOfType("task_action").length, 0, "controlled compensation uses abort, not ordinary task cancellation");
  });

  test(`B1 ${browser} request timeout restores browser ownership and stable retry identity`, async () => {
    let prepares = 0;
    const bg = fixture(browser, { onRequest(message) {
      if (message.type === "prepare_browser_handoff" && prepares++ === 0) return new Promise(() => {});
    } });
    await bg.settle();
    let suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.desktop.requestsOfType("prepare_browser_handoff").length === 1, "first wire request");
    bg.clock.advance(15000); await bg.settle();
    let status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "handoff_timeout"); assert.equal(status.owner, "browser"); retained(bg);
    const original = bg.desktop.requestsOfType("prepare_browser_handoff")[0].message;
    suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.chrome.callsNamed("downloads.cancel").length === 1, "retry transfer");
    status = await finished(bg, browser, suggestions);
    const retry = bg.desktop.requestsOfType("prepare_browser_handoff")[1].message;
    assert.equal(retry.payload.handoffId, original.payload.handoffId);
    assert.equal(status.owner, "desktop");
  });

  test(`B1 ${browser} connected-port loss restores the browser without late cancellation`, async () => {
    const bg = fixture(browser, { onRequest(message) {
      if (message.type === "prepare_browser_handoff") return new Promise(() => {});
    } });
    await bg.settle();
    const suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.desktop.requestsOfType("prepare_browser_handoff").length === 1, "pending controlled request");
    bg.chrome.nativePorts.at(-1).disconnect();
    const status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "handoff_disconnected");
    assert.equal(status.owner, "browser");
    retained(bg);
    assert.equal(bg.chrome.callsNamed("downloads.resume").length, 1);
    assert.equal(bg.desktop.requestsOfType("commit_browser_handoff").length, 0);
  });

  test(`B1 ${browser} mismatched request reply times out and restores the original`, async () => {
    const bg = fixture(browser, { onRequest(message) {
      if (message.type === "prepare_browser_handoff") {
        return { ...browserHandoffReply(message.payload.handoffId, "held", "persisted-task"), requestId: "other-request" };
      }
    } });
    await bg.settle();
    const suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.desktop.requestsOfType("prepare_browser_handoff").length === 1, "wire request");
    bg.clock.advance(15000); await bg.settle();
    const status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "handoff_timeout"); retained(bg);
    assert.equal(bg.chrome.callsNamed("downloads.resume").length, 1);
  });

  test(`B1 ${browser} synchronous native connect failure permits immediate controlled retry`, async () => {
    const bg = fixture(browser); await bg.settle();
    const connect = bg.chrome.api.runtime.connectNative;
    bg.chrome.api.runtime.connectNative = () => { throw new Error("synthetic-secret"); };
    let status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.reason, "desktop_unavailable"); retained(bg);
    await bg.waitFor(() => bg.evaluate("browserHandoffOperations.size === 0"), "failed operation retirement");
    bg.chrome.api.runtime.connectNative = connect;
    const suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.desktop.requestsOfType("commit_browser_handoff").length === 1, "immediate successful retry");
    status = await finished(bg, browser, suggestions); assert.equal(status.owner, "desktop");
  });

  test(`B1 ${browser} history cleanup failure cannot undo committed ownership`, async () => {
    const bg = fixture(browser, { chrome: { storage: { eraseBrowserHistory: true } } }); await bg.settle();
    const erase = bg.chrome.api.downloads.erase;
    bg.chrome.api.downloads.erase = async query => { await erase(query); throw new Error("synthetic-secret"); };
    const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.state, "transferred"); assert.equal(status.owner, "desktop");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1); assert.equal(bg.chrome.callsNamed("downloads.erase").length, 1);
    assert.equal(bg.desktop.requestsOfType("abort_browser_handoff").length, 0);
  });
}
