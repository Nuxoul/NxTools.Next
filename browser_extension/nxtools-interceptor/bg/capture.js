"use strict";

// Network/request/page resource capture into the cache.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

async function captureNetworkResource(details) {
  let owner = details.tabId > 0 ? bucketForTab(details.tabId) : null;
  const settings = await getSettings();
  if (!settings.captureResources || !isHttpUrl(details.url) || !isUsableGoogleVideoUrl(details.url)) {
    return;
  }
  if (!isSiteEnabled(details.initiator, details.originUrl, details.documentUrl, details.url)) {
    return;
  }

  const redirectHint = resolveRedirectHint(details);
  const meta = responseMeta(details);
  meta.assetHint = meta.assetHint || redirectHint?.assetHint || null;
  meta.mime = mimeFromUrl(details.url) || meta.assetHint?.mime || meta.mime || redirectHint?.mime || "";
  meta.filename = meta.filename || redirectHint?.filename || "";
  meta.size = Number(meta.size || 0) > 0 ? meta.size : Number(redirectHint?.size || 0);
  meta.supportsRange = Boolean(meta.supportsRange || redirectHint?.supportsRange);
  const filename = ensureFileNameForUrl(meta.filename || getFileNameFromUrl(details.url), details.url, meta.mime);
  const extension = getFileExtension(filename || details.url) || extensionFromMime(meta.mime) || "";
  if (details.type !== "media" && !shouldTrackBlobSource(details, { ...meta, filename }) && !shouldCaptureOpaqueBinaryResponse(details, { ...meta, filename })) {
    if (details.type === "media") {
      sendExtensionLog("warn", "resource_rejected", "浏览器标记为媒体但未通过资源类型识别", {
        reason: "unsupported_media_resource",
        url: details.url,
        mime: meta.mime,
        extension,
        statusCode: details.statusCode
      }, 30000);
    }
    return;
  }

  const tabId = await resolveTabIdForRequest(details);
  const resolvedTabId = tabId || redirectHint?.tabId || 0;
  if (!resolvedTabId) {
    sendExtensionLog("warn", "resource_no_tab", "捕获到资源但无法归属到浏览器标签页", {
      reason: "missing_tab",
      url: details.url,
      mime: meta.mime,
      statusCode: details.statusCode
    }, 30000);
    return;
  }
  owner ||= bucketForTab(resolvedTabId);

  let tab = await tabInfo(resolvedTabId);
  const headerSnapshot = resolveHeaderSnapshot(details.url, details);
  const source = { ...redirectHint, ...headerSnapshot, ...details };
  if (tab && !matchesRequestContext({ ...cookieSourceDetails(tab), tabId: resolvedTabId, tabIdObserved: true }, source)) {
    tab = null;
  }
  const headers = {
    ...(redirectHint?.headers || {}),
    ...(headerSnapshot?.headers || {})
  };
  const referer = headers.referer ||
    (details.initiator && details.initiator !== "null" ? details.initiator : "") ||
    redirectHint?.initiator ||
    tab?.url ||
    "";
  if (referer) {
    headers.referer = referer;
  }

  cacheResource({
    id: `${resolvedTabId}:${normalizeResourceUrlKey(details.url)}`,
    tabId: resolvedTabId,
    frameId: Number.isInteger(details.frameId) && details.frameId >= 0 ? details.frameId : 0,
    frameDocumentId: details.documentId || "",
    frameUrl: details.documentUrl || details.originUrl || "",
    url: details.url,
    pageTitle: tab?.title || "",
    pageUrl: details.initiator && details.initiator !== "null" ? details.initiator : redirectHint?.initiator || tab?.url || "",
    filename,
    filenameSource: meta.filename
      ? NxToolsCapturePolicy.Sources.filename.contentDisposition
      : NxToolsCapturePolicy.Sources.filename.url,
    metadataSource: NxToolsCapturePolicy.Sources.metadata.response,
    discovery: meta.discovery,
    headerSource: NxToolsCapturePolicy.Sources.header.browser,
    referrerSource: NxToolsCapturePolicy.Sources.referrer.browser,
    mime: meta.mime,
    size: meta.size,
    mediaHint: meta.assetHint || null,
    supportsRange: meta.supportsRange || details.statusCode === 206 || Boolean(headerSnapshot?.supportsRange),
    referer,
    requestHeaders: headers,
    cookieContext: await cookieContextForCapture(source, tab, settings),
    source: "webRequest-response",
    capturedAt: Date.now()
  }, owner);
}

