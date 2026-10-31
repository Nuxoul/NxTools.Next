"use strict";

// Request header capture, cookies, redirects and Content-Disposition names.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

function browserAcceptLanguageHeader() {
  const languages = Array.isArray(globalThis.navigator?.languages)
    ? globalThis.navigator.languages.filter(Boolean).slice(0, 4)
    : [];
  if (languages.length === 0 && globalThis.navigator?.language) {
    languages.push(globalThis.navigator.language);
  }

  return languages
    .map((language, index) => index === 0 ? language : `${language};q=${Math.max(0.5, 1 - index * 0.1).toFixed(1)}`)
    .join(",");
}

function defaultHeadersForUrl(url) {
  const headers = {};
  const userAgent = globalThis.navigator?.userAgent || "";
  const acceptLanguage = browserAcceptLanguageHeader();
  if (userAgent) {
    headers["user-agent"] = userAgent;
  }
  headers.accept = "*/*";
  if (acceptLanguage) {
    headers["accept-language"] = acceptLanguage;
  }

  if (isGoogleVideoUrl(url)) {
    headers.origin = "https://www.youtube.com";
    headers["sec-fetch-dest"] = "video";
    headers["sec-fetch-mode"] = "no-cors";
    headers["sec-fetch-site"] = "cross-site";
  }

  return headers;
}

