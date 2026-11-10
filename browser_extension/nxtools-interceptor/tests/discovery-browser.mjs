// Supplied-Page REF-12 workload. No TaskSpace or installed-extension ownership.
import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { runPopup } from "./intake-control-browser.mjs";
import { createProgress, runPageJob } from "./browser-stages.mjs";

export const WORKLOAD = Object.freeze({ rounds: 24, fragments: 80, duplicates: 24, idleMs: 8100,
  settings: { captureResources: true, interceptDownloads: true, captureMinimumKiB: 0, captureFileTypes: "",
    captureExcludedSites: "", captureExcludedAddresses: "", mediaOverlayEnabled: true } });
export const TRIAL_ORDER = Object.freeze([
  { group: 1, variant: "baseline" }, { group: 1, variant: "candidate" },
  { group: 2, variant: "candidate" }, { group: 2, variant: "baseline" },
  { group: 3, variant: "baseline" }, { group: 3, variant: "candidate" }
]);
function directory(value) {
  if (value instanceof URL) return new URL(value.href.endsWith("/") ? value.href : value.href + "/");
  if (!path.isAbsolute(value)) throw new Error("source directory must be absolute");
  return pathToFileURL(value + (value.endsWith(path.sep) ? "" : path.sep));
}
export async function readSourceBundle(sourceDirectory) {
  const root = directory(sourceDirectory);
  const read = name => fs.readFile(new URL(name, root), "utf8");
  const manifest = JSON.parse(await read("manifest.json"));
  const files = [...new Set([...manifest.background.scripts, ...manifest.content_scripts[0].js, "page-hook.js"])];
  const sources = Object.fromEntries(await Promise.all(files.map(async file => [file, await read(file)])));
  return { root: root.href, manifest, sources };
}
export function taskDuration(metrics) {
  const value = metrics.metrics?.find(metric => metric.name === "TaskDuration")?.value;
  if (!Number.isFinite(value)) throw new Error("CDP Performance.TaskDuration is unavailable");
  return value;
}
function distribution(values, count = 3) {
  if (!values.length || values.length !== count || values.some(value => !Number.isFinite(value) || value < 0)) throw new Error("three finite nonnegative trials required");
  const sorted = [...values].sort((left, right) => left - right), last = sorted.length - 1;
  return { raw: values, median: (sorted[Math.floor(last / 2)] + sorted[Math.ceil(last / 2)]) / 2, slowest: sorted[last] };
}
export function summarizeTrials(trials) {
  return Object.fromEntries(["baseline", "candidate"].map(variant => {
    const rows = trials.filter(trial => trial.variant === variant);
    return [variant, { activeTaskMs: distribution(rows.map(row => row.active.taskMs)), idleTaskMs: distribution(rows.map(row => row.idle.taskMs)),
      discoveryLatencyMs: distribution(rows.map(row => distribution(row.active.discoveryLatenciesMs, row.active.discoveryLatenciesMs.length).median)),
      slowestDiscoveryLatencyMs: distribution(rows.map(row => Math.max(...row.active.discoveryLatenciesMs))),
      retainedCandidates: rows.map(row => row.active.candidates.length), observations: rows.map(row => row.active.observations.length) }];
  }));
}