function shouldCaptureRequestResource(details) {
  if (!isHttpUrl(details.url)) {
    return false;
  }
  if (!isUsableGoogleVideoUrl(details.url)) {
    return false;
  }

  const filename = getFileNameFromUrl(details.url);
  const extension = getFileExtension(filename || details.url);
  const mime = mimeFromUrl(details.url);
  if (details.type === "media" || isMediaResource(extension, mime, details.url) || isBilibiliMediaUrl(details.url) || isDouyinMediaUrl(details.url)) {
    return true;
  }

  const headers = normalizeHeaders(details.requestHeaders);
  const accept = String(headers.accept || "").toLowerCase();
  const destination = String(headers["sec-fetch-dest"] || "").toLowerCase();
  const hasRange = Boolean(headers.range);
  const requestType = String(details.type || "").toLowerCase();
  if (hasRange && !["main_frame", "sub_frame", "stylesheet", "script", "image", "font"].includes(requestType)) {
    return true;
  }

  const urlText = String(details.url || "").toLowerCase();
  return (hasRange || accept.includes("video/") || accept.includes("audio/") || destination === "video" || destination === "audio") &&
    (isLikelyMediaUrl(urlText) || hasRange || destination === "video" || destination === "audio");
}

async function captureRequestResource(details) {
  let owner = details.tabId > 0 ? bucketForTab(details.tabId) : null;
  const settings = await getSettings();
  if (!settings.captureResources || !shouldCaptureRequestResource(details)) {
    return;
  }
  if (!isSiteEnabled(details.initiator, details.originUrl, details.documentUrl, details.url)) {
    return;
  }

  const tabId = await resolveTabIdForRequest(details);
  if (!tabId) {
    sendExtensionLog("warn", "request_resource_no_tab", "请求阶段捕获到资源但无法归属到标签页", {
      reason: "missing_tab",
      url: details.url,
      source: details.type || ""
    }, 30000);
    return;
  }
  owner ||= bucketForTab(tabId);

  let tab = await tabInfo(tabId);
  const headerSnapshot = resolveHeaderSnapshot(details.url, details);
  const source = { ...headerSnapshot, ...details };
  if (tab && !matchesRequestContext({ ...cookieSourceDetails(tab), tabId, tabIdObserved: true }, source)) {
    tab = null;
  }
  const headers = { ...(headerSnapshot?.headers || {}) };
  const referer = headers.referer || (details.initiator && details.initiator !== "null" ? details.initiator : tab?.url) || "";
  if (referer) {
    headers.referer = referer;
  }

  const mime = mimeFromUrl(details.url);
  const filename = ensureFileNameForUrl(getFileNameFromUrl(details.url), details.url, mime);
  cacheResource({
    id: `${tabId}:${normalizeResourceUrlKey(details.url)}`,
    tabId,
    frameId: Number.isInteger(details.frameId) && details.frameId >= 0 ? details.frameId : 0,
    frameDocumentId: details.documentId || "",
    frameUrl: details.documentUrl || details.originUrl || "",
    url: details.url,
    pageTitle: tab?.title || "",
    pageUrl: details.initiator && details.initiator !== "null" ? details.initiator : tab?.url || "",
    filename,
    filenameSource: NxToolsCapturePolicy.Sources.filename.url,
    metadataSource: NxToolsCapturePolicy.Sources.metadata.request,
    headerSource: NxToolsCapturePolicy.Sources.header.browser,
    referrerSource: NxToolsCapturePolicy.Sources.referrer.browser,
    mime,
    size: 0,
    supportsRange: Boolean(headerSnapshot?.supportsRange),
    referer,
    requestHeaders: headers,
    cookieContext: await cookieContextForCapture(source, tab, settings),
    source: "webRequest-request",
    capturedAt: Date.now()
  }, owner);
}

