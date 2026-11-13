"use strict";

// Popup rendering: tasks, resources, media tools, site toggle and settings.
// Builds DOM nodes directly; event handlers delegate to popup/actions.js.

function renderTasks(tasks) {
  elements.taskCountText.textContent = String(tasks.length);
  elements.taskTabCount.textContent = String(Math.min(tasks.length, 99));
  if (tasks.length === 0) {
    elements.taskList.replaceChildren(emptyState("桌面端暂无任务", "桌面端运行后可查看和管理下载任务。"));
    return;
  }

  const items = tasks.slice(0, 8).map((task) => {
    const name = task.displayFileName || task.DisplayFileName || task.url || "download";
    const progress = progressValue(task);
    const percent = progressLabel(progress, task);
    const primaryAction = taskActionLabel(task);
    const taskId = task.id || task.Id || "";
    const actions = el("div", { className: "task-actions" });
    if (primaryAction && taskId) {
      actions.append(el("button", {
        className: "mini-send",
        text: primaryAction.label,
        dataset: { taskId, action: primaryAction.action },
        attrs: { "aria-label": `${primaryAction.label} ${name}` }
      }));
    }
    if (taskId && !isCompletedStatus(task.status)) {
      actions.append(el("button", {
        className: "mini-send danger",
        text: "取消",
        dataset: { taskId, action: "cancel" },
        attrs: { "aria-label": `取消 ${name}` }
      }));
    }
    const bar = el("progress", { className: "bar", text: percent, attrs: { max: "100", "aria-label": `${name} 下载进度` } });
    bar.value = Number(progress.toFixed(2));
    return el("article", { className: "item task" },
      el("div", { className: "item-main" },
        el("strong", { text: name, title: name }),
        el("small", {},
          el("span", { className: "task-status", text: statusLabel(task.status), dataset: { statusTone: statusTone(task.status) } }),
          document.createTextNode(` · ${formatBytes(task.downloadedBytes)} / ${formatBytes(task.totalBytes)}`))),
      el("span", { className: "badge", text: percent }),
      el("div", { className: "task-progress" }, bar, actions));
  });
  elements.taskList.replaceChildren(...items);

  elements.taskList.querySelectorAll("[data-task-id][data-action]").forEach((button) => {
    button.addEventListener("click", () => runTaskAction(button));
  });
}

function resourceMetaText(resource) {
  const info = resource.description || {};
  return [
    resourceKindLabel(info.kind || resourceKind(resource)),
    info.resolution || "",
    resource.size ? formatBytes(resource.size) : "",
    isProtectedResource(resource) ? "受保护，无法发送" : ""
  ].filter(Boolean).join(" · ");
}

function resourceDetails(resource) {
  const info = resource.description || {};
  const blobNote = isBlobResource(resource) ? "Blob 临时地址" : "";
  const trackHint = resourceTrackHint(resource);
  // EX-06: the row keeps type/resolution/size; badge, track hint, MIME, source
  // and the full address stay reachable in the expanded detail and tooltip.
  return {
    title: [info.resolution || "", resourceKindLabel(info.kind), info.badge || "", trackHint, blobNote].filter(Boolean).join(" · "),
    origin: resource.domain || compactUrl(resource.url),
    detail: [resource.mime || "", sourceLabel(resource.source)].filter(Boolean).join(" · ")
  };
}

function inspectionFlags(inspection) {
  if (!inspection) return [];
  return [
    inspectionKindLabel(inspection.kind),
    inspection.requiresMerge ? "需要合并" : "",
    inspection.encrypted ? "已加密" : "",
    inspection.live ? "直播" : ""
  ].filter(Boolean);
}

