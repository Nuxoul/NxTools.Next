"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");
const item = { id: 91, tabId: 7, url: "https://fixture.test/archive.zip", finalUrl: "https://fixture.test/archive.zip",
  filename: "archive.zip", referrer: "https://fixture.test/page", startTime: "2026-11-13T00:00:00Z", state: "in_progress", paused: false };
const recoveryId = "12345678-1234-4abc-8def-1234567890ab";
function fixture(options = {}) {
  let phase = options.phase || "pending";
  let handoffId = options.recovery ? recoveryId : "";
  let transfers = 0;
  const events = [];
  const browser = { ...item, ...(options.browserState || {}) };
  const bg = loadBackground({ browser: options.firefox ? "firefox" : "chrome", chrome: {
    tabs: [{ id: 7, active: true, url: item.referrer }], storage: { openDialog: options.openDialog !== false, captureResources: false },
    sessionGetFails: options.sessionGetFails,
    sessionStorage: options.recovery ? { nxtools_browser_handoffs: [{ downloadId: 91, handoffId: recoveryId,
      browserStartedAt: item.startTime, phase: options.recoveryPhase || "browser_cancelled", taskId: "held-task", startedAt: 1800000000000,
      eraseBrowserHistory: false }] } : {}
  }, desktop: { respond(message) {
    const id = message.payload?.handoffId || handoffId;
    const reply = () => ({ ok: true,
      accepted: ["held", "committed"].includes(phase), handoffId: id,
      ...(["held", "committed"].includes(phase) ? { taskId: "held-task" } : {}), data: {
        browserHandoffState: phase, handoffState: phase === "pending" ? "awaiting_confirmation" : phase === "held" ? "persisted" : phase === "committed" ? "transferred" : "rejected",
        reason: phase === "pending" ? "confirmation_pending" : phase === "held" ? "desktop_accepted" : phase === "committed" ? "browser_transferred" : "confirmation_cancelled", queued: phase === "held"
      } });
    if (message.type === "prepare_browser_handoff" || message.type === "create_task") {
      events.push(message.type); handoffId = id; return reply();
    }
    if (message.type === "get_browser_handoff") { events.push("status"); return reply(); }
    if (message.type === "abort_browser_handoff") { events.push("abort"); phase = "aborted"; return reply(); }
    if (message.type === "commit_browser_handoff") {
      events.push("commit");
      assert.equal(browser.state, "interrupted", "browser cancellation precedes desktop activation");
      assert.equal(browser.paused, false, "a paused Firefox item is not cancellation evidence");
      assert.equal(message.payload.taskId, "held-task");
      if (phase !== "committed") transfers += 1;
      phase = "committed"; return reply();
    }
    if (message.type === "get_snapshot") return { ok: true, data: { tasks: [] } };
    return { ok: true };
  } } });
  const record = (name, id) => { events.push(name); bg.chrome.calls.push({ name: `downloads.${name}`, args: [id] }); };
  let searchFailure = options.searchFailsAfterCancel;
  bg.chrome.api.downloads.search = async query => {
    if (events.includes("cancel") && searchFailure) throw new Error("history temporarily unavailable");
    if (options.missingHistory) return [];
    return [{ ...browser }].filter(value => query.id == null || value.id === query.id);
  };
  bg.chrome.api.downloads.pause = async id => {
    record("pause", id);
    if (options.pauseFails) throw new Error("cannot pause");
    if (!options.falsePause) {
      browser.paused = true;
      if (options.firefox) Object.assign(browser, { state: "interrupted", error: "USER_CANCELED" });
    }
  };
  bg.chrome.api.downloads.resume = async id => {
    record("resume", id);
    Object.assign(browser, { state: "in_progress", paused: false });
    delete browser.error;
  };
  bg.chrome.api.downloads.cancel = async id => { record("cancel", id); if (options.cancelFails) throw new Error("cannot cancel");
    if (options.falseCancel) return;
    browser.state = "interrupted"; browser.error = "USER_CANCELED"; browser.paused = false;
    if (options.cancelReplyFails) throw new Error("cancel reply unavailable"); };
  let suggestions = 0;
  const fire = () => (bg.chrome.api.downloads.onDeterminingFilename || bg.chrome.api.downloads.onCreated).fire({ ...item }, () => { suggestions += 1; });
  return { bg, events, browser, fire, phase(value) { phase = value; }, restoreSearch() { searchFailure = false; },
    transfers: () => transfers, suggestions: () => suggestions };
}
async function finish(f) { await f.bg.waitFor(() => f.events.includes("commit") || f.events.includes("resume")); await f.bg.settle(); }

