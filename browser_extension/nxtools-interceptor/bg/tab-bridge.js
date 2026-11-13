"use strict";

// Active tab resolution, frame messaging and content/page-hook injection.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

async function activeTab() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0] || null;
  if (tab?.id) {
    lastActiveTabId = tab.id;
    schedulePersistResources();
  }
  return tab;
}

async function tabInfo(tabId) {
  if (!tabId || tabId <= 0) {
    return null;
  }
  try {
    return await chrome.tabs.get(tabId);
  } catch {
    return null;
  }
}

function urlsLikelySamePage(left, right) {
  const normalizedLeft = normalizeUrlKey(left);
  const normalizedRight = normalizeUrlKey(right);
  if (!normalizedLeft || !normalizedRight) {
    return false;
  }
  return normalizedLeft === normalizedRight || normalizedLeft.startsWith(normalizedRight) || normalizedRight.startsWith(normalizedLeft);
}

async function resolveTabIdFromPageUrl(pageUrl) {
  const normalizedPageUrl = normalizeUrlKey(pageUrl);
  if (!normalizedPageUrl) {
    return 0;
  }

  const tabs = await chrome.tabs.query({});
  const exactMatch = tabs.find((tab) => tab.id && tab.url && urlsLikelySamePage(tab.url, normalizedPageUrl));
  if (exactMatch?.id) {
    return exactMatch.id;
  }

  try {
    const pageOrigin = new URL(normalizedPageUrl).origin;
    const originMatch = tabs.find((tab) => {
      if (!tab.id || !tab.url) {
        return false;
      }
      try {
        return new URL(tab.url).origin === pageOrigin;
      } catch {
        return false;
      }
    });
    return originMatch?.id || 0;
  } catch {
    return 0;
  }
}

async function resolveTabIdForRequest(details) {
  if (details.tabId > 0) {
    return details.tabId;
  }

  const snapshotTabId = resolveHeaderSnapshot(details.url, details)?.tabId || 0;
  if (snapshotTabId > 0) {
    return snapshotTabId;
  }

  const initiator = details.initiator && details.initiator !== "null" ? details.initiator : "";
  if (initiator) {
    const matchedTabId = await resolveTabIdFromPageUrl(initiator);
    if (matchedTabId > 0) {
      return matchedTabId;
    }
  }

  const current = await activeTab();
  return current?.id || lastActiveTabId || 0;
}

function frameInfo(tabId, frameId = 0) {
  return new Promise(resolve => {
    if (!chrome.webNavigation?.getFrame) return resolve(null);
    try {
      chrome.webNavigation.getFrame({ tabId, frameId }, frame => {
        resolve(chrome.runtime.lastError ? null : frame || null);
      });
    } catch { resolve(null); }
  });
}

function sameFrameDocument(original, current) {
  return Boolean(original && current && normalizeUrlKey(original.url) === normalizeUrlKey(current.url) &&
    (!original.documentId || original.documentId === current.documentId));
}

function tabFrames(tabId) {
  return new Promise((resolve) => {
    if (!chrome.webNavigation?.getAllFrames || !tabId || tabId <= 0) {
      resolve([{ frameId: 0 }]);
      return;
    }

    chrome.webNavigation.getAllFrames({ tabId }, (frames) => {
      const lastError = chrome.runtime.lastError;
      if (lastError || !Array.isArray(frames) || frames.length === 0) {
        resolve([{ frameId: 0 }]);
        return;
      }
      resolve(frames.map((frame) => ({ frameId: frame.frameId })));
    });
  });
}

function sendMessageToTab(tabId, message, options = {}) {
  return new Promise((resolve) => {
    if (!tabId || tabId <= 0) {
      resolve({ ok: false, status: "no_tab", message: "当前没有可操作的标签页" });
      return;
    }

    const sendOptions = Number.isInteger(options.frameId) ? { frameId: options.frameId } : undefined;
    const callback = (response) => {
      const lastError = chrome.runtime.lastError;
      if (lastError) {
        resolve({ ok: false, status: "no_receiver", message: lastError.message || "页面脚本未响应" });
        return;
      }
      resolve({ ok: true, response });
    };

    if (sendOptions) {
      chrome.tabs.sendMessage(tabId, message, sendOptions, callback);
    } else {
      chrome.tabs.sendMessage(tabId, message, callback);
    }
  });
}

