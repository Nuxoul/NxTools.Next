"use strict";

(function installCapturePolicy(root, factory) {
  const api = factory();
  root.NxToolsCapturePolicy = api;
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
})(typeof globalThis !== "undefined" ? globalThis : this, () => {
  const GENERIC_MIME_TYPES = new Set([
    "",
    "application/octet-stream",
    "binary/octet-stream",
    "audio/*",
    "image/*",
    "video/*"
  ]);

  const FILENAME_SOURCE_RANK = {
    "content-disposition": 100,
    page: 50,
    url: 30,
    inferred: 10
  };

  function nonEmpty(value) {
    return typeof value === "string" && value.trim().length > 0;
  }

  function isWeakFilename(value) {
    const text = String(value || "").trim().toLowerCase();
    if (!text) {
      return true;
    }

    const stem = text.replace(/\.[a-z0-9]{1,8}$/i, "");
    return /^(?:blob|download|file|media|resource|stream|unknown)(?:[-_. ]?\d+)?$/i.test(stem);
  }

  function filenameScore(resource) {
    const name = String(resource?.filename || "").trim();
    if (!name) {
      return 0;
    }
    const source = String(resource?.filenameSource || "inferred").toLowerCase();
    return (FILENAME_SOURCE_RANK[source] || FILENAME_SOURCE_RANK.inferred) +
      (isWeakFilename(name) ? 0 : 25);
  }

  function chooseFilename(existing, incoming) {
    const existingScore = filenameScore(existing);
    const incomingScore = filenameScore(incoming);
    if (incomingScore === 0) {
      return {
        filename: existing?.filename || "",
        filenameSource: existing?.filenameSource || ""
      };
    }
    if (existingScore > incomingScore) {
      return {
        filename: existing?.filename || "",
        filenameSource: existing?.filenameSource || ""
      };
    }
    return {
      filename: incoming.filename,
      filenameSource: incoming.filenameSource || "inferred"
    };
  }

  function chooseMime(existingMime, incomingMime) {
    const existing = String(existingMime || "").trim().toLowerCase();
    const incoming = String(incomingMime || "").trim().toLowerCase();
    if (!incoming) {
      return existing;
    }
    if (!GENERIC_MIME_TYPES.has(incoming) || GENERIC_MIME_TYPES.has(existing)) {
      return incoming;
    }
    return existing;
  }

  function preferText(incoming, existing) {
    return nonEmpty(incoming) ? incoming : existing || "";
  }

  function mergeCapturedResource(existingResource, incomingResource) {
    const existing = existingResource || {};
    const incoming = incomingResource || {};
    const filename = chooseFilename(existing, incoming);
    const existingSize = Number(existing.size || 0);
    const incomingSize = Number(incoming.size || 0);
    const existingCapturedAt = Number(existing.capturedAt || 0);
    const incomingCapturedAt = Number(incoming.capturedAt || 0);

    return {
      ...existing,
      ...incoming,
      id: preferText(incoming.id, existing.id),
      url: preferText(incoming.url, existing.url),
      blobUrl: preferText(incoming.blobUrl, existing.blobUrl),
      pageKey: preferText(incoming.pageKey, existing.pageKey),
      pageTitle: preferText(incoming.pageTitle, existing.pageTitle),
      pageUrl: preferText(incoming.pageUrl, existing.pageUrl),
      referer: preferText(incoming.referer, existing.referer),
      filename: filename.filename,
      filenameSource: filename.filenameSource,
      mime: chooseMime(existing.mime, incoming.mime),
      size: Math.max(
        Number.isFinite(existingSize) ? existingSize : 0,
        Number.isFinite(incomingSize) ? incomingSize : 0),
      width: Math.max(Number(existing.width || 0), Number(incoming.width || 0)),
      height: Math.max(Number(existing.height || 0), Number(incoming.height || 0)),
      supportsRange: Boolean(existing.supportsRange || incoming.supportsRange),
      mediaHint: incoming.mediaHint || incoming.assetHint || existing.mediaHint || existing.assetHint || null,
      assetHint: incoming.assetHint || existing.assetHint || null,
      requestHeaders: {
        ...(existing.requestHeaders || {}),
        ...(incoming.requestHeaders || {})
      },
      capturedAt: Math.max(existingCapturedAt, incomingCapturedAt) || Date.now()
    };
  }

  function projectResourceForPopup(resource) {
    const {
      requestHeaders: _requestHeaders,
      referer: _referer,
      pageUrl: _pageUrl,
      ...presentation
    } = resource || {};
    return {
      ...presentation,
      hasRequestContext: Boolean(
        Object.keys(resource?.requestHeaders || {}).length > 0 ||
        nonEmpty(resource?.referer))
    };
  }

  function uniqueCapturedResources(resources) {
    const unique = new Map();
    for (const resource of resources || []) {
      const key = String(resource?.id || resource?.url || "").trim();
      if (!key) {
        continue;
      }
      unique.set(key, mergeCapturedResource(unique.get(key), resource));
    }
    return [...unique.values()];
  }

  return {
    mergeCapturedResource,
    projectResourceForPopup,
    uniqueCapturedResources
  };
});
