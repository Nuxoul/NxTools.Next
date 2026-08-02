"use strict";

if (typeof importScripts === "function") {
  importScripts("native-transport.js", "handoff-policy.js");
}

const nativeTransport = new NxToolsNativeTransport.NativeTransport(chrome.runtime);

const DEFAULT_SETTINGS = {
  interceptDownloads: true,
  openDialog: true,
  eraseBrowserHistory: true,
  includeRequestHeaders: true,
  includeCookies: true,
  captureResources: true,
  mediaOverlayEnabled: true,
  dropTargetEnabled: true,
  dropTargetPosition: "right-bottom",
  dropTargetWidth: 188,
  dropTargetHeight: 86
};

const RECENT_REQUEST_TTL_MS = 180000;
const RESOURCE_TTL_MS = 20 * 60 * 1000;
const REQUEST_CACHE_LIMIT = 800;
const RESOURCE_LIMIT_PER_TAB = 120;
const RESOURCE_SCAN_INTERVAL_MS = 3500;
const EXTENSION_LOG_THROTTLE_MS = 12000;
const GOOGLEVIDEO_MIN_REMAINING_MS = 30 * 1000;
const GOOGLEVIDEO_MAX_FUTURE_MS = 14 * 24 * 60 * 60 * 1000;
const STORAGE_RESOURCE_KEY = "nxtools_resource_cache";
const STORAGE_LAST_TAB_KEY = "nxtools_last_active_tab";
const STORAGE_MOBILE_UA_TABS_KEY = "nxtools_mobile_ua_tabs";
const STORAGE_DISABLED_SITES_KEY = "nxtools_disabled_sites";
const PREEMPTED_DOWNLOAD_TTL_MS = 90 * 1000;

const USEFUL_HEADER_ALLOWLIST = new Set([
  "accept",
  "accept-language",
  "authorization",
  "cache-control",
  "cookie",
  "origin",
  "pragma",
  "priority",
  "range",
  "referer",
  "sec-ch-ua",
  "sec-ch-ua-arch",
  "sec-ch-ua-bitness",
  "sec-ch-ua-full-version",
  "sec-ch-ua-full-version-list",
  "sec-ch-ua-mobile",
  "sec-ch-ua-model",
  "sec-ch-ua-platform",
  "sec-ch-ua-platform-version",
  "sec-fetch-dest",
  "sec-fetch-mode",
  "sec-fetch-site",
  "user-agent"
]);

const HLS_TYPES = new Set([
  "application/vnd.apple.mpegurl",
  "application/x-mpegurl",
  "application/mpegurl",
  "application/octet-stream-m3u8"
]);

const MPD_TYPES = new Set(["application/dash+xml"]);
const VIDEO_EXTENSIONS = new Set(["3g2", "3gp", "asf", "avi", "cmfv", "divx", "f4f", "f4m", "f4v", "flv", "gif", "hlv", "ismv", "m2t", "m2ts", "m4s", "m4v", "mkv", "mov", "mp2t", "mp4", "mp4v", "mpeg", "mpeg4", "movie", "mpg", "ogv", "smil", "ts", "vid", "webm", "wmv"]);
const AUDIO_EXTENSIONS = new Set(["aac", "acc", "ac3", "adts", "aif", "aiff", "alac", "ape", "cmfa", "f4a", "f4b", "flac", "m4s", "m4a", "mp3", "mp4a", "mpa", "oga", "ogg", "opus", "wav", "weba", "wma"]);
const MEDIA_EXTENSIONS = new Set([...VIDEO_EXTENSIONS, ...AUDIO_EXTENSIONS, "m3u", "m3u8", "mpd"]);
const IMAGE_EXTENSIONS = new Set(["avif", "bmp", "gif", "jpeg", "jpg", "png", "svg", "webp"]);
const DOWNLOAD_EXTENSIONS = new Set([
  "7z", "apk", "app", "bat", "bin", "bz2", "csv", "deb", "dmg", "doc", "docx", "exe", "gz",
  "iso", "jar", "msi", "pdf", "pkg", "ppt", "pptx", "rar", "rpm", "sh", "tar", "tgz", "txt",
  "xls", "xlsx", "xz", "zip"
]);
const MIME_EXTENSIONS = {
  "application/dash+xml": "mpd",
  "application/mpegurl": "m3u8",
  "application/vnd.apple.mpegurl": "m3u8",
  "application/x-mpegurl": "m3u8",
  "audio/aac": "aac",
  "audio/flac": "flac",
  "audio/mp4": "m4a",
  "audio/mpeg": "mp3",
  "audio/ogg": "ogg",
  "audio/wav": "wav",
  "audio/webm": "webm",
  "video/mp2t": "ts",
  "video/mp4": "mp4",
  "video/quicktime": "mov",
  "video/webm": "webm",
  "video/x-flv": "flv",
  "video/x-m4v": "m4v",
  "video/x-ms-wmv": "wmv",
  "image/avif": "avif",
  "image/bmp": "bmp",
  "image/gif": "gif",
  "image/jpeg": "jpg",
  "image/jpg": "jpg",
  "image/png": "png",
  "image/svg+xml": "svg",
  "image/webp": "webp",
  "application/zip": "zip",
  "application/x-zip-compressed": "zip",
  "application/x-7z-compressed": "7z",
  "application/x-rar-compressed": "rar",
  "application/vnd.rar": "rar",
  "application/gzip": "gz",
  "application/x-gzip": "gz",
  "application/x-bzip2": "bz2",
  "application/x-xz": "xz",
  "application/x-tar": "tar",
  "application/x-msdownload": "exe",
  "application/vnd.microsoft.portable-executable": "exe",
  "application/x-msi": "msi",
  "application/x-apple-diskimage": "dmg",
  "application/vnd.android.package-archive": "apk",
  "application/x-debian-package": "deb",
  "application/x-deb": "deb",
  "application/x-rpm": "rpm",
  "application/x-iso9660-image": "iso",
  "application/x-bittorrent": "torrent",
  "application/java-archive": "jar",
  "application/pdf": "pdf",
  "application/msword": "doc",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document": "docx",
  "application/vnd.ms-excel": "xls",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": "xlsx",
  "application/vnd.ms-powerpoint": "ppt",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation": "pptx",
  "application/epub+zip": "epub",
  "application/rtf": "rtf",
  "application/json": "json",
  "application/xml": "xml",
  "text/plain": "txt",
  "text/csv": "csv",
  "text/markdown": "md"
};

const ASSET_TYPE_MEDIA_HINTS = {
  "1": { kind: "image", mime: "image/*", extension: "png", label: "image" },
  "3": { kind: "audio", mime: "audio/*", extension: "ogg", label: "audio" },
  "34": { kind: "download", mime: "application/octet-stream", extension: "bin", label: "asset" },
  "62": { kind: "video", mime: "video/*", extension: "mp4", label: "video" }
};

const MOBILE_USER_AGENT =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1";

const recentRequestsByUrl = new Map();
const recentRequestsByRequestId = new Map();
const redirectHintsByRequestId = new Map();
const redirectHintsByUrl = new Map();
// Authoritative Content-Disposition filenames remembered per normalized URL, plus
// a redirect target→source map so a disposition seen on the final hop (e.g. the
// codeload.github.com 200) can be applied back to the URL the user actually clicked.
const contentDispositionByUrl = new Map();
const redirectSourceByTarget = new Map();
const preemptedBrowserDownloads = new Map();
const resourcesByTab = new Map();
const resourcesById = new Map();
let lastActiveTabId = 0;
let persistTimer = 0;
let desktopConnectPromise = null;
let connectionState = "disconnected";
let connectionMessage = "未连接 NxTools";
let lastSnapshot = null;
const mobileUserAgentTabs = new Set();
const extensionLogTimes = new Map();
const selectedMediaIndexByTab = new Map();
const lastFrameScanByTab = new Map();
const disabledSites = new Set();
let cachedSettings = { ...DEFAULT_SETTINGS };

const LIKELY_MEDIA_URL_PATTERN = /(?:bilivideo|hdslb|upos-|mcdn|googlevideo|videoplayback|douyinvod|bytecdn|weibocdn|akamaized|cloudfront|dash|hls|segment|fragment|frag|chunk|playurl|play_url|playback|stream|manifest|playlist|media-video|media-audio|mime=video|mime=audio|mime_type=video|mime_type=audio|video_mp4|audio_mp4)/i;
const BINARY_DOWNLOAD_EXTENSIONS = new Set([
  "7z", "apk", "app", "bin", "bz2", "deb", "dmg", "doc", "docx", "exe", "gz", "iso",
  "jar", "msi", "pkg", "ppt", "pptx", "rar", "rpm", "tar", "tgz", "xls", "xlsx", "xz", "zip"
]);
const FORCED_DOWNLOAD_MIME_TYPES = new Set([
  "application/gzip",
  "application/java-archive",
  "application/msword",
  "application/octet-stream",
  "application/vnd.android.package-archive",
  "application/vnd.apple.installer+xml",
  "application/vnd.ms-excel",
  "application/vnd.ms-powerpoint",
  "application/vnd.microsoft.portable-executable",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/x-7z-compressed",
  "application/x-apple-diskimage",
  "application/x-bittorrent",
  "application/x-bzip2",
  "application/x-deb",
  "application/x-gzip",
  "application/x-iso9660-image",
  "application/x-msdownload",
  "application/x-msi",
  "application/x-rar-compressed",
  "application/x-rpm",
  "application/x-tar",
  "application/x-xz",
  "application/x-zip-compressed",
  "application/zip"
]);

function parseUrlOrNull(url) {
  try {
    return new URL(url);
  } catch {
    return null;
  }
}

function normalizeUrlKey(url, allowBlob = false) {
  const raw = String(url || "").trim();
  if (!raw) {
    return "";
  }
  if (allowBlob && raw.startsWith("blob:")) {
    return raw;
  }
  try {
    const parsed = new URL(raw);
    parsed.hash = "";
    return parsed.toString();
  } catch {
    return raw.split("#", 1)[0] || raw;
  }
}

function normalizeResourceUrlKey(url) {
  const raw = String(url || "").trim();
  if (!raw) {
    return "";
  }
  if (raw.startsWith("blob:")) {
    return raw;
  }

  try {
    const parsed = new URL(raw);
    parsed.hash = "";
    return parsed.toString();
  } catch {
    return raw.split("#", 1)[0] || raw;
  }
}

function isHttpUrl(url) {
  return /^https?:\/\//i.test(String(url || ""));
}

function isGoogleVideoUrl(url) {
  const parsed = parseUrlOrNull(url);
  const host = parsed?.hostname.toLowerCase() || "";
  return host === "googlevideo.com" || host.endsWith(".googlevideo.com");
}

function googleVideoInvalidReason(url) {
  const parsed = parseUrlOrNull(url);
  if (!parsed || !isGoogleVideoUrl(parsed.toString())) {
    return "";
  }

  const expireText = parsed.searchParams.get("expire") || "";
  if (!expireText) {
    return "";
  }

  const expireMs = Number(expireText) * 1000;
  if (!Number.isFinite(expireMs) || expireMs <= 0) {
    return "invalid_expire";
  }

  const now = Date.now();
  if (expireMs <= now + GOOGLEVIDEO_MIN_REMAINING_MS) {
    return "expired";
  }

  if (expireMs >= now + GOOGLEVIDEO_MAX_FUTURE_MS) {
    return "expire_out_of_range";
  }

  return "";
}