async function prepare(page, bundle, apiSource, step) {
  await step("prepare.document", async () => {
    await page.goto("about:blank");
    await page.cdp("Network.enable");
    await page.cdp("Network.setBlockedURLs", { urls: ["http://*", "https://*", "ws://*", "wss://*"] });
    await page.cdp("Emulation.setDeviceMetricsOverride", { width: 900, height: 900, deviceScaleFactor: 1, mobile: false });
  });
  await step("prepare.frame.create", () => page.evaluate(() => {
    document.body.replaceChildren();
    window.__errors = [];
    addEventListener("error", event => { if (event.message) __errors.push(event.message); });
    addEventListener("unhandledrejection", event => __errors.push(String(event.reason)));
    const frame = document.createElement("iframe");
    frame.id = "discovery-background";
    frame.hidden = true;
    frame.onload = () => { frame.dataset.ready = "true"; };
    frame.srcdoc = "<!doctype html><html><head></head><body></body></html>";
    document.body.appendChild(frame);
  }));
  await step("prepare.frame.ready", () => page.waitForFunction(() => document.getElementById("discovery-background")?.dataset.ready === "true", undefined, { timeout: 10000 }));
  await step("prepare.background.install", () => page.evaluate(({ manifest, sources, apiSource, settings }) => {
    const bg = document.getElementById("discovery-background").contentWindow;
    const append = (target, source) => { const script = target.document.createElement("script"); script.textContent = source; target.document.head.appendChild(script); };
    bg.addEventListener("error", event => { if (event.message) __errors.push(event.message); });
    bg.addEventListener("unhandledrejection", event => __errors.push(String(event.reason)));
    append(bg, `(function(){ const module={exports:{}}; ${apiSource}\nwindow.__api=module.exports.createChromeApi(${JSON.stringify({ tabs: [{ id: 7, active: true, url: "https://fixture.test/watch", title: "REF12 fixture" }], storage: settings })}); })();`);
    bg.chrome = bg.__api.api;
    bg.fetch = async () => { throw new Error("fixture network blocked"); };
    bg.__api.nativeRegistry.onPost = (port, message) => {
      if (message.type === "hello") queueMicrotask(() => port.receive({ requestId: "fixture-hello", ok: true, type: "hello_ack" }));
      else if (message.type === "get_snapshot") queueMicrotask(() => port.receive({ requestId: message.requestId, ok: true, data: { tasks: [] } }));
    };
    // Manifest classic-script order and real script realms are identical in A/B.
    for (const file of manifest.background.scripts) append(bg, sources[file]);
    const send = (message, sender = { id: bg.chrome.runtime.id, url: bg.chrome.runtime.getURL("popup.html") }) => new Promise(resolve => {
      let answered = false;
      const results = bg.chrome.runtime.onMessage.fire(message, sender, value => { answered = true; resolve(value); });
      if (!answered && !results.includes(true)) resolve(undefined);
    });
    const work = { runtime: {}, scans: {}, timers: {}, mutations: 0, performanceEvents: 0, responseEvents: 0, explicitScans: 0 };
    window.__discoveryFixture = { bg, send, work, settings, turn: () => new Promise(resolve => setTimeout(resolve, 0)) };
  }, { ...bundle, apiSource, settings: WORKLOAD.settings }));
  await step("prepare.content.install", () => page.evaluate(({ manifest, sources }) => {
    const { bg, send, work } = __discoveryFixture;
    const count = (map, key) => { map[key] = (map[key] || 0) + 1; };
    const nativeQuery = Document.prototype.querySelectorAll;
    Document.prototype.querySelectorAll = function(selector) { count(work.scans, selector); return nativeQuery.call(this, selector); };
    const nativeInterval = window.setInterval.bind(window);
    window.setInterval = (fn, ms, ...args) => nativeInterval(() => { count(work.timers, String(ms)); fn(...args); }, ms);
    const NativeMutationObserver = window.MutationObserver;
    window.MutationObserver = class extends NativeMutationObserver { constructor(fn) { super((records, observer) => { work.mutations += records.length; fn(records, observer); }); } };
    const NativePerformanceObserver = window.PerformanceObserver;
    window.PerformanceObserver = class extends NativePerformanceObserver { constructor(fn) { super((list, observer) => { work.performanceEvents += list.getEntries().length; fn(list, observer); }); } };
    const contentListeners = [];
    const chrome = { runtime: { id: bg.chrome.runtime.id, lastError: null, getURL: () => "data:text/javascript,",
      onMessage: { addListener(fn) { contentListeners.push(fn); } },
      sendMessage(message, callback) { count(work.runtime, message.type); const promise = send(message, { id: bg.chrome.runtime.id, tab: bg.__api.tabs.get(7), frameId: 0, url: location.href }); promise.then(callback); return promise; } },
      storage: { onChanged: bg.chrome.storage.onChanged, local: { get(defaults, callback) { const promise = bg.chrome.storage.local.get(defaults); promise.then(callback); return promise; } } } };
    Object.defineProperty(window, "chrome", { configurable: true, value: chrome });
    bg.__api.setTabMessageHandler((_id, message) => new Promise(resolve => {
      if (message.type === "content_scan_now") work.explicitScans++;
      let answered = false;
      const results = contentListeners.map(fn => fn(message, { id: chrome.runtime.id }, value => { answered = true; resolve(value); }));
      if (!answered && !results.includes(true)) resolve({ ok: true });
    }));
    const append = source => { const script = document.createElement("script"); script.textContent = source; document.head.appendChild(script); };
    append(sources["page-hook.js"]);
    for (const file of manifest.content_scripts[0].js) append(sources[file]);
  }, bundle));
  await step("prepare.ready", () => runPageJob(page, "prepare.ready", () => __discoveryFixture.turn()));
}

