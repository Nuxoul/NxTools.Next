"use strict";

let currentState = null;
let currentView = "tasks";
let currentResourceFilter = "all";
let selectedResourceIds = new Set();
let previewResourceId = "";
let selectedMediaIndex = 0;
let refreshTimer = null;
let refreshInFlight = null;
let busyActionCount = 0;

const elements = {
  interceptDownloads: document.getElementById("interceptDownloads"),
  openDialog: document.getElementById("openDialog"),
  captureResources: document.getElementById("captureResources"),
  mediaOverlayEnabled: document.getElementById("mediaOverlayEnabled"),
  dropTargetEnabled: document.getElementById("dropTargetEnabled"),
  dropTargetPosition: document.getElementById("dropTargetPosition"),
  dropTargetWidth: document.getElementById("dropTargetWidth"),
  dropTargetHeight: document.getElementById("dropTargetHeight"),
  eraseBrowserHistory: document.getElementById("eraseBrowserHistory"),
  includeRequestHeaders: document.getElementById("includeRequestHeaders"),
  includeCookies: document.getElementById("includeCookies"),
  saveButton: document.getElementById("saveButton"),
  testButton: document.getElementById("testButton"),
  sendTabButton: document.getElementById("sendTabButton"),
  clearResourcesButton: document.getElementById("clearResourcesButton"),
  batchResourcesButton: document.getElementById("batchResourcesButton"),
  mergeResourcesButton: document.getElementById("mergeResourcesButton"),
  statusText: document.getElementById("statusText"),
  statusDot: document.getElementById("statusDot"),
  signalRail: document.getElementById("signalRail"),
  captureSummaryText: document.getElementById("captureSummaryText"),
  captureAgeText: document.getElementById("captureAgeText"),
  taskTabCount: document.getElementById("taskTabCount"),
  resourceTabCount: document.getElementById("resourceTabCount"),
  activeTabTitle: document.getElementById("activeTabTitle"),
  activeTabDomain: document.getElementById("activeTabDomain"),
  siteEnabledToggle: document.getElementById("siteEnabledToggle"),
  siteToggleHint: document.getElementById("siteToggleHint"),
  siteToggleRow: document.getElementById("siteToggleRow"),
  taskCountText: document.getElementById("taskCountText"),
  taskList: document.getElementById("taskList"),
  currentResourceList: document.getElementById("currentResourceList"),
  currentResourceCountText: document.getElementById("currentResourceCountText"),
  otherResourceList: document.getElementById("otherResourceList"),
  mediaStateText: document.getElementById("mediaStateText"),
  mediaSelect: document.getElementById("mediaSelect"),
  mediaPlayButton: document.getElementById("mediaPlayButton"),
  mediaPipButton: document.getElementById("mediaPipButton"),
  mediaScreenshotButton: document.getElementById("mediaScreenshotButton"),
  mediaTimeRange: document.getElementById("mediaTimeRange"),
  mediaVolumeRange: document.getElementById("mediaVolumeRange"),
  mediaMuteButton: document.getElementById("mediaMuteButton"),
  mediaLoopButton: document.getElementById("mediaLoopButton"),
  mediaSpeedInput: document.getElementById("mediaSpeedInput"),
  mediaSpeedButton: document.getElementById("mediaSpeedButton"),
  featureOverlayButton: document.getElementById("featureOverlayButton"),
  featureDeepScanButton: document.getElementById("featureDeepScanButton"),
  featureMobileUaButton: document.getElementById("featureMobileUaButton"),
  messageText: document.getElementById("messageText")
};

function sendMessage(message, timeoutMs = 10000) {
  const request = chrome.runtime.sendMessage(message);
  if (!timeoutMs) {
    return request;
  }

  return new Promise((resolve, reject) => {
    let settled = false;
    const timeoutId = setTimeout(() => {
      if (settled) {
        return;
      }
      settled = true;
      reject(new Error("等待扩展后台或 NxTools 响应超时"));
    }, timeoutMs);

    request.then(
      (value) => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timeoutId);
        resolve(value);
      },
      (error) => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timeoutId);
        reject(error);
      });
  });
}