function isUsableGoogleVideoUrl(url) {
  return !googleVideoInvalidReason(url);
}

function googleVideoIdentity(url) {
  const parsed = parseUrlOrNull(url);
  if (!parsed || !isGoogleVideoUrl(parsed.toString())) {
    return null;
  }

  return {
    origin: parsed.origin,
    pathname: parsed.pathname,
    id: parsed.searchParams.get("id") || "",
    itag: parsed.searchParams.get("itag") || "",
    mime: parsed.searchParams.get("mime") || parsed.searchParams.get("mime_type") || "",
    clen: parsed.searchParams.get("clen") || ""
  };
}

function googleVideoSnapshotScore(targetUrl, snapshotUrl) {
  const target = googleVideoIdentity(targetUrl);
  const candidate = googleVideoIdentity(snapshotUrl);
  if (!target || !candidate || target.origin !== candidate.origin || target.pathname !== candidate.pathname) {
    return 0;
  }

  for (const key of ["id", "itag"]) {
    if (target[key] && candidate[key] && target[key] !== candidate[key]) {
      return 0;
    }
  }

  let score = 600;
  for (const [key, weight] of [["id", 140], ["itag", 100], ["mime", 40], ["clen", 20]]) {
    if (target[key] && candidate[key] && target[key] === candidate[key]) {
      score += weight;
    }
  }
  return score;
}

function headerSnapshotScore(targetUrl, snapshot) {
  const snapshotUrl = snapshot?.url || "";
  if (!targetUrl || !snapshotUrl) {
    return 0;
  }

  if (normalizeUrlKey(targetUrl) === snapshotUrl) {
    return 10_000;
  }

  const googleVideoScore = googleVideoSnapshotScore(targetUrl, snapshotUrl);
  if (googleVideoScore > 0) {
    return googleVideoScore + (snapshot.supportsRange ? 10 : 0);
  }

  try {
    const parsedTarget = new URL(targetUrl);
    const parsedSnapshot = new URL(snapshotUrl);
    if (parsedSnapshot.origin === parsedTarget.origin && parsedSnapshot.pathname === parsedTarget.pathname) {
      return 100 + (snapshot.supportsRange ? 10 : 0);
    }
  } catch {
    return 0;
  }

  return 0;
}

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

function getFileNameFromPath(path) {
  const normalized = String(path || "").replace(/\\/g, "/");
  return normalized.split("/").filter(Boolean).pop()?.trim() || "";
}

