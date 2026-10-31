"use strict";

// Popup actions: background messaging, busy guards, visible feedback and all
// user-event wiring. Message types and payloads are unchanged from the original.

const SUCCESS_MESSAGE_MS = 4000;
let messageHideTimer = 0;

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

function messageTone(text) {
  if (/失败|超时|错误|拒绝|无法|未接受|不可用|重试/.test(text || "")) return "error";
  if (/^正在|中…?$|…$/.test(text || "")) return "status";
  return "success";
}

// The status bar only occupies space while there is feedback. Errors stay until
// dismissed or replaced; progress stays until replaced; success fades after 4s.
function setMessage(text) {
  clearTimeout(messageHideTimer);
  const value = String(text || "");
  const tone = messageTone(value);
  elements.messageText.textContent = value;
  elements.messageText.dataset.tone = tone;
  if (statusFooter) {
    statusFooter.hidden = !value;
    statusFooter.dataset.tone = tone;
  }
  if (value && tone === "success") {
    messageHideTimer = setTimeout(() => {
      if (statusFooter && !statusFooter.contains(document.activeElement)) statusFooter.hidden = true;
    }, SUCCESS_MESSAGE_MS);
  }
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
  button.setAttribute("aria-busy", "true");
}

function endBusy(button) {
  busyActionCount = Math.max(0, busyActionCount - 1);
  if (!button) {
    return;
  }

  button.disabled = false;
  button.removeAttribute("aria-busy");
  if (button.dataset.originalText) {
    button.textContent = button.dataset.originalText;
    delete button.dataset.originalText;
  }
}

async function runTaskAction(button) {
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

async function toggleFeature(feature, button) {
  if (button.disabled) return;
  beginBusy(null, "");
  button.disabled = true;
  button.setAttribute("aria-busy", "true");
  try {
    setMessage("正在切换功能");
    const state = await sendMessage({ type: "popup_toggle_feature", feature, currentView }, 12000);
    render(state);
    setMessage(state.operationResult?.message || "功能状态已更新");
  } catch (error) {
    setMessage(error instanceof Error ? error.message : "切换功能失败");
  } finally {
    button.disabled = false;
    button.removeAttribute("aria-busy");
    endBusy(null);
  }
}

async function mediaAction(payload) {
  if (mediaActionInFlight) return;
  mediaActionInFlight = true;
  beginBusy(null, "");
  const playback = currentState?.mediaPlaybackState || {};
  const localIndex = Number.isInteger(playback.localMediaIndex) ? playback.localMediaIndex : Number(playback.mediaIndex || 0);
  const controls = [elements.mediaPlayButton, elements.mediaPipButton, elements.mediaScreenshotButton,
    elements.mediaMuteButton, elements.mediaLoopButton, elements.mediaSpeedButton];
  controls.forEach(control => { control.disabled = true; });
  try {
    const result = await sendMessage({ type: "popup_media_action", index: localIndex,
      frameId: playback.frameId || 0, payload }, 12000);
    setMessage(result.message || (result.ok ? "媒体操作已发送" : "媒体操作失败"));
    await refresh({ force: true });
  } catch (error) { setMessage(error instanceof Error ? error.message : "媒体操作失败"); }
  finally {
    mediaActionInFlight = false;
    endBusy(null);
    controls.forEach(control => { control.disabled = !currentState?.mediaPlaybackState?.available; });
  }
}

async function saveSettings(button, progressText, doneText, failureText) {
  beginBusy(button, "保存中");
  setMessage(progressText);
  try {
    const state = await sendMessage({ type: "popup_save_settings", settings: collectSettings(), currentView }, 12000);
    if (state?.ok === false) {
      throw new Error(state.message || failureText);
    }
    render(state);
    setMessage(doneText);
  } catch (error) {
    setMessage(error instanceof Error ? error.message : failureText);
  } finally {
    endBusy(button);
  }
}

function bindPopupEvents() {
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && previewResourceId && currentView === "resources") {
      event.preventDefault();
      toggleResourcePreview(previewResourceId);
    }
  });

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
        item.setAttribute("aria-pressed", String(item === button));
      });
      if (currentState) {
        renderResourceLists();
      }
    });
  });

  elements.dismissMessageButton?.addEventListener("click", () => setMessage(""));

  elements.saveButton.addEventListener("click", () => saveSettings(elements.saveButton, "正在保存", "已保存", "保存失败"));

  elements.testButton.addEventListener("click", async () => {
    beginBusy(elements.testButton, "测试中");
    try {
      setMessage("正在测试 Native Messaging");
      const status = await sendMessage({ type: "popup_test_connection" }, 9000);
      renderConnection({ ok: Boolean(status.ok), message: status.message || "未连接" });
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
      if (state?.ok === false) {
        throw new Error(state.message || "设置网站开关失败");
      }
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

  elements.clearSelectionButton?.addEventListener("click", () => {
    selectedResourceIds.clear();
    document.querySelectorAll("[data-resource-check]").forEach((checkbox) => { checkbox.checked = false; });
    updateBatchBar();
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
      updateBatchBar();
    }
  });

  elements.featureOverlayButton.addEventListener("click", () => toggleFeature("mediaOverlay", elements.featureOverlayButton));
  elements.featureDeepScanButton.addEventListener("click", () => toggleFeature("deepSearch", elements.featureDeepScanButton));
  elements.featureMobileUaButton.addEventListener("click", () => toggleFeature("mobileUserAgent", elements.featureMobileUaButton));

  elements.mediaSelect.addEventListener("change", async () => {
    const playback = currentState?.mediaPlaybackState || {};
    selectedMediaIndex = Number(elements.mediaSelect.value || 0);
    beginBusy(null, "");
    try {
      const state = await sendMessage({ type: "popup_set_media_index", index: selectedMediaIndex,
        frameId: Number(playback.frameId || 0), currentView }, 10000);
      render(state);
    } catch (error) { setMessage(error instanceof Error ? error.message : "选择媒体失败"); }
    finally { endBusy(null); }
  });

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

  // Original auto-save on change; the explicit Save button remains the retry path.
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
    input.addEventListener("change", async () => saveSettings(null, "正在自动保存设置…", "设置已自动保存", "自动保存失败，请重试"));
  }
}
