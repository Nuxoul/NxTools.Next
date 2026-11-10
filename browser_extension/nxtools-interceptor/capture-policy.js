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
      poster: preferText(incoming.poster, existing.poster),
      supportsRange: Boolean(existing.supportsRange || incoming.supportsRange),
      discovery: mergeDiscovery(existing.discovery, incoming.discovery),
      mediaElement: mergeMediaElement(existing.mediaElement, incoming.mediaElement),
      mediaHint: incoming.mediaHint || incoming.assetHint || existing.mediaHint || existing.assetHint || null,
      assetHint: incoming.assetHint || existing.assetHint || null,
      requestHeaders: mergeRequestHeaders(existing, incoming),
      pendingBatchHandoffId: preferText(
        incoming.pendingBatchHandoffId,
        existing.pendingBatchHandoffId
      ),
      capturedAt: Math.max(existingCapturedAt, incomingCapturedAt) || Date.now()
    };
  }

  function projectResourceForPopup(resource) {
    const {
      requestHeaders: _requestHeaders,
      cookieContext: _cookieContext,
      referer: _referer,
      pageUrl: _pageUrl,
      pendingBatchHandoffId: _pendingBatchHandoffId,
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
      cookieContext: _cookieContext,
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

  function normalizeDiscovery(value, observedMime = "") {
    const mime = String(observedMime).split(";")[0].trim().toLowerCase();
    return { fragment: value?.fragment === true || /^(?:video|audio)\/iso\.segment$/.test(mime), attachment: value?.attachment === true,
      manifest: value?.manifest === true || /^(?:application\/(?:vnd\.apple\.mpegurl|x-mpegurl|dash\+xml)|audio\/(?:x-)?mpegurl)$/.test(mime) };
  }

  function mergeDiscovery(existing, incoming) {
    const left = normalizeDiscovery(existing);
    const right = normalizeDiscovery(incoming);
    return { fragment: left.fragment || right.fragment, attachment: left.attachment || right.attachment, manifest: left.manifest || right.manifest };
  }

  function normalizeMediaElement(value) {
    if (value?.kind !== "audio" && value?.kind !== "video") return null;
    return { kind: value.kind, duration: typeof value.duration === "number" && Number.isFinite(value.duration) && value.duration > 0 ? value.duration : null,
      controls: value.controls === true, autoplay: value.autoplay === true, loop: value.loop === true, presentation: value.presentation === true };
  }

  function mergeMediaElement(existing, incoming) {
    const left = normalizeMediaElement(existing);
    const right = normalizeMediaElement(incoming);
    if (!right || (left?.kind === "video" && right.kind !== "video") ||
        (left?.controls && !right.controls && right.kind !== "video")) return left;
    return right;
  }

  function discoveryCandidateReason(resource) {
    const facts = resource?.discovery;
    const manifest = /\.(?:m3u8|mpd)(?:[?#]|$)/i.test(resource?.url || "");
    if (facts?.manifest || manifest || facts?.attachment || /^blob:/i.test(resource?.url || "")) return "";
    const media = normalizeMediaElement(resource?.mediaElement);
    if (media?.kind === "audio" && media.duration !== null && media.duration <= 2 && media.autoplay &&
        !media.controls && !media.loop && media.presentation) return "decorative_audio";
    if (media) return "";
    return facts?.fragment === true ? "fragment" : "";
  }

  return {
    Sources,
    discoveryCandidateReason,
    normalizeDiscovery,
    normalizeMediaElement,
    buildPageCaptureContext,
    mergeCapturedResource,
    projectResourceForPopup,
    projectResourceForStorage,
    uniqueCapturedResources
  };
});