async function capturePageResource(sender, payload) {
  const tabId = sender.tab?.id || lastActiveTabId || 0;
  const owner = tabId > 0 ? bucketForTab(tabId) : null;
  const payloadUrl = boundedCapturedUrl(payload?.url);
  const settings = await getSettings();
  if (!settings.captureResources || !payloadUrl || !/^(https?:|blob:)/i.test(payloadUrl)) {
    return;
  }
  if (!isSiteEnabled(sender.tab?.url, payload?.pageUrl, payload?.href)) {
    return;
  }

  if (!tabId) {
    return;
  }

  const tab = await tabInfo(tabId);
  const headerSnapshot = isHttpUrl(payloadUrl) ? resolveHeaderSnapshot(payloadUrl, { tabId }) : null;
  const pageContext = NxToolsCapturePolicy.buildPageCaptureContext(
    sanitizeCapturedHeaderObject(headerSnapshot?.headers || {}),
    boundedCapturedUrl(tab?.url),
    boundedCapturedUrl(payload.href));

  const extension = String(payload.ext || "").replace(/^\./, "").toLowerCase();
  const elementKindMime = wildcardMimeForElementKind(payload.elementKind);
  const payloadMime = String(payload.mime || elementKindMime || "").toLowerCase();
  const sourceUrl = boundedCapturedUrl(payload.sourceUrl);
  const sourceMime = String(payload.sourceMime || elementKindMime || "").toLowerCase();
  const primaryUrl = sourceUrl && isHttpUrl(sourceUrl) ? sourceUrl : payloadUrl;
  const posterUrl = boundedCapturedUrl(payload.poster);
  const poster = isHttpUrl(posterUrl) ? posterUrl : "";
  const frameId = Number.isInteger(sender.frameId) ? sender.frameId : Number(payload.frameId || 0);
  const rawFileName = cleanFileName(payload.filename) || getFileNameFromUrl(primaryUrl) || (extension ? `resource.${extension}` : "");
  const filename = ensureFileNameForUrl(
    rawFileName,
    primaryUrl,
    sourceMime || payloadMime);

  cacheResource({
    id: `${tabId}:${normalizeResourceUrlKey(payloadUrl)}`,
    tabId,
    frameId,
    url: payloadUrl,
    pageKey: String(payload.pageKey || ""),
    pageTitle: tab?.title || "",
    pageUrl: pageContext.pageUrl,
    filename,
    filenameSource: cleanFileName(payload.filename)
      ? NxToolsCapturePolicy.Sources.filename.page
      : (getFileNameFromUrl(primaryUrl)
        ? NxToolsCapturePolicy.Sources.filename.url
        : NxToolsCapturePolicy.Sources.filename.inferred),
    metadataSource: NxToolsCapturePolicy.Sources.metadata.page,
    discovery: NxToolsCapturePolicy.normalizeDiscovery(null, payloadMime),
    mediaElement: NxToolsCapturePolicy.normalizeMediaElement(payload.mediaElement),
    headerSource: pageContext.headerSource,
    referrerSource: pageContext.referrerSource,
    mime: payloadMime,
    size: Number(payload.size || 0),
    width: Number(payload.width || 0),
    height: Number(payload.height || 0),
    poster,
    supportsRange: Boolean(headerSnapshot?.supportsRange),
    referer: pageContext.referer,
    requestHeaders: pageContext.headers,
    cookieContext: await cookieContextForCapture({ ...cookieSourceDetails(sender.tab), tabId }, tab, settings),
    source: String(payload.source || "content"),
    capturedAt: Date.now()
  }, owner);

  if (sourceUrl && isHttpUrl(sourceUrl)) {
    const sourceHeaderSnapshot = resolveHeaderSnapshot(sourceUrl, { tabId });
    const sourcePageContext = NxToolsCapturePolicy.buildPageCaptureContext(
      sanitizeCapturedHeaderObject(sourceHeaderSnapshot?.headers || {}),
      boundedCapturedUrl(tab?.url),
      boundedCapturedUrl(payload.href));
    cacheResource({
      id: `${tabId}:${normalizeResourceUrlKey(sourceUrl)}`,
      tabId,
      frameId,
      url: sourceUrl,
      pageKey: String(payload.pageKey || ""),
      blobUrl: payloadUrl,
      pageTitle: tab?.title || "",
      pageUrl: sourcePageContext.pageUrl,
      filename: ensureFileNameForUrl(filename || getFileNameFromUrl(sourceUrl), sourceUrl, sourceMime || payloadMime),
      filenameSource: cleanFileName(payload.filename)
        ? NxToolsCapturePolicy.Sources.filename.page
        : (getFileNameFromUrl(sourceUrl)
          ? NxToolsCapturePolicy.Sources.filename.url
          : NxToolsCapturePolicy.Sources.filename.inferred),
      metadataSource: NxToolsCapturePolicy.Sources.metadata.page,
      discovery: NxToolsCapturePolicy.normalizeDiscovery(null, sourceMime || payloadMime),
      headerSource: sourcePageContext.headerSource,
      referrerSource: sourcePageContext.referrerSource,
      mime: sourceMime || payloadMime,
      size: Number(payload.sourceSize || payload.size || 0),
      width: Number(payload.width || 0),
      height: Number(payload.height || 0),
      poster,
      supportsRange: Boolean(sourceHeaderSnapshot?.supportsRange),
      referer: sourcePageContext.referer,
      requestHeaders: sourcePageContext.headers,
      cookieContext: await cookieContextForCapture({ ...cookieSourceDetails(sender.tab), tabId }, tab, settings),
      source: `${String(payload.source || "content")}-source-url`,
      capturedAt: Date.now()
    }, owner);
  }
}