function inspectionPanel(resource) {
  const inspection = mediaInspection(resource);
  const pending = inspectingResourceIds.has(resource.id) || inspection?.state === "inspecting";
  const button = el("button", {
    text: pending ? "识别中" : (inspection ? "重新识别" : "识别媒体"),
    dataset: { resourceInspect: resource.id },
    attrs: { "aria-label": `${inspection ? "重新识别" : "识别"} ${resource.filename || "资源"} 的媒体类型` }
  });
  button.disabled = pending;
  if (pending) button.setAttribute("aria-busy", "true");
  const message = inspection?.message || "仅在点击后读取有限媒体信息，不会自动请求清单。";
  const flags = inspectionFlags(inspection);
  return el("section", { className: "inspection-summary", dataset: { state: inspection?.state || "idle" } },
    el("div", { className: "inspection-heading" },
      el("strong", { text: `媒体识别 · ${inspectionStateLabel(inspection?.state)}` }), button),
    flags.length ? el("div", { className: "inspection-flags" }, ...flags.map((flag) => el("span", { className: "inspection-flag", text: flag }))) : null,
    el("p", { text: isProtectedResource(resource) ? `${message} 此资源已禁止选择和发送。` : message }));
}

function expandedPreview(resource) {
  const kind = previewKind(resource);
  const url = resource.url || "";
  const name = resource.filename || compactUrl(url) || "resource";
  const details = resourceDetails(resource);
  const media = !url || !kind
    ? null
    : kind === "image"
      ? el("img", { attrs: { src: url, alt: name } })
      : el(kind, { attrs: { src: url, controls: "", preload: "metadata", ...(kind === "video" ? { playsinline: "" } : {}) } });
  return el("div", { className: "resource-preview-panel", attrs: { id: `resource-preview-${resource.id}` } },
    media,
    el("dl", { className: "resource-facts" },
      details.title ? el("dt", { text: "类型" }) : null, details.title ? el("dd", { text: details.title }) : null,
      el("dt", { text: "来源" }), el("dd", { text: details.detail || "未知" }),
      el("dt", { text: "地址" }), el("dd", { className: "resource-url", text: compactUrl(url), title: url })),
    inspectionPanel(resource));
}

// Row thumbnails, in order: the image itself, the captured video poster, a
// direct video's first frame, then the page poster/og:image. Media nodes are
// kept across the periodic refresh so each source is fetched once per popup,
// and a slot starts loading only once it is scrolled into view. A failed
// source falls through to the next; the type icon stays underneath.
const thumbnailEntries = new Map();
const pendingThumbnailSlots = new WeakMap();
let renderedThumbnailIds = new Set();
let thumbnailObserver = null;

function isHttpSource(value) {
  return /^https?:\/\//i.test(String(value || ""));
}

function thumbnailCandidates(resource, pageThumbnail) {
  const kind = resourceKind(resource);
  const url = String(resource.url || "");
  const candidates = [];
  if (previewKind(resource) === "image" && isHttpSource(url)) candidates.push({ type: "img", src: url });
  if (isHttpSource(resource.poster)) candidates.push({ type: "img", src: resource.poster });
  if (kind === "video" && isHttpSource(url)) candidates.push({ type: "video", src: url });
  if ((kind === "video" || kind === "streaming" || kind === "audio") && isHttpSource(pageThumbnail)) {
    candidates.push({ type: "img", src: pageThumbnail });
  }
  return candidates.filter((candidate, index) =>
    candidates.findIndex((other) => other.src === candidate.src) === index);
}

function releaseThumbnailNode(node) {
  if (!node) return;
  node.remove();
  if (node.tagName === "VIDEO") {
    node.removeAttribute("src");
    node.load();
  }
}

function thumbnailEntry(resource, pageThumbnail) {
  const candidates = thumbnailCandidates(resource, pageThumbnail);
  const signature = candidates.map((candidate) => `${candidate.type}:${candidate.src}`).join("\n");
  let entry = thumbnailEntries.get(resource.id);
  if (!entry || entry.signature !== signature) {
    releaseThumbnailNode(entry?.node);
    entry = { signature, candidates, index: 0, node: null, ready: false };
    thumbnailEntries.set(resource.id, entry);
  }
  return entry;
}