export async function activeWork(page, { step, onRound = async () => {} }) {
  await step("active.initialize", () => page.evaluate(() => {
    const f = __discoveryFixture;
    f.active = { start: performance.now(), discoveryLatenciesMs: [], keep: [
      ["https://cdn.test/master", "application/vnd.apple.mpegurl"], ["https://cdn.test/dash", "application/dash+xml"],
      ["https://cdn.test/short.mp4", "video/mp4"], ["https://cdn.test/short.mp3", "audio/mpeg"],
      ["https://cdn.test/complete.ts", "video/mp2t"], ["https://cdn.test/ordinary.zip", "application/zip"],
      ["https://cdn.test/opaque?id=1", "application/octet-stream"]
    ] };
    f.response = (url, mime, extra = {}) => {
      f.work.responseEvents++;
      f.bg.chrome.webRequest.onResponseStarted.fire({ requestId: url, tabId: 7, frameId: 0, url, type: "media", statusCode: 200,
        initiator: "https://fixture.test/watch", responseHeaders: [{ name: "Content-Type", value: mime }, { name: "Content-Length", value: "1024" }], ...extra });
    };
    const addMedia = (id, url, decorative) => {
      const element = document.createElement("audio");
      element.id = id; element.src = url; element.controls = !decorative; element.autoplay = decorative;
      if (decorative) element.setAttribute("role", "presentation");
      // Real DOM metadata/events with deterministic duration, not decoding acceptance.
      Object.defineProperty(element, "duration", { configurable: true, value: 1 });
      document.body.appendChild(element);
      element.dispatchEvent(new Event("loadedmetadata", { bubbles: true }));
      return element;
    };
    f.active.effect = addMedia("effect", "https://cdn.test/effect.mp3", true);
    addMedia("short", "https://cdn.test/short.mp3", false);
    const blobUrl = URL.createObjectURL(new Blob(["fixture"], { type: "video/mp4" }));
    window.dispatchEvent(new CustomEvent("nxtools-media-candidate", { detail: { url: blobUrl, mime: "video/mp4", source: "page-hook" } }));
    const css = document.createElement("div"); css.style.backgroundImage = "url(https://cdn.test/background.png)"; document.body.appendChild(css);
    f.readObservations = () => (f.bg.eval("serializeResources()")["7"] || []).map(resource => ({ url: resource.url.startsWith("blob:") ? "blob:fixture" : resource.url,
      mime: resource.mime, size: resource.size, discovery: resource.discovery || null, mediaElement: resource.mediaElement || null }));
    f.readCandidates = state => state.currentResources.map(resource => resource.url.startsWith("blob:") ? "blob:fixture" : resource.url).sort();
  }));
  for (let round = 0; round < WORKLOAD.rounds; round++) {
    const raw = await step(`active.round.${round + 1}`, () => runPageJob(page, `active.round.${round + 1}`, async ({ workload, round }) => {
      const f = __discoveryFixture, observedAt = performance.now();
      for (const [url, mime] of f.active.keep) f.response(url, mime);
      for (let index = 0; index < workload.fragments; index++) f.response(`https://cdn.test/part${index}`, "video/iso.segment");
      for (let index = 0; index < workload.duplicates; index++) f.response("https://cdn.test/part0", "video/iso.segment");
      window.dispatchEvent(new CustomEvent("nxtools-media-candidate", { detail: { url: "https://cdn.test/hook.mp4", mime: "video/mp4", source: "page-hook" } }));
      await f.turn();
      const state = await f.send({ type: "popup_get_state", currentView: "resources" });
      const latencyMs = performance.now() - observedAt;
      f.active.discoveryLatenciesMs.push(latencyMs);
      if (!f.active.keep.every(([url]) => state.currentResources.some(resource => resource.url === url))) throw new Error("ordinary/complete/manifest candidate lost");
      await window.NxToolsContent.choices.backgroundResourceChoiceCandidates(null,
        [{ url: "https://cdn.test/part0", mime: "video/mp4" }, { url: "https://cdn.test/short.mp3", mediaElement: { kind: "audio", controls: true } }]);
      return { round: round + 1, latencyMs, observations: f.readObservations(), candidates: f.readCandidates(state), work: structuredClone(f.work) };
    }, { workload: WORKLOAD, round }));
    await onRound(raw);
  }
  return step("active.finalize", () => runPageJob(page, "active.finalize", async () => {
    const f = __discoveryFixture;
    await f.turn();
    const state = await f.send({ type: "popup_get_state", currentView: "tasks" });
    const observations = f.readObservations(), candidates = f.readCandidates(state);
    const checks = { completeAndManifest: f.active.keep.every(([url]) => candidates.includes(url)), blob: candidates.includes("blob:fixture"),
      deepCss: candidates.includes("https://cdn.test/background.png"), hook: candidates.includes("https://cdn.test/hook.mp4"),
      fragmentsExcluded: !candidates.some(url => /\/part\d+$/.test(url)), decorativeExcluded: !candidates.includes(f.active.effect.src),
      boundedObservations: observations.length <= 120, loadErrors: [...__errors], visibility: document.visibilityState };
    return { wallMs: performance.now() - f.active.start, discoveryLatenciesMs: f.active.discoveryLatenciesMs, observations, candidates, checks, work: structuredClone(f.work) };
  }));
}
async function idleWork(page) {
  return runPageJob(page, "idle.8100ms", async idleMs => {
    const start = performance.now(), before = structuredClone(__discoveryFixture.work);
    await new Promise(resolve => setTimeout(resolve, idleMs));
    return { wallMs: performance.now() - start, before, after: structuredClone(__discoveryFixture.work), visibility: document.visibilityState };
  }, WORKLOAD.idleMs, 12000);
}

