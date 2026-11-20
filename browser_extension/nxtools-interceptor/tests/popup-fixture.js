"use strict";
// Isolated test/preview only. No native host, user storage or network APIs.
(() => {
  const resource = (id, kind, filename, size) => ({ id, filename, size, url: `https://synthetic.invalid/${id}`, domain: "synthetic.invalid", mime: kind === "audio" ? "audio/mpeg" : "application/octet-stream", source: "webRequest", description: { kind, resolution: kind === "video" ? "1920×1080" : "", badge: "合成资源" } });
  const initial = {
    settings: { interceptDownloads: true, captureFileTypes: "", captureMinimumKiB: 0, captureExcludedSites: "", captureExcludedAddresses: "", openDialog: true, captureResources: true, mediaOverlayEnabled: true, dropTargetEnabled: true, dropTargetPosition: "right-bottom", dropTargetWidth: 188, dropTargetHeight: 86, includeRequestHeaders: true, includeCookies: false, eraseBrowserHistory: false },
    capturePaused: false, capturePauseAvailable: true,
    status: { ok: true, code: "connected", message: "已连接", guidance: "可将页面和资源发送到 NxTools。" }, activeTab: { id: 7, title: "设计资料与课程媒体", url: "https://synthetic.invalid/course", domain: "synthetic.invalid" }, siteControl: { supported: true, enabled: true, host: "synthetic.invalid" },
    linkReview: null,
    captureSummary: { currentCount: 3, totalCount: 4, recentCount: 0, latestCapturedAt: Date.now() },
    currentResources: [resource("video-1", "video", "课程演示 · 第一章.mp4", 67108864), resource("audio-1", "audio", "课程讲解.mp3", 12582912), resource("file-1", "download", "练习素材.zip", 33554432)], otherResources: [resource("other-1", "download", "其他页面参考.pdf", 2097152)],
    featureStates: { mediaOverlay: true, deepSearch: false, mobileUserAgent: false }, mediaItems: [{ index: 0, label: "课程演示 · 第一章" }],
    mediaPlaybackState: { available: true, paused: true, muted: false, loop: false, mediaIndex: 0, localMediaIndex: 0, frameId: 0, duration: 240, currentTime: 80, progress: 1 / 3, volume: .8, speed: 1 }
  };
  const fixture = window.__fixture = { state: structuredClone(initial), messages: [], ports: [], failure: "", delay: 0, unhandled: [], viewStore: {} };
  window.addEventListener("unhandledrejection", event => { fixture.unhandled.push(String(event.reason)); event.preventDefault(); });
  window.chrome = { storage: { local: {
    async get(key) { return typeof key === "string" ? { [key]: fixture.viewStore[key] } : { ...fixture.viewStore }; },
    async set(values) { Object.assign(fixture.viewStore, values); }
  } }, runtime: { connect({ name }) {
    const port = { name, closed: false, disconnect() { this.closed = true; } };
    fixture.ports.push(port);
    return port;
  }, async sendMessage(message) {
    fixture.messages.push(structuredClone(message));
    if (fixture.delay) await new Promise(resolve => setTimeout(resolve, fixture.delay));
    if (fixture.failure === message.type) throw new Error("合成操作失败，请重试");
    const state = fixture.state;
    if (message.type === "popup_save_settings") state.settings = { ...state.settings, ...message.settings };
    if (message.type === "popup_set_capture_paused") { state.capturePaused = message.paused; return { ok: true, capturePaused: state.capturePaused }; }
    if (message.type === "popup_toggle_feature") state.featureStates[message.feature] = !state.featureStates[message.feature];
    if (message.type === "popup_clear_resources") { state.currentResources = []; state.captureSummary.currentCount = 0; }
    if (message.type === "popup_set_site_enabled") state.siteControl.enabled = message.enabled;
    if (message.type === "popup_test_connection") return { ok: true, code: "connected", message: "合成本地通道正常", guidance: "可将页面和资源发送到 NxTools。" };
    if (message.type === "popup_extract_links") {
      state.linkReview = { id: "review-page-1", tabId: state.activeTab.id, state: "ready", count: 2, message: "已提取 2 个页面链接，请复核后发送" };
      return { ok: true, message: state.linkReview.message, linkReview: structuredClone(state.linkReview) };
    }
    if (message.type === "popup_inspect_resource") {
      const inspection = { state: "ready", kind: "video", requiresMerge: false, encrypted: false, live: false, message: "识别为普通视频资源" };
      for (const list of [state.currentResources, state.otherResources]) { const item = list.find(resource => resource.id === message.resourceId); if (item) item.mediaInspection = inspection; }
      return { ok: true, resourceId: message.resourceId, inspection, message: inspection.message };
    }
    if (message.type === "popup_send_resources") return { ok: false, message: "1 项已接收，剩余项目可重试", acceptedResourceIds: message.resourceIds.slice(0, 1), data: { queued: true } };
    if (["popup_send_resource", "popup_send_active_tab", "popup_media_action", "popup_merge_resources"].includes(message.type)) return { ok: true, message: "合成操作已接收", data: { queued: true } };
    return structuredClone(state);
  } } };
  // Keep production initialization/actions, but don't leave a refresh interval in fixture pages.
  window.setInterval = () => 1;
})();