function loadThumbnail(entry, slot) {
  const candidate = entry.candidates[entry.index];
  if (!candidate) return;
  const node = candidate.type === "video"
    ? el("video", { attrs: { src: candidate.src.includes("#") ? candidate.src : `${candidate.src}#t=0.1`, preload: "metadata", muted: "", playsinline: "", tabindex: "-1", "aria-hidden": "true" } })
    : el("img", { attrs: { src: candidate.src, alt: "", decoding: "async", referrerpolicy: "no-referrer" } });
  node.className = "resource-thumb-media";
  if (candidate.type === "video") node.muted = true;
  entry.node = node;
  entry.ready = false;
  node.addEventListener(candidate.type === "video" ? "loadeddata" : "load", () => {
    if (entry.node !== node) return;
    entry.ready = true;
    node.parentElement?.classList.add("has-media");
  }, { once: true });
  node.addEventListener("error", () => {
    if (entry.node !== node) return;
    const parent = node.parentElement;
    releaseThumbnailNode(node);
    parent?.classList.remove("has-media");
    entry.node = null;
    entry.ready = false;
    entry.index += 1;
    if (parent) loadThumbnail(entry, parent);
  }, { once: true });
  slot.append(node);
}

function observeThumbnail(slot, entry) {
  if (typeof IntersectionObserver !== "function") {
    loadThumbnail(entry, slot);
    return;
  }
  thumbnailObserver ||= new IntersectionObserver((records) => {
    for (const record of records) {
      if (!record.isIntersecting) continue;
      thumbnailObserver.unobserve(record.target);
      const pending = pendingThumbnailSlots.get(record.target);
      pendingThumbnailSlots.delete(record.target);
      if (pending && !pending.node && record.target.isConnected) loadThumbnail(pending, record.target);
    }
  });
  pendingThumbnailSlots.set(slot, entry);
  thumbnailObserver.observe(slot);
}

function resourceThumbnail(resource, kind, pageThumbnail) {
  const slot = el("span", { className: "resource-thumb", dataset: { kind }, title: resourceKindLabel(kind) }, kindIcon(kind));
  renderedThumbnailIds.add(resource.id);
  const entry = thumbnailEntry(resource, pageThumbnail);
  if (entry.node) {
    slot.append(entry.node);
    slot.classList.toggle("has-media", entry.ready);
  } else if (entry.index < entry.candidates.length) {
    observeThumbnail(slot, entry);
  }
  return slot;
}

function pruneThumbnailEntries() {
  for (const [resourceId, entry] of thumbnailEntries) {
    if (!renderedThumbnailIds.has(resourceId)) {
      releaseThumbnailNode(entry.node);
      thumbnailEntries.delete(resourceId);
    }
  }
}

const resourceListLimits = new WeakMap();

