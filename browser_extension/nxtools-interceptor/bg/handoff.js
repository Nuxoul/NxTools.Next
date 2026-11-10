"use strict";

// Identity-bound desktop handoff for URLs, resources, batches, Blobs and browser downloads.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

async function sendBlobResourceViaContentScript(resource) {
  const tabId = Number(resource?.tabId || lastActiveTabId || 0);
  if (!tabId || !String(resource?.url || "").startsWith("blob:")) {
    return { ok: false, fallbackToBrowser: true, message: "Blob 资源只能在原网页中读取" };
  }
  const frameId = Number.isInteger(resource.frameId) ? resource.frameId : undefined;
  const target = Number.isInteger(frameId) ? { frameId } : {};
  const infoMessage = {
    type: "content_blob_info",
    url: resource.url,
    filename: buildResourceFileName(resource)
  };
  let infoResult = await sendMessageToTab(tabId, infoMessage, target);
  if (!(infoResult.ok && infoResult.response?.ok) && Number.isInteger(frameId)) {
    const frameResults = await sendMessageToTabFrames(tabId, infoMessage);
    infoResult = frameResults.find((item) => item.ok && item.response?.ok) || infoResult;
  }
  if (!(infoResult.ok && infoResult.response?.ok)) {
    return {
      ok: false,
      fallbackToBrowser: true,
      message: infoResult.response?.message || infoResult.message || "页面脚本无法读取这个 Blob"
    };
  }

  const size = Number(infoResult.response.size || resource.size || 0);
  const fileName = cleanFileName(infoResult.response.filename)
    || buildResourceFileName(resource)
    || `blob-${Date.now()}.bin`;
  const mediaType = String(infoResult.response.mime || resource.mime || "application/octet-stream");
  const metadata = NxToolsBlobTransferPolicy.boundedMetadata(size, fileName, mediaType);
  if (!metadata) {
    return { ok: false, fallbackToBrowser: true, message: "Blob 元数据超出 NxTools 有界传输限制" };
  }
  const correlationId = nextRequestId();
  const handoffId = nextRequestId();
  const releaseBlobPort = retainDesktopPort();
  try {
  let prepared;
  try {
    prepared = await sendDesktopRequest("prepare_blob_upload", {
      payload: { ...metadata, handoffId }
    }, 15000, correlationId);
  } catch (error) {
    return {
      ok: false,
      fallbackToBrowser: true,
      message: error instanceof Error ? error.message : "NxTools 未准备 Blob 上传通道"
    };
  }
  const capability = NxToolsBlobTransferPolicy.uploadCapability(prepared, metadata.size);
  if (!capability) {
    return { ok: false, fallbackToBrowser: true, message: "NxTools 返回了无效的 Blob 上传能力" };
  }
  const { uploadId, uploadUrl, chunkBytes } = capability;

  const uploadMessage = {
    type: "content_upload_blob",
    url: resource.url,
    uploadUrl,
    expectedSize: metadata.size,
    chunkBytes
  };
  let uploadResult = await sendMessageToTab(tabId, uploadMessage, target);
  if (!(uploadResult.ok && uploadResult.response?.ok) && Number.isInteger(frameId)) {
    const frameResults = await sendMessageToTabFrames(tabId, uploadMessage);
    uploadResult = frameResults.find((item) => item.ok && item.response?.ok) || uploadResult;
  }
  if (!(uploadResult.ok && uploadResult.response?.ok)) {
    return {
      ok: false,
      fallbackToBrowser: true,
      message: uploadResult.response?.message || uploadResult.message || "Blob 上传失败"
    };
  }

  const settings = await getSettings();
  let commitError = null;
  for (let attempt = 0; attempt < 2; attempt += 1) {
    try {
      const committed = await sendDesktopRequest("commit_blob_upload", {
        payload: {
          uploadId,
          handoffId,
          fileName: metadata.fileName,
          mediaType: metadata.mediaType,
          size: metadata.size,
          openDialog: settings.openDialog
        }
      }, 20000, correlationId);
      const outcome = NxToolsHandoffPolicy.handoffOutcome(committed, handoffId);
      if (outcome.state === "awaiting_confirmation") {
        return { ...committed, message: "确认窗口已打开，尚未创建任务" };
      }
      return NxToolsBlobTransferPolicy.browserFallback(
        outcome.state === "persisted" ? committed : { ...committed, ok: false },
        handoffId,
        "NxTools 拒绝接管 Blob 下载"
      );
    } catch (error) {
      commitError = error;
      if (error?.nativeRejected === true) {
        break;
      }
    }
  }
  return {
    ok: false,
    fallbackToBrowser: true,
    message: commitError instanceof Error ? commitError.message : "NxTools 未接受 Blob 下载"
  };
  } finally {
    releaseBlobPort();
  }
}

