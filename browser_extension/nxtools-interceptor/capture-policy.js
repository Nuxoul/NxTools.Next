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

  const METADATA_SOURCE_RANK = {
    response: 100,
    page: 50,
    request: 30,
    inferred: 10
  };

  const REQUEST_CONTEXT_SOURCE_RANK = {
    browser: 100,
    page: 40,
    inferred: 10
  };

  const Sources = Object.freeze({
    filename: Object.freeze({
      contentDisposition: "content-disposition",
      page: "page",
      url: "url",
      inferred: "inferred"
    }),
    metadata: Object.freeze({
      response: "response",
      page: "page",
      request: "request",
      inferred: "inferred"
    }),
    header: Object.freeze({
      browser: "browser",
      page: "page",
      inferred: "inferred"
    }),
    referrer: Object.freeze({
      browser: "browser",
      page: "page",
      inferred: "inferred"
    })
  });

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

  function sourceRank(resource, field, ranks, legacyField = "") {
    const source = resource?.[field] || (legacyField ? resource?.[legacyField] : "") || "inferred";
    return ranks[String(source).toLowerCase()] || ranks.inferred;
  }

  function chooseMime(existingResource, incomingResource) {
    const existing = String(existingResource?.mime || "").trim().toLowerCase();
    const incoming = String(incomingResource?.mime || "").trim().toLowerCase();
    if (!incoming) {
      return existing;
    }
    const existingRank = sourceRank(existingResource, "metadataSource", METADATA_SOURCE_RANK);
    const incomingRank = sourceRank(incomingResource, "metadataSource", METADATA_SOURCE_RANK);
    if (!existing) {
      return incoming;
    }
    if (incomingRank < existingRank) {
      return existing;
    }
    if (!GENERIC_MIME_TYPES.has(incoming) || GENERIC_MIME_TYPES.has(existing)) {
      return incoming;
    }
    return existing;
  }

  function chooseSize(existingResource, incomingResource) {
    const existing = Number(existingResource?.size || 0);
    const incoming = Number(incomingResource?.size || 0);
    if (!Number.isFinite(incoming) || incoming <= 0) {
      return Number.isFinite(existing) && existing > 0 ? existing : 0;
    }
    if (!Number.isFinite(existing) || existing <= 0) {
      return incoming;
    }
    const existingRank = sourceRank(existingResource, "metadataSource", METADATA_SOURCE_RANK);
    const incomingRank = sourceRank(incomingResource, "metadataSource", METADATA_SOURCE_RANK);
    if (incomingRank > existingRank) {
      return incoming;
    }
    return incomingRank === existingRank ? Math.max(existing, incoming) : existing;
  }

  function chooseHeaderSource(existingResource, incomingResource) {
    const field = "headerSource";
    const existingRank = sourceRank(existingResource, field, REQUEST_CONTEXT_SOURCE_RANK, "requestContextSource");
    const incomingRank = sourceRank(incomingResource, field, REQUEST_CONTEXT_SOURCE_RANK, "requestContextSource");
    return incomingRank >= existingRank
      ? incomingResource?.[field] || incomingResource?.requestContextSource || Sources.referrer.inferred
      : existingResource?.[field] || existingResource?.requestContextSource || Sources.referrer.inferred;
  }

  function chooseContextField(existingResource, incomingResource, field) {
    const existing = existingResource?.[field] || "";
    const incoming = incomingResource?.[field] || "";
    const existingSource = existingResource?.referrerSource || existingResource?.requestContextSource || Sources.referrer.inferred;
    const incomingSource = incomingResource?.referrerSource || incomingResource?.requestContextSource || Sources.referrer.inferred;
    if (!nonEmpty(incoming)) {
      return { value: existing, source: existingSource };
    }
    const existingRank = sourceRank(existingResource, "referrerSource", REQUEST_CONTEXT_SOURCE_RANK, "requestContextSource");
    const incomingRank = sourceRank(incomingResource, "referrerSource", REQUEST_CONTEXT_SOURCE_RANK, "requestContextSource");
    return !nonEmpty(existing) || incomingRank >= existingRank
      ? { value: incoming, source: incomingSource }
      : { value: existing, source: existingSource };
  }

  function mergeRequestHeaders(existingResource, incomingResource) {
    const existingHeaders = existingResource?.requestHeaders || {};
    const incomingHeaders = incomingResource?.requestHeaders || {};
    const existingRank = sourceRank(existingResource, "headerSource", REQUEST_CONTEXT_SOURCE_RANK, "requestContextSource");
    const incomingRank = sourceRank(incomingResource, "headerSource", REQUEST_CONTEXT_SOURCE_RANK, "requestContextSource");
    return incomingRank >= existingRank
      ? { ...existingHeaders, ...incomingHeaders }
      : { ...existingHeaders };
  }

  function preferText(incoming, existing) {
    return nonEmpty(incoming) ? incoming : existing || "";
  }

  function mergeCapturedResource(existingResource, incomingResource) {
    const existing = existingResource || {};
    const incoming = incomingResource || {};
    const filename = chooseFilename(existing, incoming);
    const existingCapturedAt = Number(existing.capturedAt || 0);
    const incomingCapturedAt = Number(incoming.capturedAt || 0);
    const existingMetadataRank = sourceRank(existing, "metadataSource", METADATA_SOURCE_RANK);
    const incomingMetadataRank = sourceRank(incoming, "metadataSource", METADATA_SOURCE_RANK);
    const pageUrl = chooseContextField(existing, incoming, "pageUrl");
    const referer = chooseContextField(existing, incoming, "referer");

    return {
      ...existing,
      ...incoming,
      id: preferText(incoming.id, existing.id),
      url: preferText(incoming.url, existing.url),
      blobUrl: preferText(incoming.blobUrl, existing.blobUrl),
      pageKey: preferText(incoming.pageKey, existing.pageKey),
      pageTitle: preferText(incoming.pageTitle, existing.pageTitle),
      pageUrl: pageUrl.value,
      referer: referer.value,
      filename: filename.filename,
      filenameSource: filename.filenameSource,
      metadataSource: incomingMetadataRank >= existingMetadataRank
        ? incoming.metadataSource || Sources.metadata.inferred
        : existing.metadataSource || Sources.metadata.inferred,
      headerSource: chooseHeaderSource(existing, incoming),
      referrerSource: referer.value ? referer.source : pageUrl.source,
      mime: chooseMime(existing, incoming),
      size: chooseSize(existing, incoming),
      width: Math.max(Number(existing.width || 0), Number(incoming.width || 0)),
      height: Math.max(Number(existing.height || 0), Number(incoming.height || 0)),
      supportsRange: Boolean(existing.supportsRange || incoming.supportsRange),
      mediaHint: incoming.mediaHint || incoming.assetHint || existing.mediaHint || existing.assetHint || null,
      assetHint: incoming.assetHint || existing.assetHint || null,
      requestHeaders: mergeRequestHeaders(existing, incoming),
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

  function projectResourceForStorage(resource) {
    const {
      requestHeaders: _requestHeaders,
      referer: _referer,
      pageUrl: _pageUrl,
      pageTitle: _pageTitle,
      pageKey: _pageKey,
      ...persistent
    } = resource || {};
    return persistent;
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

  function buildPageCaptureContext(browserHeaders, tabUrl, payloadHref) {
    const headers = { ...(browserHeaders || {}) };
    const browserReferrer = String(headers.referer || tabUrl || "").trim();
    const pageReferrer = String(payloadHref || "").trim();
    const referer = browserReferrer || pageReferrer;
    return {
      headers,
      referer,
      pageUrl: referer,
      headerSource: Object.keys(headers).length > 0
        ? Sources.header.browser
        : Sources.header.inferred,
      referrerSource: browserReferrer
        ? Sources.referrer.browser
        : (pageReferrer ? Sources.referrer.page : Sources.referrer.inferred)
    };
  }

  return {
    Sources,
    buildPageCaptureContext,
    mergeCapturedResource,
    projectResourceForPopup,
    projectResourceForStorage,
    uniqueCapturedResources
  };
});