function renderResources(target, resources, emptyText, emptyHint, pageThumbnail = "") {
  const visibleResources = filterResourceList(resources);
  const visibleLimit = resourceListLimits.get(target) || 24;
  if (!visibleResources.length) {
    const searchHint = currentResourceSearch.trim() ? "没有匹配的资源；已选择项目仍会保留。" : emptyHint;
    target.replaceChildren(emptyState(emptyText, searchHint));
    updateBatchBar();
    return;
  }

  const rows = visibleResources.slice(0, visibleLimit).map((resource) => {
    const name = resource.filename || resource.url || "resource";
    const kind = resourceKind(resource);
    const isOpen = previewResourceId === resource.id;
    const protectedResource = isProtectedResource(resource);
    const inspectingResource = inspectingResourceIds.has(resource.id) || mediaInspection(resource)?.state === "inspecting";
    const details = resourceDetails(resource);
    const check = el("input", {
      className: "resource-check",
      type: "checkbox",
      dataset: { resourceCheck: resource.id },
      attrs: { "aria-label": `${protectedResource ? "无法选择受保护资源" : "选择"} ${name}` }
    });
    check.checked = selectedResourceIds.has(resource.id) && !protectedResource;
    check.disabled = protectedResource || inspectingResource;
    if (protectedResource) check.title = "受保护媒体无法发送";
    const actions = el("div", { className: "resource-actions" });
    actions.append(el("button", {
      className: "preview-action ghost",
      text: isOpen ? "收起" : "详情",
      dataset: { resourcePreview: resource.id },
      attrs: { "aria-expanded": String(isOpen), "aria-controls": `resource-preview-${resource.id}`, "aria-label": `${isOpen ? "收起详情" : "查看详情"} ${name}` }
    }));
    const sendButton = el("button", {
      className: "mini-send",
      text: "下载",
      dataset: { resourceId: resource.id },
      attrs: { "aria-label": `发送 ${name} 到 NxTools` }
    });
    sendButton.disabled = protectedResource || inspectingResource;
    if (protectedResource) sendButton.title = "受保护媒体无法发送";
    actions.append(sendButton);
    return el("article", {
      className: `item resource${isOpen ? " selected" : ""}${protectedResource ? " resource-protected" : ""}`,
      dataset: { resourceRow: resource.id, kind }
    },
      check,
      resourceThumbnail(resource, kind, pageThumbnail),
      el("div", { className: "resource-main" },
        el("div", { className: "resource-title" }, el("strong", { text: name, title: [name, details.title, details.origin].filter(Boolean).join("\n") })),
        el("div", { className: "resource-meta", text: resourceMetaText(resource) })),
      actions,
      isOpen ? expandedPreview(resource) : null);
  });
  target.replaceChildren(...rows);
  if (visibleResources.length > visibleLimit) {
    const more = el("button", { className: "ghost", text: `显示更多（${visibleLimit}/${visibleResources.length}）`,
      dataset: { resourceMore: target.id } });
    more.addEventListener("click", () => {
      resourceListLimits.set(target, visibleLimit + 24);
      renderResourceLists();
      target.querySelectorAll("[data-resource-preview]")[visibleLimit]?.focus({ preventScroll: true });
    });
    target.append(more);
  }

  target.querySelectorAll("[data-resource-row]").forEach((row) => {
    row.addEventListener("click", (event) => {
      if (event.target?.closest?.("button,input,a,audio,video")) return;
      toggleResourcePreview(row.dataset.resourceRow || "");
    });
  });
  target.querySelectorAll("[data-resource-preview]").forEach((button) => {
    button.addEventListener("click", () => toggleResourcePreview(button.dataset.resourcePreview));
  });
  target.querySelectorAll("[data-resource-inspect]").forEach((button) => {
    button.addEventListener("click", () => inspectResourceFromButton(button, button.dataset.resourceInspect));
  });
  target.querySelectorAll("[data-resource-id]").forEach((button) => {
    button.addEventListener("click", () => sendResourceFromButton(button, button.dataset.resourceId));
  });
  target.querySelectorAll("[data-resource-check]").forEach((checkbox) => {
    checkbox.addEventListener("change", () => {
      if (checkbox.disabled) return;
      if (checkbox.checked) selectedResourceIds.add(checkbox.dataset.resourceCheck);
      else selectedResourceIds.delete(checkbox.dataset.resourceCheck);
      updateBatchBar();
    });
  });

  updateBatchBar();
}

function renderResourceLists(state = currentState) {
  thumbnailObserver?.disconnect();
  renderedThumbnailIds = new Set();
  renderResources(elements.currentResourceList, state?.currentResources || [], "当前页面还没有匹配的资源",
    "播放页面媒体或开始浏览，资源会出现在这里。勾选可批量加入队列，详情不会下载。", state?.activeTab?.thumbnail || "");
  renderResources(elements.otherResourceList, state?.otherResources || [], "其他页面没有匹配的资源");
  elements.otherResourceCountText.textContent = String(filterResourceList(state?.otherResources || []).length);
  pruneThumbnailEntries();
}

function renderLinkReview(state = currentState) {
  const review = localLinkReviewStatus || state?.linkReview || null;
  elements.linkReviewStatus.hidden = !review;
  if (!review) return;
  const count = Number(review.count || 0);
  const prefix = review.state === "collecting" ? "正在提取链接" : review.state === "failed" ? "链接提取失败" : `链接待复核${count ? ` · ${count} 项` : ""}`;
  elements.linkReviewStatus.textContent = review.message ? `${prefix}：${review.message}` : prefix;
  elements.linkReviewStatus.dataset.state = review.state || "ready";
}