function collectSettings() {
  return {
    interceptDownloads: elements.interceptDownloads.checked,
    openDialog: elements.openDialog.checked,
    captureResources: elements.captureResources.checked,
    mediaOverlayEnabled: elements.mediaOverlayEnabled.checked,
    dropTargetEnabled: elements.dropTargetEnabled.checked,
    dropTargetPosition: elements.dropTargetPosition.value,
    dropTargetWidth: Math.max(96, Math.min(360, Number(elements.dropTargetWidth.value || 188))),
    dropTargetHeight: Math.max(48, Math.min(220, Number(elements.dropTargetHeight.value || 86))),
    eraseBrowserHistory: elements.eraseBrowserHistory.checked,
    includeRequestHeaders: elements.includeRequestHeaders.checked,
    includeCookies: elements.includeCookies.checked
  };
}

function setMessage(text) {
  elements.messageText.textContent = text || "";
}

function isQueuedDesktopResult(result) {
  return Boolean(result?.data?.queued);
}

function closePopupSoon() {
  setTimeout(() => window.close(), 180);
}

function beginBusy(button, busyText) {
  busyActionCount += 1;
  if (!button) {
    return;
  }

  button.dataset.originalText = button.textContent || "";
  button.textContent = busyText;
  button.disabled = true;
}

function endBusy(button) {
  busyActionCount = Math.max(0, busyActionCount - 1);
  if (!button) {
    return;
  }

  button.disabled = false;
  if (button.dataset.originalText) {
    button.textContent = button.dataset.originalText;
    delete button.dataset.originalText;
  }
}

function formatBytes(value) {
  const bytes = Number(value || 0);
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "--";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = bytes;
  let index = 0;
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index += 1;
  }
  return `${size.toFixed(size >= 100 || index === 0 ? 0 : 1)} ${units[index]}`;
}