async function addDownloadToNxTools(payload, correlationId) {
  const settings = await getSettings();
  return sendDesktopRequest("create_task", {
    payload: {
      source: "browser_extension",
      openDialog: settings.openDialog,
      startImmediately: !settings.openDialog,
      ...payload,
      handoffId: payload.handoffId || nextRequestId()
    }
  }, 15000, correlationId);
}

async function sendDownloadPayload({ handoffId = "", url, fileName, headers = {}, size = 0, supportsRange = false, contextUrl = "", cookieSource = {} }) {
  const startedAt = Date.now();
  const effectiveHandoffId = handoffId || nextRequestId();
  const invalidGoogleVideoReason = googleVideoInvalidReason(url);
  if (invalidGoogleVideoReason) {
    sendExtensionLog("warn", "handoff_rejected", "已拒绝过期或异常的 YouTube 直链", {
      url,
      reason: invalidGoogleVideoReason
    }, 30000, effectiveHandoffId);
    return {
      ok: false,
      message: "捕获到的 YouTube 直链已过期，请让视频播放几秒后重新选择资源"
    };
  }

  const settings = await getSettings();
  // A Content-Disposition filename observed for this URL (or its redirect source)
  // is authoritative and overrides any URL-path-derived guess.
  await ensureDispositionNamesRestored();
  const dispositionName = dispositionFileNameForUrl(url);
  const resolvedFileName = ensureFileNameForUrl(dispositionName || fileName, url);
  const googleVideoIdentityInfo = googleVideoIdentity(url);
  const resolvedSize = Number(size || 0) > 0
    ? Number(size || 0)
    : Number.parseInt(googleVideoIdentityInfo?.clen || "", 10) || 0;
  const enrichedHeaders = await enrichHeadersForDownload(url, headers, settings, contextUrl, cookieSource);
  const headerPayload = headerObjectToPayload(enrichedHeaders, settings);
  const headerNames = Object.keys(headerPayload.headers);
  sendExtensionLog("info", "handoff_prepare", "准备发送资源到 NxTools", {
    url,
    fileName: resolvedFileName,
    size: resolvedSize,
    supportsRange: Boolean(supportsRange),
    headerCount: headerNames.length + (headerPayload.cookie ? 1 : 0),
    headerNames: headerNames.slice(0, 16).join(","),
    cookieIncluded: Boolean(headerPayload.cookie)
  }, 15000, effectiveHandoffId);

  try {
    const result = await addDownloadToNxTools({
      handoffId: effectiveHandoffId,
      url,
      fileName: resolvedFileName,
      ...headerPayload,
      size: resolvedSize,
      supportsRange
    }, effectiveHandoffId);
    const outcome = NxToolsHandoffPolicy.handoffOutcome(result, effectiveHandoffId);
    const queued = Boolean(result?.data?.queued);
    sendExtensionLog(result?.ok === false || queued ? "warn" : "info", "handoff_result", result?.ok === false
      ? "资源发送到 NxTools 失败"
      : queued
        ? "任务已加入 NxTools 等待列表"
        : "资源已发送到 NxTools", {
      url,
      reason: result?.message || "",
      queued,
      size: resolvedSize
    }, result?.ok === false ? 0 : 15000, effectiveHandoffId);
    if (outcome.state === "awaiting_confirmation") {
      recordBrowserHandoff({ correlationId: effectiveHandoffId, startedAt }, outcome, false);
      return { ...result, message: "确认窗口已打开，尚未创建任务" };
    }
    if (outcome.state !== "persisted") {
      return {
        ...result,
        ok: false,
        accepted: false,
        data: { ...result?.data, handoffState: outcome.state, reason: outcome.reason },
        message: result?.message || result?.error || "NxTools 未确认接收下载任务"
      };
    }
    return result;
  } catch (error) {
    sendExtensionLog("error", "handoff_failed", "资源发送到 NxTools 异常", {
      url,
      reason: error instanceof Error ? error.message : String(error)
    }, 0, effectiveHandoffId);
    throw error;
  }
}