function handleLinkReviewNavigation(state) {
  const review = state?.linkReview;
  const activeTabId = Number(state?.activeTab?.id || 0);
  if (!review?.id || Number(review.tabId || 0) !== activeTabId || review.id === lastHandledLinkReviewId) return;
  lastHandledLinkReviewId = review.id;
  setView("resources", { trackIntent: true, persist: true });
}

// The batch bar appears only while resources are selected (1..32 per request).
function updateBatchBar() {
  const selectedCount = selectedResourceIds.size;
  if (batchToolbar) batchToolbar.hidden = selectedCount === 0;
  elements.currentResourceCountText.textContent = selectedCount > 32 ? `已选 ${selectedCount} 项 · 最多 32` : `已选 ${selectedCount} 项`;
  const pending = [...selectedResourceIds].some(id => inspectingResourceIds.has(id) || mediaInspection(findResourceById(id))?.state === "inspecting");
  elements.batchResourcesButton.disabled = selectedCount === 0 || selectedCount > 32 || pending;
  elements.batchResourcesButton.textContent = selectedCount > 0 ? `加入队列 ${selectedCount}/32` : "批量加入队列";
}

function renderCaptureSummary(state) {
  const summary = state.captureSummary || {};
  const currentCount = Number(summary.currentCount || 0);
  const totalCount = Number(summary.totalCount || 0);
  const recentCount = Number(summary.recentCount || 0);
  const connected = Boolean(state.status?.ok);
  const enabled = state.settings?.captureResources !== false && state.siteControl?.enabled !== false;

  elements.resourceTabCount.textContent = String(Math.min(currentCount, 99));
  elements.captureSummaryText.textContent = enabled ? `${currentCount} / ${totalCount}` : "已暂停";
  elements.captureSummaryText.title = enabled ? `当前页面 ${currentCount} 项，共 ${totalCount} 项` : "当前网站捕获已暂停";
  const age = relativeCaptureAge(summary.latestCapturedAt);
  elements.captureAgeText.textContent = enabled && recentCount > 0 ? `${recentCount} 新` : (enabled ? age : "");
  elements.signalRail.dataset.signalState = connected ? (enabled ? (recentCount > 0 ? "capturing" : "ready") : "paused") : "offline";
}

function renderFeatureButton(button, enabled) {
  button.classList.toggle("active", Boolean(enabled));
  button.setAttribute("aria-pressed", String(Boolean(enabled)));
}

function renderAdvanced(state) {
  const featureStates = state.featureStates || {};
  renderFeatureButton(elements.featureOverlayButton, featureStates.mediaOverlay);
  renderFeatureButton(elements.featureDeepScanButton, featureStates.deepSearch);
  renderFeatureButton(elements.featureMobileUaButton, featureStates.mobileUserAgent);

  const mediaItems = Array.isArray(state.mediaItems) ? state.mediaItems : [];
  const playback = state.mediaPlaybackState || {};
  if (Number.isInteger(playback.mediaIndex) && playback.mediaIndex >= 0) {
    selectedMediaIndex = playback.mediaIndex;
  }

  const options = mediaItems.length
    ? mediaItems.map((item) => {
        const option = el("option", { text: item.label, attrs: { value: item.index } });
        option.selected = item.index === selectedMediaIndex;
        return option;
      })
    : [el("option", { text: "未检测到媒体", attrs: { value: "" } })];
  elements.mediaSelect.replaceChildren(...options);

  const available = Boolean(playback.available);
  elements.mediaStateText.textContent = available
    ? `${formatTime(playback.currentTime)} / ${formatTime(playback.duration)}`
    : playback.message || "当前页面未检测到媒体";

  for (const control of [
    elements.mediaSelect,
    elements.mediaPlayButton,
    elements.mediaPipButton,
    elements.mediaScreenshotButton,
    elements.mediaTimeRange,
    elements.mediaVolumeRange,
    elements.mediaMuteButton,
    elements.mediaLoopButton,
    elements.mediaSpeedInput,
    elements.mediaSpeedButton
  ]) {
    control.disabled = !available;
  }

  elements.mediaPlayButton.textContent = playback.paused ? "播放" : "暂停";
  elements.mediaMuteButton.textContent = playback.muted ? "取消静音" : "静音";
  elements.mediaLoopButton.textContent = playback.loop ? "关闭循环" : "循环";
  elements.mediaMuteButton.setAttribute("aria-pressed", String(Boolean(playback.muted)));
  elements.mediaLoopButton.setAttribute("aria-pressed", String(Boolean(playback.loop)));
  elements.mediaSpeedInput.value = String(Number(playback.speed || 1).toFixed(2).replace(/\.?0+$/, ""));
  elements.mediaTimeRange.value = String(Math.round(Math.max(0, Math.min(1, playback.progress || 0)) * 1000));
  elements.mediaVolumeRange.value = String(Math.round(Math.max(0, Math.min(1, playback.volume ?? 1)) * 100));
}

