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
    resource.size ? formatBytes(resource.size) : ""
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
  return el("div", { className: "resource-preview-panel" },
    media,
    el("dl", { className: "resource-facts" },
      details.title ? el("dt", { text: "类型" }) : null, details.title ? el("dd", { text: details.title }) : null,
      el("dt", { text: "来源" }), el("dd", { text: details.detail || "未知" }),
      el("dt", { text: "地址" }), el("dd", { className: "resource-url", text: compactUrl(url), title: url })));
}

function renderResources(target, resources, emptyText, emptyHint) {
  const visibleResources = filterResourceList(resources);
  if (!visibleResources.length) {
    target.replaceChildren(emptyState(emptyText, emptyHint));
    updateBatchBar();
    return;
  }

  const rows = visibleResources.slice(0, 24).map((resource) => {
    const name = resource.filename || resource.url || "resource";
    const kind = resourceKind(resource);
    const isOpen = previewResourceId === resource.id;
    const details = resourceDetails(resource);
    const check = el("input", {
      className: "resource-check",
      type: "checkbox",
      dataset: { resourceCheck: resource.id },
      attrs: { "aria-label": `选择 ${name}` }
    });
    check.checked = selectedResourceIds.has(resource.id);
    const actions = el("div", { className: "resource-actions" });
    if (isPreviewableResource(resource)) {
      actions.append(el("button", {
        className: "preview-action ghost",
        text: isOpen ? "收起" : "预览",
        dataset: { resourcePreview: resource.id },
        attrs: { "aria-expanded": String(isOpen), "aria-label": `${isOpen ? "收起预览" : "预览"} ${name}` }
      }));
    }
    actions.append(el("button", {
      className: "mini-send",
      text: "下载",
      dataset: { resourceId: resource.id },
      attrs: { "aria-label": `发送 ${name} 到 NxTools` }
    }));
    return el("article", {
      className: `item resource${isOpen ? " selected" : ""}`,
      dataset: { resourceRow: resource.id, kind }
    },
      check,
      el("span", { className: "resource-thumb", dataset: { kind }, title: resourceKindLabel(kind) }, kindIcon(kind)),
      el("div", { className: "resource-main" },
        el("div", { className: "resource-title" }, el("strong", { text: name, title: [name, details.title, details.origin].filter(Boolean).join("\n") })),
        el("div", { className: "resource-meta", text: resourceMetaText(resource) })),
      actions,
      isOpen ? expandedPreview(resource) : null);
  });
  target.replaceChildren(...rows);

  target.querySelectorAll("[data-resource-row]").forEach((row) => {
    row.addEventListener("click", (event) => {
      if (event.target?.closest?.("button,input,a,audio,video")) {
        return;
      }
      toggleResourcePreview(row.dataset.resourceRow || "");
    });
  });
  target.querySelectorAll("[data-resource-preview]").forEach((button) => {
    button.addEventListener("click", () => toggleResourcePreview(button.dataset.resourcePreview));
  });
  target.querySelectorAll("[data-resource-id]").forEach((button) => {
    button.addEventListener("click", () => sendResourceFromButton(button, button.dataset.resourceId));
  });
  target.querySelectorAll("[data-resource-check]").forEach((checkbox) => {
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) {
        selectedResourceIds.add(checkbox.dataset.resourceCheck);
      } else {
        selectedResourceIds.delete(checkbox.dataset.resourceCheck);
      }
      updateBatchBar();
    });
  });

  updateBatchBar();
}

function renderResourceLists(state = currentState) {
  renderResources(elements.currentResourceList, state?.currentResources || [], "当前页面还没有捕获到资源",
    "播放页面媒体或开始浏览，资源会出现在这里。勾选可批量加入队列，预览不会下载。");
  renderResources(elements.otherResourceList, state?.otherResources || [], "暂无其他页面资源");
}

// The batch bar appears only while resources are selected (1..32 per request).
function updateBatchBar() {
  const selectedCount = selectedResourceIds.size;
  if (batchToolbar) batchToolbar.hidden = selectedCount === 0;
  elements.currentResourceCountText.textContent = selectedCount > 32 ? `已选 ${selectedCount} 项 · 最多 32` : `已选 ${selectedCount} 项`;
  elements.batchResourcesButton.disabled = selectedCount === 0 || selectedCount > 32;
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

function renderConnection(status) {
  elements.statusText.textContent = status.ok ? "桌面在线" : (status.message || "桌面离线");
  elements.statusText.title = elements.statusText.textContent;
  elements.statusDot.classList.toggle("ok", Boolean(status.ok));
  elements.statusDot.classList.toggle("bad", !status.ok);
}

function render(state) {
  const focused = document.activeElement;
  const focusAttribute = ["data-resource-preview", "data-resource-check", "data-resource-id", "data-task-id"].find(name => focused?.hasAttribute?.(name));
  const focusValue = focusAttribute ? focused.getAttribute(focusAttribute) : "";
  const focusAction = focused?.dataset?.action || "";
  const mediaDraft = focused === elements.mediaSpeedInput ? focused.value : null;
  currentState = state;
  const liveResourceIds = new Set(allResourcesFromState(state).map((resource) => resource.id));
  selectedResourceIds = new Set(
    [...selectedResourceIds].filter((resourceId) => liveResourceIds.has(resourceId))
  );
  renderSettings(state.settings || {});
  renderConnection(state.status || {});

  const activeTab = state.activeTab || {};
  elements.activeTabTitle.textContent = activeTab.title || activeTab.url || "-";
  elements.activeTabTitle.title = activeTab.title || activeTab.url || "";
  elements.activeTabDomain.textContent = activeTab.domain || "未检测到网页";
  elements.sendTabButton.disabled = !/^https?:\/\//i.test(activeTab.url || "");

  renderSiteToggle(state.siteControl || {});
  renderCaptureSummary(state);
  renderTasks(Array.isArray(state.tasks) ? state.tasks : []);
  if (!previewResourceId || !findResourceById(previewResourceId, state)) {
    previewResourceId = "";
  }
  renderResourceLists(state);
  renderAdvanced(state);
  if (mediaDraft !== null) elements.mediaSpeedInput.value = mediaDraft;
  if (focusAttribute) {
    const actionSelector = focusAction ? `[data-action="${CSS.escape(focusAction)}"]` : "";
    document.querySelector(`[${focusAttribute}="${CSS.escape(focusValue)}"]${actionSelector}`)?.focus({ preventScroll: true });
  }
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

function toggleResourcePreview(resourceId) {
  previewResourceId = previewResourceId === resourceId ? "" : resourceId;
  renderResourceLists();
  document.querySelector(`[data-resource-preview="${CSS.escape(resourceId)}"]`)?.focus({ preventScroll: true });
  if (previewResourceId) document.querySelector(`[data-resource-row="${CSS.escape(resourceId)}"] .resource-preview-panel`)?.scrollIntoView({ block: "nearest", behavior: "instant" });
}
