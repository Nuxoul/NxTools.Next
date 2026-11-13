"use strict";

// One-shot isolated-world extraction; it has no extension or download authority.
function collectDocumentLinks(mode, selectedText = "") {
  const links = [];
  const seen = new Set();
  const selection = globalThis.getSelection?.();
  const add = (value) => {
    if (links.length >= 120 || typeof value !== "string" || value.length > 16 * 1024) return;
    let address;
    try { address = new URL(value, document.baseURI); } catch { return; }
    if (!/^https?:$/.test(address.protocol)) return;
    address.hash = "";
    const current = new URL(location.href);
    current.hash = "";
    if (address.href === current.href || seen.has(address.href)) return;
    seen.add(address.href);
    links.push({ url: address.href });
  };
  for (const anchor of document.querySelectorAll("a[href]")) {
    if (mode === "selection" && !selection?.containsNode(anchor, true)) continue;
    add(anchor.href);
    if (links.length >= 120) break;
  }
  if (mode === "selection") {
    const text = String(selectedText || selection || "").slice(0, 64 * 1024);
    for (const match of text.matchAll(/https?:\/\/[^\s<>"`]+/gi)) {
      add(match[0].replace(/[.,;]+$/, ""));
      if (links.length >= 120) break;
    }
  }
  return { pageUrl: location.href, links };
}

function executeLinkCollection(tabId, frameId, mode, selectedText) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error("timeout")), 3500);
    try {
      chrome.scripting.executeScript({
        target: { tabId, frameIds: [frameId] },
        func: collectDocumentLinks,
        args: [mode, selectedText]
      }, (results) => {
        clearTimeout(timeout);
        if (chrome.runtime.lastError) reject(new Error("unavailable"));
        else resolve(results?.[0]?.result);
      });
    } catch {
      clearTimeout(timeout);
      reject(new Error("unavailable"));
    }
  });
}

async function collectLinksForReview(tab, { mode = "page", frameId = 0, selectionText = "" } = {}) {
  if (!tab?.id || !isHttpUrl(tab.url) || !["page", "selection"].includes(mode)) {
    return { ok: false, message: "当前页面无法提取 HTTP/HTTPS 链接" };
  }
  const id = nextRequestId();
  const startedAt = Date.now();
  const owner = bucketForTab(tab.id);
  const frame = Number.isInteger(frameId) && frameId >= 0 ? frameId : 0;
  linkReview = { id, tabId: tab.id, state: "collecting", count: 0, message: "正在提取链接，完成后请勾选加入队列" };
  sendExtensionLog("info", "link_collection_started", "开始提取页面链接", { reason: "started", elapsedMs: 0 }, 0, id);
  const stillOwned = () => linkReview?.id === id && resourcesByTab.get(tab.id) === owner;
  try {
    const originalFrame = await frameInfo(tab.id, frame);
    if (!originalFrame) throw new Error("unavailable");
    const result = await executeLinkCollection(tab.id, frame, mode, String(selectionText).slice(0, 64 * 1024));
    const currentTab = await tabInfo(tab.id);
    if (!stillOwned() || !currentTab || normalizeUrlKey(currentTab.url) !== normalizeUrlKey(tab.url) ||
        !matchesRequestContext(cookieSourceDetails(tab), cookieSourceDetails(currentTab))) throw new Error("page_changed");
    if (!sameFrameDocument(originalFrame, await frameInfo(tab.id, frame))) throw new Error("page_changed");
    if (!result || !Array.isArray(result.links) || !isHttpUrl(result.pageUrl) ||
        normalizeUrlKey(result.pageUrl) !== normalizeUrlKey(originalFrame.url)) throw new Error("invalid_response");
    const settings = await getSettings();
    const cookieContext = await cookieContextForCapture({ ...cookieSourceDetails(tab), tabId: tab.id }, tab, settings);
    if (!stillOwned() || !sameFrameDocument(originalFrame, await frameInfo(tab.id, frame))) throw new Error("page_changed");
    const urls = new Set();
    for (const link of result.links.slice(0, RESOURCE_LIMIT_PER_TAB * 2)) {
      const url = boundedCapturedUrl(link?.url);
      if (!isHttpUrl(url) || urls.has(normalizeResourceUrlKey(url))) continue;
      urls.add(normalizeResourceUrlKey(url));
      const snapshot = resolveHeaderSnapshot(url, { ...cookieSourceDetails(tab), tabId: tab.id });
      cacheResource({
        id: `${tab.id}:${normalizeResourceUrlKey(url)}`, tabId: tab.id, frameId: frame,
        frameUrl: originalFrame.url, frameDocumentId: originalFrame.documentId || "", url,
        filename: getFileNameFromUrl(url), filenameSource: NxToolsCapturePolicy.Sources.filename.url,
        mime: mimeFromUrl(url), metadataSource: NxToolsCapturePolicy.Sources.metadata.inferred,
        pageUrl: tab.url, pageTitle: tab.title || "", referer: snapshot?.headers?.referer || result.pageUrl,
        referrerSource: snapshot?.headers?.referer ? NxToolsCapturePolicy.Sources.referrer.browser : NxToolsCapturePolicy.Sources.referrer.page,
        requestHeaders: snapshot?.headers || {}, headerSource: NxToolsCapturePolicy.Sources.header.browser,
        cookieContext, source: mode === "selection" ? "selected-link" : "page-link", capturedAt: Date.now()
      }, owner);
      if (urls.size >= RESOURCE_LIMIT_PER_TAB) break;
    }
    linkReview = { id, tabId: tab.id, state: "ready", count: urls.size,
      message: urls.size ? `已提取 ${urls.size} 个链接，请勾选后加入队列（每批最多 32 项）` : "没有找到可提取的 HTTP/HTTPS 链接" };
    sendExtensionLog("info", "link_collection_completed", "页面链接提取完成", {
      reason: "ready", elapsedMs: Math.max(0, Date.now() - startedAt)
    }, 0, id);
    return { ok: true, message: linkReview.message, linkReview: { ...linkReview } };
  } catch (error) {
    const reason = ["timeout", "page_changed", "invalid_response"].includes(error?.message) ? error.message : "unavailable";
    const message = reason === "page_changed" ? "页面已变化，请在当前页面重新提取链接" : "无法提取链接，请刷新页面后重试";
    if (linkReview?.id === id) linkReview = { id, tabId: tab.id, state: "failed", count: 0, message };
    sendExtensionLog("warn", "link_collection_failed", "页面链接提取未完成", {
      reason, elapsedMs: Math.max(0, Date.now() - startedAt)
    }, 0, id);
    return { ok: false, message, linkReview: linkReview?.id === id ? { ...linkReview } : null };
  }
}

function openLinkReviewPopup() {
  try {
    const opening = actionApi()?.openPopup?.();
    opening?.catch(() => setBadge("…", "#138a72"));
  } catch {
    setBadge("…", "#138a72");
  }
}