function renderSiteToggle(site) {
  if (!elements.siteEnabledToggle) {
    return;
  }

  const supported = Boolean(site.supported);
  const enabled = supported ? site.enabled !== false : false;
  elements.siteEnabledToggle.checked = enabled;
  elements.siteEnabledToggle.disabled = !supported;

  if (!supported) {
    elements.siteToggleHint.textContent = "当前页面不支持按网站设置";
  } else {
    elements.siteToggleHint.textContent = `${site.host || "当前网站"} · ${enabled ? "自动接管中" : "已停用，交还浏览器"}`;
  }

  elements.siteToggleRow?.classList.toggle("disabled", !supported);
  elements.siteToggleRow?.classList.toggle("off", supported && !enabled);
}

function renderSettings(settings) {
  elements.interceptDownloads.checked = Boolean(settings.interceptDownloads);
  for (const key of captureRuleKeys) {
    if (!captureRuleDrafts.has(key) && document.activeElement !== elements[key]) elements[key].value = String(settings[key] ?? "");
  }
  elements.openDialog.checked = settings.openDialog !== false;
  elements.captureResources.checked = settings.captureResources !== false;
  elements.mediaOverlayEnabled.checked = settings.mediaOverlayEnabled !== false;
  elements.dropTargetEnabled.checked = settings.dropTargetEnabled !== false;
  elements.dropTargetPosition.value = settings.dropTargetPosition || "right-bottom";
  elements.dropTargetWidth.value = String(settings.dropTargetWidth || 188);
  elements.dropTargetHeight.value = String(settings.dropTargetHeight || 86);
  elements.eraseBrowserHistory.checked = Boolean(settings.eraseBrowserHistory);
  elements.includeRequestHeaders.checked = Boolean(settings.includeRequestHeaders);
  elements.includeCookies.checked = Boolean(settings.includeCookies);
}

function renderHomeCapture(state) {
  const settingsEnabled = state.settings?.interceptDownloads !== false;
  const site = state.siteControl || {};
  if (!settingsEnabled) {
    elements.homeCaptureStatus.textContent = "自动接管已关闭";
    elements.homeCaptureHint.textContent = "可在设置中启用；临时暂停不会更改此总开关。";
  } else if (state.capturePauseAvailable === false) {
    elements.homeCaptureStatus.textContent = "自动接管会话状态不可用";
    elements.homeCaptureHint.textContent = "浏览器保留下载；重试只读取会话状态。";
  } else if (state.capturePaused) {
    elements.homeCaptureStatus.textContent = "本次浏览器会话已暂停";
    elements.homeCaptureHint.textContent = "手动发送仍可用，不会更改下载接管总开关。";
  } else if (site.supported && site.enabled === false) {
    elements.homeCaptureStatus.textContent = "当前网站已停用自动接管";
    elements.homeCaptureHint.textContent = "可用上方网站开关恢复；手动发送仍可用。";
  } else if (!site.supported) {
    elements.homeCaptureStatus.textContent = "当前页面不支持自动接管";
    elements.homeCaptureHint.textContent = "仅 HTTP/HTTPS 页面支持，浏览器继续处理下载。";
  } else {
    elements.homeCaptureStatus.textContent = "自动接管已启用";
    elements.homeCaptureHint.textContent = "仅在 NxTools 明确接受后取消浏览器原下载。";
  }
}