async function sendMessageToTabFrames(tabId, message) {
  const frames = await tabFrames(tabId);
  const results = await Promise.all(frames.map((frame) => sendMessageToTab(tabId, message, frame)
    .then((result) => ({ ...result, frameId: frame.frameId }))));
  return results;
}

// Same ordered list as manifest content_scripts[0].js (tests enforce parity).
const CONTENT_SCRIPT_FILES = [
  "shadow-styles.js",
  "blob-transfer-policy.js",
  "capture-policy.js",
  "content/constants.js",
  "content/runtime.js",
  "content/media-info.js",
  "content/media-inspection.js",
  "content/resource-choice-text.js",
  "content/blob-bridge.js",
  "content/media-geometry.js",
  "content-script.js"
];

function injectContentBridge(tabId, pageUrl = "") {
  return new Promise((resolve) => {
    if (!chrome.scripting?.executeScript || !tabId || tabId <= 0) {
      resolve(false);
      return;
    }

    chrome.scripting.executeScript({
      target: { tabId, allFrames: true },
      files: CONTENT_SCRIPT_FILES,
      injectImmediately: false
    }, () => {
      const lastError = chrome.runtime.lastError;
      if (lastError) {
        sendExtensionLog("warn", "content_bridge_inject_failed", "页面媒体桥接脚本注入失败", {
          reason: lastError.message || "executeScript failed",
          pageUrl,
          tabId
        }, 30000);
        resolve(false);
        return;
      }

      resolve(true);
    });
  });
}

async function scanTabForResources(tabId, force = false) {
  if (!tabId || tabId <= 0) {
    return [];
  }

  const owner = bucketForTab(tabId);
  const now = Date.now();
  const lastScanAt = lastFrameScanByTab.get(tabId) || 0;
  if (!force && now - lastScanAt < RESOURCE_SCAN_INTERVAL_MS) {
    return [];
  }

  lastFrameScanByTab.set(tabId, now);
  await injectContentBridge(tabId);
  if (resourcesByTab.get(tabId) !== owner) return [];
  const frames = await tabFrames(tabId);
  await Promise.all(frames.map((frame) => injectPageHook(tabId, frame.frameId).catch(() => false)));
  if (resourcesByTab.get(tabId) !== owner) return [];
  const results = await Promise.all(frames.map((frame) => sendMessageToTab(tabId, { type: "content_scan_now" }, frame)
    .then((result) => ({ ...result, frameId: frame.frameId }))));
  if (resourcesByTab.get(tabId) !== owner) return [];

  const okCount = results.filter((result) => result.ok).length;
  if (okCount === 0) {
    sendExtensionLog("warn", "page_scan_no_receiver", "页面资源扫描没有收到内容脚本响应", {
      reason: "content_script_unavailable",
      tabId,
      frameCount: frames.length
    }, 30000);
  }

  if (okCount > 0) {
    // Prefer the top frame; embedded players report from their own frame.
    const thumbnail = [...results]
      .sort((left, right) => Number(left.frameId !== 0) - Number(right.frameId !== 0))
      .map((result) => (result.ok ? boundedCapturedUrl(result.response?.pageThumbnail) : ""))
      .find(isHttpUrl);
    if (thumbnail) {
      pageThumbnailByTab.set(tabId, thumbnail);
    } else {
      pageThumbnailByTab.delete(tabId);
    }
  }

  return results;
}

function injectPageHook(tabId, frameId = 0, pageUrl = "") {
  return new Promise((resolve) => {
    if (!chrome.scripting?.executeScript || !tabId || tabId <= 0) {
      resolve(false);
      return;
    }

    chrome.scripting.executeScript({
      target: { tabId, frameIds: [Number(frameId) || 0] },
      files: ["page-hook.js"],
      world: "MAIN",
      injectImmediately: true
    }, () => {
      const lastError = chrome.runtime.lastError;
      if (lastError) {
        sendExtensionLog("warn", "page_hook_inject_failed", "页面媒体 hook 注入失败", {
          reason: lastError.message || "executeScript failed",
          pageUrl,
          tabId,
          frameId
        }, 30000);
        resolve(false);
        return;
      }

      resolve(true);
    });
  });
}