async function sendDownloadItem(downloadItem, existingHandoffId = "") {
  const url = downloadItem.finalUrl || downloadItem.url;
  if (!isHttpUrl(url)) {
    return { ok: false, skipped: true, message: "不是 HTTP 下载链接" };
  }

  let headerSnapshot = resolveHeaderSnapshot(url, downloadItem) || resolveHeaderSnapshot(downloadItem.url, downloadItem);
  const downloadTabId = await resolveTabIdForDownload(downloadItem, headerSnapshot);
  if (downloadTabId > 0) {
    const source = { ...downloadItem, tabId: downloadTabId };
    headerSnapshot = resolveHeaderSnapshot(url, source) || resolveHeaderSnapshot(downloadItem.url, source);
  }
  const resource = downloadTabId > 0
    ? resourcesById.get(`${downloadTabId}:${normalizeResourceUrlKey(url)}`) || null
    : null;
  const headers = { ...(headerSnapshot?.headers || {}) };
  if (downloadItem.referrer && !headers.referer) {
    headers.referer = downloadItem.referrer;
  }

  const handoffId = existingHandoffId || await browserDownloadHandoffId(downloadItem);
  const response = await sendDownloadPayload({
    handoffId,
    url,
    fileName: getFileNameFromPath(downloadItem.filename) || resource?.filename || getFileNameFromUrl(url),
    headers,
    size: downloadItem.totalBytes > 0 ? downloadItem.totalBytes : resource?.size || 0,
    supportsRange: Boolean(downloadItem.canResume || headerSnapshot?.supportsRange || resource?.supportsRange),
    contextUrl: downloadItem.referrer || resource?.pageUrl || "",
    cookieSource: { ...cookieSourceDetails(headerSnapshot), ...cookieSourceDetails(downloadItem),
      tabId: downloadTabId > 0 ? downloadTabId : undefined }
  });
  return { response, handoffId };
}

// A browser download carries no tab. Resolve it only from the download's own
// evidence: the tab that issued the observed request, or a tab showing the
// referrer page. Never fall back to whichever tab happens to be active.
async function resolveTabIdForDownload(downloadItem, headerSnapshot) {
  if (Number.isInteger(downloadItem?.tabId) && downloadItem.tabId > 0) {
    return downloadItem.tabId;
  }
  if (headerSnapshot?.tabIdObserved === true && headerSnapshot.tabId > 0) {
    return headerSnapshot.tabId;
  }
  const referrer = normalizeUrlKey(downloadItem?.referrer || "");
  if (!referrer) {
    return 0;
  }
  try {
    const tabs = await chrome.tabs.query({});
    const matches = tabs.filter(tab => tab.id && tab.url && urlsLikelySamePage(tab.url, referrer) &&
      (!downloadItem.cookieStoreId || tab.cookieStoreId === downloadItem.cookieStoreId) &&
      (typeof downloadItem.incognito !== "boolean" || tab.incognito === downloadItem.incognito));
    return matches.length === 1 ? matches[0].id : 0;
  } catch {
    return 0;
  }
}

async function sendUrl(url, options = {}) {
  if (!isHttpUrl(url)) {
    throw new Error("只能发送 HTTP/HTTPS 链接");
  }

  const headerSnapshot = resolveHeaderSnapshot(url, options.cookieSource || {});
  const headers = { ...(headerSnapshot?.headers || {}) };
  if (options.referrer && !headers.referer) {
    headers.referer = options.referrer;
  }

  return sendDownloadPayload({
    handoffId: options.handoffId || "",
    url,
    fileName: options.fileName || getFileNameFromUrl(url),
    headers,
    size: options.size || 0,
    supportsRange: Boolean(options.supportsRange || headerSnapshot?.supportsRange),
    contextUrl: options.referrer || "",
    cookieSource: options.cookieSource || { ...cookieSourceDetails(headerSnapshot), tabId: headerSnapshot?.tabIdObserved ? headerSnapshot.tabId : undefined }
  });
}

