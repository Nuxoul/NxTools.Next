"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");

const download = {
  id: 81, startTime: "2026-11-10T00:00:00Z", filename: "C:\\Downloads\\example.zip",
  url: "https://origin.example.test/start.zip", finalUrl: "https://cdn.example.test/example.zip",
  referrer: "https://page.example.test/start", totalBytes: 2 * 1024 * 1024,
  cookieStoreId: "fixture-origin"
};
function fixture(browser, chrome = {}, pending = false) {
  return loadBackground({ browser, chrome, desktop: { respond(message) {
    if (message.type !== "create_task") return { ok: true, data: { tasks: [] } };
    return pending
      ? { ok: true, accepted: false, handoffId: message.payload.handoffId,
        data: { handoffState: "awaiting_confirmation", reason: "confirmation_pending" } }
      : { ok: true, accepted: true, handoffId: message.payload.handoffId, taskId: "durable-task",
        data: { handoffState: "persisted", reason: "desktop_accepted" } };
  } } });
}
async function fire(bg, browser, overrides = {}) {
  let suggestions = 0;
  const previous = bg.evaluate("lastHandoff");
  const item = { ...download, ...overrides };
  if (browser === "chrome") {
    assert.equal(bg.chrome.api.downloads.onDeterminingFilename.fire(item, () => { suggestions += 1; })[0], true);
  } else bg.chrome.api.downloads.onCreated.fire(item);
  await bg.settle();
  await bg.waitFor(() => {
    if (browser === "chrome") return suggestions === 1;
    const current = bg.evaluate("lastHandoff");
    return current !== previous && current && !["submitting", "persisted"].includes(current.state);
  }, "current automatic decision");
  const state = JSON.parse(JSON.stringify(bg.evaluate("lastHandoff")));
  if (browser === "chrome") assert.equal(suggestions, 1);
  return state;
}
function retained(bg) {
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
  assert.equal(bg.chrome.callsNamed("downloads.erase").length, 0);
}
function excluded(bg, state, reason = "rule_excluded") {
  assert.equal(state.state, "skipped");
  assert.equal(state.reason, reason);
  assert.equal(state.owner, "browser");
  assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
  assert.equal(bg.chrome.nativePorts.length, 0, "rule decisions do not launch a native host");
  assert.equal(bg.chrome.callsNamed("cookies.getAll").length, 0, "excluded inputs do not read credentials");
  retained(bg);
}

