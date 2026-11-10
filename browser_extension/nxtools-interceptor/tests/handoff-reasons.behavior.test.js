"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");

const item = {
  id: 41, url: "https://files.example.test/private.zip?credential=synthetic",
  finalUrl: "https://files.example.test/private.zip?credential=synthetic",
  referrer: "https://files.example.test/private", filename: "private.zip",
  startTime: "2026-11-09T00:00:00Z", totalBytes: 2048, cookieStoreId: "fixture-store"
};
function hostReply(message, overrides = {}) {
  return { ok: true, accepted: true, handoffId: message.payload.handoffId, taskId: "persisted-task",
    data: { handoffState: "persisted", reason: "desktop_accepted" }, ...overrides };
}
function fixture(browser, scenario = {}) {
  const bg = loadBackground({ browser, chrome: scenario.chrome,
    desktop: { helloFails: scenario.helloFails, respond(message) {
      if (message.type === "create_task") return scenario.reply ? scenario.reply(message) : hostReply(message);
      if (message.type === "task_action") return { ok: !scenario.rollbackFails, error: scenario.rollbackFails ? "synthetic-secret" : undefined };
      return { ok: true, data: { tasks: [] } };
    } }
  });
  return bg;
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
  await bg.waitFor(() => browser === "chrome" ? suggestions() === 1 :
    Boolean(bg.evaluate("typeof lastHandoff !== 'undefined' && lastHandoff && !['submitting','persisted'].includes(lastHandoff.state)")), "handoff terminal state");
  if (browser === "chrome") assert.equal(suggestions(), 1, "filename suggestion resolves once");
  return JSON.parse(JSON.stringify(bg.evaluate("typeof lastHandoff === 'undefined' ? null : lastHandoff")));
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

  for (const [name, response, reason, state] of [
    ["refusal", { ok: false, error: "synthetic-secret", code: "storage_failed" }, "desktop_refused", "rejected"],
    ["confirmation", { accepted: false, taskId: undefined, data: { handoffState: "awaiting_confirmation", reason: "confirmation_pending" } }, "confirmation_pending", "awaiting_confirmation"],
    ["channel enqueue", { accepted: false, data: { queued: true } }, "queued_not_accepted", "queued"],
    ["UI incorrectly marked accepted", { accepted: true, data: { handoffState: "awaiting_confirmation", reason: "confirmation_pending" } }, "desktop_refused", "rejected"],
    ["missing handoff", { handoffId: undefined }, "identity_mismatch", "rejected"],
    ["wrong handoff", { handoffId: "other-handoff" }, "identity_mismatch", "rejected"]
  ]) test(`B1 ${browser} ${name} cannot cancel or erase the browser download`, async () => {
    const bg = fixture(browser, { reply: message => hostReply(message, response) }); await bg.settle();
    const status = await finished(bg, browser, fire(bg, browser));
    retained(bg); assert.equal(status.reason, reason); assert.equal(status.state, state); assert.equal(status.owner, "browser");
    const wire = bg.desktop.requestsOfType("create_task")[0].message;
    assert.equal(status.correlationId, wire.requestId); assert.equal(wire.requestId, wire.payload.handoffId);
    assert.ok(!JSON.stringify(status).includes("synthetic-secret"));
  });

  test(`B1 ${browser} persisted queued acceptance transfers ownership, not final download success`, async () => {
    const bg = fixture(browser, { reply: message => hostReply(message, { data: { handoffState: "persisted", reason: "desktop_accepted", queued: true } }) });
    await bg.settle(); const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.state, "transferred"); assert.equal(status.reason, "browser_transferred");
    assert.equal(status.owner, "desktop"); assert.equal(status.queued, true);
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1);
    const logs = bg.chrome.nativePorts.flatMap(port => port.sent).filter(message => message.type === "extension_log");
    const phases = logs.filter(message => message.payload.event === "handoff_state").map(message => message.payload.handoffState);
    assert.deepEqual(phases, ["submitting", "persisted", "transferred"]);
    for (const log of logs.filter(message => message.payload.event === "handoff_state")) {
      assert.equal(log.payload.correlationId, status.correlationId);
      assert.equal(typeof log.payload.elapsedMs, "number");
      assert.ok(!/private|credential|synthetic|cookie|headers/i.test(JSON.stringify(log.payload)), "diagnostics carry no payload");
    }
    const popup = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.deepEqual(JSON.parse(JSON.stringify(popup.lastHandoff)), status, "actual popup projection retains only safe handoff status");
    assert.ok(!JSON.stringify(bg.chrome.storage).includes("browser_transferred"), "status is transient, not stored");
  });

  for (const rollbackFails of [false, true]) test(`B1 ${browser} browser cancellation failure compensates desktop (rollback failure=${rollbackFails})`, async () => {
    const bg = fixture(browser, { chrome: { cancelFails: true }, rollbackFails }); await bg.settle();
    const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.reason, "browser_cancel_failed"); assert.equal(status.state, "failed"); assert.equal(status.owner, "browser");
    assert.equal(status.compensation, rollbackFails ? "failed" : "succeeded");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1); assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
    const rollback = bg.desktop.requestsOfType("task_action")[0].message;
    assert.equal(rollback.taskId, "persisted-task"); assert.equal(rollback.action, "cancel");
    assert.equal(rollback.requestId, status.correlationId, "compensation keeps lifecycle correlation");
  });

  test(`B1 ${browser} request timeout and disconnect keep browser ownership and stable retry identity`, async () => {
    const bg = fixture(browser, { reply: () => undefined }); await bg.settle();
    let suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.desktop.requestsOfType("create_task").length === 1, "first wire request");
    bg.clock.advance(15000); let status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "handoff_timeout"); assert.equal(status.owner, "browser"); retained(bg);
    const original = bg.desktop.requestsOfType("create_task")[0].message;
    suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.desktop.requestsOfType("create_task").length === 2, "retry wire request");
    const retry = bg.desktop.requestsOfType("create_task")[1].message;
    assert.equal(retry.payload.handoffId, original.payload.handoffId);
    bg.chrome.nativePorts.at(-1).disconnect(); status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "handoff_disconnected"); assert.equal(status.owner, "browser"); retained(bg);
  });

  test(`B1 ${browser} unavailable desktop and connect timeout have distinct reasons`, async () => {
    const bg = fixture(browser, { helloFails: true }); await bg.settle();
    let suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.chrome.nativePorts.length === 1, "native connection attempt");
    bg.chrome.nativePorts[0].disconnect(); let status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "desktop_unavailable"); retained(bg);
    suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.chrome.nativePorts.length === 2, "second connection attempt");
    bg.clock.advance(1800); status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "handoff_timeout", "timeout cannot be masked by synchronous disconnect"); retained(bg);
  });

  test(`B1 ${browser} mismatched request reply never resolves a pending handoff`, async () => {
    const bg = fixture(browser, { reply: message => hostReply(message, { requestId: "other-request" }) }); await bg.settle();
    const suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.desktop.requestsOfType("create_task").length === 1, "wire request"); await bg.settle(); retained(bg);
    bg.clock.advance(15000); const status = await finished(bg, browser, suggestions);
    assert.equal(status.reason, "handoff_timeout"); retained(bg);
  });
  test(`B1 ${browser} synchronous native connect failure permits immediate idempotent retry`, async () => {
    const bg = fixture(browser); await bg.settle();
    const connect = bg.chrome.api.runtime.connectNative;
    bg.chrome.api.runtime.connectNative = () => { throw new Error("synthetic-secret"); };
    let status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.reason, "desktop_unavailable"); retained(bg);
    bg.chrome.api.runtime.connectNative = connect;
    const suggestions = fire(bg, browser);
    await bg.waitFor(() => bg.chrome.callsNamed("downloads.cancel").length === 1, "immediate successful retry");
    status = await finished(bg, browser, suggestions); assert.equal(status.owner, "desktop");
  });
  test(`B1 ${browser} history cleanup failure cannot undo a successful ownership transfer`, async () => {
    const bg = fixture(browser, { chrome: { storage: { eraseBrowserHistory: true } } }); await bg.settle();
    const erase = bg.chrome.api.downloads.erase;
    bg.chrome.api.downloads.erase = async query => { await erase(query); throw new Error("synthetic-secret"); };
    const status = await finished(bg, browser, fire(bg, browser));
    assert.equal(status.state, "transferred"); assert.equal(status.owner, "desktop");
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1); assert.equal(bg.chrome.callsNamed("downloads.erase").length, 1);
    assert.equal(bg.desktop.requestsOfType("task_action").length, 0);
  });
}
