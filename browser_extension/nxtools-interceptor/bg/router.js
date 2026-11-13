"use strict";

// Runtime message router with sender-context checks.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

// Message types each sender context may use. Popup/extension pages own settings,
// task actions and resource dispatch; page content scripts (untrusted page
// context) only report captures and request page-scoped handoffs.
const CONTENT_MESSAGE_TYPES = new Set([
  "content_media_resource",
  "content_page_context_changed",
  "content_blob_download_request",
  "content_direct_download_request",
  "content_send_latest_resource",
  "content_get_resource_choices",
  "page_download_media",
  "drop_target_download"
]);

function messageSenderContext(sender) {
  if (!sender || sender.id !== chrome.runtime.id) {
    return "foreign";
  }
  if (sender.tab && Number.isInteger(sender.tab.id) && sender.tab.id >= 0) {
    return "content";
  }
  const extensionOrigin = chrome.runtime.getURL("");
  if (typeof sender.url === "string" && sender.url.startsWith(extensionOrigin)) {
    return "extension_page";
  }
  return "foreign";
}

function isMessageAllowedFrom(type, context) {
  if (context === "content") return CONTENT_MESSAGE_TYPES.has(type);
  if (context === "extension_page") return true;
  return false;
}

chrome.runtime.onConnect.addListener((port) => {
  if (port.name !== "nxtools-popup" ||
      messageSenderContext(port.sender) !== "extension_page" ||
      port.sender.url !== chrome.runtime.getURL("popup.html")) return;
  const release = retainDesktopPort();
  popupPortOwners += 1;
  let closed = false;
  port.onDisconnect.addListener(() => {
    if (closed) return;
    closed = true;
    popupPortOwners -= 1;
    release();
  });
  // Popup-open is demand. Its refreshes have no reconnect authority after loss.
  void ensureDesktopConnection().catch(() => {});
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || typeof message !== "object") {
    return false;
  }

  const senderContext = messageSenderContext(sender);
  if (!isMessageAllowedFrom(String(message.type || ""), senderContext)) {
    if (senderContext === "foreign") {
      return false;
    }
    sendExtensionLog("warn", "runtime_message_sender_rejected", "扩展消息来源不允许该操作", {
      reason: senderContext
    }, 30000);
    sendResponse({ ok: false, message: "当前上下文不允许该操作" });
    return false;
  }

  if (message.type === "content_media_resource") {
    void capturePageResource(sender, message.payload).then(() => sendResponse({ ok: true }), (error) => {
      sendExtensionLog("error", "content_media_resource_failed", "内容脚本资源写入缓存失败", {
        reason: error instanceof Error ? error.message : String(error),
        resourceUrl: message.payload?.url || "",
        pageUrl: sender.tab?.url || "",
        tabId: sender.tab?.id || 0
      }, 0);
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "资源缓存失败"
      });
    });
    return true;
  }

  if (message.type === "content_page_context_changed") {
    const tabId = sender.tab?.id || lastActiveTabId || 0;
    if (tabId > 0) {
      clearTabResources(tabId);
      sendExtensionLog("info", "page_context_resources_cleared", "页面上下文变更，已清理旧资源缓存", {
        tabId,
        pageUrl: message.href || sender.tab?.url || "",
        reason: message.reason || ""
      }, 30000);
    }
    sendResponse({ ok: true });
    return true;
  }

  if (message.type === "content_blob_download_request") {
    void (async () => {
      const settings = await getSettings();
      const payload = message.payload || {};
      const blobUrl = String(payload.url || "");
      if (!blobUrl.startsWith("blob:")) {
        return { ok: false, fallbackToBrowser: true, message: "不是 Blob 下载请求" };
      }

      const excluded = await automaticCaptureDecision({
        url: payload.sourceUrl || blobUrl, finalUrl: blobUrl,
        referrer: sender.tab?.url || payload.href || payload.pageUrl,
        filename: payload.filename, mime: payload.mime, totalBytes: payload.size
      }, settings);
      if (excluded) return automaticCaptureFallback(excluded);

      await capturePageResource(sender, payload);
      const tabId = sender.tab?.id || lastActiveTabId || 0;
      const sourceUrl = String(payload.sourceUrl || "");
      let resource = sourceUrl && isHttpUrl(sourceUrl)
        ? findResourceByUrl(sourceUrl, tabId)
        : null;
      resource ??= findResourceByUrl(blobUrl, tabId);

      if (!resource) {
        sendExtensionLog("warn", "blob_download_resource_missing", "Blob 点击下载没有找到可接管的缓存资源", {
          url: blobUrl,
          sourceUrl,
          pageUrl: payload.href || sender.tab?.url || "",
          tabId
        }, 0);
        return { ok: false, fallbackToBrowser: true, message: "没有找到 Blob 缓存，交还浏览器下载" };
      }

      sendExtensionLog("info", "blob_download_intercepted", "已拦截页面 Blob 点击下载并发送到 NxTools", {
        url: resource.url,
        blobUrl,
        sourceUrl,
        pageUrl: payload.href || sender.tab?.url || "",
        tabId,
        filename: resource.filename || payload.filename || ""
      }, 0);

      return await sendResource(resource.id);
    })().then(sendResponse, (error) => {
      sendExtensionLog("error", "blob_download_intercept_failed", "Blob 点击下载接管失败", {
        reason: error instanceof Error ? error.message : String(error),
        resourceUrl: message.payload?.url || "",
        pageUrl: sender.tab?.url || "",
        tabId: sender.tab?.id || 0
      }, 0);
      sendResponse({
        ok: false,
        fallbackToBrowser: true,
        message: error instanceof Error ? error.message : "Blob 下载接管失败"
      });
    });
    return true;
  }

  if (message.type === "content_direct_download_request") {
    void (async () => {
      const settings = await getSettings();
      const url = String(message.url || "");

      if (!isHttpUrl(url)) {
        return { ok: false, fallbackToBrowser: true, message: "不是 HTTP/HTTPS 下载链接" };
      }

      const excluded = await automaticCaptureDecision({
        url, referrer: sender.tab?.url || message.href,
        filename: message.filename, mime: message.mime, totalBytes: message.size
      }, settings);
      if (excluded) return automaticCaptureFallback(excluded);

      const result = await sendUrl(url, {
        cookieSource: { ...cookieSourceDetails(sender.tab), tabId: sender.tab?.id },
        fileName: cleanFileName(message.filename || "") || getFileNameFromUrl(url),
        referrer: message.href || sender.tab?.url || ""
      });
      if (result?.ok === false) {
        return { ok: false, fallbackToBrowser: true, message: result.message || "NxTools 拒绝接管" };
      }
      setBadge(result.data?.handoffState === "awaiting_confirmation" ? "…" : "OK", "#138a72");
      setTimeout(() => setBadge("", "#138a72"), 1600);
      return { ...result, message: result.message || "已发送到 NxTools" };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        fallbackToBrowser: true,
        message: error instanceof Error ? error.message : "下载链接接管失败"
      });
    });
    return true;
  }

  if (message.type === "content_send_latest_resource") {
    void (async () => {
      const tabId = sender.tab?.id || lastActiveTabId || 0;
      const resources = [...(resourcesByTab.get(tabId)?.values() || [])]
        .sort((left, right) => right.capturedAt - left.capturedAt);
      if (resources.length === 0) {
        return { ok: false, message: "当前页面还没有嗅探到资源" };
      }

      return await sendResource(resources[0].id);
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送资源失败"
      });
    });
    return true;
  }

  if (message.type === "content_get_resource_choices") {
    void (async () => {
      const tabId = sender.tab?.id || lastActiveTabId || 0;
      const choices = buildContentResourceChoices(tabId, message);
      sendExtensionLog("info", "content_resource_choices_built", "已为悬浮窗生成媒体资源列表", {
        tabId,
        pageUrl: message.href || sender.tab?.url || "",
        count: choices.length,
        sample: choices.slice(0, 5).map((choice) => `${choice.label} ${choice.size || 0}`).join(" | ")
      }, 10000);
      return {
        ok: true,
        choices,
        excludedUrls: excludedLocalResourceUrls(tabId, message.localCandidates)
      };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "读取媒体资源列表失败",
        choices: []
      });
    });
    return true;
  }

  if (message.type === "page_download_media") {
    void downloadPageMedia(sender, message).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送当前媒体失败"
      });
    });
    return true;
  }

  if (message.type === "drop_target_download") {
    void (async () => {
      const url = String(message.url || "");
      if (!isHttpUrl(url)) {
        return { ok: false, message: "拖放内容不是 HTTP/HTTPS 链接" };
      }

      sendExtensionLog("info", "drop_target_download", "浏览器 Drop Target 已接收拖放链接", {
        url,
        pageUrl: message.href || sender.tab?.url || "",
        tabId: sender.tab?.id || lastActiveTabId || 0
      }, 0);

      const result = await sendUrl(url, {
        cookieSource: { ...cookieSourceDetails(sender.tab), tabId: sender.tab?.id },
        fileName: getFileNameFromUrl(url) || message.filename || "",
        referrer: message.href || sender.tab?.url || ""
      });
      if (result?.ok !== true) {
        return { ...result, ok: false, message: result?.message || "NxTools 未接受拖放下载" };
      }
      setBadge(result.data?.handoffState === "awaiting_confirmation" ? "…" : "OK", "#138a72");
      setTimeout(() => setBadge("", "#138a72"), 1600);
      return { ...result, message: result.message || "已发送到 NxTools" };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "拖放下载失败"
      });
    });
    return true;
  }

  if (message.type === "popup_get_state") {
    const index = Number.isInteger(message.mediaIndex) ? message.mediaIndex : null;
    void buildPopupState(index, String(message.currentView || "tasks")).then(sendResponse);
    return true;
  }

  if (message.type === "popup_set_capture_paused") {
    const correlationId = nextRequestId();
    const startedAt = Date.now();
    sendExtensionLog("info", "capture_pause_started", "会话接管状态开始更新", {
      reason: message.paused ? "paused" : "resumed", elapsedMs: 0
    }, 0, correlationId);
    void (async () => {
      if (typeof message.paused !== "boolean") throw new Error("capture_pause_invalid");
      const paused = await setCapturePaused(message.paused);
      sendExtensionLog("info", "capture_pause_applied", "会话接管状态已更新", {
        reason: paused ? "paused" : "resumed", elapsedMs: Math.max(0, Date.now() - startedAt)
      }, 0, correlationId);
      return { ok: true, capturePaused: paused, correlationId };
    })().then(sendResponse, () => {
      sendExtensionLog("warn", "capture_pause_failed", "会话接管状态更新失败", {
        reason: "session_storage_failed", elapsedMs: Math.max(0, Date.now() - startedAt)
      }, 0, correlationId);
      sendResponse({ ok: false, correlationId, message: "无法保存会话暂停状态，请重试" });
    });
    return true;
  }

  if (message.type === "popup_save_settings") {
    const next = sanitizeSettings(message.settings);
    void chrome.storage.local.set(next).then(async () => {
      cachedSettings = next;
      settingsLoaded = true;
      sendResponse(await buildPopupState(null, String(message.currentView || "settings")));
    }, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "保存设置失败"
      });
    });
    return true;
  }

  if (message.type === "popup_set_site_enabled") {
    void (async () => {
      const tab = await activeTab();
      const host = siteHostKey(tab?.url || "") || String(message.host || "").trim().toLowerCase();
      if (!host) {
        return { ok: false, message: "当前页面无法按网站设置" };
      }

      const enabled = Boolean(message.enabled);
      await setSiteEnabled(host, enabled);
      return {
        ok: true,
        host,
        enabled,
        message: enabled ? `已在 ${host} 启用 NxTools` : `已在 ${host} 停用 NxTools（仅交还自动接管）`
      };
    })().then(async (result) => {
      const state = await buildPopupState(null, String(message.currentView || "tasks"));
      sendResponse({ ...state, operationResult: result });
    }, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "设置网站开关失败"
      });
    });
    return true;
  }

  if (message.type === "popup_test_connection") {
    void checkApiStatus(true).then(sendResponse);
    return true;
  }

  if (message.type === "popup_send_active_tab") {
    void (async () => {
      const tab = await activeTab();
      if (!tab?.url || !isHttpUrl(tab.url)) {
        return { ok: false, message: "当前页面不是可下载链接" };
      }

      const result = await sendUrl(tab.url, {
        cookieSource: { ...cookieSourceDetails(tab), tabId: tab.id },
        fileName: getFileNameFromUrl(tab.url),
        referrer: tab.url
      });
      return result;
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送失败"
      });
    });
    return true;
  }

  if (message.type === "popup_extract_links") {
    void activeTab().then(tab => collectLinksForReview(tab, { mode: message.mode || "page" }))
      .then(sendResponse, () => sendResponse({ ok: false, message: "无法提取链接，请刷新页面后重试" }));
    return true;
  }

  if (message.type === "popup_inspect_resource") {
    void inspectMediaResource(String(message.resourceId || "")).then(sendResponse,
      () => sendResponse({ ok: false, message: "无法识别媒体，请重试" }));
    return true;
  }

  if (message.type === "popup_send_resource") {
    void sendResource(String(message.resourceId || "")).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送资源失败"
      });
    });
    return true;
  }

  if (message.type === "popup_send_resources") {
    void sendResourcesBatch(message.resourceIds).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "批量发送资源失败"
      });
    });
    return true;
  }

  if (message.type === "popup_merge_resources") {
    void mergeResources(Array.isArray(message.resourceIds) ? message.resourceIds : []).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "合并资源失败"
      });
    });
    return true;
  }

  if (message.type === "popup_toggle_feature") {
    void (async () => {
      const tab = await activeTab();
      const tabId = tab?.id || lastActiveTabId || 0;
      if (tabId > 0) {
        await injectContentBridge(tabId, tab?.url || "");
      }
      return toggleFeature(String(message.feature || ""), tabId);
    })().then(async (result) => {
      const state = await buildPopupState(null, String(message.currentView || "advanced"));
      sendResponse({ ...state, operationResult: result });
    }, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "切换扩展功能失败"
      });
    });
    return true;
  }

  if (message.type === "popup_set_media_index") {
    void (async () => {
      const tab = await activeTab();
      const tabId = tab?.id || lastActiveTabId || 0;
      const index = Number(message.index || 0);
      selectedMediaIndexByTab.set(tabId, Math.max(0, index));
      return buildPopupState(index, String(message.currentView || "advanced"));
    })().then(sendResponse);
    return true;
  }

  if (message.type === "popup_media_action") {
    void (async () => {
      const tab = await activeTab();
      const tabId = tab?.id || lastActiveTabId || 0;
      const index = Number(message.index || 0);
      const frameId = Number(message.frameId || 0);
      const payload = { ...(message.payload || {}), index };
      if (payload.Message === "screenshot" && chrome.tabs?.captureVisibleTab && chrome.downloads?.download) {
        const dataUrl = await chrome.tabs.captureVisibleTab(tab.windowId, { format: "png" });
        const fileName = `${cleanFileName(tab.title || "page") || "page"}-${Date.now()}.png`;
        await chrome.downloads.download({
          url: dataUrl,
          filename: `NxTools Screenshots/${fileName}`,
          saveAs: false
        });
        return { ok: true, message: "截图已保存到浏览器下载目录" };
      }

      const result = await sendMessageToTab(tabId, payload, Number.isInteger(frameId) ? { frameId } : {});
      if (!result.ok) {
        return { ok: false, message: result.message };
      }
      return { ok: true, state: result.response };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "媒体操作失败"
      });
    });
    return true;
  }

  if (message.type === "popup_task_action") {
    void sendDesktopRequest("task_action", {
      taskId: String(message.taskId || ""),
      action: String(message.action || "")
    }).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "任务操作失败"
      });
    });
    return true;
  }

  if (message.type === "popup_clear_resources") {
    if (typeof message.tabId === "number" && message.tabId > 0) {
      clearTab(message.tabId);
    }
    void buildPopupState(null, String(message.currentView || "resources")).then(sendResponse);
    return true;
  }

  return false;
});
