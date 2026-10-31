"use strict";

// Context menu creation and click handling.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

function createContextMenus() {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "nxtools-download-link",
      title: "使用 NxTools 下载",
      contexts: ["link", "image", "audio", "video"],
      targetUrlPatterns: ["http://*/*", "https://*/*"]
    });

    chrome.contextMenus.create({
      id: "nxtools-download-page",
      title: "使用 NxTools 下载当前页面",
      contexts: ["page"],
      documentUrlPatterns: ["http://*/*", "https://*/*"]
    });

    chrome.contextMenus.create({
      id: "nxtools-open-popup",
      title: "打开 NxTools 资源面板",
      contexts: ["page", "link", "image", "audio", "video"]
    });
  });
}

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "nxtools-open-popup") {
    actionApi()?.openPopup?.();
    return;
  }

  if (info.menuItemId !== "nxtools-download-link" &&
      info.menuItemId !== "nxtools-download-page") {
    return;
  }

  const url = info.linkUrl || info.srcUrl || info.pageUrl;
  if (!url) {
    return;
  }

  const correlationId = nextRequestId();
  void sendUrl(url, {
    handoffId: correlationId,
    referrer: info.pageUrl || tab?.url || ""
  }).then((result) => {
    if (result?.ok !== true || result?.accepted !== true) {
      throw new Error(result?.message || "NxTools 未接受右键下载请求");
    }
    sendExtensionLog("info", "context_menu_handoff_completed", "右键下载已由 NxTools 接受", {}, 0, correlationId);
    setBadge("OK", "#138a72");
    setTimeout(() => setBadge("", "#138a72"), 1600);
  }).catch((error) => {
    sendExtensionLog("warn", "context_menu_handoff_failed", "右键下载未交接", {}, 0, correlationId);
    console.warn("Failed to send context menu URL to NxTools.", error);
    setBadge("!", "#9a3412");
  });
});
