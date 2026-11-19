"use strict";

// R1 regressions at the real listener/router boundary. Each test loads the
// production background scripts through the harness; nothing is extracted by
// string markers.

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground, browserHandoffReply, createControlledHandoffResponder } = require("./harness/load-background.cjs");

const DOWNLOAD_URL = "https://files.example.test/archive.zip";

function acceptingDesktop() {
  const controlled = createControlledHandoffResponder({ fallback(message) {
    if (message.type === "create_task") {
      return { ok: true, accepted: true, handoffId: message.payload.handoffId, taskId: "task-1" };
    }
    if (message.type === "task_action") return { ok: true };
    return { ok: true, data: {} };
  } });
  return { respond: controlled.respond };
}

function rejectingDesktop() {
  const controlled = createControlledHandoffResponder({ onRequest(message) {
    if (message.type === "prepare_browser_handoff") return browserHandoffReply(message.payload.handoffId, "aborted");
  }, fallback(message) {
    if (message.type === "create_task") {
      return { ok: true, accepted: false, handoffId: message.payload.handoffId, message: "disk full" };
    }
    return { ok: true, data: {} };
  } });
  return { respond: controlled.respond };
}

function downloadItem(overrides = {}) {
  return {
    id: 41,
    url: DOWNLOAD_URL,
    finalUrl: DOWNLOAD_URL,
    filename: "",
    referrer: "https://files.example.test/list",
    startTime: "2026-10-22T00:00:00.000Z",
    totalBytes: 2048,
    canResume: true,
    ...overrides
  };
}

// --- R1.1 -----------------------------------------------------------------

test("R1.1 onDeterminingFilename keeps suggest pending by returning true", async () => {
  const bg = loadBackground({ desktop: acceptingDesktop() });
  await bg.settle();
  let suggestCalls = 0;
  const [returned] = bg.chrome.api.downloads.onDeterminingFilename.fire(downloadItem(), () => { suggestCalls += 1; });
  assert.equal(returned, true, "async suggest requires the listener to return true (Chrome API contract)");
  assert.equal(suggestCalls, 0, "suggest must not resolve before the handoff settles");
  await bg.waitFor(() => bg.chrome.callsNamed("downloads.cancel").length === 1, "controlled browser cancellation");
  assert.equal(suggestCalls, 1, "suggest is resolved exactly once after the controlled handoff settles");
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 1, "held handoff cancels the browser item before commit");
});

test("R1.1 every hand-back path resolves suggest exactly once", async () => {
  for (const scenario of ["rejected", "disabled", "non-http"]) {
    const bg = loadBackground({
      desktop: rejectingDesktop(),
      chrome: { storage: scenario === "disabled" ? { interceptDownloads: false } : {} }
    });
    await bg.settle();
    let suggestCalls = 0;
    const item = scenario === "non-http"
      ? downloadItem({ url: "ftp://files.example.test/a.zip", finalUrl: "ftp://files.example.test/a.zip" })
      : downloadItem();
    const [returned] = bg.chrome.api.downloads.onDeterminingFilename.fire(item, () => { suggestCalls += 1; });
    assert.equal(returned, true, `${scenario}: listener returns true`);
    await bg.waitFor(() => suggestCalls === 1, `${scenario} filename suggestion`);
    assert.equal(suggestCalls, 1, `${scenario}: suggest resolved exactly once`);
    assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0, `${scenario}: browser item untouched`);
  }
});

// --- R1.2 -----------------------------------------------------------------

test("R1.2 Firefox event page (no importScripts) loads the complete background script set", async () => {
  const bg = loadBackground({ browser: "firefox", desktop: acceptingDesktop() });
  await bg.settle();
  assert.equal(typeof bg.realm.NxToolsHandoffPolicy?.completeAcceptedHandoff, "function",
    "handoff policy must be loaded in Firefox");
  assert.equal(typeof bg.realm.NxToolsBatchHandoffPolicy?.selectBatchResources, "function",
    "batch policy must be loaded in Firefox");
  assert.ok(bg.chrome.api.downloads.onCreated.listeners.length >= 1, "Firefox download listener registered");
});

test("R1.2 manifest background.scripts mirrors the service worker import list", () => {
  const bg = loadBackground();
  const scripts = bg.manifest.background.scripts;
  assert.ok(Array.isArray(scripts) && scripts.length > 1, "manifest declares background.scripts for Firefox");
  const entry = bg.manifest.background.service_worker;
  assert.equal(scripts.at(-1), entry, "Firefox loads dependencies first and the shared entry last");
  assert.deepEqual(scripts.slice(0, -1), bg.loaded.slice(1),
    "Firefox dependency order equals the Chrome importScripts order");
  assert.equal(bg.loaded[0], entry);
  const fs = require("node:fs");
  const path = require("node:path");
  for (const file of scripts) {
    assert.ok(fs.existsSync(path.join(__dirname, "..", file)), `${file} exists`);
  }
  assert.equal(bg.manifest.minimum_chrome_version, "121", "Chrome <121 rejects MV3 background.scripts");
});

// --- R1.3 -----------------------------------------------------------------

test("R1.3 drop target reports failure when the desktop does not accept", async () => {
  const bg = loadBackground({ desktop: rejectingDesktop() });
  await bg.settle();
  const response = await bg.sendRuntimeMessage(
    { type: "drop_target_download", url: DOWNLOAD_URL, href: "https://files.example.test/list" },
    bg.senders.content()
  );
  assert.equal(response?.ok, false, "rejected handoff must not be reported as sent");
});

