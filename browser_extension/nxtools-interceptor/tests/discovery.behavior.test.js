"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");

function fixture(options = {}) {
  return loadBackground({
    ...options,
    chrome: { tabs: [{ id: 7, active: true, url: "https://example.test/watch", title: "fixture" }], ...options.chrome },
    desktop: options.desktop || { respond: message => message.type === "get_snapshot" ? { ok: true, type: "snapshot", data: { tasks: [] } } : undefined }
  });
}
async function response(bg, url, mime, extra = {}) {
  bg.chrome.api.webRequest.onResponseStarted.fire({ requestId: url, tabId: 7, frameId: 0,
    url, type: "media", statusCode: 200, initiator: "https://example.test",
    responseHeaders: [{ name: "Content-Type", value: mime }, { name: "Content-Length", value: "1024" }], ...extra });
  await bg.settle();
}
async function popup(bg) {
  return bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "tasks" }, bg.senders.popup());
}

test("response-identified ISO fragments stay observations, not Popup candidates", async () => {
  const bg = fixture();
  await bg.settle();
  await response(bg, "https://cdn.test/master", "application/vnd.apple.mpegurl");
  await response(bg, "https://cdn.test/short.mp4", "video/mp4");
  await response(bg, "https://cdn.test/part.m4s", "video/iso.segment");
  await response(bg, "https://cdn.test/part.m4s", "video/iso.segment");
  const state = await popup(bg);
  assert.deepEqual(Array.from(state.currentResources, resource => resource.url).sort(),
    ["https://cdn.test/master", "https://cdn.test/short.mp4"], "only complete/unknown resources are selectable");
  bg.clock.advance(600);
  await bg.settle();
  const observations = bg.chrome.storage.local.nxtools_resource_cache["7"];
  assert.equal(observations.length, 3, "one original bounded observation per stable URL, including suppressed facts");
  assert.equal(observations.find(resource => resource.url.endsWith("part.m4s")).discovery.fragment, true);
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
});

async function pageResource(bg, url, extra = {}, tabId = 7) {
  await bg.sendRuntimeMessage({ type: "content_media_resource", payload: { url, mime: "audio/mpeg", size: 1024,
    href: "https://example.test/watch", ...extra } }, bg.senders.content({ id: tabId, url: "https://example.test/watch" }));
  await bg.settle();
}
async function choices(bg) {
  return bg.sendRuntimeMessage({ type: "content_get_resource_choices", href: "https://example.test/watch" }, bg.senders.content({ id: 7, url: "https://example.test/watch" }));
}
test("range evidence excludes fragments but preserves complete, manifest, attachment and stable sources", async () => {
  const bg = fixture();
  await bg.settle();
  const headers = (mime, range, disposition = "") => [{ name: "Content-Type", value: mime },
    { name: "Content-Range", value: range }, { name: "Content-Disposition", value: disposition }];
  await response(bg, "https://cdn.test/fragment?range=0-99", "video/mp4", { statusCode: 206, responseHeaders: headers("video/mp4", "bytes 0-99/1000") });
  await response(bg, "https://cdn.test/stable.mp4", "video/mp4", { statusCode: 206, responseHeaders: headers("video/mp4", "bytes 0-99/1000") });
  await response(bg, "https://cdn.test/complete.ts?range=0-99", "video/mp2t", { responseHeaders: headers("video/mp2t", "bytes 0-99/100") });
  await response(bg, "https://cdn.test/unknown.m4s", "video/mp4");
  await response(bg, "https://cdn.test/manifest?range=0-99", "application/dash+xml", { responseHeaders: headers("application/dash+xml", "bytes 0-99/1000") });
  await response(bg, "https://cdn.test/save?range=0-99", "video/mp4", { responseHeaders: headers("video/mp4", "bytes 0-99/1000", "attachment; filename=save.mp4") });
  await response(bg, "https://cdn.test/opaque?id=1", "application/octet-stream");
  await response(bg, "https://cdn.test/ordinary.zip", "application/zip");
  await response(bg, "https://cdn.test/malformed?range=abc", "video/mp4", { responseHeaders: headers("video/mp4", "bytes 0-99/1000") });
  await response(bg, "https://cdn.test/archive.zip?range=0-99", "application/zip", { type: "xmlhttprequest", statusCode: 206, responseHeaders: headers("application/zip", "bytes 0-99/1000") });
  const state = await popup(bg);
  assert.equal(state.currentResources.length, 9);
  assert.ok(!state.currentResources.some(resource => resource.url.includes("/fragment?")));
  const overlay = await choices(bg);
  assert.ok(!overlay.choices.some(resource => resource.url.includes("/fragment?")), "same selection applies before chooser enrichment");
});