function renderCapturePause(state) {
  const buttons = capturePauseButtons();
  const busy = buttons.some((button) => button.getAttribute("aria-busy") === "true");
  if (!busy) {
    for (const button of buttons) {
      button.textContent = state.capturePauseAvailable === false ? "重试会话状态" : state.capturePaused ? "恢复自动接管" : "临时暂停";
      button.setAttribute("aria-pressed", String(Boolean(state.capturePaused)));
      button.disabled = false;
    }
  }
  elements.capturePauseHint.textContent = state.capturePauseAvailable === false
    ? "会话状态不可用，请重试；浏览器保留当前下载。"
    : "临时暂停自动接管，不更改启用设置；重启浏览器后恢复";
  renderHomeCapture(state);
}

function connectionPresentation(status = {}) {
  const defaults = {
    connected: ["已连接", "桌面任务与发送功能可用。"],
    host_not_registered: ["浏览器接管未注册", "请在 NxTools 下载设置中重新注册浏览器接管，然后测试连接。"],
    host_forbidden: ["浏览器接管被拒绝", "请确认当前浏览器已在 NxTools 中获准，然后重新注册。"],
    host_start_failed: ["无法启动本地连接", "请启动或修复 NxTools 桌面端，然后测试连接。"],
    connection_timeout: ["连接 NxTools 超时", "请确认桌面端正在运行，再测试连接。"],
    connection_lost: ["与 NxTools 的连接已断开", "请确认桌面端仍在运行，再测试连接。"],
    desktop_unavailable: ["NxTools 未运行", "请先启动 NxTools 桌面端，再测试连接。"],
    native_rejected: ["NxTools 拒绝了连接请求", "请更新或重启 NxTools，然后测试连接。"]
  };
  const fallback = defaults[status.code] || defaults.desktop_unavailable;
  return {
    message: String(status.message || (status.ok ? defaults.connected[0] : fallback[0])),
    guidance: String(status.guidance || (status.ok ? defaults.connected[1] : fallback[1]))
  };
}

function renderConnection(status) {
  const presentation = connectionPresentation(status);
  elements.statusText.textContent = presentation.message;
  elements.statusText.removeAttribute("title");
  elements.statusDot.classList.toggle("ok", Boolean(status.ok));
  elements.statusDot.classList.toggle("bad", !status.ok);
  elements.homeConnectionMessage.textContent = presentation.message;
  elements.homeConnectionGuidance.textContent = presentation.guidance;
  elements.homeConnectionPanel.dataset.connected = String(Boolean(status.ok));
}

function renderHandoff(status) {
  const explanations = {
    rule_excluded: "自动接管规则已排除，下载由浏览器继续处理。",
    temporarily_paused: "自动接管已暂停，下载由浏览器继续处理。",
    unsupported_request_method: "此下载使用 POST 等非 GET 请求，已保留在浏览器，未重放请求。",
    desktop_unavailable: "桌面未连接，下载仍由浏览器处理。",
    desktop_refused: "桌面拒绝接管，下载仍由浏览器处理。",
    handoff_timeout: "接管等待超时，未交出浏览器下载所有权。",
    handoff_disconnected: "接管连接已断开，未交出浏览器下载所有权。",
    identity_mismatch: "接管回复身份不匹配，下载仍由浏览器处理。",
    confirmation_pending: "确认窗口已打开，尚未创建任务；浏览器下载保留。",
    queued_not_accepted: "请求仅进入等待队列，尚未持久接受；浏览器下载保留。",
    handoff_started: "正在请求桌面接管，浏览器下载暂时保留。",
    desktop_accepted: "桌面已持久接受，正在取消浏览器原下载。",
    browser_transferred: status?.queued
      ? "桌面已持久接受并排队，浏览器原下载已取消；尚未下载完成。"
      : "下载已交给桌面，浏览器原下载已取消；结果以桌面任务为准。",
    browser_cancel_failed: status?.compensation === "failed"
      ? "浏览器取消失败，撤销桌面任务也未获确认；请检查桌面任务，原下载仍保留。"
      : "浏览器取消失败，已请求撤销桌面任务；原下载仍保留。"
  };
  const explanation = explanations[status?.reason] || "";
  elements.handoffStatus.hidden = !explanation;
  elements.handoffStatus.textContent = explanation ? `最近接管：${explanation}` : "";
  elements.handoffStatus.dataset.state = status?.state || "";
}