async function sendResource(resourceId, overrides = {}) {
  const resource = resourcesById.get(resourceId);
  if (!resource) {
    sendExtensionLog("warn", "send_resource_missing", "Popup 请求发送资源但缓存中不存在", {
      reason: "resource_not_found",
      resourceUrl: resourceId
    }, 0);
    return { ok: false, message: "资源不存在" };
  }

  if (String(resource.url || "").startsWith("blob:")) {
    const sourceResource = findBlobSourceResource(resource);
    if (sourceResource) {
      sendExtensionLog("info", "blob_source_resolved", "Blob 资源已反查到真实网络地址", {
        blobUrl: resource.url,
        url: sourceResource.url,
        tabId: resource.tabId || 0,
        source: sourceResource.source || "",
        size: sourceResource.size || 0,
        mime: sourceResource.mime || ""
      }, 0);
      return await sendResource(sourceResource.id);
    }

    return await sendBlobResourceViaContentScript(resource);
  }

  const outboundResource = {
    ...resource,
    filename: overrides.filename || resource.filename,
    mime: overrides.mime || resource.mime,
    size: Number(overrides.size || 0) > 0 ? Number(overrides.size || 0) : Number(resource.size || 0),
    width: Number(overrides.width || 0) > 0 ? Number(overrides.width || 0) : Number(resource.width || 0),
    height: Number(overrides.height || 0) > 0 ? Number(overrides.height || 0) : Number(resource.height || 0)
  };

  resource.pendingBatchHandoffId ||= nextRequestId();
  const result = await NxToolsBatchHandoffPolicy.runSingleResourceHandoff({
    resource,
    persistHandoff: persistResourcesNow,
    sendHandoff: (handoffId) => sendDownloadPayload({
      handoffId,
      url: outboundResource.url,
      fileName: buildResourceFileName(outboundResource),
      headers: outboundResource.requestHeaders,
      size: outboundResource.size,
      supportsRange: outboundResource.supportsRange,
      contextUrl: outboundResource.pageUrl || outboundResource.referer || "",
      cookieSource: { cookieContext: outboundResource.cookieContext ?? null }
    }),
    resolveCurrentResource: (resourceId) => resourcesById.get(resourceId)
  });
  schedulePersistResources();
  return result;
}

async function sendResourcesBatch(resourceIds) {
  const resources = [...new Set(Array.isArray(resourceIds) ? resourceIds.map(String) : [])]
    .map((resourceId) => resourcesById.get(resourceId))
    .filter(Boolean);
  const selection = NxToolsBatchHandoffPolicy.selectBatchResources(resources);
  if (selection.error) {
    return { ok: false, message: selection.error };
  }
  if (selection.items.length === 0) {
    return {
      ok: false,
      message: selection.unsupportedIds.length > 0
        ? "Blob 资源请使用单项下载"
        : "没有可批量提交的 HTTP/HTTPS 资源"
    };
  }

  const settings = await getSettings();
  const handoffToResource = new Map();
  const items = [];
  for (const resource of selection.items) {
    resource.pendingBatchHandoffId ||= nextRequestId();
    const handoffId = resource.pendingBatchHandoffId;
    handoffToResource.set(handoffId, resource);
    await ensureDispositionNamesRestored();
    const dispositionName = dispositionFileNameForUrl(resource.url);
    const fileName = ensureFileNameForUrl(
      dispositionName || buildResourceFileName(resource),
      resource.url,
      resource.mime
    );
    const headers = await enrichHeadersForDownload(
      resource.url,
      resource.requestHeaders,
      settings,
      resource.pageUrl || resource.referer || "",
      { cookieContext: resource.cookieContext ?? null }
    );
    const headerPayload = headerObjectToPayload(headers, settings);
    items.push({
      handoffId,
      url: resource.url,
      fileName,
      ...headerPayload,
      ...(settings.includeRequestHeaders ? { referrer: resource.referer || resource.pageUrl || "" } : {}),
      size: Number(resource.size || 0),
      startImmediately: false,
      openDialog: false
    });
  }
  await persistResourcesNow();

  const correlationId = nextRequestId();
  const { validation } = await NxToolsBatchHandoffPolicy.runBatchHandoffRequest({
    correlationId,
    handoffIds: items.map((item) => item.handoffId),
    sendRequest: () => sendDesktopRequest("create_tasks", {
      payload: { items }
    }, 20000, correlationId),
    emitLifecycle: (phase, details, lifecycleCorrelationId) => {
      const failed = details.failedCount > 0 || details.transportFailed === true;
      sendExtensionLog(
        failed ? "warn" : "info",
        `batch_handoff_${phase}`,
        phase === "started" ? "开始批量发送资源" : "批量发送资源完成",
        details,
        0,
        lifecycleCorrelationId
      );
    }
  });
  if (!validation.valid) {
    return { ok: false, message: "NxTools 返回了无效的批量关联响应" };
  }

  const acceptedResourceIds = [];
  const failedResourceIds = [];
  for (const handoffId of validation.acceptedHandoffIds) {
    const resource = handoffToResource.get(handoffId);
    if (resource) {
      acceptedResourceIds.push(resource.id);
      NxToolsBatchHandoffPolicy.applyAcceptedHandoff({
        resourceId: resource.id,
        handoffId,
        observedResource: resource,
        resolveCurrentResource: (resourceId) => resourcesById.get(resourceId)
      });
    }
  }
  for (const handoffId of validation.failedHandoffIds) {
    const resource = handoffToResource.get(handoffId);
    if (resource) {
      failedResourceIds.push(resource.id);
    }
  }
  schedulePersistResources();
  return {
    ok: true,
    accepted: failedResourceIds.length === 0,
    message: [
      `已加入 ${acceptedResourceIds.length} 个`,
      failedResourceIds.length > 0 ? `${failedResourceIds.length} 个失败` : "",
      selection.unsupportedIds.length > 0 ? `${selection.unsupportedIds.length} 个 Blob 请单独下载` : ""
    ].filter(Boolean).join("，"),
    acceptedResourceIds,
    failedResourceIds,
    unsupportedResourceIds: selection.unsupportedIds,
    data: { queued: true }
  };
}

