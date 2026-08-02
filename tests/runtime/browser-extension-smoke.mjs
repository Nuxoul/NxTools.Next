import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import puppeteer from "puppeteer-core";

const EXPECTED_EXTENSION_ID = "ajcjlgdnfcagknjcajhbiikagimgdnaf";

function argument(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback;
}

function requireFile(filePath, label) {
  if (!fs.statSync(filePath, { throwIfNoEntry: false })?.isFile()) {
    throw new Error(`${label} does not exist: ${filePath}`);
  }
}

function readSnapshot(cliPath) {
  const json = execFileSync(cliPath, ["download", "list", "--json"], {
    encoding: "utf8",
    windowsHide: true,
  });
  return JSON.parse(json);
}

function runCli(cliPath, args) {
  return execFileSync(cliPath, args, {
    encoding: "utf8",
    windowsHide: true,
  });
}

function formatException(details) {
  return details?.exceptionDetails?.exception?.description
    || details?.exceptionDetails?.text
    || "Unknown extension exception";
}

async function readBrowserDownloads(extensionPage) {
  return extensionPage.evaluate(
    () =>
      new Promise((resolve, reject) => {
        chrome.downloads.search({}, (items) => {
          const error = chrome.runtime.lastError;
          if (error) {
            reject(new Error(error.message));
            return;
          }
          resolve(
            items.map((item) => ({
              id: item.id,
              url: item.finalUrl || item.url,
              state: item.state,
              error: item.error || "",
            })),
          );
        });
      }),
  );
}