function relativeCaptureAge(value) {
  const timestamp = Number(value || 0);
  if (!timestamp) {
    return "等待资源";
  }
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 8) return "刚刚";
  if (seconds < 60) return `${seconds}s 前`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m 前`;
  return `${Math.floor(minutes / 60)}h 前`;
}

function statusLabel(status) {
  const text = String(status || "").toLowerCase();
  if (text.includes("downloading")) return "下载中";
  if (text.includes("created") || text.includes("waiting")) return "等待";
  if (text.includes("queued")) return "排队";
  if (text.includes("paused")) return "暂停";
  if (text.includes("completed")) return "完成";
  if (text.includes("failed")) return "失败";
  if (text.includes("verifying")) return "校验";
  if (text.includes("finalizing")) return "收尾";
  if (text.includes("cancelled")) return "已取消";
  return status || "-";
}

function isCompletedStatus(status) {
  return String(status || "").toLowerCase().includes("completed");
}

function isActiveStatus(status) {
  const text = String(status || "").toLowerCase();
  return text.includes("downloading") || text.includes("waiting") || text.includes("queued") ||
    text.includes("finalizing") || text.includes("verifying") || text.includes("probing");
}

function resourceKind(resource) {
  return resource?.description?.kind || "download";
}

function isPreviewableResource(resource) {
  const kind = resourceKind(resource);
  const mime = String(resource?.mime || "").toLowerCase();
  return kind === "image" || kind === "audio" || kind === "video" ||
    mime.startsWith("image/") || mime.startsWith("audio/") || mime.startsWith("video/");
}

function isBlobResource(resource) {
  return String(resource?.url || "").startsWith("blob:");
}

function filterResourceList(resources) {
  if (currentResourceFilter === "all") {
    return resources;
  }
  return resources.filter((resource) => resourceKind(resource) === currentResourceFilter);
}

function sourceLabel(source) {
  const text = String(source || "").trim();
  if (!text) {
    return "来源未知";
  }

  return text
    .replace(/^webRequest-/, "网络-")
    .replace(/^content-/, "页面-")
    .replace(/^performance:/, "性能:")
    .replace(/^xhr-response$/, "XHR 响应")
    .replace(/^fetch-response$/, "Fetch 响应");
}

function resourceKindLabel(kind) {
  switch (kind) {
    case "streaming":
      return "流媒体";
    case "video":
      return "视频";
    case "audio":
      return "音频";
    case "image":
      return "图片";
    case "download":
      return "文件";
    default:
      return kind || "资源";
  }
}

function resourceTrackHint(resource) {
  const text = [
    resource?.description?.kind,
    resource?.description?.badge,
    resource?.mime,
    resource?.url,
    resource?.filename
  ].join(" ").toLowerCase();

  if (text.includes("mime_type=audio") || text.includes("mime=audio") || text.includes("audio/") || text.includes("media-audio")) {
    return "通常为音频轨";
  }

  if (text.includes("mime_type=video") || text.includes("mime=video") || text.includes("video/") || text.includes("media-video")) {
    return "通常为仅视频轨";
  }

  if (text.includes(".m3u8") || text.includes("mpegurl")) {
    return "播放列表";
  }

  return "";
}

function compactUrl(url) {
  const text = String(url || "");
  if (!text) {
    return "";
  }

  if (text.startsWith("blob:")) {
    try {
      const inner = new URL(text.slice(5));
      const shortPath = inner.pathname.length > 54 ? `${inner.pathname.slice(0, 51)}...` : inner.pathname;
      return `blob:${inner.hostname}${shortPath}`;
    } catch {
      return text.length > 110 ? `${text.slice(0, 107)}...` : text;
    }
  }

  try {
    const parsed = new URL(text);
    return `${parsed.hostname}${parsed.pathname}`;
  } catch {
    return text.length > 110 ? `${text.slice(0, 107)}...` : text;
  }
}

function allResourcesFromState(state = currentState) {
  return [
    ...(Array.isArray(state?.currentResources) ? state.currentResources : []),
    ...(Array.isArray(state?.otherResources) ? state.otherResources : [])
  ];
}

function findResourceById(resourceId, state = currentState) {
  return allResourcesFromState(state).find((resource) => resource.id === resourceId) || null;
}

function resourcePreviewMarkup(resource) {
  const kind = resourceKind(resource);
  const mime = String(resource.mime || "").toLowerCase();
  const url = resource.url || "";
  if (kind === "image" || mime.startsWith("image/")) {
    return `<img class="resource-thumb preview-thumb" src="${escapeHtml(url)}" alt="">`;
  }

  if (kind === "audio" || mime.startsWith("audio/")) {
    return `<span class="resource-thumb symbol">音频<br>预览</span>`;
  }

  if (kind === "video" || mime.startsWith("video/")) {
    return `<span class="resource-thumb symbol">视频<br>预览</span>`;
  }

  return `<span class="resource-thumb symbol">${escapeHtml(resourceKindLabel(kind))}</span>`;
}

function expandedResourcePreviewMarkup(resource) {
  if (!isPreviewableResource(resource)) {
    return "";
  }

  const kind = resourceKind(resource);
  const mime = String(resource.mime || "").toLowerCase();
  const url = resource.url || "";
  const name = resource.filename || compactUrl(url) || "resource";
  if (!url) {
    return "";
  }

  if (kind === "image" || mime.startsWith("image/")) {
    return `<div class="resource-preview-panel"><img src="${escapeHtml(url)}" alt="${escapeHtml(name)}"></div>`;
  }

  if (kind === "audio" || mime.startsWith("audio/")) {
    return `<div class="resource-preview-panel"><audio src="${escapeHtml(url)}" controls preload="metadata"></audio></div>`;
  }

  if (kind === "video" || mime.startsWith("video/")) {
    return `<div class="resource-preview-panel"><video src="${escapeHtml(url)}" controls preload="metadata" playsinline></video></div>`;
  }

  return "";
}

function updateMergeButton() {
  const selectedCount = selectedResourceIds.size;
  elements.batchResourcesButton.disabled = selectedCount === 0 || selectedCount > 32;
  elements.batchResourcesButton.textContent = selectedCount > 0
    ? `加入队列 ${selectedCount}/32`
    : "批量加入队列";
  elements.mergeResourcesButton.disabled = true;
  elements.mergeResourcesButton.textContent = selectedCount > 0
    ? `合并 ${selectedCount}/2 · 后续`
    : "合并 · 后续";
}

function formatTime(value) {
  const seconds = Math.max(0, Math.floor(Number(value || 0)));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  if (hours > 0) {
    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
  }
  return `${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
}

