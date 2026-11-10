"use strict";

// Resource lookup, media choices for the page overlay and Blob source resolution.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

function findResourceByUrl(rawUrl, tabId = 0) {
  if (!rawUrl || !isUsableGoogleVideoUrl(rawUrl)) {
    return null;
  }

  const suffix = `:${normalizeResourceUrlKey(rawUrl)}`;
  const candidates = tabId > 0
    ? [...(resourcesByTab.get(tabId)?.values() || [])]
    : [...resourcesById.values()];

  const targetKey = normalizeResourceUrlKey(rawUrl);
  return candidates
    .filter((resource) =>
      isUsableGoogleVideoUrl(resource.url) &&
      (resource.id.endsWith(suffix) || normalizeResourceUrlKey(resource.url) === targetKey))
    .sort((left, right) => right.capturedAt - left.capturedAt)[0] || null;
}

function canSendOneClickResource(resource) {
  if (!isUsableGoogleVideoUrl(resource?.url || "")) {
    return false;
  }

  const info = describeResource(resource);
  return info.kind === "streaming" || info.kind === "video" || info.kind === "audio" || info.kind === "image" || resource.size > 0 || resource.supportsRange;
}

function mediaSelectionRank(resource) {
  const info = describeResource(resource);
  const kindScore = info.kind === "streaming"
    ? 50_000_000
    : info.kind === "video"
      ? 40_000_000
      : info.kind === "audio"
        ? 30_000_000
        : 0;
  const sizeScore = Math.min(Number(resource.size || 0), 25_000_000);
  return kindScore + sizeScore + Number(resource.capturedAt || 0) / 100000;
}