async function mergeResources(resourceIds) {
  return {
    ok: false,
    message: "音视频合并将在 HLS/DASH 阶段接入；当前版本不会创建两个无法自动合并的任务"
  };
}

function emitBrowserHandoffState(status) {
  sendExtensionLog(["failed", "rejected"].includes(status.state) ? "warn" : "info",
    "handoff_state", "browser download handoff state", status, 0, status.correlationId);
}

function recordBrowserHandoff(context, outcome, emit = true) {
  lastHandoff = {
    correlationId: context.correlationId,
    state: outcome.state,
    reason: outcome.reason,
    owner: outcome.state === "transferred" ? "desktop" : "browser",
    elapsedMs: Math.max(0, Date.now() - context.startedAt),
    queued: outcome.queued === true,
    ...(outcome.boundaryCode ? { boundaryCode: outcome.boundaryCode } : {}),
    ...(outcome.compensation ? { compensation: outcome.compensation } : {})
  };
  if (emit) emitBrowserHandoffState(lastHandoff);
}

async function interceptBrowserDownload(downloadItem, options = {}) {
  // The onDeterminingFilename listener returns true and resolves the filename
  // exactly once here. While the desktop handoff is in flight we deliberately
  // hold the suggestion back: resolving it early lets Chrome proceed to its own
  // Save As dialog before the browser download is canceled. Every terminal path,
  // including unexpected failures, resolves with the default suggestion.
  let suggested = false;
  const yieldToBrowser = () => {
    if (suggested) return;
    suggested = true;
    try {
      options.suggest?.();
    } catch {
      // Chrome may already have resolved this one-shot suggestion callback.
    }
  };
  const context = { correlationId: "", startedAt: Date.now() };
  try {
    const settings = await getSettings();
    const url = downloadItem.finalUrl || downloadItem.url;
    if (!isHttpUrl(url)) return;
    context.correlationId = await browserDownloadHandoffId(downloadItem);
    const excluded = await automaticCaptureDecision(downloadItem, settings);
    if (excluded) {
      recordBrowserHandoff(context, excluded);
      return;
    }

    recordBrowserHandoff(context, { state: "submitting", reason: "handoff_started" }, false);
    const { response, handoffId } = await sendDownloadItem(downloadItem, context.correlationId);
    const outcome = NxToolsHandoffPolicy.handoffOutcome(response, handoffId);
    recordBrowserHandoff(context, outcome);
    if (outcome.state !== "persisted") return;
    const handoff = await NxToolsHandoffPolicy.completeAcceptedHandoff({
      response,
      expectedHandoffId: handoffId,
      cancelBrowserDownload: () => chrome.downloads.cancel(downloadItem.id),
      rollbackDesktopTask: (taskId) => sendDesktopRequest("task_action", {
        taskId,
        action: "cancel"
      }, 12000, context.correlationId),
      eraseBrowserDownload: () => {
        if (settings.eraseBrowserHistory || options.eraseBrowserHistory) {
          return chrome.downloads.erase({ id: downloadItem.id });
        }
        return Promise.resolve();
      }
    });
    if (!handoff.transferred) return;
    recordBrowserHandoff(context, { state: "transferred", reason: "browser_transferred", queued: outcome.queued });
    setBadge("OK", "#138a72");
    setTimeout(() => setBadge("", "#138a72"), 1600);
  } catch (error) {
    const reason = ["desktop_unavailable", "desktop_refused", "handoff_timeout", "handoff_disconnected", "browser_cancel_failed", "identity_mismatch"]
      .includes(error?.handoffReason) ? error.handoffReason : "desktop_refused";
    recordBrowserHandoff(context, {
      state: ["desktop_refused", "identity_mismatch"].includes(reason) ? "rejected" : "failed", reason, boundaryCode: error?.boundaryCode,
      compensation: error?.compensation
    });
    setBadge("!", "#9a3412");
  } finally {
    yieldToBrowser();
  }
}
