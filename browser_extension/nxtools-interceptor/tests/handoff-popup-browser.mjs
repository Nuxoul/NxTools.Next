// Run through ego-browser nodejs; production popup, synthetic APIs, blocked network.
const fs = await import("node:fs/promises");
const { fileURLToPath } = await import("node:url");
const { createRequire } = await import("node:module");
const require = createRequire(import.meta.url);
const { loadBackground } = require("./harness/load-background.cjs");
const root = new URL("../", import.meta.url);
const read = name => fs.readFile(new URL(name, root), "utf8");
const source = await read("popup.html");
const styles = await read("popup-tokens.css") + "\n" + await read("popup.css");
const icon = (await fs.readFile(new URL("icons/icon-32.png", root))).toString("base64");
const html = source.replace(/<script\b[\s\S]*?<\/script>/gi, "").replace(/<link\b[^>]*>/gi, "")
  .replace("icons/icon-32.png", "data:image/png;base64," + icon).replace("</head>", `<style>${styles}</style></head>`);
const scripts = await Promise.all([...source.matchAll(/<script src="([^"]+)"><\/script>/g)].map(match => read(match[1])));
const fixture = await read("tests/popup-fixture.js");
const item = { id: 41, url: "https://synthetic.invalid/secret.zip", finalUrl: "https://synthetic.invalid/secret.zip", referrer: "https://synthetic.invalid/page", startTime: "2026-11-09T00:00:00Z" };
const cases = [
  ["temporarily_paused", "已暂停", { settings: { interceptDownloads: false } }],
  ["rule_excluded", "规则已排除", { settings: { nxtools_disabled_sites: ["synthetic.invalid"] } }],
  ["desktop_unavailable", "桌面未连接", { unavailable: true }],
  ["desktop_refused", "桌面拒绝", { response: { ok: false } }],
  ["handoff_timeout", "等待超时", { timeout: true }],
  ["handoff_disconnected", "连接已断开", { disconnect: true }],
  ["confirmation_pending", "尚未创建任务", { response: { accepted: false, taskId: undefined, data: { handoffState: "awaiting_confirmation", reason: "confirmation_pending", queued: false } } }],
  ["queued_not_accepted", "仅进入等待队列", { response: { accepted: false, data: { queued: true } } }],
  ["identity_mismatch", "身份不匹配", { response: { handoffId: "other" } }],
  ["browser_transferred", "尚未下载完成", { response: { data: { handoffState: "persisted", queued: true } } }],
  ["browser_cancel_failed", "原下载仍保留", { cancelFails: true }],
  ["browser_cancel_failed", "撤销桌面任务也未获确认", { cancelFails: true, rollbackFails: true }]
];
const states = [];
for (const [reason, expected, scenario] of cases) {
  const bg = loadBackground({ chrome: { storage: scenario.settings, cancelFails: scenario.cancelFails }, desktop: { respond(message) {
    if (message.type === "create_task") {
      if (scenario.timeout || scenario.disconnect) return undefined;
      return { ok: true, accepted: true, handoffId: message.payload.handoffId, taskId: "fixture-task", data: { handoffState: "persisted" }, ...scenario.response };
    }
    if (message.type === "task_action") return { ok: !scenario.rollbackFails };
    return { ok: true, data: { tasks: [] } };
  } } });
  await bg.settle();
  const connect = bg.chrome.api.runtime.connectNative;
  if (scenario.unavailable) bg.chrome.api.runtime.connectNative = () => { throw new Error("synthetic-secret"); };
  let suggested = 0;
  bg.chrome.api.downloads.onDeterminingFilename.fire(item, () => suggested++);
  if (scenario.timeout || scenario.disconnect) {
    await bg.waitFor(() => bg.desktop.requestsOfType("create_task").length === 1);
    if (scenario.timeout) bg.clock.advance(15000);
    else bg.chrome.nativePorts.at(-1).disconnect();
  }
  await bg.waitFor(() => suggested === 1);
  bg.chrome.api.runtime.connectNative = connect;
  const state = await bg.sendRuntimeMessage({ type: "popup_get_state" }, bg.senders.popup());
  if (state.lastHandoff?.reason !== reason) throw new Error(`Background reason mismatch: ${reason}`);
  states.push({ reason, expected, state });
}
const manualBackground = loadBackground({ chrome: { tabs: [{ id: 7, url: item.url, active: true }] }, desktop: { respond(message) {
  if (message.type === "create_task") return { ok: true, accepted: false, handoffId: message.payload.handoffId,
    data: { handoffState: "awaiting_confirmation", reason: "confirmation_pending", queued: false } };
  return { ok: true, data: { tasks: [] } };
} } });
await manualBackground.settle();
const manualReply = JSON.parse(JSON.stringify(await manualBackground.sendRuntimeMessage(
  { type: "popup_send_active_tab" }, manualBackground.senders.popup())));
const task = await taskSpace("NxTools B1 acceptance repair 20261109");
console.log({ spaceId: task.spaceId, test: fileURLToPath(import.meta.url) });
const page = task.page("p1");
await page.goto("about:blank");
await page.cdp("Network.enable");
await page.cdp("Network.setBlockedURLs", { urls: ["http://*", "https://*", "ws://*", "wss://*"] });
await page.cdp("Emulation.setDeviceMetricsOverride", { width: 900, height: 900, deviceScaleFactor: 1, mobile: false });
const tree = await page.cdp("Page.getFrameTree");
await page.cdp("Page.setDocumentContent", { frameId: tree.frameTree.frame.id, html });
await page.evaluate(fixture + "\n" + scripts.join("\n;\n"));
await page.waitForFunction(() => Boolean(window.__fixture?.messages.length));
console.log(await page.snapshot());
await page.focus("#siteEnabledToggle");
const results = await page.evaluate(states => {
  const checks = [];
  const check = (pass, label) => checks.push({ pass: Boolean(pass), label });
  const header = document.querySelector(".topbar") || document.querySelector("header");
  const headerHeight = header.getBoundingClientRect().height;
  check(handoffStatus.hidden, "no-history-status-hidden");
  for (const { reason, expected, state } of states) {
    render(state);
    check(!handoffStatus.hidden && handoffStatus.textContent.includes(expected), `visible-${reason}-${expected}`);
    check(handoffStatus.getAttribute("role") === "status" && handoffStatus.getAttribute("aria-live") === "polite", `accessible-${reason}`);
    check(document.activeElement === siteEnabledToggle, `focus-preserved-${reason}`);
    check(header.getBoundingClientRect().height === headerHeight, `header-unchanged-${reason}`);
    check(handoffStatus.scrollWidth <= handoffStatus.clientWidth, `no-overflow-${reason}`);
    check(!handoffStatus.textContent.includes("synthetic"), `no-payload-${reason}`);
  }
  render({ ...states[0].state, lastHandoff: null });
  check(handoffStatus.hidden, "history-cleared-hides-status");
  check(__fixture.unhandled.length === 0, "no-unhandled-promises");
  window.dispatchEvent(new Event("pagehide"));
  check(__fixture.ports[0].closed, "popup-port-released");
  return checks;
}, states);
await page.evaluate(reply => {
  const originalSend = chrome.runtime.sendMessage;
  window.__manualChecks = { closeScheduled: 0 };
  const originalTimeout = window.setTimeout;
  window.setTimeout = (callback, milliseconds, ...args) => {
    if (milliseconds === 180) { __manualChecks.closeScheduled++; return 0; }
    return originalTimeout(callback, milliseconds, ...args);
  };
  chrome.runtime.sendMessage = message => ["popup_send_active_tab", "popup_send_resource"].includes(message.type)
    ? Promise.resolve(structuredClone(reply)) : originalSend(message);
  setView("resources");
  render(__fixture.state);
}, manualReply);
for (const selector of ["#sendTabButton", '[data-resource-id="video-1"]']) {
  await page.click(selector, { label: "check manual confirmation feedback" });
  await page.waitForFunction(() => busyActionCount === 0 && messageText.textContent === "确认窗口已打开，尚未创建任务");
  const checks = await page.evaluate(() => [
    { pass: !statusFooter.hidden, label: "manual-pending-visible" },
    { pass: messageText.dataset.tone === "status", label: "manual-pending-normal-status" },
    { pass: __manualChecks.closeScheduled === 0, label: "manual-pending-keeps-popup" },
    { pass: !elements.sendTabButton.disabled && !document.querySelector('[data-resource-id="video-1"]').disabled, label: "manual-buttons-restored" }
  ]);
  results.push(...checks);
}
const contentFiles = JSON.parse(await read("manifest.json")).content_scripts[0].js;
const contentScripts = await Promise.all(contentFiles.map(read));
await page.goto("about:blank");
const contentTree = await page.cdp("Page.getFrameTree");
await page.cdp("Page.setDocumentContent", { frameId: contentTree.frameTree.frame.id,
  html: '<html><head><title>synthetic media</title></head><body><video width="640" height="360" src="https://synthetic.invalid/video.mp4"></video></body></html>' });
await page.evaluate(reply => {
  window.__content = { sent: [], reply };
  Object.defineProperty(window, "chrome", { configurable: true, value: {
    runtime: { id: "fixture", lastError: null, getURL: () => "data:text/javascript,",
      onMessage: { addListener() {} }, sendMessage(message, callback) {
        __content.sent.push(message);
        const result = message.type === "content_get_resource_choices" ? { ok: true, choices: [] } : __content.reply;
        if (callback) queueMicrotask(() => callback(result));
        return Promise.resolve(result);
      } },
    storage: { local: { get(defaults, callback) { queueMicrotask(() => callback(defaults)); } }, onChanged: { addListener() {} } }
  } });
}, manualReply);
await page.evaluate(contentScripts.join("\n;\n"));
await page.waitForFunction(() => document.getElementById("nxToolsMediaDownload")?.style.display === "block");
await page.click(".trigger", { label: "check media confirmation feedback" });
await page.waitForFunction(() => document.getElementById("nxToolsMediaDownload").shadowRoot.querySelector(".status").textContent === "确认窗口已打开，尚未创建任务");
results.push(...await page.evaluate(() => {
  const root = document.getElementById("nxToolsMediaDownload").shadowRoot;
  return [
    { pass: !root.querySelector(".status").classList.contains("error"), label: "media-pending-is-not-error" },
    { pass: !root.querySelector(".trigger").disabled, label: "media-trigger-restored" },
    { pass: __content.sent.some(message => message.type === "page_download_media"), label: "production-media-message-sent" }
  ];
}));
const failures = results.filter(result => !result.pass);
console.log(JSON.stringify({ checks: results.length, failures, results }, null, 2));
if (!failures.length) await task.finish({ keep: [] });
else throw new Error(`${failures.length} popup/content checks failed in space ${task.spaceId}`);