function progressValue(task) {
  if (isCompletedStatus(task.status)) {
    return 100;
  }

  const total = Number(task.totalBytes || 0);
  const done = Number(task.downloadedBytes || 0);
  let value = 0;
  if (Number.isFinite(total) && total > 0 && Number.isFinite(done) && done >= 0) {
    value = (Math.min(done, total) / total) * 100;
  } else {
    const progress = Number(task.progress || 0);
    if (Number.isFinite(progress) && progress > 0) {
      value = progress <= 1 ? progress * 100 : progress;
    }
  }

  value = Math.max(0, Math.min(100, value));
  return isActiveStatus(task.status) && value >= 100 ? 99.5 : value;
}

function progressLabel(value, task) {
  if (isCompletedStatus(task.status)) {
    return "100%";
  }
  return `${Math.floor(Math.max(0, Math.min(99, value)))}%`;
}

function taskActionLabel(task) {
  const text = String(task.status || "").toLowerCase();
  if (text.includes("created") || text.includes("waiting") || text.includes("paused") || text.includes("failed")) {
    return { action: "resume", label: "继续" };
  }
  if (text.includes("downloading") || text.includes("queued") || text.includes("probing")) {
    return { action: "pause", label: "暂停" };
  }
  return null;
}

function emptyList(target, text) {
  target.innerHTML = `<div class="empty">${text}</div>`;
}

function renderTasks(tasks) {
  elements.taskCountText.textContent = String(tasks.length);
  elements.taskTabCount.textContent = String(Math.min(tasks.length, 99));
  if (tasks.length === 0) {
    emptyList(elements.taskList, "桌面端暂无任务");
    return;
  }

  elements.taskList.innerHTML = tasks.slice(0, 8).map((task) => {
    const name = task.displayFileName || task.DisplayFileName || task.url || "download";
    const progress = progressValue(task);
    const primaryAction = taskActionLabel(task);
    const taskId = task.id || task.Id || "";
    return `
      <article class="item">
        <div class="item-main">
          <strong title="${escapeHtml(name)}">${escapeHtml(name)}</strong>
          <small>${escapeHtml(statusLabel(task.status))} · ${formatBytes(task.downloadedBytes)} / ${formatBytes(task.totalBytes)}</small>
        </div>
        <span class="badge">${progressLabel(progress, task)}</span>
        <div class="task-actions">
          ${primaryAction && taskId ? `<button class="mini-send" data-task-id="${escapeHtml(taskId)}" data-action="${primaryAction.action}">${primaryAction.label}</button>` : ""}
          ${taskId && !isCompletedStatus(task.status) ? `<button class="mini-send danger" data-task-id="${escapeHtml(taskId)}" data-action="cancel">取消</button>` : ""}
        </div>
        <div class="bar"><i style="width:${progress.toFixed(2)}%"></i></div>
      </article>
    `;
  }).join("");

  elements.taskList.querySelectorAll("[data-task-id][data-action]").forEach((button) => {
    button.addEventListener("click", async () => {
      beginBusy(button, "处理中");
      try {
        setMessage("正在发送任务操作");
        const result = await sendMessage({
          type: "popup_task_action",
          taskId: button.dataset.taskId,
          action: button.dataset.action
        }, 12000);
        setMessage(result.message || (result.ok ? "操作已发送" : "操作失败"));
        await refresh({ force: true });
      } catch (error) {
        setMessage(error instanceof Error ? error.message : "任务操作失败");
      } finally {
        endBusy(button);
      }
    });
  });
}

async function sendResourceFromButton(button, resourceId) {
  beginBusy(button, "发送中");
  try {
    setMessage("正在发送到 NxTools");
    const result = await sendMessage({
      type: "popup_send_resource",
      resourceId
    }, 14000);
    setMessage(result.message || (result.ok ? "已发送到 NxTools" : "发送失败"));
    if (result.ok !== false && !isQueuedDesktopResult(result)) {
      closePopupSoon();
      return;
    }
    await refresh({ force: true });
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "发送资源失败");
  } finally {
    endBusy(button);
  }
}

