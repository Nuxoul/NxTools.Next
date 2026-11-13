"use strict";

// Page media state, mobile User-Agent rule and feature toggles.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

function mobileRuleId(tabId) {
  return 100000 + Number(tabId || 0);
}

function updateMobileUserAgentRule(tabId, enabled) {
  const declarativeNetRequestApi = chrome["declarative" + "NetRequest"];
  if (!declarativeNetRequestApi?.updateSessionRules || !tabId || tabId <= 0) {
    return Promise.resolve();
  }

  const ruleId = mobileRuleId(tabId);
  return new Promise((resolve, reject) => {
    declarativeNetRequestApi.updateSessionRules(
      enabled
        ? {
            removeRuleIds: [ruleId],
            addRules: [
              {
                id: ruleId,
                priority: 1,
                action: {
                  type: "modifyHeaders",
                  requestHeaders: [
                    {
                      header: "User-Agent",
                      operation: "set",
                      value: MOBILE_USER_AGENT
                    }
                  ]
                },
                condition: {
                  tabIds: [tabId],
                  resourceTypes: Object.values(declarativeNetRequestApi.ResourceType)
                }
              }
            ]
          }
        : {
            removeRuleIds: [ruleId]
          },
      () => {
        const lastError = chrome.runtime.lastError;
        if (lastError) {
          reject(new Error(lastError.message || "更新 User-Agent 规则失败"));
          return;
        }
        resolve();
      }
    );
  });
}

async function persistFeatureState() {
  await chrome.storage.local.set({
    [STORAGE_MOBILE_UA_TABS_KEY]: [...mobileUserAgentTabs]
  });
}

function featureStatesForTab(tabId, settings) {
  return {
    mediaOverlay: settings.mediaOverlayEnabled !== false,
    deepSearch: settings.captureResources !== false,
    mobileUserAgent: Boolean(tabId && mobileUserAgentTabs.has(tabId))
  };
}

function createMediaLabel(source, type, frameId, localIndex, resolution = "") {
  const name = getFileNameFromUrl(source) || String(source || "").split("/").pop() || "";
  const kind = type === "audio" ? "音频" : "视频";
  const frameText = frameId > 0 ? ` · frame ${frameId}` : "";
  const resolutionText = resolution && type !== "audio" ? ` · ${resolution}` : "";
  return `${(name || `${kind} ${localIndex + 1}`).slice(0, 52)}${resolutionText}${frameText}`.slice(0, 86);
}

const pendingMediaInspections = new WeakMap();
const MEDIA_INSPECTION_STATES = new Set(["inspecting", "ready", "protected", "failed"]);
const MEDIA_INSPECTION_KINDS = new Set(["hls", "dash", "video", "audio", "file", "unknown"]);
const MEDIA_INSPECTION_REASONS = new Set(["started", "ready", "protected", "unavailable", "page_changed", "timeout", "too_large", "invalid_response", "unsupported"]);

function mediaInspectionMessage(state, kind, reason) {
  if (state === "inspecting") return "正在检查媒体资源";
  if (state === "ready") {
    if (kind === "hls") return "HLS 清单已检查";
    if (kind === "dash") return "DASH 清单已检查";
    if (kind === "audio") return "音频资源可用";
    if (kind === "video") return "视频资源可用";
    return "文件资源可用";
  }
  if (state === "protected") return "检测到受保护的媒体，NxTools 不会请求密钥或分段";
  return {
    unavailable: "当前页面无法检查此资源",
    page_changed: "页面已变化，请重新选择资源",
    timeout: "媒体检查超时，请重试",
    too_large: "媒体清单超过 2 MiB，未继续读取",
    invalid_response: "页面返回的媒体信息无效",
    unsupported: "无法识别此媒体资源"
  }[reason] || "页面返回的媒体信息无效";
}

