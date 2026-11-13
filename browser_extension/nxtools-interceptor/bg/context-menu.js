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
      id: "nxtools-download-selection",
      title: "使用 NxTools 下载选区中的链接…",
      contexts: ["selection"],
      documentUrlPatterns: ["http://*/*", "https://*/*"]
    });
    chrome.contextMenus.create({
      id: "nxtools-extract-page-links",
      title: "提取当前页面链接…",
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

  if (info.menuItemId === "nxtools-download-selection" || info.menuItemId === "nxtools-extract-page-links") {
    void collectLinksForReview(tab, {
      mode: info.menuItemId === "nxtools-download-selection" ? "selection" : "page",
      frameId: info.frameId || 0,
      selectionText: info.selectionText || ""
    }).then(result => setBadge(result.ok ? "…" : "!", result.ok ? "#138a72" : "#9a3412"));
    openLinkReviewPopup();
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
  const startedAt = Date.now();
  void sendUrl(url, {
    cookieSource: { ...cookieSourceDetails(tab), tabId: tab?.id },
    handoffId: correlationId,
    referrer: info.pageUrl || tab?.url || ""
  }).then((result) => {
    if (result?.ok !== true) {
      throw new Error(result?.message || "NxTools 未接受右键下载请求");
    }
    const outcome = NxToolsHandoffPolicy.handoffOutcome(result, correlationId);
    const pending = outcome.state === "awaiting_confirmation";
    if (pending) {
      recordBrowserHandoff({ correlationId, startedAt }, outcome);
    } else {
      sendExtensionLog("info", "context_menu_handoff_completed", "右键下载已由 NxTools 接受", {}, 0, correlationId);
    }
    setBadge(pending ? "…" : "OK", "#138a72");
    setTimeout(() => setBadge("", "#138a72"), 1600);
  }).catch((error) => {
    sendExtensionLog("warn", "context_menu_handoff_failed", "右键下载未交接", {}, 0, correlationId);
    console.warn("Failed to send context menu URL to NxTools.", error);
    setBadge("!", "#9a3412");
  });
});