function renderResources(target, resources, emptyText) {
  const visibleResources = filterResourceList(resources);
  if (!visibleResources.length) {
    emptyList(target, emptyText);
    updateMergeButton();
    return;
  }

  target.innerHTML = visibleResources.slice(0, 24).map((resource) => {
    const info = resource.description || {};
    const name = resource.filename || resource.url || "resource";
    const kind = resourceKind(resource);
    const thumb = isPreviewableResource(resource)
      ? resourcePreviewMarkup(resource)
      : `<span class="resource-thumb symbol">${escapeHtml(resourceKindLabel(kind))}</span>`;
    const isSelected = previewResourceId === resource.id;
    const expandedPreview = isSelected ? expandedResourcePreviewMarkup(resource) : "";
    const resolution = info.resolution || "";
    const blobNote = isBlobResource(resource)
      ? "Blob 临时地址"
      : "";
    const trackHint = resourceTrackHint(resource);
    // EX-06: only type, resolution and size stay as stable chips; the rest
    // moves to the title tooltip or the detail line.
    const chips = [
      resolution ? { text: resolution, important: true } : null,
      { text: resourceKindLabel(info.kind), important: true },
      resource.size ? { text: formatBytes(resource.size) } : null
    ].filter(Boolean);
    const title = [
      resolution,
      resourceKindLabel(info.kind),
      info.badge || "",
      trackHint,
      blobNote
    ].filter(Boolean).join(" · ");
    const subtitle = [
      name,
      resource.domain || compactUrl(resource.url)
    ].filter(Boolean).join("  |  ");
    const detailLine = [
      resource.size ? formatBytes(resource.size) : "",
      resource.mime || "",
      sourceLabel(resource.source)
    ].filter(Boolean).join("  ·  ");
    return `
      <article class="item resource ${previewResourceId === resource.id ? "selected" : ""}" data-resource-row="${escapeHtml(resource.id)}">
        <input class="resource-check" type="checkbox" data-resource-check="${escapeHtml(resource.id)}" ${selectedResourceIds.has(resource.id) ? "checked" : ""}>
        ${thumb}
        <div class="resource-main">
          <div class="resource-title">
            <strong title="${escapeHtml(title || name)}">${escapeHtml(title || name)}</strong>
          </div>
          <div class="resource-name" title="${escapeHtml(subtitle)}">${escapeHtml(subtitle)}</div>
          <div class="resource-meta">
            ${chips.map((chip) => `<span class="meta-chip${chip.important ? " important" : ""}" title="${escapeHtml(chip.text)}">${escapeHtml(chip.text)}</span>`).join("")}
          </div>
          <div class="resource-url" title="${escapeHtml(resource.url || "")}">${escapeHtml(detailLine || compactUrl(resource.url))}</div>
      </div>
        <div class="resource-actions">
          <button class="mini-send" data-resource-id="${escapeHtml(resource.id)}">解析</button>
        </div>
        ${expandedPreview}
      </article>
    `;
  }).join("");

  target.querySelectorAll("[data-resource-row]").forEach((row) => {
    row.addEventListener("click", (event) => {
      if (event.target?.closest?.("button,input")) {
        return;
      }
      previewResourceId = row.dataset.resourceRow || "";
      render(currentState);
    });
  });

  target.querySelectorAll(".mini-send").forEach((button) => {
    button.addEventListener("click", async () => {
      await sendResourceFromButton(button, button.dataset.resourceId);
    });
  });

  target.querySelectorAll("[data-resource-check]").forEach((checkbox) => {
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) {
        selectedResourceIds.add(checkbox.dataset.resourceCheck);
      } else {
        selectedResourceIds.delete(checkbox.dataset.resourceCheck);
      }
      updateMergeButton();
    });
  });

  updateMergeButton();
}

function renderCaptureSummary(state) {
  const summary = state.captureSummary || {};
  const currentCount = Number(summary.currentCount || 0);
  const totalCount = Number(summary.totalCount || 0);
  const recentCount = Number(summary.recentCount || 0);
  const connected = Boolean(state.status?.ok);
  const enabled = state.settings?.captureResources !== false && state.siteControl?.enabled !== false;

  elements.resourceTabCount.textContent = String(Math.min(currentCount, 99));
  elements.currentResourceCountText.textContent = `${currentCount} 个资源`;
  elements.captureSummaryText.textContent = enabled
    ? `${currentCount} 当前 · ${totalCount} 总计`
    : "当前网站捕获已暂停";
  elements.captureAgeText.textContent = enabled
    ? (recentCount > 0 ? `${recentCount} 新 · ${relativeCaptureAge(summary.latestCapturedAt)}` : relativeCaptureAge(summary.latestCapturedAt))
    : "PAUSED";
  elements.signalRail.classList.toggle("online", connected && enabled && recentCount === 0);
  elements.signalRail.classList.toggle("capturing", connected && enabled && recentCount > 0);
  elements.signalRail.classList.toggle("offline", !connected);
  elements.signalRail.classList.toggle("paused", connected && !enabled);
}