function cleanFileName(value) {
  return String(value || "")
    .trim()
    .replace(/[<>:"/\\|?*\x00-\x1f]+/g, " ")
    .replace(/\s+/g, " ")
    .replace(/[. ]+$/g, "")
    .trim()
    .slice(0, 160);
}

function getFileNameFromUrl(url) {
  try {
    const pathname = new URL(url).pathname;
    // Strip matrix parameters (";a=b") then take the last non-empty segment.
    const segment = pathname.split("/").filter(Boolean).pop() || "";
    const withoutMatrix = segment.split(";", 1)[0] || segment;
    return cleanFileName(decodeURIComponent(withoutMatrix));
  } catch {
    return "";
  }
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
}

function dispositionFileNameForUrl(url) {
  const key = normalizeUrlKey(url);
  return key ? contentDispositionByUrl.get(key) || "" : "";
}

function getFileExtension(value) {
  let name = String(value || "").trim();
  try {
    if (/^(https?:|blob:)/i.test(name)) {
      name = new URL(name).pathname;
    }
  } catch {
    // Fall back to the raw text below.
  }

  name = name.split(/[?#]/, 1)[0];
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
}

function mimeFromUrl(url) {
  try {
    const params = new URL(url).searchParams;
    const mime = (params.get("mime") || params.get("mime_type") || "").replace(/_/g, "/").toLowerCase();
    return mime && isMediaResource("", mime, url) ? mime : "";
  } catch {
    return "";
  }
}

function extensionFromMime(mime) {
  const normalized = String(mime || "").split(";", 1)[0].trim().toLowerCase();
  if (normalized === "image/*") {
    return "png";
  }
  if (normalized === "audio/*") {
    return "ogg";
  }
  if (normalized === "video/*") {
    return "mp4";
  }
  return MIME_EXTENSIONS[normalized] || "";
}

function wildcardMimeForElementKind(kind) {
  switch (String(kind || "").toLowerCase()) {
    case "audio":
      return "audio/*";
    case "video":
      return "video/*";
    case "image":
      return "image/*";
    default:
      return "";
  }
}

function isGenericResourceName(name) {
  const cleaned = cleanFileName(name);
  const stem = cleaned.replace(/\.[^.]+$/i, "").toLowerCase();
  return !cleaned ||
    /^(resource|download|video|audio|media|file|blob|unknown|index)$/i.test(stem) ||
    /^(64位|32位|x64|x86|下载|立即下载|高速下载|普通下载|安装包|最新版)$/i.test(stem);
}

function isWeakResourceTitle(name) {
  const cleaned = cleanFileName(name);
  const stem = cleaned.replace(/\.[^.]+$/i, "");
  if (isGenericResourceName(cleaned)) {
    return true;
  }

  return /^[\u4e00-\u9fff\d\s_-]{1,4}$/u.test(stem);
}

const YOUTUBE_ITAG_INFO = {
  18: { height: 360, format: "MP4", kind: "video", progressive: true },
  22: { height: 720, format: "MP4", kind: "video", progressive: true },
  37: { height: 1080, format: "MP4", kind: "video", progressive: true },
  38: { height: 3072, format: "MP4", kind: "video", progressive: true },
  133: { height: 240, format: "MP4", kind: "video" },
  134: { height: 360, format: "MP4", kind: "video" },
  135: { height: 480, format: "MP4", kind: "video" },
  136: { height: 720, format: "MP4", kind: "video" },
  137: { height: 1080, format: "MP4", kind: "video" },
  138: { height: 2160, format: "MP4", kind: "video" },
  160: { height: 144, format: "MP4", kind: "video" },
  242: { height: 240, format: "WebM", kind: "video" },
  243: { height: 360, format: "WebM", kind: "video" },
  244: { height: 480, format: "WebM", kind: "video" },
  247: { height: 720, format: "WebM", kind: "video" },
  248: { height: 1080, format: "WebM", kind: "video" },
  271: { height: 1440, format: "WebM", kind: "video" },
  272: { height: 2160, format: "WebM", kind: "video" },
  278: { height: 144, format: "WebM", kind: "video" },
  298: { height: 720, format: "MP4", kind: "video" },
  299: { height: 1080, format: "MP4", kind: "video" },
  302: { height: 720, format: "WebM", kind: "video" },
  303: { height: 1080, format: "WebM", kind: "video" },
  308: { height: 1440, format: "WebM", kind: "video" },
  313: { height: 2160, format: "WebM", kind: "video" },
  315: { height: 2160, format: "WebM", kind: "video" },
  394: { height: 144, format: "MP4 AV1", kind: "video" },
  395: { height: 240, format: "MP4 AV1", kind: "video" },
  396: { height: 360, format: "MP4 AV1", kind: "video" },
  397: { height: 480, format: "MP4 AV1", kind: "video" },
  398: { height: 720, format: "MP4 AV1", kind: "video" },
  399: { height: 1080, format: "MP4 AV1", kind: "video" },
  400: { height: 1440, format: "MP4 AV1", kind: "video" },
  401: { height: 2160, format: "MP4 AV1", kind: "video" },
  139: { format: "M4A", kind: "audio" },
  140: { format: "M4A", kind: "audio" },
  141: { format: "M4A", kind: "audio" },
  249: { format: "WebM Opus", kind: "audio" },
  250: { format: "WebM Opus", kind: "audio" },
  251: { format: "WebM Opus", kind: "audio" },
  256: { format: "M4A", kind: "audio" },
  258: { format: "M4A", kind: "audio" }
};

function youtubeItagInfoFromUrl(url) {
  try {
    const itag = Number(new URL(url).searchParams.get("itag") || 0);
    return Number.isFinite(itag) ? YOUTUBE_ITAG_INFO[itag] || null : null;
  } catch {
    return null;
  }
}

function resourceResolutionText(resource) {
  const width = Number(resource?.width || 0);
  const height = Number(resource?.height || 0);
  if (width > 0 && height > 0) {
    return `${width}x${height}`;
  }

  const youtubeInfo = youtubeItagInfoFromUrl(resource?.url || "");
  return youtubeInfo?.height ? `${youtubeInfo.height}p` : "";
}

function extensionForResource(resource) {
  const direct = getFileExtension(resource?.filename || "") || getFileExtension(resource?.url || "");
  if (direct) {
    return direct;
  }

  const assetHintExtension = resource?.assetHint?.extension || resource?.mediaHint?.extension || "";
  if (assetHintExtension) {
    return assetHintExtension;
  }

  const mime = resource?.mime || mimeFromUrl(resource?.url || "");
  const fromMime = extensionFromMime(mime);
  if (fromMime) {
    return fromMime;
  }

  return isGoogleVideoUrl(resource?.url || "") ? "mp4" : "";
}

function buildResourceFileName(resource) {
  const info = describeResource(resource || {});
  const extension = extensionForResource(resource) || info.extension || "";
  const existing = cleanFileName(resource?.filename || "");
  const existingExtension = getFileExtension(existing);
  const urlFileName = cleanFileName(getFileNameFromUrl(resource?.url || ""));
  if (existing && existingExtension && !isWeakResourceTitle(existing)) {
    return existing;
  }
  if (urlFileName && getFileExtension(urlFileName) && !isGenericResourceName(urlFileName)) {
    return urlFileName;
  }

  const resolution = resourceResolutionText(resource);
  let stem = existing && !isWeakResourceTitle(existing)
    ? existing
    : cleanFileName(resource?.pageTitle || "");

  if (!stem || isGenericResourceName(stem)) {
    stem = info.kind === "audio"
      ? "audio"
      : info.kind === "streaming"
        ? "playlist"
        : info.kind === "video"
          ? "video"
          : info.kind === "image"
            ? "image"
          : "resource";
  }

  if (resolution && !stem.includes(resolution)) {
    stem = `${stem} ${resolution}`;
  }

  return extension ? `${stem}.${extension}` : stem;
}

function ensureFileNameForUrl(fileName, url, mime = "") {
  const cleaned = cleanFileName(fileName);
  const urlFileName = cleanFileName(getFileNameFromUrl(url));
  const extension = getFileExtension(cleaned) ||
    getFileExtension(url) ||
    extensionFromMime(mime) ||
    extensionFromMime(mimeFromUrl(url)) ||
    (isGoogleVideoUrl(url) ? "mp4" : "");

  if (cleaned && getFileExtension(cleaned) && !isWeakResourceTitle(cleaned)) {
    return cleaned;
  }
  if (urlFileName && getFileExtension(urlFileName) && !isGenericResourceName(urlFileName)) {
    return urlFileName;
  }

  const stem = cleaned && !isWeakResourceTitle(cleaned)
    ? cleaned
    : urlFileName || "resource";

  if (!extension || getFileExtension(stem)) {
    return stem;
  }

  return `${stem}.${extension}`;
}

function domainFromUrl(url) {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

function siteHostKey(url) {
  const parsed = parseUrlOrNull(url);
  return parsed ? parsed.hostname.toLowerCase() : "";
}

// Resolves the first usable hostname from a prioritized list of candidate URLs
// (page initiator first, the downloaded URL last). Used to decide which site a
// page-originated request belongs to.
function firstSiteHostKey(...urls) {
  for (const url of urls) {
    const host = siteHostKey(url);
    if (host) {
      return host;
    }
  }
  return "";
}

// True unless the resolved site host has been explicitly disabled by the user.
// Fails open: when no host can be resolved, the request is treated as enabled.
function isSiteEnabled(...urls) {
  const host = firstSiteHostKey(...urls);
  return host ? !disabledSites.has(host) : true;
}

async function setSiteEnabled(host, enabled) {
  const key = String(host || "").trim().toLowerCase();
  if (!key) {
    return false;
  }

  if (enabled) {
    disabledSites.delete(key);
  } else {
    disabledSites.add(key);
  }

  await chrome.storage.local.set({ [STORAGE_DISABLED_SITES_KEY]: [...disabledSites] });
  return true;
}

function loadDisabledSites(list) {
  disabledSites.clear();
  for (const host of Array.isArray(list) ? list : []) {
    const key = String(host || "").trim().toLowerCase();
    if (key) {
      disabledSites.add(key);
    }
  }
}

function storageGet(defaults) {
  return chrome.storage.local.get(defaults);
}

function actionApi() {
  return chrome.action || chrome.browserAction || null;
}

async function getSettings() {
  const stored = await storageGet(DEFAULT_SETTINGS);
  cachedSettings = {
    ...DEFAULT_SETTINGS,
    ...stored
  };
  return cachedSettings;
}

function setBadge(text, color) {
  const action = actionApi();
  action?.setBadgeText?.({ text });
  action?.setBadgeBackgroundColor?.({ color });
}

function trimLogText(value, maxLength = 1200) {
  const text = String(value ?? "").replace(/\s+/g, " ").trim();
  return text.length <= maxLength ? text : `${text.slice(0, Math.max(0, maxLength - 3))}...`;
}

function sendExtensionLog(level, event, message, details = {}, throttleMs = EXTENSION_LOG_THROTTLE_MS) {
  const key = `${level}:${event}:${details.reason || ""}`;
  const now = Date.now();
  const last = extensionLogTimes.get(key) || 0;
  if (throttleMs > 0 && now - last < throttleMs) {
    return;
  }
  extensionLogTimes.set(key, now);

  const safeDetails = {};
  for (const name of [
    "source", "tabId", "frameId", "kind", "mime", "extension", "count", "statusCode",
    "size", "supportsRange", "headerCount", "cookieIncluded", "queued", "reason"
  ]) {
    if (details[name] !== undefined) {
      safeDetails[name] = name === "reason" ? trimLogText(details[name], 240) : details[name];
    }
  }
  const payload = {
    level,
    area: "BrowserExtension",
    event,
    message: trimLogText(message, 240),
    ...safeDetails
  };
  void nativeTransport.request("extension_log", payload, { timeoutMs: 2500 }).catch(() => {});
}

function resourceSamples(resources, limit = 5) {
  return (resources || [])
    .filter((resource) => isUsableGoogleVideoUrl(resource?.url || ""))
    .slice(0, limit)
    .map((resource) => resource.url || "")
    .filter(Boolean);
}

function logResourceCaptured(event, resource, extra = {}) {
  const info = describeResource(resource);
  sendExtensionLog("info", event, "捕获浏览器媒体/下载资源", {
    source: resource.source || extra.source || "",
    url: resource.url,
    pageUrl: resource.pageUrl || "",
    tabId: resource.tabId || 0,
    kind: info.kind,
    mime: resource.mime || "",
    extension: info.extension || "",
    count: resourcesByTab.get(resource.tabId)?.size || 0,
    statusCode: extra.statusCode || 0
  });
}

async function ensureDesktopConnection(force = false) {
  if (!force && desktopConnectPromise) {
    return desktopConnectPromise;
  }
  connectionState = "connecting";
  connectionMessage = "正在连接 NxTools Native Messaging";
  desktopConnectPromise = nativeTransport.request("ping", {}, { timeoutMs: 2500 })
    .then((result) => {
      connectionState = "connected";
      connectionMessage = "Native Messaging 已连接 NxTools";
      desktopConnectPromise = null;
      return result;
    }, (error) => {
      connectionState = "disconnected";
      connectionMessage = error instanceof Error ? error.message : "无法连接 NxTools Native Messaging";
      desktopConnectPromise = null;
      throw error;
    });
  return desktopConnectPromise;
}

async function sendDesktopRequest(type, payload = {}, timeoutMs = 12000, options = {}) {
  const idempotencyKey = options.idempotencyKey || payload.idempotencyKey || undefined;
  if (type === "create_task" && payload.payload?.blobBase64) {
    const download = { ...payload.payload };
    const blobBase64 = download.blobBase64;
    delete download.blobBase64;
    return nativeTransport.stageBlob(
      download,
      blobBase64,
      idempotencyKey || `blob:${chrome.runtime.id}:${crypto.randomUUID()}`
    );
  }
  return nativeTransport.request(type, payload, { timeoutMs, idempotencyKey });
}

function normalizeHeaders(headers) {
  const result = {};
  for (const header of headers || []) {
    const name = String(header.name || "").trim();
    const key = name.toLowerCase();
    const value = String(header.value || "").trim();
    if (!name || !value || !USEFUL_HEADER_ALLOWLIST.has(key)) {
      continue;
    }
    result[key] = value;
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

  for (const [key, value] of preemptedBrowserDownloads) {
    if (now - value.time > PREEMPTED_DOWNLOAD_TTL_MS) {
      preemptedBrowserDownloads.delete(key);
    }
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

function dispositionRequestsDownload(disposition) {
  return /(?:^|;)\s*attachment(?:;|$)/i.test(String(disposition || ""));
}

function isLikelyBrowserDownloadResponse(details, meta) {
  if (!isHttpUrl(details?.url) || !meta || details.statusCode < 200 || details.statusCode >= 400) {
    return false;
  }

  const disposition = responseHeaderValue(details.responseHeaders, "content-disposition");
  if (dispositionRequestsDownload(disposition)) {
    return true;
  }

  const extension = getFileExtension(meta.filename || getFileNameFromUrl(details.url) || details.url) ||
    extensionFromMime(meta.mime) ||
    extensionFromMime(mimeFromUrl(details.url));
  if (BINARY_DOWNLOAD_EXTENSIONS.has(extension)) {
    return true;
  }

  const type = String(meta.mime || "").toLowerCase();
  if (FORCED_DOWNLOAD_MIME_TYPES.has(type)) {
    return true;
  }

  return false;
}

function rememberPreemptedBrowserDownload(details, meta) {
  cleanupRequestCache();
  const key = normalizeUrlKey(details.url);
  if (!key) {
    return;
  }

  preemptedBrowserDownloads.set(key, {
    requestId: details.requestId,
    time: Date.now(),
    url: key,
    filename: meta?.filename || getFileNameFromUrl(details.url),
    size: Number(meta?.size || 0),
    supportsRange: Boolean(meta?.supportsRange)
  });
}

function wasPreemptedBrowserDownload(url) {
  cleanupRequestCache();
  return preemptedBrowserDownloads.has(normalizeUrlKey(url));
}

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

  const headerSnapshot = resolveHeaderSnapshot(details.url);
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

function bucketForTab(tabId) {
  const id = Number(tabId) || lastActiveTabId || 0;
  if (!resourcesByTab.has(id)) {
    resourcesByTab.set(id, new Map());
  }
  return resourcesByTab.get(id);
}

function cacheResource(resource) {
  if (!resource || !resource.id || !resource.url) {
    return;
  }
  if (!isUsableGoogleVideoUrl(resource.url)) {
    return;
  }

  const bucket = bucketForTab(resource.tabId);
  const existing = bucket.get(resource.id);
  const wasNew = !existing;
  const merged = {
    ...existing,
    ...resource,
    filename: resource.filename || existing?.filename || getFileNameFromUrl(resource.url) || "",
    mime: resource.mime || existing?.mime || "",
    size: resource.size > 0 ? resource.size : existing?.size || 0,
    mediaHint: resource.mediaHint || resource.assetHint || existing?.mediaHint || existing?.assetHint || null,
    assetHint: resource.assetHint || existing?.assetHint || null,
    width: Number(resource.width || 0) > 0 ? Number(resource.width || 0) : Number(existing?.width || 0),
    height: Number(resource.height || 0) > 0 ? Number(resource.height || 0) : Number(existing?.height || 0),
    supportsRange: Boolean(resource.supportsRange || existing?.supportsRange),
    requestHeaders: {
      ...(existing?.requestHeaders || {}),
      ...(resource.requestHeaders || {})
    },
    capturedAt: Math.max(resource.capturedAt || Date.now(), existing?.capturedAt || 0)
  };

  bucket.set(merged.id, merged);
  resourcesById.set(merged.id, merged);

  const normalizedId = `${merged.tabId}:${normalizeResourceUrlKey(merged.url)}`;
  if (normalizedId && normalizedId !== merged.id) {
    const sameUrl = bucket.get(normalizedId);
    if (!sameUrl || Number(merged.capturedAt || 0) >= Number(sameUrl.capturedAt || 0)) {
      bucket.set(normalizedId, merged);
      resourcesById.set(normalizedId, merged);
    }
  }

  if (wasNew ||
      (resource.mime && resource.mime !== existing?.mime) ||
      (Number(resource.size || 0) > Number(existing?.size || 0))) {
    logResourceCaptured("resource_cached", merged, {
      source: resource.source || "cache"
    });
  }

  const mergedUrl = normalizeResourceUrlKey(merged.url);
  if (mergedUrl && (merged.size > 0 || merged.mime || merged.supportsRange)) {
    for (const cached of resourcesById.values()) {
      if (cached.id === merged.id || normalizeResourceUrlKey(cached.url) !== mergedUrl) {
        continue;
      }
      cached.filename = cached.filename && !isGenericResourceName(cached.filename) ? cached.filename : merged.filename;
      cached.mime = merged.mime || cached.mime;
      cached.size = merged.size > 0 ? merged.size : cached.size;
      cached.mediaHint = merged.mediaHint || cached.mediaHint || null;
      cached.assetHint = merged.assetHint || cached.assetHint || null;
      cached.width = merged.width > 0 ? merged.width : cached.width;
      cached.height = merged.height > 0 ? merged.height : cached.height;
      cached.supportsRange = Boolean(cached.supportsRange || merged.supportsRange);
      cached.requestHeaders = {
        ...(cached.requestHeaders || {}),
        ...(merged.requestHeaders || {})
      };
    }
  }

  const sorted = [...bucket.values()].sort((left, right) => right.capturedAt - left.capturedAt);
  for (const stale of sorted.slice(RESOURCE_LIMIT_PER_TAB)) {
    bucket.delete(stale.id);
    resourcesById.delete(stale.id);
  }
  schedulePersistResources();
}

function pruneResources() {
  const now = Date.now();
  for (const [tabId, bucket] of resourcesByTab) {
    for (const [id, resource] of bucket) {
      if (now - resource.capturedAt > RESOURCE_TTL_MS || !isUsableGoogleVideoUrl(resource.url)) {
        bucket.delete(id);
        resourcesById.delete(id);
      }
    }
    if (bucket.size === 0) {
      resourcesByTab.delete(tabId);
    }
  }
}

function serializeResources() {
  pruneResources();
  const result = {};
  for (const [tabId, bucket] of resourcesByTab) {
    result[String(tabId)] = [...bucket.values()]
      .sort((left, right) => right.capturedAt - left.capturedAt)
      .slice(0, RESOURCE_LIMIT_PER_TAB);
  }
  return result;
}

function schedulePersistResources() {
  if (persistTimer) {
    return;
  }

  persistTimer = setTimeout(() => {
    persistTimer = 0;
    chrome.storage.local.set({
      [STORAGE_RESOURCE_KEY]: serializeResources(),
      [STORAGE_LAST_TAB_KEY]: lastActiveTabId
    });
  }, 600);
}

async function loadPersistentResources() {
  const stored = await chrome.storage.local.get({
    [STORAGE_RESOURCE_KEY]: {},
    [STORAGE_LAST_TAB_KEY]: 0,
    [STORAGE_MOBILE_UA_TABS_KEY]: [],
    [STORAGE_DISABLED_SITES_KEY]: []
  });

  lastActiveTabId = Number(stored[STORAGE_LAST_TAB_KEY] || 0);
  loadDisabledSites(stored[STORAGE_DISABLED_SITES_KEY]);
  mobileUserAgentTabs.clear();
  for (const tabId of stored[STORAGE_MOBILE_UA_TABS_KEY] || []) {
    if (Number.isInteger(tabId) && tabId > 0) {
      mobileUserAgentTabs.add(tabId);
      void updateMobileUserAgentRule(tabId, true);
    }
  }

  resourcesByTab.clear();
  resourcesById.clear();
  for (const [tabIdText, resources] of Object.entries(stored[STORAGE_RESOURCE_KEY] || {})) {
    const tabId = Number(tabIdText);
    if (!Number.isInteger(tabId) || tabId <= 0 || !Array.isArray(resources)) {
      continue;
    }
    const bucket = bucketForTab(tabId);
    for (const resource of resources) {
      if (!resource?.id || !resource?.url || !isUsableGoogleVideoUrl(resource.url)) {
        continue;
      }
      bucket.set(resource.id, resource);
      resourcesById.set(resource.id, resource);
    }
  }
  pruneResources();
}

function mobileRuleId(tabId) {
  return 100000 + Number(tabId || 0);
}

function updateMobileUserAgentRule(tabId, enabled) {
  const declarativeNetRequestApi = chrome["declarative" + "NetRequest"];
  if (!declarativeNetRequestApi?.updateSessionRules || !tabId || tabId <= 0) {
    return Promise.resolve();
  }

  const ruleId = mobileRuleId(tabId);
  return new Promise((resolve, reject) => {
    declarativeNetRequestApi.updateSessionRules(
      enabled
        ? {
            removeRuleIds: [ruleId],
            addRules: [
              {
                id: ruleId,
                priority: 1,
                action: {
                  type: "modifyHeaders",
                  requestHeaders: [
                    {
                      header: "User-Agent",
                      operation: "set",
                      value: MOBILE_USER_AGENT
                    }
                  ]
                },
                condition: {
                  tabIds: [tabId],
                  resourceTypes: Object.values(declarativeNetRequestApi.ResourceType)
                }
              }
            ]
          }
        : {
            removeRuleIds: [ruleId]
          },
      () => {
        const lastError = chrome.runtime.lastError;
        if (lastError) {
          reject(new Error(lastError.message || "更新 User-Agent 规则失败"));
          return;
        }
        resolve();
      }
    );
  });
}

async function persistFeatureState() {
  await chrome.storage.local.set({
    [STORAGE_MOBILE_UA_TABS_KEY]: [...mobileUserAgentTabs]
  });
}

function featureStatesForTab(tabId, settings) {
  return {
    mediaOverlay: settings.mediaOverlayEnabled !== false,
    deepSearch: settings.captureResources !== false,
    mobileUserAgent: Boolean(tabId && mobileUserAgentTabs.has(tabId))
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

async function activeTab() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0] || null;
  if (tab?.id) {
    lastActiveTabId = tab.id;
    schedulePersistResources();
  }
  return tab;
}

async function tabInfo(tabId) {
  if (!tabId || tabId <= 0) {
    return null;
  }
  try {
    return await chrome.tabs.get(tabId);
  } catch {
    return null;
  }
}

function urlsLikelySamePage(left, right) {
  const normalizedLeft = normalizeUrlKey(left);
  const normalizedRight = normalizeUrlKey(right);
  if (!normalizedLeft || !normalizedRight) {
    return false;
  }
  return normalizedLeft === normalizedRight || normalizedLeft.startsWith(normalizedRight) || normalizedRight.startsWith(normalizedLeft);
}

async function resolveTabIdFromPageUrl(pageUrl) {
  const normalizedPageUrl = normalizeUrlKey(pageUrl);
  if (!normalizedPageUrl) {
    return 0;
  }

  const tabs = await chrome.tabs.query({});
  const exactMatch = tabs.find((tab) => tab.id && tab.url && urlsLikelySamePage(tab.url, normalizedPageUrl));
  if (exactMatch?.id) {
    return exactMatch.id;
  }

  try {
    const pageOrigin = new URL(normalizedPageUrl).origin;
    const originMatch = tabs.find((tab) => {
      if (!tab.id || !tab.url) {
        return false;
      }
      try {
        return new URL(tab.url).origin === pageOrigin;
      } catch {
        return false;
      }
    });
    return originMatch?.id || 0;
  } catch {
    return 0;
  }
}

async function resolveTabIdForRequest(details) {
  if (details.tabId > 0) {
    return details.tabId;
  }

  const snapshotTabId = resolveHeaderSnapshot(details.url)?.tabId || 0;
  if (snapshotTabId > 0) {
    return snapshotTabId;
  }

  const initiator = details.initiator && details.initiator !== "null" ? details.initiator : "";
  if (initiator) {
    const matchedTabId = await resolveTabIdFromPageUrl(initiator);
    if (matchedTabId > 0) {
      return matchedTabId;
    }
  }

  const current = await activeTab();
  return current?.id || lastActiveTabId || 0;
}

function tabFrames(tabId) {
  return new Promise((resolve) => {
    if (!chrome.webNavigation?.getAllFrames || !tabId || tabId <= 0) {
      resolve([{ frameId: 0 }]);
      return;
    }

    chrome.webNavigation.getAllFrames({ tabId }, (frames) => {
      const lastError = chrome.runtime.lastError;
      if (lastError || !Array.isArray(frames) || frames.length === 0) {
        resolve([{ frameId: 0 }]);
        return;
      }
      resolve(frames.map((frame) => ({ frameId: frame.frameId })));
    });
  });
}

function sendMessageToTab(tabId, message, options = {}) {
  return new Promise((resolve) => {
    if (!tabId || tabId <= 0) {
      resolve({ ok: false, status: "no_tab", message: "当前没有可操作的标签页" });
      return;
    }

    const sendOptions = Number.isInteger(options.frameId) ? { frameId: options.frameId } : undefined;
    const callback = (response) => {
      const lastError = chrome.runtime.lastError;
      if (lastError) {
        resolve({ ok: false, status: "no_receiver", message: lastError.message || "页面脚本未响应" });
        return;
      }
      resolve({ ok: true, response });
    };

    if (sendOptions) {
      chrome.tabs.sendMessage(tabId, message, sendOptions, callback);
    } else {
      chrome.tabs.sendMessage(tabId, message, callback);
    }
  });
}

async function sendMessageToTabFrames(tabId, message) {
  const frames = await tabFrames(tabId);
  const results = await Promise.all(frames.map((frame) => sendMessageToTab(tabId, message, frame)
    .then((result) => ({ ...result, frameId: frame.frameId }))));
  return results;
}

function injectContentBridge(tabId, pageUrl = "") {
  return new Promise((resolve) => {
    if (!chrome.scripting?.executeScript || !tabId || tabId <= 0) {
      resolve(false);
      return;
    }

    chrome.scripting.executeScript({
      target: { tabId, allFrames: true },
      files: ["content-script.js"],
      injectImmediately: false
    }, () => {
      const lastError = chrome.runtime.lastError;
      if (lastError) {
        sendExtensionLog("warn", "content_bridge_inject_failed", "页面媒体桥接脚本注入失败", {
          reason: lastError.message || "executeScript failed",
          pageUrl,
          tabId
        }, 30000);
        resolve(false);
        return;
      }

      resolve(true);
    });
  });
}

async function scanTabForResources(tabId, force = false) {
  if (!tabId || tabId <= 0) {
    return [];
  }

  const now = Date.now();
  const lastScanAt = lastFrameScanByTab.get(tabId) || 0;
  if (!force && now - lastScanAt < RESOURCE_SCAN_INTERVAL_MS) {
    return [];
  }

  lastFrameScanByTab.set(tabId, now);
  await injectContentBridge(tabId);
  const frames = await tabFrames(tabId);
  await Promise.all(frames.map((frame) => injectPageHook(tabId, frame.frameId).catch(() => false)));
  const results = await Promise.all(frames.map((frame) => sendMessageToTab(tabId, { type: "content_scan_now" }, frame)
    .then((result) => ({ ...result, frameId: frame.frameId }))));

  const okCount = results.filter((result) => result.ok).length;
  if (okCount === 0) {
    sendExtensionLog("warn", "page_scan_no_receiver", "页面资源扫描没有收到内容脚本响应", {
      reason: "content_script_unavailable",
      tabId,
      frameCount: frames.length
    }, 30000);
  }

  return results;
}

function injectPageHook(tabId, frameId = 0, pageUrl = "") {
  return new Promise((resolve) => {
    if (!chrome.scripting?.executeScript || !tabId || tabId <= 0) {
      resolve(false);
      return;
    }

    chrome.scripting.executeScript({
      target: { tabId, frameIds: [Number(frameId) || 0] },
      files: ["page-hook.js"],
      world: "MAIN",
      injectImmediately: true
    }, () => {
      const lastError = chrome.runtime.lastError;
      if (lastError) {
        sendExtensionLog("warn", "page_hook_inject_failed", "页面媒体 hook 注入失败", {
          reason: lastError.message || "executeScript failed",
          pageUrl,
          tabId,
          frameId
        }, 30000);
        resolve(false);
        return;
      }

      resolve(true);
    });
  });
}

function createMediaLabel(source, type, frameId, localIndex, resolution = "") {
  const name = getFileNameFromUrl(source) || String(source || "").split("/").pop() || "";
  const kind = type === "audio" ? "音频" : "视频";
  const frameText = frameId > 0 ? ` · frame ${frameId}` : "";
  const resolutionText = resolution && type !== "audio" ? ` · ${resolution}` : "";
  return `${(name || `${kind} ${localIndex + 1}`).slice(0, 52)}${resolutionText}${frameText}`.slice(0, 86);
}

async function getMediaState(tabId, index = 0) {
  const results = await sendMessageToTabFrames(tabId, {
    Message: "getVideoState",
    index: 0
  });

  const mediaFrames = results
    .filter((item) => item.ok && Number(item.response?.count || 0) > 0)
    .map((item) => ({
      frameId: item.frameId || 0,
      state: item.response,
      activeScore: Number(item.response?.activeScore || 0),
      activeIndex: Math.max(0, Number(item.response?.activeIndex || 0))
    }));

  if (mediaFrames.length === 0) {
    const result = results.find((item) => item.ok) || results[0] || { ok: false, message: "页面脚本未响应", frameId: 0 };
    return {
      mediaItems: [],
      mediaPlaybackState: {
        available: false,
        message: result.message || "当前页面未检测到可控制媒体",
        tabId: tabId || null,
        frameId: result.frameId || 0,
        mediaIndex: -1,
        currentTime: 0,
        duration: 0,
        progress: 0,
        volume: 1,
        paused: true,
        loop: false,
        muted: false,
        speed: 1
      }
    };
  }

  const mediaItems = [];
  for (const frame of mediaFrames) {
    const sources = Array.isArray(frame.state?.src) ? frame.state.src : [];
    const types = Array.isArray(frame.state?.types) ? frame.state.types : [];
    const widths = Array.isArray(frame.state?.widths) ? frame.state.widths : [];
    const heights = Array.isArray(frame.state?.heights) ? frame.state.heights : [];
    const resolutions = Array.isArray(frame.state?.resolutions) ? frame.state.resolutions : [];
    const count = Number(frame.state?.count || 0);
    for (let localIndex = 0; localIndex < count; localIndex += 1) {
      const source = sources[localIndex] || "";
      const type = types[localIndex] || "video";
      const width = Number(widths[localIndex] || 0);
      const height = Number(heights[localIndex] || 0);
      const resolution = String(resolutions[localIndex] || (width > 0 && height > 0 ? `${width}x${height}` : ""));
      mediaItems.push({
        index: mediaItems.length,
        frameId: frame.frameId,
        localIndex,
        source,
        width,
        height,
        resolution,
        label: createMediaLabel(source, type, frame.frameId, localIndex, resolution)
      });
    }
  }

  const activeFrame = [...mediaFrames].sort((left, right) => right.activeScore - left.activeScore)[0];
  const activeGlobalIndex = mediaItems.find((item) =>
    item.frameId === activeFrame.frameId &&
    item.localIndex === activeFrame.activeIndex)?.index ?? 0;
  const requestedIndex = Number.isInteger(index) && index >= 0 && index < mediaItems.length
    ? index
    : activeGlobalIndex;
  const selected = mediaItems[requestedIndex] || mediaItems[activeGlobalIndex] || mediaItems[0];
  let selectedState = mediaFrames.find((frame) => frame.frameId === selected.frameId)?.state;

  if (selected.localIndex !== 0) {
    const refreshed = await sendMessageToTab(tabId, {
      Message: "getVideoState",
      index: selected.localIndex
    }, { frameId: selected.frameId });
    if (refreshed.ok && refreshed.response) {
      selectedState = refreshed.response;
    }
  }

  const state = selectedState || {};
  return {
    mediaItems,
    mediaPlaybackState: {
      available: true,
      message: "",
      tabId,
      frameId: selected.frameId || 0,
      mediaIndex: selected.index,
      localMediaIndex: selected.localIndex,
      currentTime: Number(state.currentTime || 0),
      duration: Number(state.duration || 0),
      progress: Number(state.time || 0),
      volume: Number(state.volume ?? 1),
      paused: Boolean(state.paused),
      loop: Boolean(state.loop),
      muted: Boolean(state.muted),
      speed: Number(state.speed || 1),
      width: Number(selected.width || 0),
      height: Number(selected.height || 0),
      resolution: selected.resolution || ""
    }
  };
}

async function captureNetworkResource(details) {
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

  const tab = await tabInfo(resolvedTabId);
  const headerSnapshot = resolveHeaderSnapshot(details.url);
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
    url: details.url,
    pageTitle: tab?.title || "",
    pageUrl: details.initiator && details.initiator !== "null" ? details.initiator : redirectHint?.initiator || tab?.url || "",
    filename,
    mime: meta.mime,
    size: meta.size,
    mediaHint: meta.assetHint || null,
    supportsRange: meta.supportsRange || details.statusCode === 206 || Boolean(headerSnapshot?.supportsRange),
    referer,
    requestHeaders: headers,
    source: "webRequest-response",
    capturedAt: Date.now()
  });
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

  const tab = await tabInfo(tabId);
  const headerSnapshot = resolveHeaderSnapshot(details.url);
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
    url: details.url,
    pageTitle: tab?.title || "",
    pageUrl: details.initiator && details.initiator !== "null" ? details.initiator : tab?.url || "",
    filename,
    mime,
    size: 0,
    supportsRange: Boolean(headerSnapshot?.supportsRange),
    referer,
    requestHeaders: headers,
    source: "webRequest-request",
    capturedAt: Date.now()
  });
}

async function capturePageResource(sender, payload) {
  const settings = await getSettings();
  if (!settings.captureResources || !payload?.url || !/^(https?:|blob:)/i.test(payload.url)) {
    return;
  }
  if (!isSiteEnabled(sender.tab?.url, payload?.pageUrl, payload?.href)) {
    return;
  }

  const tabId = sender.tab?.id || lastActiveTabId || 0;
  if (!tabId) {
    return;
  }

  const tab = await tabInfo(tabId);
  const headerSnapshot = isHttpUrl(payload.url) ? resolveHeaderSnapshot(payload.url) : null;
  const headers = {
    ...(headerSnapshot?.headers || {}),
    ...(payload.requestHeaders || {})
  };
  const referer = headers.referer || payload.href || tab?.url || "";
  if (referer) {
    headers.referer = referer;
  }

  const extension = String(payload.ext || "").replace(/^\./, "").toLowerCase();
  const elementKindMime = wildcardMimeForElementKind(payload.elementKind);
  const payloadMime = String(payload.mime || elementKindMime || "").toLowerCase();
  const sourceUrl = String(payload.sourceUrl || "").trim();
  const sourceMime = String(payload.sourceMime || elementKindMime || "").toLowerCase();
  const primaryUrl = sourceUrl && isHttpUrl(sourceUrl) ? sourceUrl : payload.url;
  const frameId = Number.isInteger(sender.frameId) ? sender.frameId : Number(payload.frameId || 0);
  const rawFileName = cleanFileName(payload.filename) || getFileNameFromUrl(primaryUrl) || (extension ? `resource.${extension}` : "");
  const filename = ensureFileNameForUrl(
    rawFileName,
    primaryUrl,
    sourceMime || payloadMime);

  cacheResource({
    id: `${tabId}:${normalizeResourceUrlKey(payload.url)}`,
    tabId,
    frameId,
    url: payload.url,
    pageKey: String(payload.pageKey || ""),
    pageTitle: tab?.title || "",
    pageUrl: payload.href || tab?.url || "",
    filename,
    mime: payloadMime,
    size: Number(payload.size || 0),
    width: Number(payload.width || 0),
    height: Number(payload.height || 0),
    supportsRange: Boolean(headerSnapshot?.supportsRange),
    referer,
    requestHeaders: headers,
    source: String(payload.source || "content"),
    capturedAt: Date.now()
  });

  if (sourceUrl && isHttpUrl(sourceUrl)) {
    const sourceHeaderSnapshot = resolveHeaderSnapshot(sourceUrl);
    cacheResource({
      id: `${tabId}:${normalizeResourceUrlKey(sourceUrl)}`,
      tabId,
      frameId,
      url: sourceUrl,
      pageKey: String(payload.pageKey || ""),
      blobUrl: payload.url,
      pageTitle: tab?.title || "",
      pageUrl: payload.href || tab?.url || "",
      filename: ensureFileNameForUrl(filename || getFileNameFromUrl(sourceUrl), sourceUrl, sourceMime || payloadMime),
      mime: sourceMime || payloadMime,
      size: Number(payload.sourceSize || payload.size || 0),
      width: Number(payload.width || 0),
      height: Number(payload.height || 0),
      supportsRange: Boolean(sourceHeaderSnapshot?.supportsRange),
      referer,
      requestHeaders: {
        ...(sourceHeaderSnapshot?.headers || {}),
        ...(payload.requestHeaders || {})
      },
      source: `${String(payload.source || "content")}-source-url`,
      capturedAt: Date.now()
    });
  }
}

async function readBlobResourceViaContentScript(resource) {
  const tabId = Number(resource?.tabId || lastActiveTabId || 0);
  if (!tabId || !String(resource?.url || "").startsWith("blob:")) {
    return { ok: false, message: "Blob 资源只能在原网页中读取" };
  }

  const frameId = Number.isInteger(resource.frameId) ? resource.frameId : undefined;
  const message = {
    type: "content_read_blob",
    url: resource.url,
    filename: buildResourceFileName(resource)
  };
  let result = await sendMessageToTab(tabId, message, Number.isInteger(frameId) ? { frameId } : {});
  if (!(result.ok && result.response?.ok) && Number.isInteger(frameId)) {
    const frameResults = await sendMessageToTabFrames(tabId, message);
    result = frameResults.find((item) => item.ok && item.response?.ok) || result;
  }

  if (result.ok && result.response?.ok) {
    return result.response;
  }

  return {
    ok: false,
    message: result.response?.message || result.message || "页面脚本无法读取这个 Blob"
  };
}

async function sendBlobResourceViaContentScript(resource) {
  const readResult = await readBlobResourceViaContentScript(resource);
  const tabId = Number(resource?.tabId || lastActiveTabId || 0);
  const frameId = Number.isInteger(resource?.frameId) ? resource.frameId : undefined;
  if (readResult.ok) {
    const fileName = ensureFileNameForUrl(
      readResult.filename || buildResourceFileName(resource),
      resource.url,
      readResult.mime || resource.mime);
    const result = await addDownloadToNxTools({
      url: resource.url,
      fileName,
      size: Number(readResult.size || resource.size || 0),
      supportsRange: true,
      blobBase64: readResult.base64,
      blobMime: readResult.mime || resource.mime || "",
      headers: resource.requestHeaders || {}
    });
    resource.sentToDesktopAt = Date.now();
    schedulePersistResources();
    sendExtensionLog("info", "blob_handoff_desktop", "Blob 资源已读取并发送到 NxTools 解析窗口", {
      url: resource.url,
      tabId,
      frameId: Number.isInteger(frameId) ? frameId : -1,
      filename: fileName,
      size: Number(readResult.size || 0)
    }, 15000);
    return result;
  }

  sendExtensionLog("warn", "blob_download_failed", "Blob 资源无法直接交给 NxTools，且页面上下文下载失败", {
    url: resource.url,
    tabId,
    frameId: Number.isInteger(frameId) ? frameId : -1,
    reason: readResult.message || ""
  }, 0);
  return {
    ok: false,
    message: `${readResult.message || "页面脚本无法访问这个 Blob"}。Blob 链接不是可复用下载地址，需要在创建它的网页里读取。`
  };
}

async function addDownloadToNxTools(payload, options = {}) {
  const settings = await getSettings();
  return sendDesktopRequest("create_task", {
    payload: {
      source: "browser_extension",
      openDialog: settings.openDialog,
      startImmediately: !settings.openDialog,
      ...payload
    }
  }, 15000, options);
}

async function sendDownloadPayload({ url, fileName, headers = {}, size = 0, supportsRange = false, contextUrl = "", idempotencyKey = "" }) {
  const invalidGoogleVideoReason = googleVideoInvalidReason(url);
  if (invalidGoogleVideoReason) {
    sendExtensionLog("warn", "handoff_rejected", "已拒绝过期或异常的 YouTube 直链", {
      url,
      reason: invalidGoogleVideoReason
    }, 30000);
    return {
      ok: false,
      message: "捕获到的 YouTube 直链已过期，请让视频播放几秒后重新选择资源"
    };
  }

  const settings = await getSettings();
  // A Content-Disposition filename observed for this URL (or its redirect source)
  // is authoritative and overrides any URL-path-derived guess.
  const dispositionName = dispositionFileNameForUrl(url);
  const resolvedFileName = ensureFileNameForUrl(dispositionName || fileName, url);
  const googleVideoIdentityInfo = googleVideoIdentity(url);
  const resolvedSize = Number(size || 0) > 0
    ? Number(size || 0)
    : Number.parseInt(googleVideoIdentityInfo?.clen || "", 10) || 0;
  const enrichedHeaders = await enrichHeadersForDownload(url, headers, settings, contextUrl);
  const headerPayload = headerObjectToPayload(enrichedHeaders, settings);
  const headerNames = Object.keys(enrichedHeaders || {}).filter((name) => name.toLowerCase() !== "cookie");
  sendExtensionLog("info", "handoff_prepare", "准备发送资源到 NxTools", {
    url,
    fileName: resolvedFileName,
    size: resolvedSize,
    supportsRange: Boolean(supportsRange),
    headerCount: headerNames.length + (enrichedHeaders?.cookie ? 1 : 0),
    headerNames: headerNames.slice(0, 16).join(","),
    cookieIncluded: Boolean(enrichedHeaders?.cookie)
  }, 15000);

  try {
    const result = await addDownloadToNxTools({
      url,
      fileName: resolvedFileName,
      headers: enrichedHeaders,
      headerText: headerPayload.headerText,
      cookie: headerPayload.cookie,
      size: resolvedSize,
      supportsRange
    }, {
      idempotencyKey: idempotencyKey || `manual:${chrome.runtime.id}:${crypto.randomUUID()}`
    });
    const queued = Boolean(result?.data?.queued);
    sendExtensionLog(result?.ok === false || queued ? "warn" : "info", "handoff_result", result?.ok === false
      ? "资源发送到 NxTools 失败"
      : queued
        ? "NxTools 正在排队打开下载确认窗口"
        : "资源已发送到 NxTools", {
      url,
      reason: result?.message || "",
      queued,
      size: resolvedSize
    }, result?.ok === false ? 0 : 15000);
    return result;
  } catch (error) {
    sendExtensionLog("error", "handoff_failed", "资源发送到 NxTools 异常", {
      url,
      reason: error instanceof Error ? error.message : String(error)
    }, 0);
    throw error;
  }
}

async function sendDownloadItem(downloadItem) {
  const url = downloadItem.finalUrl || downloadItem.url;
  if (!isHttpUrl(url)) {
    return { ok: false, skipped: true, message: "不是 HTTP 下载链接" };
  }

  const headerSnapshot = resolveHeaderSnapshot(url) || resolveHeaderSnapshot(downloadItem.url);
  const resource = resourcesById.get(`${lastActiveTabId}:${normalizeResourceUrlKey(url)}`) || null;
  const headers = { ...(headerSnapshot?.headers || {}) };
  if (downloadItem.referrer && !headers.referer) {
    headers.referer = downloadItem.referrer;
  }

  return sendDownloadPayload({
    url,
    fileName: getFileNameFromPath(downloadItem.filename) || resource?.filename || getFileNameFromUrl(url),
    headers,
    size: downloadItem.totalBytes > 0 ? downloadItem.totalBytes : resource?.size || 0,
    supportsRange: Boolean(downloadItem.canResume || headerSnapshot?.supportsRange || resource?.supportsRange),
    contextUrl: downloadItem.referrer || resource?.pageUrl || "",
    idempotencyKey: `browser-download:${chrome.runtime.id}:${downloadItem.id}:${downloadItem.startTime || url}`
  });
}

async function sendPreemptedBrowserDownload(details, meta) {
  const headerSnapshot = resolveHeaderSnapshot(details.url);
  const tabId = await resolveTabIdForRequest(details);
  const tab = await tabInfo(tabId);
  const headers = { ...(headerSnapshot?.headers || {}) };
  const referer = headers.referer ||
    (details.initiator && details.initiator !== "null" ? details.initiator : tab?.url) ||
    "";
  if (referer) {
    headers.referer = referer;
  }

  const fileName = ensureFileNameForUrl(meta.filename || getFileNameFromUrl(details.url), details.url, meta.mime);
  cacheResource({
    id: `${tabId || lastActiveTabId || 0}:${normalizeResourceUrlKey(details.url)}`,
    tabId: tabId || lastActiveTabId || 0,
    url: details.url,
    pageTitle: tab?.title || "",
    pageUrl: referer || tab?.url || "",
    filename: fileName,
    mime: meta.mime,
    size: meta.size,
    supportsRange: Boolean(meta.supportsRange || headerSnapshot?.supportsRange),
    referer,
    requestHeaders: headers,
    source: "webRequest-download",
    capturedAt: Date.now()
  });

  return sendDownloadPayload({
    url: details.url,
    fileName,
    headers,
    size: meta.size,
    supportsRange: Boolean(meta.supportsRange || headerSnapshot?.supportsRange),
    contextUrl: referer || tab?.url || ""
  });
}

async function sendUrl(url, options = {}) {
  if (!isHttpUrl(url)) {
    throw new Error("只能发送 HTTP/HTTPS 链接");
  }

  const headerSnapshot = resolveHeaderSnapshot(url);
  const headers = { ...(headerSnapshot?.headers || {}) };
  if (options.referrer && !headers.referer) {
    headers.referer = options.referrer;
  }

  return sendDownloadPayload({
    url,
    fileName: options.fileName || getFileNameFromUrl(url),
    headers,
    size: options.size || 0,
    supportsRange: Boolean(options.supportsRange || headerSnapshot?.supportsRange),
    contextUrl: options.referrer || ""
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

  const result = await sendDownloadPayload({
    url: outboundResource.url,
    fileName: buildResourceFileName(outboundResource),
    headers: outboundResource.requestHeaders,
    size: outboundResource.size,
    supportsRange: outboundResource.supportsRange,
    contextUrl: outboundResource.pageUrl || outboundResource.referer || ""
  });
  resource.sentToDesktopAt = Date.now();
  schedulePersistResources();
  return result;
}

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

async function mergeResources(resourceIds) {
  const ids = [...new Set((resourceIds || []).map(String).filter(Boolean))];
  const resources = ids.map((id) => resourcesById.get(id)).filter(Boolean);
  if (resources.some((resource) => !isUsableGoogleVideoUrl(resource.url))) {
    return { ok: false, message: "选择的媒体资源已过期，请播放几秒后重新捕获" };
  }
  if (resources.length < 1 || resources.length > 100) {
    return { ok: false, message: "请选择 1 到 100 个资源批量下载" };
  }
  if (resources.some((resource) => String(resource.url || "").startsWith("blob:"))) {
    const results = [];
    for (const resource of resources) {
      results.push(await sendResource(resource.id));
    }
    const failed = results.find((result) => result?.ok === false);
    return failed || { ok: true, message: `已将 ${results.length} 个 Blob 资源发送到 NxTools` };
  }
  const settings = await getSettings();
  const items = [];
  for (const resource of resources) {
    const headers = await enrichHeadersForDownload(
      resource.url,
      resource.requestHeaders || {},
      settings,
      resource.pageUrl || resource.referer || ""
    );
    const headerPayload = headerObjectToPayload(headers, settings);
    items.push({
      url: resource.url,
      fileName: buildResourceFileName(resource),
      headers,
      cookie: headerPayload.cookie,
      size: Number(resource.size || 0),
      supportsRange: Boolean(resource.supportsRange),
      openDialog: settings.openDialog,
      startImmediately: !settings.openDialog
    });
  }
  const result = await sendDesktopRequest("create_batch", { items }, 30000, {
    idempotencyKey: `browser-batch:${chrome.runtime.id}:${crypto.randomUUID()}`
  });
  for (const resource of resources) {
    resource.sentToDesktopAt = Date.now();
  }
  schedulePersistResources();
  return result?.ok === false
    ? result
    : { ok: true, message: `已将 ${resources.length} 个资源批量发送到 NxTools`, data: result?.data };
}

async function interceptBrowserDownload(downloadItem, options = {}) {
  const settings = await getSettings();
  const url = downloadItem.finalUrl || downloadItem.url;
  if (!settings.interceptDownloads || !isHttpUrl(url)) {
    return;
  }
  if (!isSiteEnabled(downloadItem.referrer, url)) {
    sendExtensionLog("info", "browser_download_site_disabled", "下载发起站点已被用户停用，交还浏览器原生下载", {
      url,
      pageUrl: downloadItem.referrer || ""
    }, 15000);
    return;
  }
  if (wasPreemptedBrowserDownload(url)) {
    sendExtensionLog("info", "browser_download_already_preempted", "浏览器下载项已由响应头阶段接管，跳过二次处理", {
      url
    }, 15000);
    return;
  }

  try {
    const outcome = await NxToolsHandoffPolicy.handoffBrowserDownload({
      handoff: () => sendDownloadItem(downloadItem),
      cancelBrowser: () => chrome.downloads.cancel(downloadItem.id),
      compensate: async (accepted, cancelError) => {
        const taskId = String(accepted?.data?.taskId || "");
        const dialogRequestId = String(accepted?.data?.dialogRequestId || "");
        if (taskId) {
          await sendDesktopRequest("task_action", { taskId, action: "cancel" }, 5000).catch(() => {});
        } else if (dialogRequestId) {
          await sendDesktopRequest("dismiss_dialog", { dialogRequestId }, 5000).catch(() => {});
        }
        sendExtensionLog("warn", "browser_download_cancel_failed", "浏览器下载无法取消，已撤销 NxTools 任务", {
          reason: cancelError instanceof Error ? cancelError.message : String(cancelError)
        }, 0);
      },
      eraseBrowser: settings.eraseBrowserHistory || options.eraseBrowserHistory
        ? async () => {
          try {
            await chrome.downloads.erase({ id: downloadItem.id });
          } catch (eraseError) {
            console.warn("NxTools failed to erase cancelled browser download entry.", eraseError);
          }
        }
        : null
    });
    if (outcome.state === "browser_retained") {
      setBadge("!", "#9a3412");
      return;
    }
    setBadge("OK", "#138a72");
    setTimeout(() => setBadge("", "#138a72"), 1600);
  } catch (error) {
    console.warn("NxTools handoff failed.", error);
    setBadge("!", "#9a3412");
    // The authenticated browser request has not been cancelled, so its native download continues.
  }
}

function createContextMenus() {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "nxtools-download-link",
      title: "使用 NxTools 解析下载",
      contexts: ["link", "image", "audio", "video"],
      targetUrlPatterns: ["http://*/*", "https://*/*"]
    });

    chrome.contextMenus.create({
      id: "nxtools-download-page",
      title: "使用 NxTools 解析当前页面",
      contexts: ["page"],
      documentUrlPatterns: ["http://*/*", "https://*/*"]
    });

    chrome.contextMenus.create({
      id: "nxtools-open-popup",
      title: "打开 NxTools 资源面板",
      contexts: ["page", "link", "image", "audio", "video"]
    });
  });
}

async function checkApiStatus() {
  try {
    const response = await sendDesktopRequest("get_snapshot", {}, 2500);
    lastSnapshot = response.data || lastSnapshot;
    return {
      ok: true,
      message: connectionMessage || "Native Messaging 已连接 NxTools",
      transport: "native_messaging",
      data: response.data || lastSnapshot
    };
  } catch (error) {
    return {
      ok: false,
      transport: "native_messaging",
      message: error instanceof Error ? error.message : "无法连接 NxTools Native Messaging",
      data: lastSnapshot
    };
  }
}

function resourcesForPopup(activeTabId) {
  pruneResources();
  const current = activeTabId ? [...(resourcesByTab.get(activeTabId)?.values() || [])] : [];
  const other = [];
  for (const [tabId, bucket] of resourcesByTab) {
    if (tabId === activeTabId) {
      continue;
    }
    other.push(...bucket.values());
  }

  const enrich = (resource) => ({
    ...resource,
    domain: domainFromUrl(resource.url),
    description: describeResource(resource)
  });

  const bySizeThenTime = (left, right) => {
    const leftSize = Number(left.size || left.sourceSize || 0);
    const rightSize = Number(right.size || right.sourceSize || 0);
    if (leftSize !== rightSize) {
      return rightSize - leftSize;
    }
    return Number(right.capturedAt || 0) - Number(left.capturedAt || 0);
  };

  return {
    currentResources: current.sort(bySizeThenTime).map(enrich),
    otherResources: other.sort(bySizeThenTime).map(enrich)
  };
}

async function buildPopupState(preferredMediaIndex = null, currentView = "tasks") {
  const settings = await getSettings();
  const status = await checkApiStatus();
  const tab = await activeTab();
  const tabId = tab?.id || lastActiveTabId || 0;
  const shouldScanPage = currentView === "resources" || currentView === "advanced";
  const shouldReadMedia = currentView === "advanced";
  if (settings.captureResources !== false && tabId > 0 && shouldScanPage) {
    await scanTabForResources(tabId);
  }

  const resources = resourcesForPopup(tabId);
  const savedIndex = selectedMediaIndexByTab.get(tabId) ?? 0;
  const requestedIndex = Number.isInteger(preferredMediaIndex) ? preferredMediaIndex : savedIndex;
  const mediaState = shouldReadMedia
    ? await getMediaState(tabId, requestedIndex)
    : {
        mediaItems: [],
        mediaPlaybackState: {
          available: false,
          message: "切换到高级页后检测页面媒体",
          tabId: tabId || null,
          frameId: 0,
          mediaIndex: -1,
          currentTime: 0,
          duration: 0,
          progress: 0,
          volume: 1,
          paused: true,
          loop: false,
          muted: false,
          speed: 1
        }
      };
  const nextIndex = mediaState?.mediaPlaybackState?.mediaIndex;
  if (Number.isInteger(nextIndex) && nextIndex >= 0) {
    selectedMediaIndexByTab.set(tabId, nextIndex);
  }

  if (mediaState?.mediaPlaybackState?.available === false &&
      resources.currentResources.some(canSendOneClickResource)) {
    sendExtensionLog("warn", "page_media_dom_empty_resources_present", "页面媒体控件未检测到 DOM 媒体，但当前页已有可下载资源", {
      tabId,
      pageUrl: tab?.url || "",
      resourceCount: resources.currentResources.length,
      sampleUrls: resourceSamples(resources.currentResources, 5)
    }, 30000);
  }

  const activeHost = siteHostKey(tab?.url || "");
  return {
    settings,
    status,
    tasks: Array.isArray(status.data?.tasks) ? status.data.tasks : [],
    featureStates: featureStatesForTab(tabId, settings),
    siteControl: {
      host: activeHost,
      supported: isHttpUrl(tab?.url || ""),
      enabled: activeHost ? !disabledSites.has(activeHost) : true
    },
    ...mediaState,
    activeTab: tab
      ? {
          id: tab.id,
          title: tab.title || "",
          url: tab.url || "",
          domain: domainFromUrl(tab.url || "")
        }
      : null,
    ...resources
  };
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

function clearTabResources(tabId) {
  const bucket = resourcesByTab.get(tabId);
  if (bucket) {
    for (const resourceId of bucket.keys()) {
      resourcesById.delete(resourceId);
    }
    resourcesByTab.delete(tabId);
    schedulePersistResources();
  }
}

function clearTab(tabId) {
  clearTabResources(tabId);
  if (mobileUserAgentTabs.delete(tabId)) {
    void updateMobileUserAgentRule(tabId, false);
    void persistFeatureState();
  }
}

chrome.runtime.onInstalled.addListener(() => {
  createContextMenus();
  setBadge("", "#138a72");
});

chrome.runtime.onStartup.addListener(() => {
  createContextMenus();
  void loadPersistentResources();
  void ensureDesktopConnection();
});

registerHeaderCapture();
void loadPersistentResources();
void ensureDesktopConnection();
void activeTab().then((tab) => {
  if (tab?.id && /^https?:/i.test(tab.url || "")) {
    void injectContentBridge(tab.id, tab.url || "");
  }
});

chrome.tabs.onActivated.addListener((activeInfo) => {
  lastActiveTabId = activeInfo.tabId;
  schedulePersistResources();
  void injectContentBridge(activeInfo.tabId);
});

chrome.windows.onFocusChanged.addListener((windowId) => {
  if (windowId !== chrome.windows.WINDOW_ID_NONE) {
    void activeTab().then((tab) => {
      if (tab?.id && /^https?:/i.test(tab.url || "")) {
        void injectContentBridge(tab.id, tab.url || "");
      }
    });
  }
});

chrome.tabs.onRemoved.addListener((tabId) => {
  clearTab(tabId);
});

chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName !== "local") {
    return;
  }

  if (changes[STORAGE_DISABLED_SITES_KEY]) {
    loadDisabledSites(changes[STORAGE_DISABLED_SITES_KEY].newValue);
  }

  if (!changes.mediaOverlayEnabled) {
    return;
  }

  const enabled = changes.mediaOverlayEnabled.newValue !== false;
  chrome.tabs.query({}, (tabs) => {
    for (const tab of tabs) {
      if (!tab.id || !/^https?:/i.test(tab.url || "")) {
        continue;
      }
      void sendMessageToTab(tab.id, {
        type: "media_overlay_set_enabled",
        enabled
      }).then((result) => {
        if (!result.ok && enabled) {
          void injectContentBridge(tab.id, tab.url || "").then(() => sendMessageToTab(tab.id, {
            type: "media_overlay_set_enabled",
            enabled
          }));
        }
      });
    }
  });
});