for (const browser of ["chrome", "firefox"]) {
  for (const [name, storage, overrides] of [
    ["file type", { captureFileTypes: "pdf, mp4" }, {}],
    ["known minimum length", { captureMinimumKiB: 4096 }, {}],
    ["page host", { captureExcludedSites: "page.example.test" }, {}],
    ["original host after redirect", { captureExcludedSites: "origin.example.test" }, {}],
    ["final host after redirect", { captureExcludedSites: "cdn.example.test" }, {}],
    ["original address", { captureExcludedAddresses: "origin.example.test/start" }, {}],
    ["redirected address", { captureExcludedAddresses: "CDN.EXAMPLE.TEST/EXAMPLE" }, {}],
  ]) test(`B2 ${browser} ${name} exclusion retains browser without host or Cookie demand`, async () => {
    const bg = fixture(browser, { storage });
    await bg.settle();
    excluded(bg, await fire(bg, browser, overrides));
  });

  test(`B2 ${browser} unknown length follows original confirmation without extension fetch`, async () => {
    const bg = fixture(browser, { storage: { captureFileTypes: ".ZIP", captureMinimumKiB: 4096 } }, true);
    await bg.settle();
    const state = await fire(bg, browser, { totalBytes: -1 });
    assert.equal(state.reason, "confirmation_pending");
    assert.equal(state.state, "awaiting_confirmation");
    assert.equal(bg.desktop.requestsOfType("create_task").length, 1);
    retained(bg);
  });

  test(`B2 ${browser} exact hosts do not exclude sibling or child domains`, async () => {
    const bg = fixture(browser, { storage: { captureExcludedSites: "example.test", captureFileTypes: "zip" } });
    await bg.settle();
    const state = await fire(bg, browser);
    assert.equal(state.state, "transferred");
    assert.equal(bg.desktop.requestsOfType("create_task").length, 1);
  });

  test(`B2 ${browser} restored session pause precedes exclusions and matching rules`, async () => {
    const bg = fixture(browser, {
      storage: { captureFileTypes: "pdf", captureExcludedSites: "page.example.test" },
      sessionStorage: { nxtools_capture_paused: true }
    });
    excluded(bg, await fire(bg, browser), "temporarily_paused");
  });

  test(`B2 ${browser} session pause/resume is separate from persistent enablement and survives recreation`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    const paused = await bg.sendRuntimeMessage({ type: "popup_set_capture_paused", paused: true }, bg.senders.popup());
    assert.equal(paused?.ok, true);
    assert.equal(paused.capturePaused, true);
    assert.equal(bg.chrome.storage.session.nxtools_capture_paused, true);
    assert.notEqual(bg.chrome.storage.local.interceptDownloads, false);
    excluded(bg, await fire(bg, browser), "temporarily_paused");
    const recreated = fixture(browser, {
      storage: bg.chrome.storage.local, sessionStorage: bg.chrome.storage.session
    });
    excluded(recreated, await fire(recreated, browser), "temporarily_paused");
    const resumed = await recreated.sendRuntimeMessage({ type: "popup_set_capture_paused", paused: false }, recreated.senders.popup());
    assert.equal(resumed?.ok, true);
    assert.equal(resumed.capturePaused, false);
    assert.equal((await fire(recreated, browser, { id: 82 })).state, "transferred");
  });

  test(`B2 ${browser} external settings reload applies before the next real download event`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    await bg.chrome.api.storage.local.set({ captureMinimumKiB: 4096 });
    excluded(bg, await fire(bg, browser));
  });

  test(`B2 ${browser} a failed pause save does not publish a false applied state`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    const set = bg.chrome.api.storage.session.set;
    bg.chrome.api.storage.session.set = async () => { throw new Error("synthetic-private-error"); };
    const result = await bg.sendRuntimeMessage({ type: "popup_set_capture_paused", paused: true }, bg.senders.popup());
    assert.equal(result?.ok, false);
    assert.ok(!JSON.stringify(result).includes("synthetic-private-error"));
    bg.chrome.api.storage.session.set = set;
    assert.equal((await fire(bg, browser)).state, "transferred");
  });

  test(`B2 ${browser} automatic direct/Blob substitutes respect pause while explicit drop remains available`, async () => {
    const bg = fixture(browser, { sessionStorage: { nxtools_capture_paused: true } }, true);
    await bg.settle();
    const sender = bg.senders.content({ id: 7, url: download.referrer });
    for (const message of [
      { type: "content_direct_download_request", url: download.url, href: download.referrer, filename: "example.zip" },
      { type: "content_blob_download_request", payload: { url: "blob:https://page.example.test/fixture", filename: "example.zip", href: download.referrer } },
    ]) {
      const result = await bg.sendRuntimeMessage(message, sender);
      assert.equal(result?.ok, false);
      assert.equal(result.fallbackToBrowser, true);
      assert.equal(result.data?.reason, "temporarily_paused");
    }
    assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
    const manual = await bg.sendRuntimeMessage({ type: "drop_target_download", url: download.url, href: download.referrer }, sender);
    assert.equal(manual.ok, true);
    assert.equal(manual.accepted, false);
    assert.equal(manual.data.handoffState, "awaiting_confirmation");
    retained(bg);
  });

  test(`B2 ${browser} duplicate events retain the original durable handoff identity`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    await fire(bg, browser);
    await fire(bg, browser);
    const frames = bg.desktop.requestsOfType("create_task").map(entry => entry.message);
    assert.equal(new Set(frames.map(frame => frame.payload.handoffId)).size, 1);
    assert.ok(frames.every(frame => frame.requestId === frame.payload.handoffId));
  });
}

test("B2 pause diagnostics retain one correlation, fixed outcome and elapsed without payload", async () => {
  const bg = fixture("chrome");
  await bg.settle();
  await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  const result = await bg.sendRuntimeMessage({ type: "popup_set_capture_paused", paused: true }, bg.senders.popup());
  const records = bg.chrome.nativePorts.flatMap(port => port.sent)
    .filter(frame => frame.type === "extension_log" && frame.payload.event.startsWith("capture_pause_"));
  assert.deepEqual(records.map(frame => frame.payload.event), ["capture_pause_started", "capture_pause_applied"]);
  for (const frame of records) {
    assert.equal(frame.payload.correlationId, result.correlationId);
    assert.equal(typeof frame.payload.elapsedMs, "number");
    assert.equal(frame.payload.reason, "paused");
    assert.ok(!/https?:|cookie|authorization|fixture-origin|private-error/.test(JSON.stringify(frame)));
  }
});