function escapeHtml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderFeatureButton(button, enabled) {
  button.classList.toggle("active", Boolean(enabled));
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

  elements.mediaSelect.innerHTML = mediaItems.length
    ? mediaItems.map((item) => `<option value="${item.index}" ${item.index === selectedMediaIndex ? "selected" : ""}>${escapeHtml(item.label)}</option>`).join("")
    : `<option value="">未检测到媒体</option>`;

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

function render(state) {
  currentState = state;
  const liveResourceIds = new Set([
    ...(state.currentResources || []),
    ...(state.otherResources || [])
  ].map((resource) => resource.id));
  selectedResourceIds = new Set(
    [...selectedResourceIds].filter((resourceId) => liveResourceIds.has(resourceId))
  );
  const settings = state.settings || {};
  elements.interceptDownloads.checked = Boolean(settings.interceptDownloads);
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

  const status = state.status || {};
  elements.statusText.textContent = status.ok ? "桌面在线" : (status.message || "桌面离线");
  elements.statusDot.classList.toggle("ok", Boolean(status.ok));
  elements.statusDot.classList.toggle("bad", !status.ok);

  const activeTab = state.activeTab || {};
  elements.activeTabTitle.textContent = activeTab.title || activeTab.url || "-";
  elements.activeTabDomain.textContent = activeTab.domain || "未检测到网页";
  elements.sendTabButton.disabled = !/^https?:\/\//i.test(activeTab.url || "");

  renderSiteToggle(state.siteControl || {});
  renderCaptureSummary(state);

  renderTasks(Array.isArray(state.tasks) ? state.tasks : []);
  if (!previewResourceId || !findResourceById(previewResourceId, state)) {
    previewResourceId = "";
  }
  renderResources(elements.currentResourceList, state.currentResources || [], "当前页面还没有嗅探到资源");
  renderResources(elements.otherResourceList, state.otherResources || [], "暂无其他页面资源");
  renderAdvanced(state);
}

function setView(view) {
  currentView = view;
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
}

async function refresh(options = {}) {
  if (busyActionCount > 0 && !options.force) {
    return currentState;
  }

  if (refreshInFlight) {
    return refreshInFlight;
  }

  refreshInFlight = sendMessage({
    type: "popup_get_state",
    mediaIndex: selectedMediaIndex,
    currentView
  }, 9000).then((state) => {
    render(state);
    return state;
  }).catch((error) => {
    setMessage(error instanceof Error ? error.message : "刷新失败");
    throw error;
  }).finally(() => {
    refreshInFlight = null;
  });

  return refreshInFlight;
}

function startRefreshLoop() {
  if (refreshTimer !== null) {
    clearInterval(refreshTimer);
  }

  refreshTimer = setInterval(() => {
    if (currentView !== "settings" && busyActionCount === 0) {
      refresh().catch(() => {});
    }
  }, 1500);
}

document.querySelectorAll(".tab").forEach((button) => {
  button.addEventListener("click", () => {
    setView(button.dataset.view);
    refresh({ force: true }).catch(() => {});
  });
  button.addEventListener("keydown", (event) => {
    const tabs = [...document.querySelectorAll(".tab")];
    const currentIndex = tabs.indexOf(button);
    let nextIndex = currentIndex;
    if (event.key === "ArrowRight") nextIndex = (currentIndex + 1) % tabs.length;
    if (event.key === "ArrowLeft") nextIndex = (currentIndex - 1 + tabs.length) % tabs.length;
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = tabs.length - 1;
    if (nextIndex === currentIndex) return;
    event.preventDefault();
    const nextTab = tabs[nextIndex];
    setView(nextTab.dataset.view);
    nextTab.focus();
    refresh({ force: true }).catch(() => {});
  });
});

document.querySelectorAll("[data-resource-filter]").forEach((button) => {
  button.addEventListener("click", () => {
    currentResourceFilter = button.dataset.resourceFilter || "all";
    document.querySelectorAll("[data-resource-filter]").forEach((item) => {
      item.classList.toggle("active", item === button);
    });
    if (currentState) {
      renderResources(elements.currentResourceList, currentState.currentResources || [], "当前页面还没有嗅探到资源");
      renderResources(elements.otherResourceList, currentState.otherResources || [], "暂无其他页面资源");
    }
  });
});

elements.saveButton.addEventListener("click", async () => {
  beginBusy(elements.saveButton, "保存中");
  try {
    setMessage("正在保存");
    const state = await sendMessage({
      type: "popup_save_settings",
      settings: collectSettings()
    }, 12000);
    render(state);
    setMessage("已保存");
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "保存失败");
  } finally {
    endBusy(elements.saveButton);
  }
});