function normalizeMediaInspection(value, fallbackReason = "invalid_response") {
  const source = value?.inspection && typeof value.inspection === "object" ? value.inspection : value;
  let state = MEDIA_INSPECTION_STATES.has(source?.state) ? source.state : "failed";
  if ((state === "ready" || state === "protected") && value?.inspection && value.ok !== true) state = "failed";
  let kind = MEDIA_INSPECTION_KINDS.has(source?.kind) ? source.kind : "unknown";
  let reason = MEDIA_INSPECTION_REASONS.has(source?.reason) ? source.reason : fallbackReason;
  if (state === "inspecting") reason = "started";
  else if (state === "ready") reason = "ready";
  else if (state === "protected") reason = "protected";
  else if (!["unavailable", "page_changed", "timeout", "too_large", "invalid_response", "unsupported"].includes(reason)) reason = "invalid_response";
  if (state === "failed" && !MEDIA_INSPECTION_KINDS.has(source?.kind)) kind = "unknown";
  return {
    state,
    kind,
    requiresMerge: (state === "ready" || state === "protected") && source?.requiresMerge === true,
    encrypted: state === "protected" || ((state === "ready" || state === "protected") && source?.encrypted === true),
    live: (state === "ready" || state === "protected") && source?.live === true,
    message: mediaInspectionMessage(state, kind, reason),
    reason
  };
}

function mediaInspectionResult(resourceId, inspection) {
  return { ok: inspection.state === "ready" || inspection.state === "protected", resourceId, inspection: { ...inspection }, message: inspection.message };
}

function setMediaInspection(resource, inspection) {
  Object.defineProperty(resource, "mediaInspection", {
    value: { ...inspection }, writable: true, configurable: true, enumerable: false
  });
}

function mediaInspectionSource(resource) {
  return { tabId: resource.tabId, frameId: resource.frameId, url: resource.url, pageKey: resource.pageKey,
    frameUrl: resource.frameUrl, frameDocumentId: resource.frameDocumentId, pageUrl: resource.pageUrl, source: resource.source };
}

function mediaInspectionOwnerMatches(resourceId, resource, bucket, source) {
  if (resourcesById.get(resourceId) !== resource || resourcesByTab.get(source.tabId) !== bucket ||
      ![...new Set(bucket?.values() || [])].includes(resource)) return false;
  return Object.entries(source).every(([field, value]) => resource[field] === value);
}