if (chrome.webNavigation?.onCommitted) {
  chrome.webNavigation.onCommitted.addListener((details) => {
    if (details.frameId === 0) {
      clearTab(details.tabId);
      if (/^https?:\/\//i.test(details.url || "")) {
        void injectContentBridge(details.tabId, details.url);
      }
    }

    if (/^https?:\/\//i.test(details.url || "")) {
      void injectPageHook(details.tabId, details.frameId, details.url);
    }
  });
}

if (chrome.webNavigation?.onHistoryStateUpdated) {
  chrome.webNavigation.onHistoryStateUpdated.addListener((details) => {
    if (details.frameId === 0) {
      clearTabResources(details.tabId);
      if (/^https?:\/\//i.test(details.url || "")) {
        void injectContentBridge(details.tabId, details.url);
      }
    }

    if (/^https?:\/\//i.test(details.url || "")) {
      void injectPageHook(details.tabId, details.frameId, details.url);
    }
  });
}

const determiningFilenameEvent = chrome.downloads?.["onDetermining" + "Filename"];
if (determiningFilenameEvent?.addListener) {
  determiningFilenameEvent.addListener((downloadItem, suggest) => {
    suggest();
    void interceptBrowserDownload(downloadItem);
  });
} else if (chrome.downloads.onCreated?.addListener) {
  chrome.downloads.onCreated.addListener((downloadItem) => {
    void interceptBrowserDownload(downloadItem, { eraseBrowserHistory: true });
  });
}

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "nxtools-open-popup") {
    actionApi()?.openPopup?.();
    return;
  }

  const url = info.linkUrl || info.srcUrl || info.pageUrl;
  if (!url) {
    return;
  }

  void sendUrl(url, {
    referrer: info.pageUrl || tab?.url || ""
  }).then(
    () => {
      setBadge("OK", "#138a72");
      setTimeout(() => setBadge("", "#138a72"), 1600);
    },
    (error) => {
      console.warn("Failed to send context menu URL to NxTools.", error);
      setBadge("!", "#9a3412");
    }
  );
});