elements.testButton.addEventListener("click", async () => {
  beginBusy(elements.testButton, "测试中");
  try {
    setMessage("正在测试 Native Messaging");
    const status = await sendMessage({ type: "popup_test_connection" }, 9000);
    elements.statusText.textContent = status.message || "未连接";
    elements.statusDot.classList.toggle("ok", Boolean(status.ok));
    elements.statusDot.classList.toggle("bad", !status.ok);
    setMessage(status.ok ? "Native Messaging 连接正常" : "连接失败，请确认 NxTools 正在运行且浏览器接管已注册");
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "连接测试失败");
  } finally {
    endBusy(elements.testButton);
  }
});

elements.sendTabButton.addEventListener("click", async () => {
  beginBusy(elements.sendTabButton, "发送中");
  try {
    setMessage("正在发送当前页面");
    const result = await sendMessage({ type: "popup_send_active_tab" }, 14000);
    setMessage(result.message || (result.ok ? "已发送" : "发送失败"));
    if (result.ok !== false && !isQueuedDesktopResult(result)) {
      closePopupSoon();
    }
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "发送当前页面失败");
  } finally {
    endBusy(elements.sendTabButton);
  }
});

elements.siteEnabledToggle.addEventListener("change", async () => {
  const enabled = elements.siteEnabledToggle.checked;
  const host = currentState?.siteControl?.host || "";
  beginBusy(null, "");
  try {
    setMessage(enabled ? "正在为本网站启用接管" : "正在为本网站停用接管");
    const state = await sendMessage({
      type: "popup_set_site_enabled",
      host,
      enabled,
      currentView
    }, 10000);
    render(state);
    setMessage(state.operationResult?.message || "已更新本网站设置");
  } catch (error) {
    elements.siteEnabledToggle.checked = !enabled;
    setMessage(error instanceof Error ? error.message : "设置网站开关失败");
  } finally {
    endBusy(null);
  }
});

elements.clearResourcesButton.addEventListener("click", async () => {
  beginBusy(elements.clearResourcesButton, "清空中");
  try {
    const tabId = currentState?.activeTab?.id || 0;
    const state = await sendMessage({ type: "popup_clear_resources", tabId, currentView }, 9000);
    selectedResourceIds.clear();
    render(state);
    setMessage("已清空当前页面资源");
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "清空失败");
  } finally {
    endBusy(elements.clearResourcesButton);
  }
});

elements.batchResourcesButton.addEventListener("click", async () => {
  if (selectedResourceIds.size === 0 || selectedResourceIds.size > 32) {
    setMessage("请选择 1–32 个普通 HTTP/HTTPS 资源");
    return;
  }
  beginBusy(elements.batchResourcesButton, "提交中");
  try {
    const result = await sendMessage({
      type: "popup_send_resources",
      resourceIds: [...selectedResourceIds]
    }, 24000);
    for (const resourceId of result.acceptedResourceIds || []) {
      selectedResourceIds.delete(resourceId);
    }
    setMessage(result.message || (result.ok ? "资源已加入队列" : "批量提交失败"));
    await refresh({ force: true });
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "批量提交失败");
  } finally {
    endBusy(elements.batchResourcesButton);
    updateMergeButton();
  }
});