test("DOM decoration needs all observed facts and stronger playable evidence wins", async () => {
  const bg = fixture();
  await bg.settle();
  const decorative = { kind: "audio", duration: 1, controls: false, autoplay: true, loop: false, presentation: true };
  await pageResource(bg, "https://cdn.test/effect.mp3", { mediaElement: decorative });
  await pageResource(bg, "https://cdn.test/effect.mp3", { source: "performance" });
  await pageResource(bg, "https://cdn.test/short.mp3", { mediaElement: { ...decorative, autoplay: false } });
  await pageResource(bg, "https://cdn.test/unknown.mp3", { mediaElement: { ...decorative, duration: null } });
  await pageResource(bg, "blob:https://example.test/blob", { mediaElement: decorative });
  assert.deepEqual(Array.from((await popup(bg)).currentResources, r => r.url).sort(),
    ["blob:https://example.test/blob", "https://cdn.test/short.mp3", "https://cdn.test/unknown.mp3"]);
  await pageResource(bg, "https://cdn.test/effect.mp3", { mediaElement: { ...decorative, controls: true } });
  await pageResource(bg, "https://cdn.test/effect.mp3", { mediaElement: decorative });
  assert.equal((await popup(bg)).currentResources.length, 4, "independently controlled use cannot be erased by an incidental decoration");
  await response(bg, "https://cdn.test/direct", "video/iso.segment");
  await pageResource(bg, "https://cdn.test/direct", { mediaElement: { kind: "video", controls: true }, mime: "video/mp4" });
  assert.ok((await popup(bg)).currentResources.some(r => r.url.endsWith("/direct")));
});

test("retained candidates survive fragment capacity pressure and negative facts survive storage restore", async () => {
  const bg = fixture();
  await bg.settle();
  await response(bg, "https://cdn.test/keep.mp4", "video/mp4");
  for (let index = 0; index < 125; index++) {
    bg.clock.advance(1);
    await response(bg, `https://cdn.test/part${index}`, "video/iso.segment");
  }
  assert.deepEqual(Array.from((await popup(bg)).currentResources, r => r.url), ["https://cdn.test/keep.mp4"]);
  bg.clock.advance(600);
  await bg.settle();
  assert.equal(bg.chrome.storage.local.nxtools_resource_cache["7"].length, 120);
  const restored = fixture({ chrome: { storage: bg.chrome.storage.local } });
  await restored.settle();
  await pageResource(restored, "https://cdn.test/part124", { source: "performance", mime: "video/mp4" });
  assert.deepEqual(Array.from((await popup(restored)).currentResources, r => r.url), ["https://cdn.test/keep.mp4"]);
  restored.clock.advance(20 * 60 * 1000 + 1);
  assert.equal((await popup(restored)).currentResources.length, 0, "original TTL retires both kinds of observations");
});

for (const transition of ["navigation", "history", "context", "close", "replace"]) {
  test(`late captures retire on ${transition} without touching another tab`, async () => {
    const bg = fixture({ chrome: { tabs: [{ id: 7, active: true, url: "https://example.test/watch" }, { id: 8, url: "https://other.test/watch" }] } });
    await bg.settle();
    await pageResource(bg, "https://cdn.test/other.mp4", { mime: "video/mp4" }, 8);
    const originalGet = bg.chrome.api.tabs.get;
    const pending = [];
    bg.chrome.api.tabs.get = id => id === 7 ? new Promise(resolve => pending.push(resolve)) : originalGet(id);
    bg.chrome.api.webRequest.onResponseStarted.fire({ requestId: "late", tabId: 7, url: "https://cdn.test/late.mp4", type: "media", responseHeaders: [{ name: "Content-Type", value: "video/mp4" }] });
    bg.chrome.api.webRequest.onBeforeSendHeaders.fire({ requestId: "request-late", tabId: 7, url: "https://cdn.test/request.mp4", type: "media", requestHeaders: [] });
    const pageReply = pageResource(bg, "https://cdn.test/page.mp4", { mime: "video/mp4" });
    await bg.waitFor(() => pending.length >= 3);
    await bg.settle();
    if (transition === "navigation") bg.chrome.api.webNavigation.onCommitted.fire({ tabId: 7, frameId: 0, url: "https://example.test/next" });
    if (transition === "history") bg.chrome.api.webNavigation.onHistoryStateUpdated.fire({ tabId: 7, frameId: 0, url: "https://example.test/next" });
    if (transition === "context") await bg.sendRuntimeMessage({ type: "content_page_context_changed", href: "https://example.test/next" }, bg.senders.content());
    if (transition === "close") bg.chrome.api.tabs.onRemoved.fire(7);
    if (transition === "replace") bg.chrome.api.tabs.onReplaced.fire(8, 7);
    bg.chrome.api.tabs.get = originalGet;
    for (const resolve of pending) resolve({ id: 7, url: "https://example.test/watch" });
    await pageReply;
    await bg.settle();
    const state = await popup(bg);
    assert.equal(state.currentResources.length, 0, "retired callbacks cannot recreate old observations");
    assert.equal(state.otherResources.length, 1, "replacement's independent observations remain owned by that tab");
    assert.equal(state.otherResources[0].url, "https://cdn.test/other.mp4");
  });
}