for (const browser of ["chrome", "firefox"]) {
  test(`B2 ${browser} Popup rule save normalizes, persists before publication and survives recreation`, async () => {
    const bg = fixture(browser);
    await bg.settle();
    const write = bg.chrome.api.storage.local.set;
    let release;
    bg.chrome.api.storage.local.set = values => new Promise(resolve => {
      release = () => write(values).then(resolve);
    });
    const saving = bg.sendRuntimeMessage({ type: "popup_save_settings", currentView: "settings", settings: {
      interceptDownloads: true, captureFileTypes: ".ZIP; PDF; .ZIP", captureMinimumKiB: 128,
      captureExcludedSites: "EXAMPLE.TEST, https://cdn.test/path", captureExcludedAddresses: "/Private/\n?SKIP=1"
    } }, bg.senders.popup());
    await bg.waitFor(() => Boolean(release));
    assert.equal(bg.evaluate("cachedSettings.captureMinimumKiB"), 0);
    assert.equal(bg.chrome.storage.local.captureMinimumKiB, undefined);
    await release();
    const reply = await saving;
    assert.equal(reply.settings.captureFileTypes, "zip, pdf");
    assert.equal(reply.settings.captureMinimumKiB, 128);
    assert.equal(reply.settings.captureExcludedSites, "example.test\ncdn.test");
    assert.equal(reply.settings.captureExcludedAddresses, "/private/\n?skip=1");
    const recreated = fixture(browser, { storage: bg.chrome.storage.local });
    await recreated.settle();
    const state = await recreated.sendRuntimeMessage({ type: "popup_get_state", currentView: "settings" }, recreated.senders.popup());
    assert.deepEqual(JSON.parse(JSON.stringify(state.settings)), JSON.parse(JSON.stringify(reply.settings)));
  });

  test(`B2 ${browser} failed rule save preserves applied settings and explicit retry applies`, async () => {
    const bg = fixture(browser, { storage: { captureMinimumKiB: 128 } });
    await bg.settle();
    const write = bg.chrome.api.storage.local.set;
    bg.chrome.api.storage.local.set = async () => { throw new Error("synthetic write failure"); };
    const message = { type: "popup_save_settings", currentView: "settings", settings: {
      interceptDownloads: true, captureMinimumKiB: 4096, captureFileTypes: "zip"
    } };
    const failure = await bg.sendRuntimeMessage(message, bg.senders.popup());
    assert.equal(failure.ok, false);
    assert.equal(bg.evaluate("cachedSettings.captureMinimumKiB"), 128);
    assert.equal(bg.chrome.storage.local.captureMinimumKiB, 128);
    bg.chrome.api.storage.local.set = write;
    const success = await bg.sendRuntimeMessage(message, bg.senders.popup());
    assert.equal(success.settings.captureMinimumKiB, 4096);
    const hostCount = bg.chrome.nativePorts.length;
    const decision = await fire(bg, browser);
    assert.equal(decision.reason, "rule_excluded");
    assert.equal(bg.chrome.nativePorts.length, hostCount);
    assert.equal(bg.desktop.requestsOfType("create_task").length, 0);
    retained(bg);
  });

  test(`B2 ${browser} session read failure retains browser and a later read can recover`, async () => {
    const bg = fixture(browser, { sessionGetFails: true });
    await bg.settle();
    excluded(bg, await fire(bg, browser), "temporarily_paused");
    const state = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.equal(state.capturePaused, true);
    assert.equal(state.capturePauseAvailable, false);
    bg.chrome.api.storage.session.get = async defaults => ({ ...defaults, nxtools_capture_paused: false });
    const recovered = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
    assert.equal(recovered.capturePaused, false);
    assert.equal(recovered.capturePauseAvailable, true);
    assert.equal((await fire(bg, browser, { id: 82 })).state, "transferred");
  });

  test(`B2 ${browser} MIME fallback and exact threshold use the original handoff`, async () => {
    const bg = fixture(browser, { storage: { captureFileTypes: "pdf", captureMinimumKiB: 128 } });
    await bg.settle();
    const state = await fire(bg, browser, { filename: "", url: "https://origin.test/get", finalUrl: "https://cdn.test/get",
      mime: "application/pdf", totalBytes: 128 * 1024 });
    assert.equal(state.state, "transferred");
    assert.equal(bg.desktop.requestsOfType("create_task").length, 1);
  });

  test(`B2 ${browser} zero unknown size and Content-Disposition keep confirmation authority`, async () => {
    const bg = fixture(browser, { storage: { captureFileTypes: "pdf", captureMinimumKiB: 4096 } }, true);
    await bg.settle();
    bg.chrome.api.webRequest.onResponseStarted.fire({ requestId: "b2-disposition", tabId: 7, type: "other",
      url: download.finalUrl, statusCode: 200, responseHeaders: [
        { name: "Content-Disposition", value: 'attachment; filename="report.pdf"' },
        { name: "Content-Type", value: "application/pdf" }
      ] });
    await bg.settle();
    const state = await fire(bg, browser, { totalBytes: 0 });
    assert.equal(state.reason, "confirmation_pending");
    const request = bg.desktop.requestsOfType("create_task")[0].message;
    assert.equal(request.payload.fileName, "report.pdf");
    retained(bg);
  });
}

test("B2 failed pause emits one correlated terminal and does not start an idle host", async () => {
  const bg = fixture("chrome");
  await bg.settle();
  bg.chrome.api.storage.session.set = async () => { throw new Error("private failure text"); };
  const idle = await bg.sendRuntimeMessage({ type: "popup_set_capture_paused", paused: true }, bg.senders.popup());
  assert.equal(idle.ok, false);
  assert.equal(bg.chrome.nativePorts.length, 0);
  await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  const failed = await bg.sendRuntimeMessage({ type: "popup_set_capture_paused", paused: true }, bg.senders.popup());
  const records = bg.chrome.nativePorts.flatMap(port => port.sent).filter(frame =>
    frame.type === "extension_log" && frame.payload.correlationId === failed.correlationId);
  assert.deepEqual(records.map(frame => frame.payload.event), ["capture_pause_started", "capture_pause_failed"]);
  assert.equal(records[1].payload.reason, "session_storage_failed");
  assert.equal(typeof records[1].payload.elapsedMs, "number");
  assert.ok(!JSON.stringify(records).includes("private failure text"));
});
