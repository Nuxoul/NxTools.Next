"use strict";

// URL normalization, Google Video identity, filename and MIME/extension inference.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

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