function formatBytesCompact(bytes) {
  const units = ["B", "KB", "MB", "GB"];
  let value = Math.max(0, Number(bytes || 0));
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${Math.round(value)} ${units[unit]}` : `${value.toFixed(value >= 10 ? 1 : 2)} ${units[unit]}`;
}

function resourceChoicePayload(resource, index, context = {}) {
  const info = describeResource(resource);
  const selectedWidth = Number(context.width || 0);
  const selectedHeight = Number(context.height || 0);
  const fallbackResolution = String(context.resolution || (selectedWidth > 0 && selectedHeight > 0 ? `${selectedWidth}x${selectedHeight}` : ""));
  const width = Number(resource.width || 0) > 0 ? Number(resource.width || 0) : selectedWidth;
  const height = Number(resource.height || 0) > 0 ? Number(resource.height || 0) : selectedHeight;
  const resolution = info.resolution || (width > 0 && height > 0 ? `${width}x${height}` : fallbackResolution);
  const kindText = info.kind === "audio" ? "音频" : info.kind === "video" ? "视频" : info.kind === "streaming" ? "流媒体" : "媒体";
  const extension = info.extension || extensionForResource(resource);
  const format = extension ? extension.toUpperCase() : info.badge || "";
  const size = Number(resource.size || 0);
  const filename = buildResourceFileName({
    ...resource,
    width,
    height
  });
  const labelPrefix = resolution || `资源 ${index + 1}`;
  const label = [labelPrefix, kindText, format].filter(Boolean).join(" · ");
  const meta = [
    size > 0 ? formatBytesCompact(size) : "",
    filename && !/^videoplayback(?:\.\w+)?$/i.test(filename) ? filename : "",
    resource.source ? String(resource.source).replace(/^webRequest-/, "") : ""
  ].filter(Boolean).join(" · ");

  return {
    resourceId: resource.id,
    url: resource.url,
    filename,
    mime: resource.mime || mimeFromUrl(resource.url) || "",
    size,
    width,
    height,
    resolution,
    kind: info.kind,
    extension,
    format,
    label,
    meta,
    source: resource.source || "",
    capturedAt: Number(resource.capturedAt || 0)
  };
}

function buildContentResourceChoices(tabId, payload = {}) {
  const cutoff = Date.now() - 10 * 60 * 1000;
  const requestedPageKey = String(payload.pageKey || "");
  const requestedHref = String(payload.href || "");
  const context = {
    width: Number(payload.width || 0),
    height: Number(payload.height || 0),
    resolution: String(payload.resolution || "")
  };
  const resources = [...(resourcesByTab.get(tabId)?.values() || [])]
    .filter((resource) =>
      resource &&
      !NxToolsCapturePolicy.discoveryCandidateReason(resource) &&
      Number(resource.capturedAt || 0) >= cutoff &&
      isUsableGoogleVideoUrl(resource.url) &&
      canSendOneClickResource(resource) &&
      (
        !requestedPageKey ||
        !resource.pageKey ||
        resource.pageKey === requestedPageKey ||
        urlsLikelySamePage(resource.pageUrl || "", requestedHref)
      ))
    .sort((left, right) =>
      mediaSelectionRank(right) - mediaSelectionRank(left) ||
      Number(right.capturedAt || 0) - Number(left.capturedAt || 0) ||
      Number(right.size || 0) - Number(left.size || 0));

  const seenIds = new Set();
  const seenUrls = new Set();
  const choices = [];
  for (const resource of resources) {
    const id = String(resource.id || "");
    const urlKey = normalizeResourceUrlKey(resource.url || "");
    if (!id || seenIds.has(id) || seenUrls.has(urlKey)) {
      continue;
    }
    seenIds.add(id);
    if (urlKey) {
      seenUrls.add(urlKey);
    }
    choices.push(resourceChoicePayload(resource, choices.length, context));
    if (choices.length >= 12) {
      break;
    }
  }

  return choices;
}

function excludedLocalResourceUrls(tabId, candidates) {
  const bucket = resourcesByTab.get(tabId);
  return (Array.isArray(candidates) ? candidates : []).slice(0, 80).filter(candidate => {
    const url = boundedCapturedUrl(candidate?.url);
    if (!url) return false;
    const cached = bucket?.get(`${tabId}:${normalizeResourceUrlKey(url)}`);
    return Boolean(NxToolsCapturePolicy.discoveryCandidateReason(NxToolsCapturePolicy.mergeCapturedResource(cached, {
      url, mime: boundedCapturedText(candidate.mime, 256), mediaElement: NxToolsCapturePolicy.normalizeMediaElement(candidate.mediaElement),
      discovery: NxToolsCapturePolicy.normalizeDiscovery(null, candidate.mime),
    })));
  }).map(candidate => candidate.url);
}

function isBlobResource(resource) {
  return String(resource?.url || "").startsWith("blob:");
}

function blobOrigin(url) {
  const text = String(url || "");
  if (!text.startsWith("blob:")) {
    return "";
  }

  try {
    return new URL(text.slice(5)).origin;
  } catch {
    return "";
  }
}

function sameBlobSource(resource, blobResource) {
  const sourceUrl = String(blobResource?.sourceUrl || "");
  if (sourceUrl && sameResource(resource?.url || "", sourceUrl)) {
    return true;
  }

  const blobUrl = String(blobResource?.url || "");
  return Boolean(blobUrl && String(resource?.blobUrl || "") === blobUrl);
}

function blobSourceCandidateRank(resource, blobResource) {
  const info = describeResource(resource);
  const sourceMatch = sameBlobSource(resource, blobResource) ? 2_000_000_000 : 0;
  const sourceUrl = String(blobResource?.sourceUrl || "");
  const blobOriginText = blobOrigin(blobResource?.url || "");
  let originScore = 0;
  try {
    if (sourceUrl && new URL(resource.url).origin === new URL(sourceUrl).origin) {
      originScore = 200_000_000;
    } else if (blobOriginText && new URL(resource.url).origin === blobOriginText) {
      originScore = 100_000_000;
    }
  } catch {
    originScore = 0;
  }

  const blobSize = Number(blobResource?.sourceSize || blobResource?.size || 0);
  const resourceSize = Number(resource?.size || 0);
  const sizeDiff = blobSize > 0 && resourceSize > 0 ? Math.abs(blobSize - resourceSize) : Number.MAX_SAFE_INTEGER;
  const sizeScore = sizeDiff === 0
    ? 500_000_000
    : sizeDiff <= Math.max(4096, blobSize * 0.02)
      ? 200_000_000 - Math.min(sizeDiff, 50_000_000)
      : 0;
  const mimeScore = blobResource?.mime && resource?.mime && String(blobResource.mime).split(";", 1)[0] === String(resource.mime).split(";", 1)[0]
    ? 90_000_000
    : 0;
  const kindScore = info.kind === "streaming"
    ? 80_000_000
    : info.kind === "video" || info.kind === "audio"
      ? 70_000_000
      : describeResource(resource).kind === "image"
        ? 60_000_000
        : 0;
  const timeScore = Math.max(0, 5 * 60 * 1000 - Math.abs(Number(resource.capturedAt || 0) - Number(blobResource.capturedAt || 0)));
  return sourceMatch + originScore + sizeScore + mimeScore + kindScore + timeScore + Math.min(resourceSize, 20_000_000);
}

function isReliableBlobSourceCandidate(resource, blobResource) {
  if (sameBlobSource(resource, blobResource)) {
    return true;
  }

  const sourceUrl = String(blobResource?.sourceUrl || "");
  if (sourceUrl) {
    return sameResource(resource?.url || "", sourceUrl);
  }

  const blobSize = Number(blobResource?.sourceSize || blobResource?.size || 0);
  const resourceSize = Number(resource?.size || 0);
  const blobMime = String(blobResource?.sourceMime || blobResource?.mime || "").split(";", 1)[0].toLowerCase();
  const resourceMime = String(resource?.mime || "").split(";", 1)[0].toLowerCase();
  const blobKind = describeResource(blobResource).kind;
  const resourceKind = describeResource(resource).kind;
  const sizeMatches = blobSize > 0 &&
    resourceSize > 0 &&
    Math.abs(blobSize - resourceSize) <= Math.max(4096, blobSize * 0.02);
  const mimeMatches = Boolean(blobMime && resourceMime && (
    blobMime === resourceMime ||
    (blobMime.endsWith("/*") && resourceMime.startsWith(blobMime.slice(0, -1))) ||
    (resourceMime.endsWith("/*") && blobMime.startsWith(resourceMime.slice(0, -1)))
  ));
  const kindMatches = blobKind === resourceKind && (blobKind === "audio" || blobKind === "video" || blobKind === "image");

  return sizeMatches && (mimeMatches || kindMatches);
}

function findBlobSourceResource(blobResource) {
  if (!isBlobResource(blobResource)) {
    return null;
  }

  const tabId = Number(blobResource.tabId || 0);
  const cutoff = Number(blobResource.capturedAt || Date.now()) - 5 * 60 * 1000;
  const candidates = [...(resourcesByTab.get(tabId)?.values() || [])]
    .filter((resource) =>
      resource &&
      resource.id !== blobResource.id &&
      isHttpUrl(resource.url) &&
      Number(resource.capturedAt || 0) >= cutoff &&
      isReliableBlobSourceCandidate(resource, blobResource));

  if (candidates.length === 0) {
    return null;
  }

  return candidates.sort((left, right) => blobSourceCandidateRank(right, blobResource) - blobSourceCandidateRank(left, blobResource))[0] || null;
}

function findBestPayloadResource(payload, tabId) {
  const urls = payloadResourceUrls(payload);
  const candidateMap = payloadResourceCandidateMap(payload);
  if (urls.length === 0) {
    return null;
  }

  const candidates = [];
  for (let index = 0; index < urls.length; index += 1) {
    const resource = findResourceByUrl(urls[index], tabId);
    if (resource && canSendOneClickResource(resource)) {
      candidates.push({
        resource,
        index
      });
    }
  }

  if (candidates.length === 0) {
    return null;
  }

  const requestedAt = Number(payload.requestedAt || 0);
  candidates.sort((left, right) => {
    const candidateLeft = payloadCandidateInfo(left.resource, candidateMap);
    const candidateRight = payloadCandidateInfo(right.resource, candidateMap);
    const candidateRankLeft = candidateLeft ? 1_000_000_000 - candidateLeft.index * 10_000_000 : 0;
    const candidateRankRight = candidateRight ? 1_000_000_000 - candidateRight.index * 10_000_000 : 0;
    const candidateTimeLeft = Number(candidateLeft?.capturedAt || left.resource.capturedAt || 0);
    const candidateTimeRight = Number(candidateRight?.capturedAt || right.resource.capturedAt || 0);
    const recentLeft = requestedAt > 0 && Math.abs(Number(left.resource.capturedAt || 0) - requestedAt) <= 15000 ? 1 : 0;
    const recentRight = requestedAt > 0 && Math.abs(Number(right.resource.capturedAt || 0) - requestedAt) <= 15000 ? 1 : 0;
    return candidateRankRight - candidateRankLeft ||
      candidateTimeRight - candidateTimeLeft ||
      recentRight - recentLeft ||
      left.index - right.index ||
      mediaSelectionRank(right.resource) - mediaSelectionRank(left.resource);
  });

  return candidates[0].resource;
}