test("explicit demand keeps deep fallback, clears old throttle and rejects retired scan thumbnails", async () => {
  const bg = fixture();
  await bg.settle();
  let release;
  bg.chrome.setTabMessageHandler((id, message) => message.type === "content_scan_now" ? new Promise(resolve => { release = resolve; }) : { ok: true });
  const first = bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "resources" }, bg.senders.popup());
  await bg.waitFor(() => Boolean(release));
  bg.chrome.api.webNavigation.onHistoryStateUpdated.fire({ tabId: 7, frameId: 0, url: "https://example.test/next" });
  release({ ok: true, pageThumbnail: "https://cdn.test/old.jpg" });
  await first;
  assert.equal((await popup(bg)).activeTab.thumbnail, "");
  bg.chrome.setTabMessageHandler(() => ({ ok: true, pageThumbnail: "https://cdn.test/new.jpg" }));
  const next = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "resources" }, bg.senders.popup());
  assert.equal(bg.chrome.tabMessages.filter(call => call.message.type === "content_scan_now").length, 2, "new page demand is not throttled by retired page");
  assert.equal(next.activeTab.thumbnail, "https://cdn.test/new.jpg");
  await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "resources" }, bg.senders.popup());
  assert.equal(bg.chrome.tabMessages.filter(call => call.message.type === "content_scan_now").length, 2, "same-page original throttle remains");
});

test("cross-tab enrichment never transfers DOM or response selection identity", async () => {
  const bg = fixture({ chrome: { tabs: [{ id: 7, active: true, url: "https://example.test/watch" }, { id: 8, url: "https://other.test/watch" }] } });
  await bg.settle();
  await response(bg, "https://cdn.test/shared", "video/iso.segment");
  await pageResource(bg, "https://cdn.test/shared", { mime: "video/mp4", mediaElement: { kind: "video", controls: true } }, 8);
  const state = await popup(bg);
  assert.equal(state.currentResources.length, 0);
  assert.equal(state.otherResources.length, 1);
  assert.equal(state.otherResources[0].tabId, 8);
});

test("chooser response filters local weak fallbacks without overriding current DOM playability", async () => {
  const bg = fixture();
  await bg.settle();
  const url = "https://cdn.test/part.m4s";
  await response(bg, url, "video/iso.segment");
  const weak = await bg.sendRuntimeMessage({ type: "content_get_resource_choices", localCandidates: [{ url, mime: "video/mp4" }] }, bg.senders.content());
  assert.deepEqual(Array.from(weak.excludedUrls || []), [url]);
  const played = await bg.sendRuntimeMessage({ type: "content_get_resource_choices", localCandidates: [{ url, mediaElement: { kind: "video", controls: true } }] }, bg.senders.content());
  assert.deepEqual(Array.from(played.excludedUrls || []), []);
});

test("same-URL MIME enrichment cannot create selection evidence in another tab", async () => {
  const bg = fixture({ chrome: { tabs: [{ id: 7, active: true, url: "https://example.test/watch" }, { id: 8, url: "https://other.test/watch" }] } });
  await bg.settle();
  await pageResource(bg, "https://cdn.test/shared-weak", { mime: "video/mp4", source: "performance" }, 8);
  await response(bg, "https://cdn.test/shared-weak", "video/iso.segment");
  const state = await popup(bg);
  assert.equal(state.currentResources.length, 0);
  assert.equal(state.otherResources.length, 1);
});