test("R1.3 drop target reports success only for an accepted handoff", async () => {
  const bg = loadBackground({ desktop: acceptingDesktop() });
  await bg.settle();
  const response = await bg.sendRuntimeMessage(
    { type: "drop_target_download", url: DOWNLOAD_URL, href: "https://files.example.test/list" },
    bg.senders.content()
  );
  assert.equal(response?.ok, true);
});

// --- R1.4 -----------------------------------------------------------------

test("R1.4 popup-only messages are refused from a page content script", async () => {
  const bg = loadBackground({ desktop: acceptingDesktop() });
  await bg.settle();
  const response = await bg.sendRuntimeMessage(
    { type: "popup_save_settings", settings: { interceptDownloads: false } },
    bg.senders.content()
  );
  assert.equal(response?.ok, false, "content script cannot change extension settings");
  assert.notEqual(bg.chrome.storage.local.interceptDownloads, false);
  const action = await bg.sendRuntimeMessage(
    { type: "popup_task_action", taskId: "task-1", action: "cancel" },
    bg.senders.content()
  );
  assert.equal(action?.ok, false, "content script cannot drive desktop task actions");
  assert.equal(bg.desktop.requestsOfType("task_action").length, 0);
});

test("R1.4 messages from another extension are ignored", async () => {
  const bg = loadBackground({ desktop: acceptingDesktop() });
  await bg.settle();
  const writesBefore = bg.chrome.callsNamed("storage.local.set").length;
  const response = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.foreign());
  assert.equal(response, undefined, "foreign sender gets no response and no popup state");
  const saved = await bg.sendRuntimeMessage({ type: "popup_save_settings", settings: { interceptDownloads: false } },
    bg.senders.foreign());
  assert.equal(saved, undefined);
  await bg.settle();
  assert.equal(bg.chrome.callsNamed("storage.local.set").length, writesBefore, "no storage write for a foreign sender");
  assert.notEqual(bg.chrome.storage.local.interceptDownloads, false, "foreign sender cannot write settings");
});

test("R1.4 popup settings save keeps only known keys with valid values", async () => {
  const bg = loadBackground({ desktop: acceptingDesktop() });
  await bg.settle();
  await bg.sendRuntimeMessage({
    type: "popup_save_settings",
    settings: {
      interceptDownloads: false,
      dropTargetWidth: 99999,
      dropTargetPosition: "javascript:alert(1)",
      injected: "x",
      openDialog: "yes"
    }
  }, bg.senders.popup());
  const stored = bg.chrome.storage.local;
  assert.equal(stored.interceptDownloads, false, "valid boolean persisted");
  assert.equal("injected" in stored, false, "unknown key dropped");
  assert.ok(stored.dropTargetWidth >= 96 && stored.dropTargetWidth <= 360, "width clamped");
  assert.notEqual(stored.dropTargetPosition, "javascript:alert(1)", "position limited to known values");
  assert.equal(typeof stored.openDialog, "boolean", "type mismatch falls back to boolean default");
});

// --- R1.5 -----------------------------------------------------------------

test("R1.5 browser download does not borrow a resource from the last active tab", async () => {
  const bg = loadBackground({
    desktop: acceptingDesktop(),
    chrome: { tabs: [
      { id: 3, url: "https://other.example.test/", active: true },
      { id: 9, url: "https://files.example.test/list", active: false }
    ] }
  });
  await bg.settle();
  // A cached resource with the same URL exists only in the unrelated active tab.
  await bg.sendRuntimeMessage({
    type: "content_media_resource",
    payload: { url: DOWNLOAD_URL, filename: "wrong-name-from-other-tab.zip", href: "https://other.example.test/" }
  }, bg.senders.content({ id: 3, url: "https://other.example.test/" }));
  bg.chrome.api.tabs.onActivated.fire({ tabId: 3 });
  await bg.settle();
  bg.chrome.api.downloads.onDeterminingFilename.fire(downloadItem({ filename: "" }), () => {});
  await bg.waitFor(() => bg.desktop.requestsOfType("prepare_browser_handoff").length === 1, "download handoff sent");
  const request = bg.desktop.requestsOfType("prepare_browser_handoff")[0];
  assert.ok(request, "handoff sent");
  assert.notEqual(request.message.payload.fileName, "wrong-name-from-other-tab.zip",
    "resource metadata must come from the download's own context");
});

// --- R1.6 -----------------------------------------------------------------

test("R1.6 Content-Disposition filename survives a worker restart without persisting headers", async () => {
  const first = loadBackground({ desktop: acceptingDesktop() });
  await first.settle();
  first.chrome.api.webRequest.onResponseStarted.fire({
    requestId: "r1",
    url: DOWNLOAD_URL,
    tabId: 9,
    type: "main_frame",
    statusCode: 200,
    responseHeaders: [
      { name: "Content-Disposition", value: "attachment; filename=\"real-name.zip\"" },
      { name: "Content-Type", value: "application/zip" }
    ]
  });
  await first.settle();
  const session = first.chrome.storage.session;
  const serialized = JSON.stringify(session);
  assert.ok(serialized.includes("real-name.zip"), "disposition filename persisted to session storage");
  assert.ok(!/cookie|authorization/i.test(serialized), "no credentials in session storage");

  const second = loadBackground({ desktop: acceptingDesktop(), chrome: { sessionStorage: JSON.parse(serialized) } });
  await second.settle();
  second.chrome.api.downloads.onDeterminingFilename.fire(downloadItem({ filename: "" }), () => {});
  await second.waitFor(() => second.desktop.requestsOfType("prepare_browser_handoff").length === 1, "restored worker native handoff");
  const request = second.desktop.requestsOfType("prepare_browser_handoff")[0];
  assert.equal(request?.message.payload.fileName, "real-name.zip", "restored worker keeps authoritative filename");
});