async function main() {
  const root = path.resolve(import.meta.dirname, "..", "..");
  const browserPath = path.resolve(
    argument(
      "--browser",
      "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    ),
  );
  const extensionPath = path.resolve(
    argument(
      "--extension",
      path.join(root, "artifacts", "win-x64", "browser_extension", "nxtools-interceptor"),
    ),
  );
  const cliPath = path.resolve(
    argument("--cli", path.join(root, "artifacts", "win-x64", "NxTools.Cli.exe")),
  );
  const downloadUrl = argument(
    "--url",
    "http://hkg.download.datapacket.com/100mb.bin",
  );
  const outputDirectory = path.resolve(
    argument("--output", path.join(root, "artifacts", "browser-automation")),
  );
  const uiScenario = path.join(root, "tests", "runtime", "browser-download-ui-smoke.json");
  const uiAutomation = path.join(root, "tests", "runtime", "windows-ui-automation.ps1");
  const desktopPath = path.join(path.dirname(cliPath), "NxTools.exe");
  requireFile(browserPath, "Browser");
  requireFile(cliPath, "NxTools CLI");
  requireFile(path.join(extensionPath, "manifest.json"), "Extension manifest");
  fs.mkdirSync(outputDirectory, { recursive: true });
  const profile = fs.mkdtempSync(path.join(outputDirectory, "chrome-profile-"));
  const screenshotPath = path.join(outputDirectory, "browser-extension-popup.png");
  const extensionErrors = [];
  const attachedTargets = new WeakSet();
  const diagnosticSessions = new Set();
  let acceptedTaskId = "";
  let browser;
  try {
    browser = await puppeteer.launch({
      executablePath: browserPath,
      dumpio: process.env.NXTOOLS_BROWSER_DUMPIO === "1",
      headless: false,
      pipe: true,
      userDataDir: profile,
      enableExtensions: true,
      args: [
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-component-update",
        "--disable-background-mode",
        "--window-position=-32000,-32000",
        "--window-size=600,720",
        "--enable-unsafe-extension-debugging",
        "--disable-features=HttpsFirstBalancedModeAutoEnable",
      ],
    });

    const attachExtensionDiagnostics = async (target) => {
      if (attachedTargets.has(target)
          || !["service_worker", "background_page"].includes(target.type())
          || !target.url().startsWith(`chrome-extension://${EXPECTED_EXTENSION_ID}/`)) {
        return;
      }

      attachedTargets.add(target);
      const session = await target.createCDPSession();
      diagnosticSessions.add(session);
      session.on("Runtime.exceptionThrown", (details) => {
        extensionErrors.push(formatException(details));
      });
      session.on("Log.entryAdded", ({ entry }) => {
        if (entry.level === "error" || /blocking webRequest|webRequestBlocking/i.test(entry.text || "")) {
          extensionErrors.push(entry.text || "Unknown extension log error");
        }
      });
      await Promise.all([session.send("Runtime.enable"), session.send("Log.enable")]);
    };
    browser.on("targetcreated", (target) => {
      void attachExtensionDiagnostics(target).catch((error) => {
        extensionErrors.push(`Unable to inspect extension background: ${error.message}`);
      });
    });

    const installedId = await browser.installExtension(extensionPath);
    if (installedId !== EXPECTED_EXTENSION_ID) {
      throw new Error(
        `Chrome loaded unexpected extension identity ${installedId}; expected=${EXPECTED_EXTENSION_ID}`,
      );
    }
    await Promise.all(browser.targets().map(attachExtensionDiagnostics));
    const popupPage = await browser.newPage();
    popupPage.on("pageerror", (error) => extensionErrors.push(error.message));
    popupPage.on("console", (message) => {
      if (message.type() === "error" || /blocking webRequest|webRequestBlocking/i.test(message.text())) {
        extensionErrors.push(message.text());
      }
    });
    await popupPage.setViewport({ width: 430, height: 700, deviceScaleFactor: 1 });
    await popupPage.goto(
      `chrome-extension://${EXPECTED_EXTENSION_ID}/popup.html`,
      { waitUntil: "domcontentloaded" },
    );
    await popupPage.waitForFunction(
      () => document.querySelector("#statusText")?.textContent?.length > 0,
      { timeout: 10_000 },
    );

    const beforeDownloads = await readBrowserDownloads(popupPage);
    const beforeSnapshot = readSnapshot(cliPath);
    const beforeTaskIds = new Set(
      beforeSnapshot.result.tasks.map((item) => item.task.id),
    );

    const connectionStatus = await popupPage.evaluate(
      () =>
        new Promise((resolve) =>
          chrome.runtime.sendMessage({ type: "popup_test_connection" }, resolve),
        ),
    );
    if (!connectionStatus?.ok) {
      throw new Error(
        `Extension Native Messaging check failed: ${connectionStatus?.message || "unknown"}`,
      );
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
    if (extensionErrors.length > 0) {
      throw new Error(`Extension background reported errors:\n${extensionErrors.join("\n")}`);
    }

    await popupPage.evaluate(
      (url) =>
        new Promise((resolve, reject) => {
          chrome.downloads.download({ url, saveAs: false }, (downloadId) => {
            const error = chrome.runtime.lastError;
            if (error) {
              reject(new Error(error.message));
              return;
            }
            resolve(downloadId);
          });
        }),
      downloadUrl,
    );

    const browserDownloadDirectory = path.join(outputDirectory, "downloads");
    fs.mkdirSync(browserDownloadDirectory, { recursive: true });
    const browserDownloadName = `browser-ui-${Date.now()}.bin`;
    const browserDownloadPath = path.join(browserDownloadDirectory, browserDownloadName);
    const automationEnvironment = {
      ...process.env,
      NXTOOLS_UI_DOWNLOAD_DIRECTORY: browserDownloadDirectory,
      NXTOOLS_UI_DOWNLOAD_NAME: browserDownloadName,
      NXTOOLS_UI_DOWNLOAD_PATH: browserDownloadPath,
      NXTOOLS_UI_EXPECTED_BYTES: process.env.NXTOOLS_BROWSER_EXPECTED_BYTES || "",
      NXTOOLS_UI_EXPECTED_SHA256: process.env.NXTOOLS_BROWSER_EXPECTED_SHA256 || "",
    };
    execFileSync(
      "powershell.exe",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        uiAutomation,
        "-Scenario",
        uiScenario,
        "-Executable",
        desktopPath,
        "-OutputDirectory",
        outputDirectory,
      ],
      { cwd: root, env: automationEnvironment, stdio: "inherit", windowsHide: true },
    );

    let acceptedTask;
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      const snapshot = readSnapshot(cliPath);
      acceptedTask = snapshot.result.tasks.find(
        (item) => item.task.source_url === downloadUrl && !beforeTaskIds.has(item.task.id),
      );
      if (acceptedTask) break;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    if (!acceptedTask) {
      throw new Error("The browser download was not durably accepted by NxTools.");
    }
    acceptedTaskId = acceptedTask.task.id;

    const afterDownloads = await readBrowserDownloads(popupPage);
    const beforeDownloadIds = new Set(beforeDownloads.map((item) => item.id));
    const duplicate = afterDownloads.find(
      (item) => !beforeDownloadIds.has(item.id) && item.state === "in_progress",
    );
    if (duplicate) {
      throw new Error("Chrome retained an in-progress duplicate after NxTools acceptance.");
    }

    await new Promise((resolve) => setTimeout(resolve, 250));
    if (extensionErrors.length > 0) {
      throw new Error(`Extension background reported errors:\n${extensionErrors.join("\n")}`);
    }

    await popupPage.reload({ waitUntil: "networkidle0" });
    await popupPage.screenshot({ path: screenshotPath, fullPage: true });

    console.log(`PASS Chrome loaded original extension identity ${EXPECTED_EXTENSION_ID}.`);
    console.log(`PASS browser handoff opened the NxTools UI and completed one verified task: ${acceptedTask.task.id}`);
    console.log("PASS Chrome retained no in-progress duplicate after acceptance.");
    console.log(`Screenshot: ${screenshotPath}`);
  } finally {
    for (const session of diagnosticSessions) {
      await session.detach().catch(() => {});
    }
    await browser?.close().catch(() => {});
    if (acceptedTaskId) {
      try {
        runCli(cliPath, ["download", "cancel", acceptedTaskId, "--json"]);
      } catch {
        // The task may still be queued or may have already reached a terminal state.
      }
      runCli(cliPath, ["download", "remove", acceptedTaskId, "--delete-file", "--json"]);
    }
    const relative = path.relative(outputDirectory, profile);
    if (relative && !relative.startsWith("..") && !path.isAbsolute(relative)) {
      fs.rmSync(profile, { recursive: true, force: true });
    }
  }
}

main().catch((error) => {
  console.error(error.stack || error.message || String(error));
  process.exitCode = 1;
});