test("redirect final response owns fragment facts and original disposition naming", async () => {
  const bg = fixture();
  await bg.settle();
  bg.chrome.api.webRequest.onBeforeRedirect.fire({ requestId: "redirect", tabId: 7, url: "https://cdn.test/start", redirectUrl: "https://cdn.test/final?range=0-99", initiator: "https://example.test/watch", responseHeaders: [{ name: "Content-Disposition", value: "attachment; filename=clip.mp4" }] });
  await response(bg, "https://cdn.test/final?range=0-99", "video/mp4", { requestId: "redirect", statusCode: 206,
    responseHeaders: [{ name: "Content-Type", value: "video/mp4" }, { name: "Content-Range", value: "bytes 0-99/1000" }] });
  assert.equal((await popup(bg)).currentResources.length, 0);
  await response(bg, "https://cdn.test/final?range=0-99", "video/mp4", { responseHeaders: [{ name: "Content-Type", value: "video/mp4" }, { name: "Content-Disposition", value: "attachment; filename=clip.mp4" }] });
  const retained = (await popup(bg)).currentResources[0];
  assert.equal(retained.filename, "clip.mp4");
  assert.equal(retained.discovery.attachment, true);
});

test("explicit manual and batch handoffs retain authority over suppressed discovery observations", async () => {
  const bg = fixture({ desktop: { respond(message) {
    if (message.type === "create_task") return { ok: true, accepted: false, handoffId: message.payload.handoffId,
      data: { handoffState: "awaiting_confirmation", reason: "confirmation_pending", queued: false } };
    if (message.type === "create_tasks") return { ok: true, data: { queued: true,
      items: message.payload.items.map((item, index) => ({ handoffId: item.handoffId, accepted: true, taskId: `task-${index}` })) } };
    return { ok: true, data: { tasks: [] } };
  } } });
  await bg.settle();
  const url = "https://cdn.test/fragment";
  await response(bg, url, "video/iso.segment");
  assert.equal((await popup(bg)).currentResources.length, 0);
  const manual = await bg.sendRuntimeMessage({ type: "popup_send_resource", resourceId: `7:${url}` }, bg.senders.popup());
  assert.equal(manual.ok, true);
  assert.equal(manual.accepted, false);
  assert.equal(manual.data.reason, "confirmation_pending");
  const batch = await bg.sendRuntimeMessage({ type: "popup_send_resources", resourceIds: [`7:${url}`] }, bg.senders.popup());
  assert.equal(batch.ok, true);
  assert.equal(bg.desktop.requestsOfType("create_tasks")[0].message.payload.items[0].openDialog, false);
  assert.equal(bg.chrome.callsNamed("downloads.cancel").length, 0);
});

test("same-URL enrichment preserves original frame, page key and capture source", async () => {
  const bg = fixture({ chrome: { tabs: [{ id: 7, active: true, url: "https://example.test/watch" }, { id: 8, url: "https://other.test/watch" }] } });
  await bg.settle();
  const url = "https://cdn.test/frame.mp4";
  for (const [tabId, frameId, pageKey, source] of [[7, 0, "page-a", "page-hook"], [8, 3, "page-b", "content-media-source"]]) {
    await bg.sendRuntimeMessage({ type: "content_media_resource", payload: { url, mime: "video/mp4", pageKey, source } },
      bg.senders.content({ id: tabId, url: `https://example.test/${pageKey}` }, frameId));
  }
  const state = await popup(bg);
  assert.equal(state.currentResources[0].frameId, 0);
  assert.equal(state.currentResources[0].pageKey, "page-a");
  assert.equal(state.currentResources[0].source, "page-hook");
});

test("Blob DOM playability does not transfer to a different associated network source", async () => {
  const bg = fixture();
  await bg.settle();
  const sourceUrl = "https://cdn.test/blob-part";
  await response(bg, sourceUrl, "video/iso.segment");
  await pageResource(bg, "blob:https://example.test/aggregate", { mime: "video/mp4", sourceUrl, sourceMime: "video/iso.segment", mediaElement: { kind: "video", controls: true } });
  const state = await popup(bg);
  assert.deepEqual(Array.from(state.currentResources, resource => resource.url), ["blob:https://example.test/aggregate"]);
});
