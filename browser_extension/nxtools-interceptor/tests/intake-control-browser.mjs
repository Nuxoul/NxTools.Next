// Run with ego-browser nodejs. Production Popup with isolated browser APIs.
import fs from "node:fs/promises";
import { runPageJob } from "./browser-stages.mjs";
const root = new URL("../", import.meta.url);
const directStep = async (stage, action) => { console.log(`[B2 browser] ${stage}`); return action(); };
const wait = (page, fn, argument = undefined) => page.waitForFunction(fn, argument, { timeout: 10000 });

export async function loadPopup(page, sourceRoot = root, initialize = true, { step = directStep } = {}) {
  const read = name => fs.readFile(new URL(name, sourceRoot), "utf8");
  const source = await read("popup.html");
  const styles = await read("popup-tokens.css") + "\n" + await read("popup.css");
  const icon = (await fs.readFile(new URL("icons/icon-32.png", sourceRoot))).toString("base64");
  const html = source.replace(/<script\b[\s\S]*?<\/script>/gi, "").replace(/<link\b[^>]*>/gi, "")
    .replace("icons/icon-32.png", "data:image/png;base64," + icon).replace("</head>", `<style>${styles}</style></head>`);
  await step("popup.load.document", async () => {
    await page.goto("about:blank");
    await page.cdp("Network.enable");
    await page.cdp("Network.setBlockedURLs", { urls: ["http://*", "https://*", "ws://*", "wss://*"] });
    await page.cdp("Emulation.setDeviceMetricsOverride", { width: 900, height: 900, deviceScaleFactor: 1, mobile: false });
    const tree = await page.cdp("Page.getFrameTree");
    await page.cdp("Page.setDocumentContent", { frameId: tree.frameTree.frame.id, html });
  });
  if (initialize) {
    const scripts = await Promise.all([...source.matchAll(/<script src="([^"]+)"><\/script>/g)].map(match => read(match[1])));
    const fixture = await fs.readFile(new URL("tests/popup-fixture.js", root), "utf8");
    const held = await fs.readFile(new URL("tests/harness/held-requests.cjs", root), "utf8");
    await step("popup.load.scripts", () => page.evaluate(fixture + "\n" + held + "\n" + scripts.join("\n;\n") + "\n;void 0;"));
    await step("popup.load.ready", () => wait(page, () => Boolean(currentState) && !refreshInFlight && busyActionCount === 0));
  }
  return page;
}
export async function runPopup(page, sourceRoot = root, { step = directStep, onResult = async () => {}, onHeld = async () => {} } = {}) {
  await loadPopup(page, sourceRoot, true, { step });
  const results = [];
  const record = async (label, fn, argument = null) => {
    const result = { label, pass: Boolean(await page.evaluate(fn, argument)) };
    results.push(result);
    await onResult(result);
    if (!result.pass) throw new Error(`Popup check failed: ${label}`);
  };
  await step("popup.settings.open", async () => {
    await page.click('[data-view="settings"]', { label: "open capture settings" });
    await wait(page, () => currentView === "settings" && !refreshInFlight);
    console.log(await page.snapshot());
    await record("controls-labeled-and-contained", () => {
      return ["captureFileTypes", "captureMinimumKiB", "captureExcludedSites", "captureExcludedAddresses"].every(id => {
        const node = document.getElementById(id), bounds = node.getBoundingClientRect();
        const parent = node.closest(".field").getBoundingClientRect();
        return node.labels.length && bounds.width >= 100 && bounds.height >= 28 && bounds.left >= parent.left && bounds.right <= parent.right + 1;
      });
    });
  });
  for (const [id, value] of [["captureFileTypes", "zip, pdf"], ["captureMinimumKiB", "128"],
    ["captureExcludedSites", "example.test\ncdn.test"], ["captureExcludedAddresses", "/private/\n?skip=1"]]) {
    await step(`popup.autosave.${id}`, async () => {
      await page.fill(`#${id}`, value);
      await page.press(`#${id}`, "Tab");
      await wait(page, () => busyActionCount === 0);
      await record(`autosave-${id}`, ({ id, value }) => String(__fixture.state.settings[id]) === value, { id, value });
    });
  }
  for (const value of ["abc", "-1", "1.5", "", "9007199254740992"]) {
    await step(`popup.invalid-minimum.${value || "empty"}`, async () => {
      await page.evaluate(() => { window.__beforeSaveCount = __fixture.messages.filter(m => m.type === "popup_save_settings").length; });
      await page.fill("#captureMinimumKiB", value);
      await page.press("#captureMinimumKiB", "Tab");
      await wait(page, () => busyActionCount === 0);
      await record(`invalid-minimum-${value || "empty"}-retains-applied-draft`, value =>
        __fixture.messages.filter(m => m.type === "popup_save_settings").length === __beforeSaveCount &&
        __fixture.state.settings.captureMinimumKiB === 128 && captureMinimumKiB.value === value &&
        messageText.dataset.tone === "error", value);
    });
  }
  await step("popup.save.failure", async () => {
    await page.fill("#captureMinimumKiB", "128");
    await page.press("#captureMinimumKiB", "Tab");
    await wait(page, () => busyActionCount === 0);
    await page.evaluate(() => { __fixture.failure = "popup_save_settings"; });
    await page.fill("#captureFileTypes", "mp4");
    await page.press("#captureFileTypes", "Tab");
    await wait(page, () => busyActionCount === 0);
    await record("failed-save-retains-draft", () => captureFileTypes.value === "mp4" &&
      __fixture.state.settings.captureFileTypes === "zip, pdf" && messageText.dataset.tone === "error");
  });
  await step("popup.save.failed-draft-refresh", async () => {
    await runPageJob(page, "popup.save.failed-draft-refresh", () => refresh({ force: true }));
    await record("refresh-retains-failed-draft", () => captureFileTypes.value === "mp4");
  });
  await step("popup.save.retry", async () => {
    await page.evaluate(() => { __fixture.failure = ""; });
    await page.click("#saveButton", { label: "retry capture rules save" });
    await wait(page, () => busyActionCount === 0);
    await record("explicit-save-retries-draft", () => __fixture.state.settings.captureFileTypes === "mp4" && messageText.dataset.tone === "success");
  });
  await step("popup.pause.keyboard", async () => {
    await page.focus("#capturePauseButton");
    await page.press("#capturePauseButton", "Enter");
    await wait(page, () => busyActionCount === 0);
    await record("keyboard-pause-preserves-enabled-setting", () => currentState.capturePaused &&
      __fixture.state.settings.interceptDownloads && capturePauseButton.getAttribute("aria-pressed") === "true" &&
      document.activeElement === capturePauseButton);
  });
  await step("popup.resume.keyboard", async () => {
    await page.press("#capturePauseButton", "Space");
    await wait(page, () => busyActionCount === 0);
    await record("keyboard-resume", () => !currentState.capturePaused && capturePauseButton.getAttribute("aria-pressed") === "false");
  });
  await step("popup.pause.failure", async () => {
    await page.evaluate(() => { __fixture.failure = "popup_set_capture_paused"; });
    await page.click("#capturePauseButton", { label: "check pause save failure" });
    await wait(page, () => busyActionCount === 0);
    await record("pause-failure-retains-state-and-retry", () => !currentState.capturePaused && !capturePauseButton.disabled && messageText.dataset.tone === "error");
  });
  let heldRefreshId, heldPauseId;
  await step("popup.held-refresh.capture", async () => {
    await page.evaluate(() => {
      __fixture.failure = "";
      window.__heldRequests = NxToolsHeldRequests.holdNextRequests(chrome.runtime, ["popup_get_state", "popup_set_capture_paused"]);
      void refresh({ force: true });
    });
    await wait(page, () => __heldRequests.inspect().some(row => row.type === "popup_get_state" && row.state === "held"));
    heldRefreshId = await page.evaluate(() => __heldRequests.inspect().find(row => row.type === "popup_get_state").id);
    await onHeld(await page.evaluate(() => __heldRequests.inspect()));
  });
  await step("popup.held-pause.capture", async () => {
    await page.click("#capturePauseButton", { label: "pause with earlier refresh" });
    await wait(page, () => __heldRequests.inspect().some(row => row.type === "popup_set_capture_paused" && row.state === "held"));
    heldPauseId = await page.evaluate(() => __heldRequests.inspect().find(row => row.type === "popup_set_capture_paused").id);
    await onHeld(await page.evaluate(() => __heldRequests.inspect()));
  });
  await step("popup.held-refresh.release", async () => {
    await page.evaluate(id => { __heldRequests.release(id); }, heldRefreshId);
    await wait(page, id => !refreshInFlight && __heldRequests.inspect().find(row => row.id === id).state === "settled", heldRefreshId);
    await onHeld(await page.evaluate(() => __heldRequests.inspect()));
    await record("earlier-refresh-keeps-pause-busy", () => capturePauseButton.disabled &&
      capturePauseButton.getAttribute("aria-busy") === "true" && capturePauseButton.textContent === "更新中");
  });
  await step("popup.held-pause.release", async () => {
    await page.evaluate(id => { __heldRequests.release(id); }, heldPauseId);
    await wait(page, id => busyActionCount === 0 && __heldRequests.inspect().find(row => row.id === id).state === "settled", heldPauseId);
    await onHeld(await page.evaluate(() => __heldRequests.inspect()));
    await record("delayed-pause-publishes-once", () => currentState.capturePaused && !capturePauseButton.disabled &&
      capturePauseButton.textContent === "恢复自动接管" && capturePauseButton.getAttribute("aria-pressed") === "true");
  });
  await step("popup.session.retry", async () => {
    await page.evaluate(() => {
      __fixture.state.capturePauseAvailable = false;
      __fixture.state.capturePaused = true;
      chrome.runtime.sendMessage = async message => {
        __fixture.messages.push(structuredClone(message));
        if (message.type === "popup_get_state") __fixture.state.capturePauseAvailable = true;
        return structuredClone(__fixture.state);
      };
      render(structuredClone(__fixture.state));
    });
    await record("unavailable-session-offers-retry", () => !capturePauseButton.disabled && /重试/.test(capturePauseButton.textContent));
    await page.click("#capturePauseButton", { label: "retry session state read" });
    await wait(page, () => busyActionCount === 0);
    await record("session-retry-refreshes-without-toggle", () => currentState.capturePauseAvailable &&
      currentState.capturePaused && __fixture.messages.at(-1).type === "popup_get_state");
  });
  await step("popup.release", async () => {
    await record("no-unhandled-promises", () => __fixture.unhandled.length === 0);
    await page.evaluate(() => window.dispatchEvent(new Event("pagehide")));
    await record("popup-releases-port", () => __fixture.ports.every(port => port.closed));
  });
  return { page: page.label, checks: results.length, failures: results.filter(result => !result.pass), results };
}