async function toggleFeature(feature, tabId) {
  const settings = await getSettings();
  if (feature === "mediaOverlay") {
    const enabled = settings.mediaOverlayEnabled === false;
    await chrome.storage.local.set({ mediaOverlayEnabled: enabled });
    const tabs = await chrome.tabs.query({});
    for (const tab of tabs) {
      if (!tab.id || !/^https?:/i.test(tab.url || "")) {
        continue;
      }
      void sendMessageToTab(tab.id, {
        type: "media_overlay_set_enabled",
        enabled
      });
    }
    return { ok: true, message: enabled ? "已开启媒体悬浮下载按钮" : "已关闭媒体悬浮下载按钮" };
  }

  if (feature === "deepSearch") {
    const enabled = settings.captureResources === false;
    await chrome.storage.local.set({ captureResources: enabled });
    if (enabled && tabId > 0) {
      await scanTabForResources(tabId, true);
    }
    return { ok: true, message: enabled ? "已开启深度资源嗅探" : "已关闭深度资源嗅探" };
  }

  if (feature === "mobileUserAgent") {
    if (!tabId || tabId <= 0) {
      return { ok: false, message: "当前没有可操作的标签页" };
    }

    const enabled = !mobileUserAgentTabs.has(tabId);
    if (enabled) {
      mobileUserAgentTabs.add(tabId);
    } else {
      mobileUserAgentTabs.delete(tabId);
    }

    await updateMobileUserAgentRule(tabId, enabled);
    await persistFeatureState();
    return { ok: true, message: enabled ? "已开启手机 User-Agent，刷新后更稳定" : "已关闭手机 User-Agent" };
  }

  return { ok: false, message: "不支持的扩展功能" };
}

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || typeof message !== "object") {
    return false;
  }

  if (message.type === "content_media_resource") {
    void capturePageResource(sender, message.payload).then(() => sendResponse({ ok: true }), (error) => {
      sendExtensionLog("error", "content_media_resource_failed", "内容脚本资源写入缓存失败", {
        reason: error instanceof Error ? error.message : String(error),
        resourceUrl: message.payload?.url || "",
        pageUrl: sender.tab?.url || "",
        tabId: sender.tab?.id || 0
      }, 0);
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "资源缓存失败"
      });
    });
    return true;
  }

  if (message.type === "content_page_context_changed") {
    const tabId = sender.tab?.id || lastActiveTabId || 0;
    if (tabId > 0) {
      clearTabResources(tabId);
      sendExtensionLog("info", "page_context_resources_cleared", "页面上下文变更，已清理旧资源缓存", {
        tabId,
        pageUrl: message.href || sender.tab?.url || "",
        reason: message.reason || ""
      }, 30000);
    }
    sendResponse({ ok: true });
    return true;
  }

  if (message.type === "content_blob_download_request") {
    void (async () => {
      const settings = await getSettings();
      const payload = message.payload || {};
      const blobUrl = String(payload.url || "");
      if (!blobUrl.startsWith("blob:")) {
        return { ok: false, fallbackToBrowser: true, message: "不是 Blob 下载请求" };
      }

      if (!settings.interceptDownloads) {
        sendExtensionLog("info", "blob_download_intercept_disabled", "Blob 点击下载未接管，因为浏览器下载拦截已关闭", {
          url: blobUrl,
          pageUrl: payload.href || sender.tab?.url || "",
          tabId: sender.tab?.id || 0
        }, 15000);
        return { ok: false, fallbackToBrowser: true, message: "浏览器下载拦截已关闭，交还浏览器下载" };
      }

      if (!isSiteEnabled(sender.tab?.url, payload.pageUrl, payload.href)) {
        return { ok: false, fallbackToBrowser: true, message: "已在本网站停用 NxTools，交还浏览器下载" };
      }

      await capturePageResource(sender, payload);
      const tabId = sender.tab?.id || lastActiveTabId || 0;
      const sourceUrl = String(payload.sourceUrl || "");
      let resource = sourceUrl && isHttpUrl(sourceUrl)
        ? findResourceByUrl(sourceUrl, tabId)
        : null;
      resource ??= findResourceByUrl(blobUrl, tabId);

      if (!resource) {
        sendExtensionLog("warn", "blob_download_resource_missing", "Blob 点击下载没有找到可接管的缓存资源", {
          url: blobUrl,
          sourceUrl,
          pageUrl: payload.href || sender.tab?.url || "",
          tabId
        }, 0);
        return { ok: false, fallbackToBrowser: true, message: "没有找到 Blob 缓存，交还浏览器下载" };
      }

      sendExtensionLog("info", "blob_download_intercepted", "已拦截页面 Blob 点击下载并发送到 NxTools", {
        url: resource.url,
        blobUrl,
        sourceUrl,
        pageUrl: payload.href || sender.tab?.url || "",
        tabId,
        filename: resource.filename || payload.filename || ""
      }, 0);

      return await sendResource(resource.id);
    })().then(sendResponse, (error) => {
      sendExtensionLog("error", "blob_download_intercept_failed", "Blob 点击下载接管失败", {
        reason: error instanceof Error ? error.message : String(error),
        resourceUrl: message.payload?.url || "",
        pageUrl: sender.tab?.url || "",
        tabId: sender.tab?.id || 0
      }, 0);
      sendResponse({
        ok: false,
        fallbackToBrowser: true,
        message: error instanceof Error ? error.message : "Blob 下载接管失败"
      });
    });
    return true;
  }

  if (message.type === "content_direct_download_request") {
    void (async () => {
      const settings = await getSettings();
      const url = String(message.url || "");
      if (!settings.interceptDownloads) {
        return { ok: false, fallbackToBrowser: true, message: "浏览器下载拦截已关闭" };
      }

      if (!isHttpUrl(url)) {
        return { ok: false, fallbackToBrowser: true, message: "不是 HTTP/HTTPS 下载链接" };
      }

      if (!isSiteEnabled(sender.tab?.url, message.href, url)) {
        return { ok: false, fallbackToBrowser: true, message: "已在本网站停用 NxTools，交还浏览器下载" };
      }

      const result = await sendUrl(url, {
        fileName: cleanFileName(message.filename || "") || getFileNameFromUrl(url),
        referrer: message.href || sender.tab?.url || ""
      });
      if (result?.ok === false) {
        return { ok: false, fallbackToBrowser: true, message: result.message || "NxTools 拒绝接管" };
      }
      setBadge("OK", "#138a72");
      setTimeout(() => setBadge("", "#138a72"), 1600);
      return { ok: true, message: "已发送到 NxTools" };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        fallbackToBrowser: true,
        message: error instanceof Error ? error.message : "下载链接接管失败"
      });
    });
    return true;
  }

  if (message.type === "content_send_latest_resource") {
    void (async () => {
      const tabId = sender.tab?.id || lastActiveTabId || 0;
      const resources = [...(resourcesByTab.get(tabId)?.values() || [])]
        .sort((left, right) => right.capturedAt - left.capturedAt);
      if (resources.length === 0) {
        return { ok: false, message: "当前页面还没有嗅探到资源" };
      }

      return await sendResource(resources[0].id);
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送资源失败"
      });
    });
    return true;
  }

  if (message.type === "content_get_resource_choices") {
    void (async () => {
      const tabId = sender.tab?.id || lastActiveTabId || 0;
      const choices = buildContentResourceChoices(tabId, message);
      sendExtensionLog("info", "content_resource_choices_built", "已为悬浮窗生成媒体资源列表", {
        tabId,
        pageUrl: message.href || sender.tab?.url || "",
        count: choices.length,
        sample: choices.slice(0, 5).map((choice) => `${choice.label} ${choice.size || 0}`).join(" | ")
      }, 10000);
      return {
        ok: true,
        choices
      };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "读取媒体资源列表失败",
        choices: []
      });
    });
    return true;
  }

  if (message.type === "page_download_media") {
    void downloadPageMedia(sender, message).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送当前媒体失败"
      });
    });
    return true;
  }

  if (message.type === "drop_target_download") {
    void (async () => {
      const url = String(message.url || "");
      if (!isHttpUrl(url)) {
        return { ok: false, message: "拖放内容不是 HTTP/HTTPS 链接" };
      }

      sendExtensionLog("info", "drop_target_download", "浏览器 Drop Target 已接收拖放链接", {
        url,
        pageUrl: message.href || sender.tab?.url || "",
        tabId: sender.tab?.id || lastActiveTabId || 0
      }, 0);

      await sendUrl(url, {
        fileName: getFileNameFromUrl(url) || message.filename || "",
        referrer: message.href || sender.tab?.url || ""
      });
      return { ok: true, message: "已发送到 NxTools" };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "拖放下载失败"
      });
    });
    return true;
  }

  if (message.type === "popup_get_state") {
    const index = Number.isInteger(message.mediaIndex) ? message.mediaIndex : null;
    void buildPopupState(index, String(message.currentView || "tasks")).then(sendResponse);
    return true;
  }

  if (message.type === "popup_save_settings") {
    const next = {
      ...DEFAULT_SETTINGS,
      ...(message.settings || {})
    };
    void chrome.storage.local.set(next).then(async () => {
      sendResponse(await buildPopupState(null, String(message.currentView || "settings")));
    });
    return true;
  }

  if (message.type === "popup_set_site_enabled") {
    void (async () => {
      const tab = await activeTab();
      const host = siteHostKey(tab?.url || "") || String(message.host || "").trim().toLowerCase();
      if (!host) {
        return { ok: false, message: "当前页面无法按网站设置" };
      }

      const enabled = Boolean(message.enabled);
      await setSiteEnabled(host, enabled);
      return {
        ok: true,
        host,
        enabled,
        message: enabled ? `已在 ${host} 启用 NxTools` : `已在 ${host} 停用 NxTools（仅交还自动接管）`
      };
    })().then(async (result) => {
      const state = await buildPopupState(null, String(message.currentView || "tasks"));
      sendResponse({ ...state, operationResult: result });
    }, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "设置网站开关失败"
      });
    });
    return true;
  }

  if (message.type === "popup_test_connection") {
    void checkApiStatus().then(sendResponse);
    return true;
  }

  if (message.type === "popup_send_active_tab") {
    void (async () => {
      const tab = await activeTab();
      if (!tab?.url || !isHttpUrl(tab.url)) {
        return { ok: false, message: "当前页面不是可下载链接" };
      }

      await sendUrl(tab.url, {
        fileName: getFileNameFromUrl(tab.url),
        referrer: tab.url
      });
      return { ok: true, message: "已发送到 NxTools 解析窗口" };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送失败"
      });
    });
    return true;
  }

  if (message.type === "popup_send_resource") {
    void sendResource(String(message.resourceId || "")).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "发送资源失败"
      });
    });
    return true;
  }

  if (message.type === "popup_merge_resources") {
    void mergeResources(Array.isArray(message.resourceIds) ? message.resourceIds : []).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "合并资源失败"
      });
    });
    return true;
  }

  if (message.type === "popup_toggle_feature") {
    void (async () => {
      const tab = await activeTab();
      const tabId = tab?.id || lastActiveTabId || 0;
      if (tabId > 0) {
        await injectContentBridge(tabId, tab?.url || "");
      }
      return toggleFeature(String(message.feature || ""), tabId);
    })().then(async (result) => {
      const state = await buildPopupState(null, String(message.currentView || "advanced"));
      sendResponse({ ...state, operationResult: result });
    }, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "切换扩展功能失败"
      });
    });
    return true;
  }

  if (message.type === "popup_set_media_index") {
    void (async () => {
      const tab = await activeTab();
      const tabId = tab?.id || lastActiveTabId || 0;
      const index = Number(message.index || 0);
      selectedMediaIndexByTab.set(tabId, Math.max(0, index));
      return buildPopupState(index, String(message.currentView || "advanced"));
    })().then(sendResponse);
    return true;
  }

  if (message.type === "popup_media_action") {
    void (async () => {
      const tab = await activeTab();
      const tabId = tab?.id || lastActiveTabId || 0;
      const index = Number(message.index || 0);
      const frameId = Number(message.frameId || 0);
      const payload = { ...(message.payload || {}), index };
      if (payload.Message === "screenshot" && chrome.tabs?.captureVisibleTab && chrome.downloads?.download) {
        const dataUrl = await chrome.tabs.captureVisibleTab(tab.windowId, { format: "png" });
        const fileName = `${cleanFileName(tab.title || "page") || "page"}-${Date.now()}.png`;
        await chrome.downloads.download({
          url: dataUrl,
          filename: `NxTools Screenshots/${fileName}`,
          saveAs: false
        });
        return { ok: true, message: "截图已保存到浏览器下载目录" };
      }

      const result = await sendMessageToTab(tabId, payload, Number.isInteger(frameId) ? { frameId } : {});
      if (!result.ok) {
        return { ok: false, message: result.message };
      }
      return { ok: true, state: result.response };
    })().then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "媒体操作失败"
      });
    });
    return true;
  }

  if (message.type === "popup_task_action") {
    void sendDesktopRequest("task_action", {
      taskId: String(message.taskId || ""),
      action: String(message.action || "")
    }).then(sendResponse, (error) => {
      sendResponse({
        ok: false,
        message: error instanceof Error ? error.message : "任务操作失败"
      });
    });
    return true;
  }

  if (message.type === "popup_clear_resources") {
    if (typeof message.tabId === "number" && message.tabId > 0) {
      clearTab(message.tabId);
    }
    void buildPopupState(null, String(message.currentView || "resources")).then(sendResponse);
    return true;
  }

  return false;
});
