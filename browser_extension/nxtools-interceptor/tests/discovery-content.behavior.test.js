"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadContent } = require("./harness/load-content.cjs");
const policy = require("../capture-policy.js");
const resources = fixture => fixture.messages.filter(message => message.type === "content_media_resource").map(message => message.payload);

test("loadedmetadata and actual media attributes bypass unchanged URL/mime/geometry dedup", () => {
  const f = loadContent();
  const audio = new f.HTMLAudioElement({ src: "https://cdn.test/effect.mp3", role: "presentation" });
  Object.assign(audio, { src: "https://cdn.test/effect.mp3", duration: NaN, autoplay: true, controls: false, loop: false });
  f.elements.push(audio);
  f.fire("loadedmetadata", { target: audio });
  assert.equal(resources(f).length, 1);
  audio.duration = 1;
  f.fire("loadedmetadata", { target: audio });
  assert.equal(resources(f).length, 2, "duration arrival republishes original resource");
  assert.equal(policy.discoveryCandidateReason(resources(f).at(-1)), "decorative_audio");
  f.fire("loadedmetadata", { target: audio });
  assert.equal(resources(f).length, 2, "identical facts stay coalesced");
  audio.controls = true;
  f.mutate(audio, "controls");
  assert.equal(resources(f).length, 3);
  assert.equal(policy.discoveryCandidateReason(resources(f).at(-1)), "");
  audio.setAttribute("role", "none");
  f.mutate(audio, "role");
  assert.equal(resources(f).length, 3, "equivalent presentation roles do not generate another message");
  audio.setAttribute("role", "button");
  f.mutate(audio, "role");
  assert.equal(resources(f).length, 4);
});

test("source elements obtain DOM facts from the original media parent", () => {
  const f = loadContent();
  const video = new f.HTMLVideoElement();
  Object.assign(video, { duration: 1, controls: true });
  const source = new f.Element("source", { src: "https://cdn.test/opaque", type: "video/mp4" });
  source.parentElement = video;
  f.fire("loadedmetadata", { target: source });
  assert.equal(resources(f)[0].mediaElement.kind, "video");
  assert.equal(resources(f)[0].mediaElement.controls, true);
});

test("visible polling, hidden events, navigation, explicit deep CSS and performance/page-hook fallbacks remain", () => {
  const f = loadContent();
  const audio = new f.HTMLAudioElement({ src: "https://cdn.test/short.mp3" });
  Object.assign(audio, { src: "https://cdn.test/short.mp3", duration: 1, controls: true });
  const image = new f.Element("div"); image.backgroundImage = "url(https://cdn.test/background.png)";
  f.elements.push(audio, image);
  const scan = f.intervals.find(timer => timer.ms === 7000);
  f.realm.document.visibilityState = "hidden";
  scan.fn();
  assert.equal(resources(f).length, 0);
  f.fire("loadedmetadata", { target: audio });
  assert.equal(resources(f).length, 1, "event discovery still works while hidden");
  f.realm.document.visibilityState = "visible";
  scan.fn();
  assert.equal(resources(f).length, 1, "same facts stay deduplicated");
  f.entries.push({ name: "https://cdn.test/performance.mp4", initiatorType: "video", startTime: 0, transferSize: 100 });
  f.performanceObservers[0].fn({ getEntries: () => f.entries });
  f.fire("nxtools-media-candidate", { detail: { url: "blob:https://example.test/id", mime: "video/mp4", source: "page-hook" } });
  f.dispatch({ type: "content_scan_now" });
  assert.ok(resources(f).some(resource => resource.source === "content-background"));
  assert.ok(resources(f).some(resource => resource.source.startsWith("performance:")));
  assert.ok(resources(f).some(resource => resource.url.startsWith("blob:")));
  f.realm.location.href = "https://example.test/next";
  f.fire("popstate");
  assert.ok(f.messages.some(message => message.type === "content_page_context_changed" && message.href.endsWith("/next")));
  const before = resources(f).length;
  f.fire("loadedmetadata", { target: audio });
  assert.equal(resources(f).length, before + 1, "new page is not deduplicated against retired page");
  assert.deepEqual(f.intervals.map(timer => timer.ms), [7000, 1000, 1000]);
});

test("original chooser merge respects background exclusions and retires delayed page replies", async () => {
  const f = loadContent();
  const local = [{ url: "https://cdn.test/part.m4s", mime: "video/mp4" }, { url: "https://cdn.test/short.mp4" }];
  f.setRuntimeResponse(() => ({ ok: true, choices: [], excludedUrls: [local[0].url] }));
  const choices = await f.realm.NxToolsContent.choices.backgroundResourceChoiceCandidates(null, local);
  assert.deepEqual(Array.from(choices, choice => choice.url), [local[1].url]);
  let release;
  f.setRuntimeResponse(() => new Promise(resolve => { release = resolve; }));
  const delayed = f.realm.NxToolsContent.choices.backgroundResourceChoiceCandidates(null, local);
  f.realm.location.href = "https://example.test/next";
  release({ ok: true, choices: [{ url: "https://cdn.test/old.mp4" }] });
  assert.equal((await delayed).length, 0);
});

test("selection keeps every incomplete decorative fact and manifest precedence across sources", () => {
  const audio = { kind: "audio", duration: 1, autoplay: true, controls: false, loop: false, presentation: true };
  for (const difference of [{ duration: null }, { duration: 0 }, { duration: 3 }, { controls: true }, { loop: true }, { autoplay: false }, { presentation: false }, { kind: "video" }]) {
    assert.equal(policy.discoveryCandidateReason({ url: "https://cdn.test/a.mp3", mediaElement: { ...audio, ...difference } }), "");
  }
  assert.equal(policy.discoveryCandidateReason({ url: "https://cdn.test/list", discovery: policy.normalizeDiscovery(null, "application/dash+xml"), mediaElement: audio }), "");
  assert.equal(policy.discoveryCandidateReason({ url: "https://cdn.test/part", discovery: policy.normalizeDiscovery(null, "audio/iso.segment") }), "fragment");
  assert.equal(policy.discoveryCandidateReason({ url: "https://cdn.test/complete.m4s", size: 1 }), "");
  const merged = policy.mergeCapturedResource({ mediaElement: { kind: "video", controls: true, duration: 1 } }, { mediaElement: { kind: "video", controls: true, duration: 10 } });
  assert.equal(merged.mediaElement.duration, 10);
});