async function inspectMediaResource(resourceId) {
  const id = String(resourceId || "");
  const resource = resourcesById.get(id);
  const correlationId = nextRequestId();
  const startedAt = Date.now();
  if (!resource) {
    const inspection = normalizeMediaInspection({ state: "failed", kind: "unknown", reason: "unavailable" });
    sendExtensionLog("warn", "media_inspection_failed", "媒体资源检查失败", { reason: inspection.reason, elapsedMs: 0 }, 0, correlationId);
    return mediaInspectionResult(id, inspection);
  }
  const existingPending = pendingMediaInspections.get(resource);
  if (existingPending) return existingPending;

  const bucket = resourcesByTab.get(resource.tabId);
  const source = mediaInspectionSource(resource);
  if (!mediaInspectionOwnerMatches(id, resource, bucket, source)) {
    return mediaInspectionResult(id, normalizeMediaInspection({ state: "failed", reason: "page_changed" }, "page_changed"));
  }
  setMediaInspection(resource, normalizeMediaInspection({ state: "inspecting", kind: "unknown", reason: "started" }));
  sendExtensionLog("info", "media_inspection_started", "媒体资源检查已开始", { reason: "started", elapsedMs: 0 }, 0, correlationId);

  const pending = (async () => {
    let inspection;
    try {
      const settings = await getSettings();
      const originalFrame = await frameInfo(resource.tabId, Number.isInteger(source.frameId) ? source.frameId : 0);
      const expectedFrameUrl = source.frameUrl || source.pageKey;
      if (!mediaInspectionOwnerMatches(id, resource, bucket, source) || !originalFrame ||
          (source.frameDocumentId && source.frameDocumentId !== originalFrame.documentId) ||
          (expectedFrameUrl && normalizeUrlKey(expectedFrameUrl) !== normalizeUrlKey(originalFrame.url))) {
        inspection = normalizeMediaInspection({ state: "failed", reason: "page_changed" }, "page_changed");
      } else {
        const sent = await sendMessageToTab(resource.tabId, {
          type: "content_inspect_media_resource",
          url: resource.url,
          pageKey: resource.pageKey || "",
          mime: resource.mime || "",
          includeCredentials: settings.includeRequestHeaders === true && settings.includeCookies === true
        }, { frameId: Number.isInteger(resource.frameId) ? resource.frameId : 0 });
        const currentFrame = await frameInfo(resource.tabId, Number.isInteger(source.frameId) ? source.frameId : 0);
        if (!mediaInspectionOwnerMatches(id, resource, bucket, source) || !sameFrameDocument(originalFrame, currentFrame)) {
          inspection = normalizeMediaInspection({ state: "failed", reason: "page_changed" }, "page_changed");
        } else if (!sent.ok) {
          inspection = normalizeMediaInspection({ state: "failed", reason: "unavailable" }, "unavailable");
        } else {
          inspection = normalizeMediaInspection(sent.response);
        }
      }
    } catch {
      inspection = normalizeMediaInspection({ state: "failed", reason: "unavailable" }, "unavailable");
    }

    const elapsedMs = Math.max(0, Date.now() - startedAt);
    const current = mediaInspectionOwnerMatches(id, resource, bucket, source);
    if (!current) inspection = normalizeMediaInspection({ state: "failed", reason: "page_changed" }, "page_changed");
    else setMediaInspection(resource, inspection);
    sendExtensionLog(inspection.state === "failed" ? "warn" : "info",
      inspection.state === "failed" ? "media_inspection_failed" : "media_inspection_completed",
      inspection.state === "failed" ? "媒体资源检查失败" : "媒体资源检查完成",
      { reason: inspection.reason, elapsedMs }, 0, correlationId);
    return mediaInspectionResult(id, inspection);
  })().finally(() => {
    if (pendingMediaInspections.get(resource) === pending) pendingMediaInspections.delete(resource);
  });
  pendingMediaInspections.set(resource, pending);
  return pending;
}