function uniqueHttpUrls(urls) {
  const seen = new Set();
  const result = [];
  for (const url of urls || []) {
    const normalized = normalizeUrlKey(url);
    if (!isHttpUrl(normalized) || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    result.push(normalized);
  }

  return result;
}

function youtubeCookieFallbackUrls(url, contextUrl = "") {
  const urls = [url, contextUrl];
  if (isGoogleVideoUrl(url) || /(^|\.)youtube\.com$/i.test(parseUrlOrNull(contextUrl)?.hostname || "")) {
    urls.push("https://www.youtube.com/");
    urls.push("https://youtube.com/");
    urls.push("https://m.youtube.com/");
  }

  return uniqueHttpUrls(urls);
}

async function cookiePairsForUrl(url) {
  if (!isHttpUrl(url) || !chrome.cookies?.getAll) {
    return [];
  }

  try {
    const cookies = await chrome.cookies.getAll({ url });
    return (cookies || [])
      .filter((cookie) => cookie?.name && cookie.value != null)
      .sort((left, right) => String(right.path || "").length - String(left.path || "").length)
      .map((cookie) => `${cookie.name}=${cookie.value}`);
  } catch {
    return [];
  }
}

async function cookieHeaderForContexts(url, contextUrl = "") {
  const pairs = [];
  const seen = new Set();
  for (const candidateUrl of youtubeCookieFallbackUrls(url, contextUrl)) {
    for (const pair of await cookiePairsForUrl(candidateUrl)) {
      if (!pair || seen.has(pair)) {
        continue;
      }

      seen.add(pair);
      pairs.push(pair);
    }
  }

  return pairs.join("; ");
}

async function enrichHeadersForDownload(url, headers, settings, contextUrl = "") {
  const merged = {
    ...defaultHeadersForUrl(url),
    ...(headers || {})
  };

  if (settings.includeCookies && !merged.cookie) {
    const cookie = await cookieHeaderForContexts(url, contextUrl || merged.referer || "");
    if (cookie) {
      merged.cookie = cookie;
    }
  }

  return merged;
}

// Decodes an RFC 5987 extended value: charset'lang'percent-encoded-bytes.
function decodeRfc5987Value(value) {
  const raw = String(value || "").trim().replace(/^["']|["']$/g, "");
  const match = /^([^']*)'[^']*'(.*)$/.exec(raw);
  const charset = (match?.[1] || "utf-8").toLowerCase();
  const encoded = match ? match[2] : raw;

  if (!charset || charset === "utf-8" || charset === "utf8") {
    try {
      return decodeURIComponent(encoded);
    } catch {
      return encoded;
    }
  }

  try {
    const bytes = [];
    encoded.replace(/%([0-9a-fA-F]{2})|(.)/g, (_, hex, ch) => {
      bytes.push(hex ? parseInt(hex, 16) : ch.charCodeAt(0));
      return "";
    });
    return new TextDecoder(charset).decode(new Uint8Array(bytes));
  } catch {
    try {
      return decodeURIComponent(encoded);
    } catch {
      return encoded;
    }
  }
}

// Parses a Content-Disposition header into a filename, following RFC 6266 priority:
// filename* (RFC 5987 extended, charset-aware) wins over a plain filename=.
function parseContentDispositionFilename(disposition) {
  const raw = String(disposition || "");
  if (!raw) {
    return "";
  }

  const extended = /filename\*\s*=\s*([^;]+)/i.exec(raw);
  if (extended?.[1]) {
    const cleaned = cleanFileName(decodeRfc5987Value(extended[1]));
    if (cleaned) {
      return cleaned;
    }
  }

  const plain = /filename\s*=\s*(?:"([^"]*)"|'([^']*)'|([^;]+))/i.exec(raw);
  const value = plain?.[1] ?? plain?.[2] ?? plain?.[3];
  if (value != null) {
    let decoded = String(value).trim();
    if (/%[0-9a-fA-F]{2}/.test(decoded)) {
      try {
        decoded = decodeURIComponent(decoded);
      } catch {
        // Keep the raw value if it is not valid percent-encoding.
      }
    }
    const cleaned = cleanFileName(decoded);
    if (cleaned) {
      return cleaned;
    }
  }

  return "";
}

// Remembers a Content-Disposition filename for a URL and propagates it back across
// any known redirect hops, so every send path can recover the authoritative name.
function rememberContentDisposition(url, fileName) {
  const cleaned = cleanFileName(fileName);
  if (!cleaned) {
    return;
  }

  let key = normalizeUrlKey(url);
  const seen = new Set();
  let hops = 0;
  while (key && !seen.has(key) && hops < 6) {
    seen.add(key);
    contentDispositionByUrl.set(key, cleaned);
    key = redirectSourceByTarget.get(key) || "";
    hops += 1;
  }

  while (contentDispositionByUrl.size > REQUEST_CACHE_LIMIT) {
    contentDispositionByUrl.delete(contentDispositionByUrl.keys().next().value);
  }
  persistDispositionNames();
}

function dispositionFileNameForUrl(url) {
  const key = normalizeUrlKey(url);
  if (!key) return "";
  return contentDispositionByUrl.get(key) || restoredDispositionNames.get(dispositionStorageKey(key)) || "";
}

// Authoritative Content-Disposition names survive MV3 worker suspension through
// chrome.storage.session (memory-only, extension-private, cleared with the
// browser session). Keys are a one-way hash of the normalized URL so signed
// query strings are never written; no header, cookie or referrer is stored.
const STORAGE_SESSION_DISPOSITION_KEY = "nxtools_disposition_names";
const DISPOSITION_SESSION_LIMIT = 200;
const restoredDispositionNames = new Map();
let dispositionRestorePromise = null;

function sessionStorageArea() {
  return chrome.storage?.session && typeof chrome.storage.session.set === "function"
    ? chrome.storage.session
    : null;
}

function dispositionStorageKey(normalizedUrl) {
  // cyrb53: fast non-cryptographic 53-bit hash; collisions only weaken a name hint.
  let h1 = 0xdeadbeef;
  let h2 = 0x41c6ce57;
  for (let index = 0; index < normalizedUrl.length; index += 1) {
    const code = normalizedUrl.charCodeAt(index);
    h1 = Math.imul(h1 ^ code, 2654435761);
    h2 = Math.imul(h2 ^ code, 1597334677);
  }
  h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
  h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
  return (4294967296 * (2097151 & h2) + (h1 >>> 0)).toString(36);
}

function persistDispositionNames() {
  const area = sessionStorageArea();
  if (!area) return;
  const merged = new Map(restoredDispositionNames);
  for (const [url, name] of contentDispositionByUrl) {
    const key = dispositionStorageKey(url);
    merged.delete(key);
    merged.set(key, name);
  }
  const entries = [...merged].slice(-DISPOSITION_SESSION_LIMIT);
  void Promise.resolve(area.set({ [STORAGE_SESSION_DISPOSITION_KEY]: entries })).catch(() => {
    // Best-effort hint cache; a lost write only means the browser-provided name is used.
  });
}

function ensureDispositionNamesRestored() {
  if (!dispositionRestorePromise) {
    const area = sessionStorageArea();
    dispositionRestorePromise = !area
      ? Promise.resolve()
      : Promise.resolve(area.get(STORAGE_SESSION_DISPOSITION_KEY)).then((stored) => {
          const entries = Array.isArray(stored?.[STORAGE_SESSION_DISPOSITION_KEY])
            ? stored[STORAGE_SESSION_DISPOSITION_KEY]
            : [];
          for (const entry of entries.slice(-DISPOSITION_SESSION_LIMIT)) {
            if (Array.isArray(entry) && typeof entry[0] === "string") {
              const name = cleanFileName(entry[1]);
              if (name && !restoredDispositionNames.has(entry[0])) {
                restoredDispositionNames.set(entry[0], name);
              }
            }
          }
        }, () => {});
  }
  return dispositionRestorePromise;
}

function normalizeHeaders(headers) {
  const result = {};
  for (const header of headers || []) {
    const name = String(header.name || "").trim();
    const key = name.toLowerCase();
    const value = String(header.value || "").trim();
    if (!name || !value ||
        name.length > MAX_CAPTURED_HEADER_NAME_LENGTH ||
        value.length > MAX_CAPTURED_HEADER_VALUE_LENGTH ||
        /[\r\n]/.test(name) || /[\r\n]/.test(value) ||
        !USEFUL_HEADER_ALLOWLIST.has(key)) {
      continue;
    }
    result[key] = value;
    if (Object.keys(result).length >= MAX_CAPTURED_HEADER_COUNT) {
      break;
    }
  }
  return result;
}

function boundedCapturedText(value, maxLength = MAX_CAPTURED_TEXT_LENGTH) {
  const text = String(value || "").trim();
  return text.length <= maxLength ? text : text.slice(0, maxLength);
}

function boundedCapturedUrl(value) {
  const url = String(value || "").trim();
  return url.length <= MAX_CAPTURED_URL_LENGTH ? url : "";
}

function boundedCapturedNumber(value) {
  const number = Number(value || 0);
  return Number.isSafeInteger(number) && number > 0 ? number : 0;
}

function sanitizeCapturedHeaderObject(headers) {
  const result = {};
  for (const [rawName, rawValue] of Object.entries(headers || {})) {
    const name = String(rawName || "").trim().toLowerCase();
    const value = String(rawValue || "").trim();
    if (!name || !value ||
        name.length > MAX_CAPTURED_HEADER_NAME_LENGTH ||
        value.length > MAX_CAPTURED_HEADER_VALUE_LENGTH ||
        /[\r\n]/.test(name) || /[\r\n]/.test(value) ||
        !USEFUL_HEADER_ALLOWLIST.has(name)) {
      continue;
    }
    result[name] = value;
    if (Object.keys(result).length >= MAX_CAPTURED_HEADER_COUNT) {
      break;
    }
  }
  return result;
}

function headerObjectToPayload(headers, settings) {
  if (!settings.includeRequestHeaders) {
    return { headerText: "", cookie: "" };
  }

  const lines = [];
  let cookie = "";
  for (const [name, value] of Object.entries(headers || {})) {
    if (!value) {
      continue;
    }
    if (name.toLowerCase() === "cookie") {
      if (settings.includeCookies) {
        cookie = value;
      }
      continue;
    }
    lines.push(`${name}: ${value}`);
  }

  return {
    headerText: lines.join("\n"),
    cookie
  };
}

function cleanupRequestCache() {
  const now = Date.now();
  for (const [key, value] of recentRequestsByUrl) {
    if (now - value.time > RECENT_REQUEST_TTL_MS) {
      recentRequestsByUrl.delete(key);
    }
  }

  for (const [key, value] of recentRequestsByRequestId) {
    if (now - value.time > RECENT_REQUEST_TTL_MS) {
      recentRequestsByRequestId.delete(key);
    }
  }

  for (const [key, value] of redirectHintsByRequestId) {
    if (now - value.time > RECENT_REQUEST_TTL_MS) {
      redirectHintsByRequestId.delete(key);
    }
  }

  for (const [key, value] of redirectHintsByUrl) {
    if (now - value.time > RECENT_REQUEST_TTL_MS) {
      redirectHintsByUrl.delete(key);
    }
  }

  while (recentRequestsByUrl.size > REQUEST_CACHE_LIMIT) {
    recentRequestsByUrl.delete(recentRequestsByUrl.keys().next().value);
  }

  while (recentRequestsByRequestId.size > REQUEST_CACHE_LIMIT) {
    recentRequestsByRequestId.delete(recentRequestsByRequestId.keys().next().value);
  }

  while (redirectHintsByRequestId.size > REQUEST_CACHE_LIMIT) {
    redirectHintsByRequestId.delete(redirectHintsByRequestId.keys().next().value);
  }

  while (redirectHintsByUrl.size > REQUEST_CACHE_LIMIT) {
    redirectHintsByUrl.delete(redirectHintsByUrl.keys().next().value);
  }

  while (redirectSourceByTarget.size > REQUEST_CACHE_LIMIT) {
    redirectSourceByTarget.delete(redirectSourceByTarget.keys().next().value);
  }
}

function rememberRequestHeaders(details) {
  if (!isHttpUrl(details.url) || !Array.isArray(details.requestHeaders)) {
    return;
  }

  const headers = normalizeHeaders(details.requestHeaders);
  if (Object.keys(headers).length === 0) {
    return;
  }

  const snapshot = {
    requestId: details.requestId,
    tabId: details.tabId > 0 ? details.tabId : lastActiveTabId || 0,
    tabIdObserved: details.tabId > 0,
    url: normalizeUrlKey(details.url),
    initiator: details.initiator || details.originUrl || "",
    time: Date.now(),
    headers,
    supportsRange: Object.keys(headers).some((name) => name === "range")
  };

  recentRequestsByUrl.set(snapshot.url, snapshot);
  recentRequestsByRequestId.set(details.requestId, snapshot);
  cleanupRequestCache();
}

function responseHeaderValue(headers, name) {
  const target = name.toLowerCase();
  for (const header of headers || []) {
    if (String(header.name || "").toLowerCase() === target) {
      return String(header.value || "").trim();
    }
  }
  return "";
}

function assetTypeHintFromHeaders(headers) {
  const typeId = responseHeaderValue(headers, "roblox-assettypeid") ||
    responseHeaderValue(headers, "x-asset-type-id") ||
    responseHeaderValue(headers, "x-assettypeid");
  return ASSET_TYPE_MEDIA_HINTS[String(typeId || "").trim()] || null;
}

function rememberRedirectHint(details) {
  if (!isHttpUrl(details?.url)) {
    return;
  }

  const location = responseHeaderValue(details.responseHeaders, "location");
  const assetHint = assetTypeHintFromHeaders(details.responseHeaders);
  const meta = responseMeta(details);
  if (!location && !assetHint && !meta.mime && !meta.filename) {
    return;
  }

  let targetUrl = "";
  if (location) {
    try {
      targetUrl = new URL(location, details.url).toString();
    } catch {
      targetUrl = location;
    }
  }

  const snapshot = resolveHeaderSnapshot(details.url);
  const hint = {
    requestId: details.requestId || "",
    sourceUrl: normalizeUrlKey(details.url),
    targetUrl: targetUrl ? normalizeUrlKey(targetUrl) : "",
    tabId: details.tabId > 0 ? details.tabId : snapshot?.tabId || lastActiveTabId || 0,
    initiator: details.initiator || details.originUrl || snapshot?.initiator || "",
    time: Date.now(),
    assetHint: assetHint || meta.assetHint || null,
    mime: assetHint?.mime || meta.mime || "",
    filename: meta.filename || "",
    size: Number(meta.size || 0),
    supportsRange: Boolean(meta.supportsRange || snapshot?.supportsRange),
    headers: { ...(snapshot?.headers || {}) }
  };

  if (hint.requestId) {
    redirectHintsByRequestId.set(hint.requestId, hint);
  }
  if (hint.sourceUrl) {
    redirectHintsByUrl.set(hint.sourceUrl, hint);
  }
  if (hint.targetUrl) {
    redirectHintsByUrl.set(hint.targetUrl, hint);
    if (hint.sourceUrl && hint.targetUrl !== hint.sourceUrl) {
      redirectSourceByTarget.set(hint.targetUrl, hint.sourceUrl);
    }
    // If the disposition is already known (e.g. the redirect itself carried it),
    // propagate it to the source URL the user actually clicked.
    if (hint.filename) {
      rememberContentDisposition(hint.targetUrl, hint.filename);
    }
  }
  cleanupRequestCache();
}

function resolveRedirectHint(details) {
  cleanupRequestCache();
  if (details?.requestId && redirectHintsByRequestId.has(details.requestId)) {
    return redirectHintsByRequestId.get(details.requestId);
  }

  const normalized = normalizeUrlKey(details?.url || "");
  if (normalized && redirectHintsByUrl.has(normalized)) {
    return redirectHintsByUrl.get(normalized);
  }

  return null;
}

function responseMeta(details) {
  const contentType = responseHeaderValue(details.responseHeaders, "content-type").split(";")[0].trim().toLowerCase();
  const disposition = responseHeaderValue(details.responseHeaders, "content-disposition");
  const lengthText = responseHeaderValue(details.responseHeaders, "content-length");
  const rangeText = responseHeaderValue(details.responseHeaders, "content-range");
  const acceptRanges = responseHeaderValue(details.responseHeaders, "accept-ranges");
  const assetHint = assetTypeHintFromHeaders(details.responseHeaders);
  const filename = parseContentDispositionFilename(disposition);
  if (filename && isHttpUrl(details.url)) {
    rememberContentDisposition(details.url, filename);
  }

  let size = Number.parseInt(lengthText, 10);
  if (!Number.isFinite(size) || size < 0) {
    size = 0;
  }

  const rangeSize = Number.parseInt(rangeText.split("/")[1] || "", 10);
  if (Number.isFinite(rangeSize) && rangeSize > 0) {
    size = rangeSize;
  }

  return {
    mime: assetHint?.mime || contentType,
    filename,
    size,
    supportsRange: details.statusCode === 206 || /bytes/i.test(acceptRanges) || Boolean(rangeText),
    assetHint
  };
}

function resolveHeaderSnapshot(url) {
  cleanupRequestCache();
  const normalized = normalizeUrlKey(url);
  if (recentRequestsByUrl.has(normalized)) {
    return recentRequestsByUrl.get(normalized);
  }

  let best = null;
  let bestScore = 0;
  for (const snapshot of recentRequestsByUrl.values()) {
    const score = headerSnapshotScore(url, snapshot);
    if (score <= 0) {
      continue;
    }
    if (!best || score > bestScore || (score === bestScore && snapshot.time > best.time)) {
      best = snapshot;
      bestScore = score;
    }
  }
  return best;
}

function registerHeaderCapture() {
  const filter = { urls: ["http://*/*", "https://*/*"] };
  const handleRequestHeaders = (details) => {
    rememberRequestHeaders(details);
    void captureRequestResource(details);
  };

  try {
    chrome.webRequest.onBeforeSendHeaders.addListener(
      handleRequestHeaders,
      filter,
      ["requestHeaders", "extraHeaders"]
    );
  } catch {
    chrome.webRequest.onBeforeSendHeaders.addListener(
      handleRequestHeaders,
      filter,
      ["requestHeaders"]
    );
  }

  chrome.webRequest.onResponseStarted.addListener(
    (details) => {
      void captureNetworkResource(details);
    },
    { urls: ["http://*/*", "https://*/*"] },
    ["responseHeaders"]
  );

  const handleRedirect = (details) => {
    rememberRedirectHint(details);
  };

  if (chrome.webRequest.onBeforeRedirect?.addListener) {
    try {
      chrome.webRequest.onBeforeRedirect.addListener(
        handleRedirect,
        filter,
        ["responseHeaders", "extraHeaders"]
      );
    } catch {
      chrome.webRequest.onBeforeRedirect.addListener(
        handleRedirect,
        filter,
        ["responseHeaders"]
      );
    }
  }

}
