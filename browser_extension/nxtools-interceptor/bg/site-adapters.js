"use strict";

// Media classification and site-specific (Bilibili/Douyin/DASH) resource rules.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

function shouldTrackBlobSource(details, meta) {
  if (!isHttpUrl(details?.url)) {
    return false;
  }

  if (meta?.assetHint?.kind) {
    return true;
  }

  const extension = getFileExtension(meta?.filename || getFileNameFromUrl(details.url) || details.url) ||
    extensionFromMime(meta?.mime) ||
    extensionFromMime(mimeFromUrl(details.url));
  const type = String(meta?.mime || "").toLowerCase();
  if (isMediaResource(extension, type, details.url) || isBilibiliMediaUrl(details.url) || isDouyinMediaUrl(details.url) || isLikelyMediaUrl(details.url)) {
    return true;
  }

  if (type.startsWith("image/") && Number(meta?.size || 0) > 0) {
    return true;
  }

  const urlText = String(details.url || "").toLowerCase();
  return /\.(gif|webp|png|jpe?g)(?:[?#]|$)/i.test(urlText);
}

function isM3u8(extension, mime, url) {
  const type = String(mime || "").toLowerCase();
  return extension === "m3u8" || extension === "m3u" || HLS_TYPES.has(type) || /\.m3u8(?:[?#]|$)/i.test(url || "");
}

function isMpd(extension, mime, url) {
  const type = String(mime || "").toLowerCase();
  return extension === "mpd" || MPD_TYPES.has(type) || /\.mpd(?:[?#]|$)/i.test(url || "");
}

function isMediaResource(extension, mime, url) {
  const type = String(mime || "").toLowerCase();
  return MEDIA_EXTENSIONS.has(extension) ||
    IMAGE_EXTENSIONS.has(extension) ||
    DOWNLOAD_EXTENSIONS.has(extension) ||
    type.startsWith("video/") ||
    type.startsWith("audio/") ||
    type.startsWith("image/") ||
    type === "video/*" ||
    type === "audio/*" ||
    type === "image/*" ||
    isM3u8(extension, type, url) ||
    isMpd(extension, type, url);
}

function isLikelyMediaUrl(url) {
  return isUsableGoogleVideoUrl(url) && LIKELY_MEDIA_URL_PATTERN.test(String(url || ""));
}

function shouldCaptureOpaqueBinaryResponse(details, meta) {
  if (!meta || details.statusCode < 200 || details.statusCode >= 400) {
    return false;
  }

  if (meta.assetHint?.kind) {
    return true;
  }

  const type = String(meta.mime || "").toLowerCase();
  const size = Number(meta.size || 0);
  const hasRangeSignal = Boolean(meta.supportsRange || details.statusCode === 206 || responseHeaderValue(details.responseHeaders, "content-range"));
  if (type && type !== "application/octet-stream" && type !== "binary/octet-stream") {
    return false;
  }

  const headerSnapshot = resolveHeaderSnapshot(details.url, details);
  const headers = headerSnapshot?.headers || {};
  const accept = String(headers.accept || "").toLowerCase();
  const destination = String(headers["sec-fetch-dest"] || "").toLowerCase();
  const requestedAsMedia = accept.includes("audio/") ||
    accept.includes("video/") ||
    destination === "audio" ||
    destination === "video" ||
    details.type === "media";

  return hasRangeSignal && size > 1024 && (requestedAsMedia || isLikelyMediaUrl(details.url));
}

function isBilibiliMediaUrl(url) {
  try {
    const parsed = new URL(url);
    const host = parsed.hostname.toLowerCase();
    const path = parsed.pathname.toLowerCase();
    return (
      (host.endsWith("bilivideo.com") || host.includes(".bilivideo.") || host.endsWith("hdslb.com")) &&
      (
        /\.m4s$/i.test(path) ||
        /\.mp4$/i.test(path) ||
        path.includes("/video/") ||
        path.includes("/audio/")
      )
    );
  } catch {
    return false;
  }
}

function isDouyinPageUrl(url) {
  try {
    const host = new URL(url).hostname.toLowerCase();
    return host === "douyin.com" || host.endsWith(".douyin.com");
  } catch {
    return false;
  }
}

function isDouyinMediaUrl(url) {
  try {
    const parsed = new URL(url);
    const host = parsed.hostname.toLowerCase();
    const text = `${parsed.pathname} ${parsed.search}`.toLowerCase();
    const isMediaHost = host.includes("douyinvod") ||
      host.includes("douyinpic") ||
      host.includes("bytecdn") ||
      host.includes("bytedance") ||
      host.includes("pstatp");
    if (!isMediaHost) {
      return false;
    }

    return (
      /(media-video|media-audio|mime_type=video|mime_type=audio|video_mp4|audio|video)/i.test(text)
    );
  } catch {
    return false;
  }
}

function douyinTrackKind(resource) {
  const text = `${resource?.filename || ""} ${resource?.mime || ""} ${resource?.url || ""}`.toLowerCase();
  if (/media-audio|mime_type=audio|audio_mp4|audio\//i.test(text)) {
    return "audio";
  }
  if (/media-video|mime_type=video|video_mp4|video\//i.test(text)) {
    return "video";
  }
  return resourceMediaKind(resource);
}

function dashTrackKind(resource) {
  const mime = String(resource?.mime || "").toLowerCase();
  if (mime.startsWith("video/")) {
    return "video";
  }
  if (mime.startsWith("audio/")) {
    return "audio";
  }

  const text = `${resource?.filename || ""} ${resource?.url || ""}`.toLowerCase();
  if (/(^|[-_.\\/])audio([-_.\\/]|$)/i.test(text)) {
    return "audio";
  }
  if (/(^|[-_.\\/])video([-_.\\/]|$)/i.test(text)) {
    return "video";
  }

  const match = text.match(/\/(\d{2,6})\.m4s(?:$|[?#])/i)
    || text.match(/[-_.\\/](\d{2,6})\.m4s(?:$|[?#])/i);
  const trackId = match?.[1] || "";
  if (/^302\d{2,3}$/.test(trackId)) {
    return "audio";
  }
  if (/^30\d{3}$/.test(trackId) || /^[1-9]\d{1,2}$/.test(trackId)) {
    return "video";
  }

  return "";
}

function describeResource(resource) {
  const extension = extensionForResource(resource);
  let kind = "download";
  let badge = extension ? extension.slice(0, 6).toUpperCase() : "资源";
  const dashKind = extension === "m4s" ? dashTrackKind(resource) : "";
  const youtubeInfo = youtubeItagInfoFromUrl(resource?.url || "");
  const resourceMime = String(resource.mime || mimeFromUrl(resource.url || "") || "").toLowerCase();
  const mediaHint = resource?.assetHint || resource?.mediaHint || null;
  if (mediaHint?.kind) {
    kind = mediaHint.kind;
    badge = extension ? extension.toUpperCase() : mediaHint.label || "资源";
  } else if (isM3u8(extension, resource.mime, resource.url)) {
    kind = "streaming";
    badge = "M3U8";
  } else if (isMpd(extension, resource.mime, resource.url)) {
    kind = "streaming";
    badge = "MPD";
  } else if (dashKind === "audio") {
    kind = "audio";
    badge = "M4S";
  } else if (dashKind === "video") {
    kind = "video";
    badge = "M4S";
  } else if (resourceMime.startsWith("video/") || youtubeInfo?.kind === "video" || isGoogleVideoUrl(resource?.url || "") || (VIDEO_EXTENSIONS.has(extension) && extension !== "gif")) {
    kind = "video";
    badge = extension ? extension.toUpperCase() : "视频";
  } else if (resourceMime.startsWith("audio/") || youtubeInfo?.kind === "audio" || AUDIO_EXTENSIONS.has(extension)) {
    kind = "audio";
    badge = extension ? extension.toUpperCase() : "音频";
  } else if (resourceMime.startsWith("image/") || IMAGE_EXTENSIONS.has(extension)) {
    kind = "image";
    badge = extension ? extension.toUpperCase() : "图片";
  } else if (DOWNLOAD_EXTENSIONS.has(extension)) {
    kind = "download";
    badge = extension ? extension.toUpperCase() : "文件";
  }
  return { extension, kind, badge, resolution: resourceResolutionText(resource) };
}

function hostEndsWith(url, suffix) {
  try {
    return new URL(url).hostname.toLowerCase().endsWith(suffix);
  } catch {
    return false;
  }
}

function sameResource(left, right) {
  try {
    const leftUrl = new URL(left);
    const rightUrl = new URL(right);
    leftUrl.hash = "";
    rightUrl.hash = "";
    return leftUrl.toString() === rightUrl.toString();
  } catch {
    return String(left || "") === String(right || "");
  }
}

function payloadResourceSet(payload) {
  return new Set([payload.url, ...(Array.isArray(payload.resourceUrls) ? payload.resourceUrls : [])].filter(Boolean));
}

function payloadResourceUrls(payload) {
  const raw = [
    payload.url,
    ...(Array.isArray(payload.resourceUrls) ? payload.resourceUrls : []),
    ...(Array.isArray(payload.resourceCandidates) ? payload.resourceCandidates.map((candidate) => candidate?.url) : [])
  ].filter(Boolean);
  const seen = new Set();
  return raw.filter((url) => {
    const key = normalizeResourceUrlKey(url);
    if (!key || seen.has(key) || !isUsableGoogleVideoUrl(url)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function payloadResourceCandidateMap(payload) {
  const result = new Map();
  const candidates = Array.isArray(payload.resourceCandidates) ? payload.resourceCandidates : [];
  for (let index = 0; index < candidates.length; index += 1) {
    const candidate = candidates[index] || {};
    const key = normalizeResourceUrlKey(candidate.url || "");
    if (!key || result.has(key)) {
      continue;
    }
    result.set(key, {
      index,
      capturedAt: Number(candidate.capturedAt || 0),
      size: Number(candidate.size || 0),
      width: Number(candidate.width || 0),
      height: Number(candidate.height || 0),
      mime: String(candidate.mime || ""),
      source: String(candidate.source || "")
    });
  }
  return result;
}

function payloadResourceIndexMap(payload) {
  const result = new Map();
  payloadResourceUrls(payload).forEach((url, index) => {
    const key = normalizeResourceUrlKey(url);
    if (key && !result.has(key)) {
      result.set(key, index);
    }
  });
  return result;
}

function payloadResourceIndex(resource, indexMap) {
  const key = normalizeResourceUrlKey(resource?.url || "");
  return key && indexMap.has(key) ? indexMap.get(key) : Number.MAX_SAFE_INTEGER;
}

function payloadCandidateInfo(resource, candidateMap) {
  const key = normalizeResourceUrlKey(resource?.url || "");
  return key ? candidateMap.get(key) || null : null;
}

function matchesPayloadResource(resource, payloadUrls) {
  for (const url of payloadUrls) {
    if (sameResource(resource.url, url)) {
      return true;
    }
  }
  return false;
}

function resourceMediaKind(resource) {
  const info = describeResource(resource);
  if (info.kind === "video" || info.kind === "audio") {
    return info.kind;
  }
  return dashTrackKind(resource);
}

function pickBilibiliTrack(resources, kind, payloadUrls, payloadIndexMap) {
  const rank = (resource) => {
    const payloadIndex = payloadResourceIndex(resource, payloadIndexMap);
    const payloadScore = Number.isFinite(payloadIndex) ? 2_000_000_000 - payloadIndex * 20_000_000 : 0;
    const mimeScore = String(resource.mime || "").startsWith(`${kind}/`) ? 10_000_000 : 0;
    return payloadScore +
      mimeScore +
      Number(resource.capturedAt || 0) / 1000 +
      Math.min(Number(resource.size || 0), 9_000_000);
  };

  return resources
    .filter((resource) => resourceMediaKind(resource) === kind)
    .sort((left, right) => rank(right) - rank(left))[0] || null;
}

function pickDouyinVideoResource(resources, payload) {
  const payloadUrls = payloadResourceSet(payload);
  const payloadIndexMap = payloadResourceIndexMap(payload);
  const candidateMap = payloadResourceCandidateMap(payload);
  const requestedAt = Number(payload.requestedAt || Date.now());
  const recentCutoff = requestedAt - 90 * 1000;
  const douyinResources = resources.filter((resource) => {
    const capturedAt = Number(resource.capturedAt || 0);
    const size = Number(resource.size || 0);
    return capturedAt >= recentCutoff &&
      isDouyinMediaUrl(resource.url) &&
      matchesPayloadResource(resource, payloadUrls) &&
      douyinTrackKind(resource) === "video" &&
      (size <= 0 || size >= 64 * 1024) &&
      canSendOneClickResource(resource);
  });

  if (douyinResources.length === 0) {
    const recentCandidates = resources
      .filter((resource) => Number(resource.capturedAt || 0) >= recentCutoff && matchesPayloadResource(resource, payloadUrls))
      .slice(0, 8);
    sendExtensionLog("warn", "douyin_video_candidates_empty", "抖音当前视频没有匹配到可发送的直连资源", {
      pageUrl: payload.href || "",
      payloadResourceCount: payloadUrls.size,
      resourceCount: resources.length,
      sampleUrls: resourceSamples(recentCandidates, 8)
    }, 0);
    return null;
  }

  const rank = (resource) => {
    const payloadIndex = payloadResourceIndex(resource, payloadIndexMap);
    const candidate = payloadCandidateInfo(resource, candidateMap);
    const payloadScore = Number.isFinite(payloadIndex) ? 400_000_000 - payloadIndex * 20_000_000 : 0;
    const candidateScore = candidate ? 900_000_000 - candidate.index * 25_000_000 : 0;
    const candidateFreshness = candidate?.capturedAt ? Math.max(0, 3 * 60 * 1000 - Math.abs(candidate.capturedAt - requestedAt)) / 2 : 0;
    const urlText = String(resource.url || "").toLowerCase();
    const explicitVideoScore = urlText.includes("media-video") ? 80_000_000 : 0;
    const mimeScore = String(resource.mime || "").startsWith("video/") ? 50_000_000 : 0;
    const extensionScore = extensionForResource(resource) === "mp4" ? 10_000_000 : 0;
    const responseScore = String(resource.source || "").includes("response") ? 8_000_000 : 0;
    const sizeScore = Math.min(Number(resource.size || 0), 40_000_000);
    const freshnessScore = Math.max(0, 3 * 60 * 1000 - Math.abs(Number(resource.capturedAt || 0) - requestedAt)) / 10;
    return candidateScore + candidateFreshness + payloadScore + explicitVideoScore + mimeScore + extensionScore + responseScore + sizeScore + freshnessScore;
  };

  return douyinResources.sort((left, right) => rank(right) - rank(left))[0];
}

function pickSiteMediaResources(resources, payload) {
  let pageUrl;
  try {
    pageUrl = new URL(payload.href || "");
  } catch {
    return null;
  }

  if (pageUrl.hostname === "www.youtube.com" || pageUrl.hostname.endsWith(".youtube.com")) {
    const recentCutoff = Date.now() - 10 * 60 * 1000;
    const youtubeResources = resources.filter((resource) => {
      const url = String(resource.url || "");
      return Number(resource.capturedAt || 0) >= recentCutoff && isGoogleVideoUrl(url);
    });

    const progressive = youtubeResources
      .filter((resource) => youtubeItagInfoFromUrl(resource.url)?.progressive)
      .sort((left, right) => {
        const leftInfo = youtubeItagInfoFromUrl(left.url) || {};
        const rightInfo = youtubeItagInfoFromUrl(right.url) || {};
        const leftScore = Number(leftInfo.height || 0) * 1_000_000 + Number(left.size || 0);
        const rightScore = Number(rightInfo.height || 0) * 1_000_000 + Number(right.size || 0);
        return rightScore - leftScore;
      })[0] || null;

    if (progressive) {
      sendExtensionLog("info", "youtube_progressive_selected", "已选择 YouTube 音视频合一资源", {
        pageUrl: pageUrl.toString(),
        resourceCount: youtubeResources.length,
        url: progressive.url,
        fileName: buildResourceFileName(progressive),
        resolution: resourceResolutionText(progressive),
        sampleUrls: resourceSamples([progressive], 1)
      }, 0);
      return {
        exclusive: true,
        resources: [progressive],
        message: "已选择 YouTube 音视频合一资源"
      };
    }

    const bestCaptured = youtubeResources
      .sort((left, right) => {
        const leftInfo = youtubeItagInfoFromUrl(left.url) || {};
        const rightInfo = youtubeItagInfoFromUrl(right.url) || {};
        const leftScore = Number(leftInfo.height || 0) * 1_000_000 + Number(left.size || 0);
        const rightScore = Number(rightInfo.height || 0) * 1_000_000 + Number(right.size || 0);
        return rightScore - leftScore;
      })[0] || null;

    if (bestCaptured) {
      sendExtensionLog("info", "youtube_captured_selected", "已选择 YouTube 已捕获资源", {
        pageUrl: pageUrl.toString(),
        resourceCount: youtubeResources.length,
        url: bestCaptured.url,
        fileName: buildResourceFileName(bestCaptured),
        resolution: resourceResolutionText(bestCaptured),
        sampleUrls: resourceSamples([bestCaptured], 1)
      }, 0);
      return {
        exclusive: true,
        resources: [bestCaptured],
        message: "已选择 YouTube 已捕获资源"
      };
    }

    return null;
  }

  if (isDouyinPageUrl(pageUrl.toString())) {
    const video = pickDouyinVideoResource(resources, payload);
    if (video) {
      sendExtensionLog("info", "douyin_video_selected", "已选择抖音当前视频资源", {
        pageUrl: pageUrl.toString(),
        resourceCount: resources.length,
        url: video.url,
        fileName: buildResourceFileName(video),
        resolution: resourceResolutionText(video),
        sampleUrls: resourceSamples([video], 1)
      }, 0);
      return {
        exclusive: true,
        resources: [video],
        message: "已选择抖音当前视频资源"
      };
    }

    return {
      exclusive: true,
      resources: [],
      message: "抖音当前视频资源还没有捕获完整，请让视频播放几秒后再点悬浮下载"
    };
  }

  if (!(pageUrl.hostname === "www.bilibili.com" || pageUrl.hostname.endsWith(".bilibili.com"))) {
    return null;
  }

  const payloadUrls = payloadResourceSet(payload);
  const payloadIndexMap = payloadResourceIndexMap(payload);
  const candidateMap = payloadResourceCandidateMap(payload);
  const recentCutoff = Date.now() - 10 * 60 * 1000;
  const biliResources = resources.filter((resource) => {
    const url = String(resource.url || "");
    return (
      Number(resource.capturedAt || 0) >= recentCutoff &&
      (
        isBilibiliMediaUrl(url) ||
        hostEndsWith(url, "bilivideo.com") ||
        hostEndsWith(url, "hdslb.com") ||
        matchesPayloadResource(resource, payloadUrls)
      )
    );
  });

  const video = pickBilibiliTrack(biliResources, "video", payloadUrls, payloadIndexMap);
  const audio = pickBilibiliTrack(biliResources, "audio", payloadUrls, payloadIndexMap);
  if (video && audio) {
    sendExtensionLog("info", "bilibili_tracks_selected", "已选择 B 站音视频轨道", {
      pageUrl: pageUrl.toString(),
      resourceCount: biliResources.length,
      videoCount: 1,
      audioCount: 1,
      sampleUrls: resourceSamples([video, audio], 2)
    }, 15000);
    return {
      exclusive: true,
      resources: [video, audio],
      message: "已捕获 B 站视频和音频轨道"
    };
  }

  const fallback = biliResources
    .filter(canSendOneClickResource)
    .sort((left, right) => {
      const leftCandidate = payloadCandidateInfo(left, candidateMap);
      const rightCandidate = payloadCandidateInfo(right, candidateMap);
      return (leftCandidate?.index ?? Number.MAX_SAFE_INTEGER) - (rightCandidate?.index ?? Number.MAX_SAFE_INTEGER) ||
      payloadResourceIndex(left, payloadIndexMap) - payloadResourceIndex(right, payloadIndexMap) ||
      Number(rightCandidate?.capturedAt || 0) - Number(leftCandidate?.capturedAt || 0) ||
      Number(right.capturedAt || 0) - Number(left.capturedAt || 0) ||
      Number(right.size || 0) - Number(left.size || 0);
    })[0];

  sendExtensionLog("warn", "bilibili_tracks_incomplete", "B 站资源捕获不完整，无法同时找到视频和音频轨道", {
    reason: "missing_video_or_audio_track",
    pageUrl: pageUrl.toString(),
    resourceCount: biliResources.length,
    videoCount: video ? 1 : 0,
    audioCount: audio ? 1 : 0,
    payloadResourceCount: payloadUrls.size,
    sampleUrls: resourceSamples(biliResources, 5)
  }, 0);

  return {
    exclusive: true,
    resources: fallback ? [fallback] : [],
    message: "B 站当前媒体需要同时捕获视频和音频轨道，请播放几秒后再打开资源嗅探"
  };
}

async function downloadPageMedia(sender, payload = {}) {
  const tabId = sender.tab?.id || lastActiveTabId || 0;
  const resources = [...(resourcesByTab.get(tabId)?.values() || [])]
    .filter((resource) => isUsableGoogleVideoUrl(resource.url))
    .sort((left, right) => right.capturedAt - left.capturedAt);
  sendExtensionLog("info", "page_media_download_requested", "页面媒体下载请求开始匹配资源", {
    pageUrl: payload.href || sender.tab?.url || "",
    tabId,
    directUrl: payload.url || "",
    selectionUrl: payload.selectionUrl || "",
    selectedResourceId: payload.selectedResourceId || "",
    preferRecentResources: Boolean(payload.preferRecentResources),
    resourceCount: resources.length,
    payloadResourceCount: Array.isArray(payload.resourceUrls) ? payload.resourceUrls.length : 0,
    payloadCandidateCount: Array.isArray(payload.resourceCandidates) ? payload.resourceCandidates.length : 0,
    payloadCandidateSamples: resourceSamples(Array.isArray(payload.resourceCandidates) ? payload.resourceCandidates : [], 5),
    sampleUrls: resourceSamples(resources, 5)
  }, 10000);

  const explicitResourceId = String(payload.selectedResourceId || "");
  if (explicitResourceId) {
    const explicitResource = resourcesById.get(explicitResourceId);
    if (explicitResource && explicitResource.tabId === tabId && isUsableGoogleVideoUrl(explicitResource.url) && canSendOneClickResource(explicitResource)) {
      const selectedWidth = Number(payload.selectedResourceWidth || payload.width || 0);
      const selectedHeight = Number(payload.selectedResourceHeight || payload.height || 0);
      sendExtensionLog("info", "page_media_resource_selected", "页面媒体已按用户选择的资源 ID 发送", {
        reason: "user_selected_resource_id",
        resourceId: explicitResourceId,
        url: explicitResource.url,
        fileName: buildResourceFileName({
          ...explicitResource,
          filename: payload.selectedResourceFileName || explicitResource.filename,
          mime: payload.selectedResourceMime || explicitResource.mime,
          size: Number(payload.selectedResourceSize || 0) > 0 ? Number(payload.selectedResourceSize || 0) : explicitResource.size,
          width: selectedWidth > 0 ? selectedWidth : explicitResource.width,
          height: selectedHeight > 0 ? selectedHeight : explicitResource.height
        }),
        resolution: payload.selectedResourceResolution || resourceResolutionText(explicitResource),
        tabId
      }, 10000);
      return sendResource(explicitResource.id, {
        filename: payload.selectedResourceFileName || "",
        mime: payload.selectedResourceMime || "",
        size: Number(payload.selectedResourceSize || 0),
        width: selectedWidth,
        height: selectedHeight
      });
    }

    sendExtensionLog("warn", "page_media_resource_selected_missing", "用户选择的资源 ID 已不存在或不可发送", {
      reason: "selected_resource_id_missing",
      resourceId: explicitResourceId,
      tabId,
      pageUrl: payload.href || sender.tab?.url || ""
    }, 0);
    return {
      ok: false,
      message: "选择的媒体资源已失效，请让视频播放几秒后重新打开列表"
    };
  }

  const siteSelection = pickSiteMediaResources(resources, payload);
  if (siteSelection?.resources.length === 1) {
    return sendResource(siteSelection.resources[0].id);
  }
  if (siteSelection?.resources.length === 2) {
    return mergeResources(siteSelection.resources.map((resource) => resource.id));
  }
  if (siteSelection?.exclusive) {
    sendExtensionLog("warn", "page_media_site_match_failed", "站点专用媒体匹配失败", {
      reason: siteSelection.message || "",
      pageUrl: payload.href || sender.tab?.url || "",
      tabId,
      resourceCount: resources.length,
      payloadCandidateCount: Array.isArray(payload.resourceCandidates) ? payload.resourceCandidates.length : 0,
      sampleUrls: resourceSamples(resources, 5)
    }, 0);
    return {
      ok: false,
      message: siteSelection.message || "请在资源嗅探页选择当前媒体资源"
    };
  }

  const explicitUrl = String(payload.selectedResourceUrl || (payload.forceDirectResource ? payload.url : "") || "");
  if (explicitUrl) {
    const explicitResource = findResourceByUrl(explicitUrl, tabId);
    if (explicitResource && canSendOneClickResource(explicitResource)) {
      sendExtensionLog("info", "page_media_resource_selected", "页面媒体已按用户选择发送资源", {
        reason: "user_selected_resource",
        url: explicitResource.url,
        selectionUrl: payload.selectionUrl || "",
        tabId,
        resolution: resourceResolutionText(explicitResource)
      }, 10000);
      return sendResource(explicitResource.id);
    }

    if (isHttpUrl(explicitUrl)) {
      sendExtensionLog("info", "page_media_resource_selected", "页面媒体已按用户选择发送直连 URL", {
        reason: "user_selected_url",
        url: explicitUrl,
        selectionUrl: payload.selectionUrl || "",
        tabId
      }, 10000);
      return sendUrl(explicitUrl, {
        cookieSource: { ...cookieSourceDetails(sender.tab), tabId },
        fileName: payload.filename || getFileNameFromUrl(explicitUrl),
        referrer: payload.href || sender.tab?.url || "",
        size: Number(payload.size || 0)
      });
    }
  }

  const bestPayloadResource = findBestPayloadResource(payload, tabId);
  const direct = payload.url && !payload.preferRecentResources ? findResourceByUrl(payload.url, tabId) : null;
  if (direct && canSendOneClickResource(direct)) {
    sendExtensionLog("info", "page_media_resource_selected", "页面媒体已选择直连资源", {
      reason: "direct_url",
      url: direct.url,
      selectionUrl: payload.selectionUrl || "",
      tabId
    }, 10000);
    return sendResource(direct.id);
  }

  if (bestPayloadResource) {
    sendExtensionLog("info", "page_media_resource_selected", "页面媒体已选择最新候选资源", {
      reason: payload.preferRecentResources ? "stale_dom_url_prefer_recent" : "payload_resource",
      url: bestPayloadResource.url,
      selectionUrl: payload.selectionUrl || "",
      tabId
    }, 10000);
    return sendResource(bestPayloadResource.id);
  }

  const streaming = resources.find((resource) => describeResource(resource).kind === "streaming");
  if (streaming) {
    return sendResource(streaming.id);
  }

  const latest = resources.find(canSendOneClickResource);
  if (latest) {
    return sendResource(latest.id);
  }

  sendExtensionLog("warn", "page_media_no_resource", "页面媒体下载请求没有匹配到可下载资源", {
    reason: "no_downloadable_resource",
    pageUrl: payload.href || sender.tab?.url || "",
    tabId,
    resourceCount: resources.length,
    payloadResourceCount: Array.isArray(payload.resourceUrls) ? payload.resourceUrls.length : 0,
    payloadCandidateCount: Array.isArray(payload.resourceCandidates) ? payload.resourceCandidates.length : 0,
    sampleUrls: resourceSamples(resources, 5)
  }, 0);

  return {
    ok: false,
    message: "尚未捕获到可下载媒体资源，请先播放视频或打开资源嗅探页"
  };
}