async function getMediaState(tabId, index = 0) {
  const results = await sendMessageToTabFrames(tabId, {
    Message: "getVideoState",
    index: 0
  });

  const mediaFrames = results
    .filter((item) => item.ok && Number(item.response?.count || 0) > 0)
    .map((item) => ({
      frameId: item.frameId || 0,
      state: item.response,
      activeScore: Number(item.response?.activeScore || 0),
      activeIndex: Math.max(0, Number(item.response?.activeIndex || 0))
    }));

  if (mediaFrames.length === 0) {
    const result = results.find((item) => item.ok) || results[0] || { ok: false, message: "页面脚本未响应", frameId: 0 };
    return {
      mediaItems: [],
      mediaPlaybackState: {
        available: false,
        message: result.message || "当前页面未检测到可控制媒体",
        tabId: tabId || null,
        frameId: result.frameId || 0,
        mediaIndex: -1,
        currentTime: 0,
        duration: 0,
        progress: 0,
        volume: 1,
        paused: true,
        loop: false,
        muted: false,
        speed: 1
      }
    };
  }

  const mediaItems = [];
  for (const frame of mediaFrames) {
    const sources = Array.isArray(frame.state?.src) ? frame.state.src : [];
    const types = Array.isArray(frame.state?.types) ? frame.state.types : [];
    const widths = Array.isArray(frame.state?.widths) ? frame.state.widths : [];
    const heights = Array.isArray(frame.state?.heights) ? frame.state.heights : [];
    const resolutions = Array.isArray(frame.state?.resolutions) ? frame.state.resolutions : [];
    const count = Number(frame.state?.count || 0);
    for (let localIndex = 0; localIndex < count; localIndex += 1) {
      const source = sources[localIndex] || "";
      const type = types[localIndex] || "video";
      const width = Number(widths[localIndex] || 0);
      const height = Number(heights[localIndex] || 0);
      const resolution = String(resolutions[localIndex] || (width > 0 && height > 0 ? `${width}x${height}` : ""));
      mediaItems.push({
        index: mediaItems.length,
        frameId: frame.frameId,
        localIndex,
        source,
        width,
        height,
        resolution,
        label: createMediaLabel(source, type, frame.frameId, localIndex, resolution)
      });
    }
  }

  const activeFrame = [...mediaFrames].sort((left, right) => right.activeScore - left.activeScore)[0];
  const activeGlobalIndex = mediaItems.find((item) =>
    item.frameId === activeFrame.frameId &&
    item.localIndex === activeFrame.activeIndex)?.index ?? 0;
  const requestedIndex = Number.isInteger(index) && index >= 0 && index < mediaItems.length
    ? index
    : activeGlobalIndex;
  const selected = mediaItems[requestedIndex] || mediaItems[activeGlobalIndex] || mediaItems[0];
  let selectedState = mediaFrames.find((frame) => frame.frameId === selected.frameId)?.state;

  if (selected.localIndex !== 0) {
    const refreshed = await sendMessageToTab(tabId, {
      Message: "getVideoState",
      index: selected.localIndex
    }, { frameId: selected.frameId });
    if (refreshed.ok && refreshed.response) {
      selectedState = refreshed.response;
    }
  }

  const state = selectedState || {};
  return {
    mediaItems,
    mediaPlaybackState: {
      available: true,
      message: "",
      tabId,
      frameId: selected.frameId || 0,
      mediaIndex: selected.index,
      localMediaIndex: selected.localIndex,
      currentTime: Number(state.currentTime || 0),
      duration: Number(state.duration || 0),
      progress: Number(state.time || 0),
      volume: Number(state.volume ?? 1),
      paused: Boolean(state.paused),
      loop: Boolean(state.loop),
      muted: Boolean(state.muted),
      speed: Number(state.speed || 1),
      width: Number(selected.width || 0),
      height: Number(selected.height || 0),
      resolution: selected.resolution || ""
    }
  };
}

async function toggleFeature(feature, tabId) {
  const settings = await getSettings();
  if (feature === "mediaOverlay") {
    const enabled = settings.mediaOverlayEnabled === false;
    cachedSettings.mediaOverlayEnabled = enabled;
    await chrome.storage.local.set({ mediaOverlayEnabled: enabled });
    const tabs = await chrome.tabs.query({});
    for (const tab of tabs) {
      if (!tab.id || !/^https?:/i.test(tab.url || "")) {
        continue;
      }
      void sendMessageToTab(tab.id, {
        type: "media_overlay_set_enabled",
        enabled
      });
    }
    return { ok: true, message: enabled ? "已开启媒体悬浮下载按钮" : "已关闭媒体悬浮下载按钮" };
  }

  if (feature === "deepSearch") {
    const enabled = settings.captureResources === false;
    cachedSettings.captureResources = enabled;
    await chrome.storage.local.set({ captureResources: enabled });
    if (enabled && tabId > 0) {
      await scanTabForResources(tabId, true);
    }
    return { ok: true, message: enabled ? "已开启深度资源嗅探" : "已关闭深度资源嗅探" };
  }

  if (feature === "mobileUserAgent") {
    if (!tabId || tabId <= 0) {
      return { ok: false, message: "当前没有可操作的标签页" };
    }

    const enabled = !mobileUserAgentTabs.has(tabId);
    if (enabled) {
      mobileUserAgentTabs.add(tabId);
    } else {
      mobileUserAgentTabs.delete(tabId);
    }

    await updateMobileUserAgentRule(tabId, enabled);
    await persistFeatureState();
    return { ok: true, message: enabled ? "已开启手机 User-Agent，刷新后更稳定" : "已关闭手机 User-Agent" };
  }

  return { ok: false, message: "不支持的扩展功能" };
}