export async function runDiscoveryComparison(page, { baselineDirectory, candidateDirectory, outputPath, onStage }) {
  const progress = createProgress(page, { outputPath, onStage }, { workload: WORKLOAD, popup: { results: [] } });
  let performanceEnabled = false;
  try {
    const { baseline, candidate, apiSource } = await progress.step("sources.read", async () => {
      const baseline = await readSourceBundle(baselineDirectory), candidate = await readSourceBundle(candidateDirectory);
      if (baseline.root === candidate.root) throw new Error("baseline and candidate directories must differ");
      const apiSource = await fs.readFile(new URL("tests/harness/chrome-api.cjs", directory(candidateDirectory)), "utf8");
      return { baseline, candidate, apiSource };
    });
    Object.assign(progress.data, { baseline: baseline.root, candidate: candidate.root });
    progress.data.popup = await runPopup(page, directory(candidateDirectory), { step: progress.step,
      onResult: async result => { progress.data.popup.results.push(result); await progress.save(); },
      onHeld: async requests => { progress.data.popup.heldRequests = requests; await progress.save(); } });
    for (const plan of TRIAL_ORDER) {
      const trial = progress.data.currentTrial = { ...plan, rounds: [], metrics: {} };
      const step = (stage, action) => progress.step(`trial.${plan.group}.${plan.variant}.${stage}`, action);
      await prepare(page, plan.variant === "baseline" ? baseline : candidate, apiSource, step);
      await step("metrics.start", async () => {
        await page.cdp("Performance.enable", { timeDomain: "threadTicks" });
        performanceEnabled = true;
        trial.metrics.start = taskDuration(await page.cdp("Performance.getMetrics"));
      });
      trial.active = await activeWork(page, { step, onRound: async raw => { trial.rounds.push(raw); await progress.save(); } });
      await step("metrics.middle", async () => { trial.metrics.middle = taskDuration(await page.cdp("Performance.getMetrics")); });
      trial.idle = await step("idle.8100ms", () => idleWork(page));
      await step("metrics.end", async () => { trial.metrics.end = taskDuration(await page.cdp("Performance.getMetrics")); });
      trial.active.taskMs = (trial.metrics.middle - trial.metrics.start) * 1000;
      trial.idle.taskMs = (trial.metrics.end - trial.metrics.middle) * 1000;
      trial.active.taskDurationSeconds = { before: trial.metrics.start, after: trial.metrics.middle };
      trial.idle.taskDurationSeconds = { before: trial.metrics.middle, after: trial.metrics.end };
      progress.data.trials.push(trial);
      progress.data.currentTrial = null;
      await progress.save();
      await page.cdp("Performance.disable");
      performanceEnabled = false;
    }
    Object.assign(progress.data, { status: "browser_measurement_for_coordinator_decision",
      evidence: "real browser DOM with isolated WebExtension APIs and deterministic media metadata",
      metric: "CDP Performance.TaskDuration (threadTicks), renderer including fixture/background iframe and identical staged instrumentation; not OS installed-extension CPU",
      summary: summarizeTrials(progress.data.trials),
      limitations: ["No installed-extension isolated worlds, actual media decoding/provider authentication, native Host or human visual acceptance.",
        "API/DOM instrumentation and per-round checkpoints are identical in both variants; no adoption decision is made by this helper."] });
    await progress.step("complete.cleanup", () => page.goto("about:blank"));
    return progress.data;
  } catch (error) {
    await progress.fail(error);
    if (performanceEnabled) {
      try { await page.cdp("Performance.disable"); }
      catch (cleanupError) { console.error("B2 metric cleanup failed:", cleanupError.message); }
    }
    throw error;
  }
}