for (const firefox of [false, true]) test(`${firefox ? "Firefox" : "Chrome"} controls the original, waits for confirmation and commits only after cancellation`, async () => {
  const f = fixture({ firefox }); await f.bg.settle(); f.fire();
  await f.bg.waitFor(() => f.events.includes("prepare_browser_handoff")); await f.bg.settle();
  assert.equal(f.browser.paused, true);
  assert.deepEqual(f.events.filter(event => event !== "status"), ["pause", "prepare_browser_handoff"]);
  assert.equal(f.transfers(), 0);
  assert.equal(f.events.includes("cancel"), false);
  const journal = JSON.stringify(f.bg.chrome.storage.session.nxtools_browser_handoffs);
  assert.ok(journal.includes("91"));
  assert.ok(!/https:|cookie|headers|archive\.zip/i.test(journal), "session journal contains no request payload");
  f.phase("held"); f.bg.clock.advance(1000); await finish(f);
  assert.equal(f.transfers(), 1);
  assert.ok(f.events.indexOf("cancel") < f.events.indexOf("commit"));
  assert.equal(f.browser.state, "interrupted");
  assert.equal((f.bg.chrome.storage.session.nxtools_browser_handoffs || []).length, 0);
  if (!firefox) assert.equal(f.suggestions(), 1);
});
for (const firefox of [false, true]) for (const phase of ["aborted", "expired"]) test(`${firefox ? "Firefox" : "Chrome"} ${phase} confirmation restores the paused browser without desktop activation`, async () => {
  const f = fixture({ firefox }); await f.bg.settle(); f.fire();
  await f.bg.waitFor(() => f.bg.chrome.storage.session.nxtools_browser_handoffs?.[0]?.phase === "waiting_confirmation", "pending browser confirmation");
  await f.bg.settle();
  assert.equal(f.browser.paused, true);
  f.phase(phase); f.bg.clock.advance(1000); await finish(f);
  assert.equal(f.browser.paused, false);
  assert.equal(f.browser.state, "in_progress");
  assert.equal(f.transfers(), 0);
  assert.equal(f.events.includes("cancel"), false);
  assert.equal(f.events.includes("commit"), false);
});
for (const control of ["pauseFails", "falsePause"]) test(`${control} retains browser ownership before desktop demand`, async () => {
  const f = fixture({ [control]: true }); await f.bg.settle(); f.fire(); await f.bg.settle();
  assert.equal(f.events.includes("prepare_browser_handoff"), false);
  assert.equal(f.events.includes("create_task"), false);
  assert.equal(f.bg.chrome.nativePorts.length, 0);
  assert.equal(f.transfers(), 0);
  assert.equal(f.browser.paused, false);
});
test("browser cancellation failure aborts the held desktop task before resuming the browser", async () => {
  const f = fixture({ phase: "held", cancelFails: true }); await f.bg.settle(); f.fire(); await finish(f);
  assert.ok(f.events.indexOf("abort") >= 0);
  assert.ok(f.events.indexOf("abort") < f.events.indexOf("resume"));
  assert.equal(f.transfers(), 0);
  assert.equal(f.events.includes("commit"), false);
  assert.equal(f.browser.state, "in_progress");
  assert.equal(f.browser.paused, false);
});
test("duplicate download callbacks keep one controlled handoff", async () => {
  const f = fixture(); await f.bg.settle(); f.fire(); f.fire(); await f.bg.settle();
  assert.equal(f.events.filter(value => value === "prepare_browser_handoff").length, 1);
  f.phase("held"); f.bg.clock.advance(1000); await finish(f);
  assert.equal(f.transfers(), 1);
});
test("worker recreation commits a durably held task whose original browser download is already cancelled", async () => {
  const f = fixture({ phase: "held", recovery: true, browserState: { state: "interrupted", error: "USER_CANCELED", paused: false } });
  await f.bg.settle();
  assert.equal(f.events.filter(value => value === "commit").length, 1);
  assert.equal(f.transfers(), 1);
  assert.equal(f.events.includes("prepare_browser_handoff"), false);
  assert.equal(f.events.includes("resume"), false);
});
test("cancel verification failure retains Held and recovers without aborting the cancelled original", async () => {
  const f = fixture({ phase: "held", searchFailsAfterCancel: true }); await f.bg.settle(); f.fire();
  await f.bg.waitFor(() => f.events.includes("cancel")); await f.bg.settle();
  assert.equal(f.events.includes("abort"), false);
  assert.equal(f.events.includes("commit"), false);
  assert.equal(f.transfers(), 0);
  assert.equal(f.bg.chrome.storage.session.nxtools_browser_handoffs[0].phase, "cancelling");
  f.restoreSearch(); await f.bg.evaluate("restoreBrowserHandoffs()"); await f.bg.settle();
  assert.equal(f.transfers(), 1);
  assert.equal(f.events.filter(event => event === "cancel").length, 1);
});
test("proven cancellation survives erased browser history on worker recreation", async () => {
  const f = fixture({ phase: "held", recovery: true, missingHistory: true,
    browserState: { state: "interrupted", error: "USER_CANCELED", paused: false } });
  await f.bg.settle();
  assert.equal(f.transfers(), 1);
  assert.equal(f.events.includes("abort"), false);
});
test("unknown cancellation with missing history never guesses ownership", async () => {
  const f = fixture({ phase: "held", recovery: true, recoveryPhase: "cancelling", missingHistory: true });
  await f.bg.settle();
  assert.equal(f.transfers(), 0);
  assert.equal(f.events.includes("abort"), false);
  assert.equal(f.bg.chrome.storage.session.nxtools_browser_handoffs.length, 1);
});
test("ineffective cancellation aborts Held before restoring browser transfer", async () => {
  const f = fixture({ phase: "held", falseCancel: true }); await f.bg.settle(); f.fire(); await finish(f);
  assert.equal(f.transfers(), 0);
  assert.ok(f.events.indexOf("abort") < f.events.indexOf("resume"));
  assert.equal(f.browser.paused, false);
});
test("unavailable session persistence leaves the browser untouched without desktop intake", async () => {
  const f = fixture(); await f.bg.settle();
  f.bg.chrome.api.storage.session.set = async () => { throw new Error("session storage unavailable"); };
  f.fire(); await f.bg.settle();
  assert.equal(f.events.includes("pause"), false);
  assert.equal(f.events.includes("prepare_browser_handoff"), false);
  assert.equal(f.browser.paused, false);
});
test("observed successful cancellation survives an API reply failure", async () => {
  const f = fixture({ phase: "held", cancelReplyFails: true }); await f.bg.settle(); f.fire(); await finish(f);
  assert.equal(f.transfers(), 1);
  assert.equal(f.events.includes("abort"), false);
  assert.equal(f.events.includes("resume"), false);
});
test("unreadable recovery journal leaves the original in the browser without overwriting recovery", async () => {
  const f = fixture({ sessionGetFails: true }); await f.bg.settle(); f.fire(); await f.bg.settle();
  assert.equal(f.events.includes("pause"), false);
  assert.equal(f.events.includes("prepare_browser_handoff"), false);
  assert.equal(f.bg.chrome.callsNamed("storage.session.set").filter(call => call.args[0].nxtools_browser_handoffs).length, 0);
});
for (const failure of ["cancelFails", "falseCancel"]) test(`Firefox ${failure} restores the original without desktop activation`, async () => {
  const f = fixture({ firefox: true, phase: "held", [failure]: true });
  await f.bg.settle(); f.fire(); await finish(f);
  assert.ok(f.events.indexOf("cancel") >= 0);
  assert.ok(f.events.indexOf("abort") > f.events.indexOf("cancel"));
  assert.ok(f.events.indexOf("resume") > f.events.indexOf("abort"));
  assert.equal(f.events.includes("commit"), false);
  assert.equal(f.transfers(), 0);
  assert.equal(f.browser.state, "in_progress");
  assert.equal(f.browser.paused, false);
  assert.equal(f.bg.chrome.callsNamed("downloads.erase").length, 0);
});

test("Firefox recovery retries cancellation instead of committing a still-paused original", async () => {
  const f = fixture({ firefox: true, phase: "held", recovery: true, recoveryPhase: "cancelling", falseCancel: true,
    browserState: { state: "interrupted", error: "USER_CANCELED", paused: true } });
  await finish(f);
  assert.equal(f.events.includes("commit"), false);
  assert.equal(f.transfers(), 0);
  assert.ok(f.events.indexOf("cancel") >= 0);
  assert.ok(f.events.indexOf("abort") > f.events.indexOf("cancel"));
  assert.ok(f.events.indexOf("resume") > f.events.indexOf("abort"));
  assert.equal(f.browser.state, "in_progress");
  assert.equal(f.browser.paused, false);
});