function render(state) {
  renderRevision += 1;
  const focused = document.activeElement;
  const focusAttribute = ["data-resource-preview", "data-resource-inspect", "data-resource-check", "data-resource-id", "data-task-id"].find(name => focused?.hasAttribute?.(name));
  const focusValue = focusAttribute ? focused.getAttribute(focusAttribute) : "";
  const focusAction = focused?.dataset?.action || "";
  const mediaDraft = focused === elements.mediaSpeedInput ? focused.value : null;
  currentState = state;
  handleLinkReviewNavigation(state);
  localLinkReviewStatus = state.linkReview || null;
  const resourceById = new Map(allResourcesFromState(state).map((resource) => [resource.id, resource]));
  selectedResourceIds = new Set(
    [...selectedResourceIds].filter((resourceId) => resourceById.has(resourceId) && !isProtectedResource(resourceById.get(resourceId)))
  );
  renderSettings(state.settings || {});
  renderCapturePause(state);
  renderConnection(state.status || {});
  renderHandoff(state.lastHandoff);

  const activeTab = state.activeTab || {};
  elements.activeTabTitle.textContent = activeTab.title || activeTab.url || "-";
  elements.activeTabTitle.title = activeTab.title || activeTab.url || "";
  elements.activeTabDomain.textContent = activeTab.domain || "未检测到网页";
  elements.sendTabButton.disabled = !/^https?:\/\//i.test(activeTab.url || "");
  elements.extractLinksButton.disabled = !/^https?:\/\//i.test(activeTab.url || "");

  renderSiteToggle(state.siteControl || {});
  renderCaptureSummary(state);
  renderTasks(Array.isArray(state.tasks) ? state.tasks : []);
  if (!previewResourceId || !findResourceById(previewResourceId, state)) previewResourceId = "";
  renderResourceLists(state);
  renderLinkReview(state);
  renderAdvanced(state);
  if (mediaDraft !== null) elements.mediaSpeedInput.value = mediaDraft;
  if (focusAttribute) {
    const actionSelector = focusAction ? `[data-action="${CSS.escape(focusAction)}"]` : "";
    document.querySelector(`[${focusAttribute}="${CSS.escape(focusValue)}"]${actionSelector}`)?.focus({ preventScroll: true });
  }
}

function setView(view, options = {}) {
  if (!["tasks", "resources", "advanced", "settings"].includes(view)) return false;
  currentView = view;
  if (options.trackIntent) viewSelectionRevision += 1;
  document.querySelectorAll(".tab").forEach((button) => {
    const active = button.dataset.view === view;
    button.classList.toggle("active", active);
    button.setAttribute("aria-selected", String(active));
    button.tabIndex = active ? 0 : -1;
  });
  document.querySelectorAll(".view").forEach((section) => {
    const active = section.id === `${view}View`;
    section.classList.toggle("active", active);
    section.setAttribute("aria-hidden", String(!active));
  });
  if (options.persist) persistViewPreference(view);
  return true;
}

function toggleResourcePreview(resourceId) {
  previewResourceId = previewResourceId === resourceId ? "" : resourceId;
  renderResourceLists();
  document.querySelector(`[data-resource-preview="${CSS.escape(resourceId)}"]`)?.focus({ preventScroll: true });
  if (previewResourceId) document.querySelector(`[data-resource-row="${CSS.escape(resourceId)}"] .resource-preview-panel`)?.scrollIntoView({ block: "nearest", behavior: "instant" });
}