elements.mergeResourcesButton.addEventListener("click", async () => {
  if (selectedResourceIds.size !== 2) {
    setMessage("请选择 2 个资源");
    return;
  }
  beginBusy(elements.mergeResourcesButton, "发送中");
  try {
    const result = await sendMessage({
      type: "popup_merge_resources",
      resourceIds: [...selectedResourceIds]
    }, 16000);
    setMessage(result.message || (result.ok ? "资源已发送" : "合并失败"));
    if (result.ok !== false) {
      selectedResourceIds.clear();
    }
    await refresh({ force: true });
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "合并发送失败");
  } finally {
    endBusy(elements.mergeResourcesButton);
  }
});

async function toggleFeature(feature) {
  setMessage("正在切换功能");
  const state = await sendMessage({ type: "popup_toggle_feature", feature, currentView }, 12000);
  render(state);
  setMessage(state.operationResult?.message || "功能状态已更新");
}

elements.featureOverlayButton.addEventListener("click", () => toggleFeature("mediaOverlay"));
elements.featureDeepScanButton.addEventListener("click", () => toggleFeature("deepSearch"));
elements.featureMobileUaButton.addEventListener("click", () => toggleFeature("mobileUserAgent"));

elements.mediaSelect.addEventListener("change", async () => {
  const playback = currentState?.mediaPlaybackState || {};
  selectedMediaIndex = Number(elements.mediaSelect.value || 0);
  const state = await sendMessage({
    type: "popup_set_media_index",
    index: selectedMediaIndex,
    frameId: Number(playback.frameId || 0),
    currentView
  }, 10000);
  render(state);
});

async function mediaAction(payload) {
  const playback = currentState?.mediaPlaybackState || {};
  const localIndex = Number.isInteger(playback.localMediaIndex)
    ? playback.localMediaIndex
    : Number(playback.mediaIndex || 0);
  const result = await sendMessage({
    type: "popup_media_action",
    index: localIndex,
    frameId: playback.frameId || 0,
    payload
  }, 12000);
  setMessage(result.message || (result.ok ? "媒体操作已发送" : "媒体操作失败"));
  await refresh({ force: true });
}

elements.mediaPlayButton.addEventListener("click", () => {
  const playback = currentState?.mediaPlaybackState || {};
  mediaAction({ Message: playback.paused ? "play" : "pause" });
});
elements.mediaPipButton.addEventListener("click", () => mediaAction({ Message: "pip" }));
elements.mediaScreenshotButton.addEventListener("click", () => mediaAction({ Message: "screenshot" }));
elements.mediaMuteButton.addEventListener("click", () => {
  const playback = currentState?.mediaPlaybackState || {};
  mediaAction({ Message: "muted", action: !playback.muted });
});
elements.mediaLoopButton.addEventListener("click", () => {
  const playback = currentState?.mediaPlaybackState || {};
  mediaAction({ Message: "loop", action: !playback.loop });
});
elements.mediaSpeedButton.addEventListener("click", () => {
  const speed = Math.max(0.25, Math.min(4, Number(elements.mediaSpeedInput.value || 1)));
  elements.mediaSpeedInput.value = String(speed);
  mediaAction({ Message: "speed", speed });
});
elements.mediaSpeedInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    elements.mediaSpeedButton.click();
  }
});
elements.mediaVolumeRange.addEventListener("change", () => {
  mediaAction({ Message: "setVolume", volume: Number(elements.mediaVolumeRange.value || 0) / 100 });
});
elements.mediaTimeRange.addEventListener("change", () => {
  const playback = currentState?.mediaPlaybackState || {};
  mediaAction({ Message: "setTime", time: (Number(elements.mediaTimeRange.value || 0) / 1000) * Number(playback.duration || 0) });
});

for (const input of [
  elements.interceptDownloads,
  elements.openDialog,
  elements.captureResources,
  elements.mediaOverlayEnabled,
  elements.dropTargetEnabled,
  elements.dropTargetPosition,
  elements.dropTargetWidth,
  elements.dropTargetHeight,
  elements.eraseBrowserHistory,
  elements.includeRequestHeaders,
  elements.includeCookies
]) {
  input.addEventListener("change", async () => {
    const state = await sendMessage({
      type: "popup_save_settings",
      settings: collectSettings(),
      currentView
    }, 12000);
    render(state);
  });
}

setView(currentView);
refresh().catch((error) => {
  setMessage(error instanceof Error ? error.message : "扩展初始化失败");
});
startRefreshLoop();
