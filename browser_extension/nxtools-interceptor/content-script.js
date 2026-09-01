"use strict";

(function installNxToolsMediaBridge() {
  if (window.__nxToolsMediaBridgeInstalled) {
    return;
  }
  window.__nxToolsMediaBridgeInstalled = true;

  const sent = new Map();
  const mediaCandidates = new Map();
  let detectedCount = 0;
  let overlayHost = null;
  let overlayEnabled = true;
  let captureResourcesEnabled = false;
  let dropTargetHost = null;
  let dropTargetVisible = false;
  let dropTargetRect = null;
  let dropTargetHideTimer = 0;
  let dropTargetSettings = {
    enabled: true,
    position: "right-bottom",
    width: 188,
    height: 86
  };
  let interceptDownloadsEnabled = true;
  let updateQueued = false;
  let selectedMediaIndex = 0;
  let resetTimer = 0;
  let lastPageKey = "";
  const OVERLAY_GAP = 8;
  const FLOATING_MEDIA_WIDTH = 28;
  const FLOATING_MEDIA_HEIGHT = 28;
  const MEDIA_CANDIDATE_TTL_MS = 3 * 60 * 1000;
  const GOOGLEVIDEO_MIN_REMAINING_MS = 30 * 1000;
  const GOOGLEVIDEO_MAX_FUTURE_MS = 14 * 24 * 60 * 60 * 1000;
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

  const INTERESTING_EXTENSIONS = new Set([
    "avif", "bmp",
    "3gp", "aac", "avi", "cmfa", "cmfv", "flac", "flv", "gif", "m3u", "m3u8", "m4a", "m4s", "m4v", "mkv", "mov", "mp2t", "mp3",
    "jpeg", "jpg", "mp4", "mpd", "ogg", "opus", "png", "svg", "ts", "wav", "weba", "webm", "webp", "wmv",
    "7z", "apk", "app", "bat", "bin", "bz2", "csv", "deb", "dmg", "doc", "docx", "exe",
    "gz", "iso", "jar", "msi", "pdf", "pkg", "ppt", "pptx", "rar", "rpm", "sh", "tar",
    "tgz", "txt", "xls", "xlsx", "xz", "zip"
  ]);

  const DIRECT_DOWNLOAD_EXTENSIONS = new Set([
    "7z", "apk", "app", "appimage", "appx", "appxbundle", "ar", "bat", "bin", "bz", "bz2",
    "cab", "cmd", "com", "cpio", "crx", "deb", "dmg", "dpkg", "exe", "flatpakref", "gz",
    "img", "ipa", "iso", "jar", "lz", "lz4", "lzh", "lzma", "msi", "msix", "msixbundle",
    "msp", "msu", "nupkg", "pacman", "pkg", "ps1", "rar", "rpm", "run", "sh", "snap",
    "tar", "taz", "tbz", "tbz2", "tgz", "txz", "vhd", "vhdx", "vmdk", "war", "wim", "xar",
    "xpi", "xz", "z", "zip", "zipx", "zst", "zstd",
    "doc", "docx", "dot", "dotx", "xls", "xlsx", "xlsm", "xlt", "xltx", "ppt",
    "pptx", "pps", "ppsx", "pot", "potx", "odt", "ods", "odp", "odg", "rtf", "csv",
    "epub", "mobi", "azw", "azw3", "djvu", "fb2",
    "psd", "psb", "ai", "eps", "sketch", "fig", "indd", "afdesign", "xd",
    "raw", "dng", "cr2", "cr3", "nef", "arw", "orf", "rw2", "tiff", "tif", "heic", "heif",
    "rbxm", "rbxl", "rbxlx", "rbxmx",
    "unitypackage", "pak", "vpk", "asset", "fbx", "obj", "blend", "dae", "3ds", "stl",
    "glb", "gltf",
    "torrent", "magnet",
    "vsix", "nbproj", "ipynb",
    "log", "dat", "db", "sqlite", "sqlite3", "mdb", "accdb",
    "key", "cer", "crt", "pem", "p12", "pfx",
    "srt", "ass", "vtt", "ssa"
  ]);

  const OVERLAY_MEDIA_EXTENSIONS = new Set([
    "3gp", "aac", "avi", "cmfa", "cmfv", "flac", "flv", "m3u", "m3u8", "m4a", "m4s", "m4v", "mkv", "mov", "mp2t", "mp3",
    "mp4", "mpd", "ogg", "opus", "ts", "wav", "weba", "webm", "wmv"
  ]);

  const MEDIA_SELECTOR = [
    "video[src]",
    "audio[src]",
    "source[src]",
    "track[src]",
    "img[src]",
    "picture source[srcset]",
    "[srcset]",
    "a[href]"
  ].join(",");

  const MEDIA_CONTAINER_SELECTOR = [
    "video",
    "audio",
    "canvas",
    "[data-bvid]",
    "[data-aid]",
    "[data-video-id]"
  ].join(",");

  function runtimeLastError() {
    try {
      return chrome.runtime.lastError || null;
    } catch {
      return { message: "Extension context invalidated." };
    }
  }

  function safeRuntimeSendMessage(message, callback) {
    try {
      if (!globalThis.chrome?.runtime?.id || typeof chrome.runtime.sendMessage !== "function") {
        callback?.(undefined, { message: "Extension context invalidated." });
        return false;
      }

      chrome.runtime.sendMessage(message, (result) => {
        callback?.(result, runtimeLastError());
      });
      return true;
    } catch (error) {
      callback?.(undefined, {
        message: error instanceof Error ? error.message : "Extension message failed."
      });
      return false;
    }
  }

  function runtimeMessage(message) {
    return new Promise((resolve) => {
      safeRuntimeSendMessage(message, (result, lastError) => {
        resolve({ result, lastError });
      });
    });
  }

  function extensionOf(url) {
    try {
      const path = new URL(url, location.href).pathname;
      const name = path.split("/").pop() || "";
      const index = name.lastIndexOf(".");
      return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
    } catch {
      return "";
    }
  }

  function isInterestingMime(mime) {
    const type = String(mime || "").toLowerCase();
    return type.startsWith("video/")
      || type.startsWith("audio/")
      || type.startsWith("image/")
      || type === "image/gif"
      || type === "image/webp"
      || type.includes("mpegurl")
      || type.includes("dash+xml")
      || type.includes("mp4")
      || type.includes("octet-stream");
  }

  function isOverlayMediaMime(mime) {
    const type = String(mime || "").toLowerCase();
    return type.startsWith("video/")
      || type.startsWith("audio/")
      || type === "video/*"
      || type === "audio/*"
      || type.includes("mpegurl")
      || type.includes("dash+xml");
  }

  function isInterestingUrl(url, extra = {}) {
    const text = String(url || "");
    if (!/^(https?:|blob:)/i.test(text)) {
      return false;
    }
    if (!isUsableGoogleVideoUrl(url)) {
      return false;
    }
    if (isInterestingMime(extra.mime)) {
      return true;
    }
    if (/^blob:/i.test(text) && extra.elementKind) {
      return true;
    }
    const extension = String(extra.ext || extensionOf(url) || "").replace(/^\./, "").toLowerCase();
    return INTERESTING_EXTENSIONS.has(extension)
      || /\.(m3u8|mpd|mp4|m4s|webm|mkv|mov|flv|ts|m4a|mp3|aac|flac|wav|ogg)(?:[?#]|$)/i.test(url)
      || /\.(gif|webp|png|jpe?g|avif|bmp|svg)(?:[?#]|$)/i.test(url)
      || /(?:bilivideo|hdslb|upos-|mcdn|googlevideo|videoplayback|douyinvod|bytecdn|weibocdn|akamaized|cloudfront|dash|hls|segment|fragment|chunk|playback|stream).*?(?:m4s|mp4|audio|video|mime_type|mime=)/i.test(url);
  }

  function isOverlayMediaResourceUrl(url, extra = {}) {
    const text = String(url || "");
    if (!/^(https?:|blob:)/i.test(text)) {
      return false;
    }
    if (!isUsableGoogleVideoUrl(text)) {
      return false;
    }
    if (isOverlayMediaMime(extra.mime)) {
      return true;
    }

    const extension = String(extra.ext || extensionOf(text) || "").replace(/^\./, "").toLowerCase();
    return OVERLAY_MEDIA_EXTENSIONS.has(extension)
      || /\.(m3u8|mpd|mp4|m4s|webm|mkv|mov|flv|ts|m4a|mp3|aac|flac|wav|ogg)(?:[?#]|$)/i.test(text)
      || /(?:bilivideo|hdslb|upos-|mcdn|googlevideo|videoplayback|douyinvod|bytecdn|weibocdn|akamaized|cloudfront|dash|hls|segment|fragment|chunk|playback).*?(?:m4s|mp4|audio|video|mime_type|mime=)/i.test(text);
  }

  function mediaElementKind(element) {
    if (element instanceof HTMLAudioElement || element?.tagName?.toLowerCase?.() === "audio") {
      return "audio";
    }
    if (element instanceof HTMLVideoElement || element?.tagName?.toLowerCase?.() === "video") {
      return "video";
    }
    if (element instanceof HTMLImageElement || element?.tagName?.toLowerCase?.() === "img") {
      return "image";
    }
    return "";
  }

  function isGoogleVideoUrl(url) {
    try {
      const host = new URL(url, location.href).hostname.toLowerCase();
      return host === "googlevideo.com" || host.endsWith(".googlevideo.com");
    } catch {
      return false;
    }
  }

  function isUsableGoogleVideoUrl(url) {
    if (!isGoogleVideoUrl(url)) {
      return true;
    }

    try {
      const params = new URL(url, location.href).searchParams;
      const expireText = params.get("expire") || "";
      if (!expireText) {
        return true;
      }

      const expireMs = Number(expireText) * 1000;
      if (!Number.isFinite(expireMs) || expireMs <= 0) {
        return false;
      }

      const now = Date.now();
      return expireMs > now + GOOGLEVIDEO_MIN_REMAINING_MS &&
        expireMs < now + GOOGLEVIDEO_MAX_FUTURE_MS;
    } catch {
      return false;
    }
  }

  function filenameFromUrl(url) {
    try {
      return decodeURIComponent(new URL(url, location.href).pathname.split("/").pop() || "");
    } catch {
      return "";
    }
  }

  function firstSrcsetUrl(srcset) {
    const text = String(srcset || "").trim();
    if (!text) {
      return "";
    }
    return text.split(",").map((part) => part.trim().split(/\s+/, 1)[0]).filter(Boolean).pop() || "";
  }

  function mediaMimeFromElement(element) {
    if (!element) {
      return "";
    }

    const type = element.getAttribute("type") || "";
    if (type) {
      return type.split(";")[0].trim().toLowerCase();
    }

    if (element instanceof HTMLImageElement || element?.tagName?.toLowerCase?.() === "img" || element?.hasAttribute?.("srcset")) {
      return "image/*";
    }

    if (element instanceof HTMLMediaElement) {
      const source = element.currentSrc || element.src || "";
      if (/\.m3u8(?:[?#]|$)/i.test(source)) {
        return "application/vnd.apple.mpegurl";
      }
      if (/\.mpd(?:[?#]|$)/i.test(source)) {
        return "application/dash+xml";
      }
      return element.tagName.toLowerCase() === "audio" ? "audio/*" : "video/*";
    }

    return "";
  }

  function mediaDimensions(element) {
    if (element instanceof HTMLImageElement) {
      const width = Number(element.naturalWidth || element.width || 0);
      const height = Number(element.naturalHeight || element.height || 0);
      if (width > 0 && height > 0) {
        return { width, height };
      }
    }

    if (element instanceof HTMLVideoElement) {
      const width = Number(element.videoWidth || 0);
      const height = Number(element.videoHeight || 0);
      if (width > 0 && height > 0) {
        return { width, height };
      }

      const rect = element.getBoundingClientRect();
      const rectWidth = Math.round(rect.width || element.clientWidth || 0);
      const rectHeight = Math.round(rect.height || element.clientHeight || 0);
      return rectWidth > 0 && rectHeight > 0 ? { width: rectWidth, height: rectHeight } : { width: 0, height: 0 };
    }

    return { width: 0, height: 0 };
  }

  function mediaResolutionText(element) {
    const dimensions = mediaDimensions(element);
    return dimensions.width > 0 && dimensions.height > 0
      ? `${dimensions.width}x${dimensions.height}`
      : "";
  }

  function mediaCandidateKey(url) {
    try {
      const parsed = new URL(url, location.href);
      parsed.hash = "";
      return parsed.toString();
    } catch {
      return String(url || "").split("#", 1)[0];
    }
  }

  function escapeHtml(value) {
    return String(value || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function currentYoutubeVideoId() {
    try {
      const parsed = new URL(location.href);
      const id = parsed.searchParams.get("v");
      if (id) {
        return id;
      }
    } catch {
      // Ignore malformed page URLs.
    }

    const watchFlexy = document.querySelector("ytd-watch-flexy[video-id]");
    const attributeId = watchFlexy?.getAttribute?.("video-id");
    if (attributeId) {
      return attributeId;
    }

    const player = document.querySelector("#movie_player[data-video-id], .html5-video-player[data-video-id]");
    return player?.getAttribute?.("data-video-id") || "";
  }

  function currentPageKey() {
    const isYoutube = /(^|\.)youtube\.com$/i.test(location.hostname);
    const videoId = isYoutube ? currentYoutubeVideoId() : "";
    if (videoId) {
      return `${location.origin}${location.pathname}?v=${videoId}`;
    }

    return location.href.split("#", 1)[0];
  }

  function resetMediaCandidatesForNavigation(reason = "navigation") {
    const nextKey = currentPageKey();
    if (lastPageKey === nextKey) {
      return false;
    }

    lastPageKey = nextKey;
    mediaCandidates.clear();
    sent.clear();
    detectedCount = 0;
    scheduleOverlayUpdate();
    safeRuntimeSendMessage({
      type: "content_page_context_changed",
      href: location.href,
      pageKey: nextKey,
      reason
    });
    return true;
  }

  function ensurePageContext() {
    if (!lastPageKey) {
      lastPageKey = currentPageKey();
      return;
    }

    resetMediaCandidatesForNavigation("detected-url-change");
  }

  function performanceEntryCapturedAt(entry) {
    const origin = Number(performance?.timeOrigin || 0);
    const start = Number(entry?.startTime || 0);
    return origin > 0 && start >= 0 ? Math.round(origin + start) : Date.now();
  }

  function trackMediaCandidate(url, extra = {}) {
    let absoluteUrl;
    try {
      absoluteUrl = new URL(url, location.href).toString();
    } catch {
      return;
    }
    if (!isOverlayMediaResourceUrl(absoluteUrl, extra)) {
      return;
    }
    if (!isUsableGoogleVideoUrl(absoluteUrl)) {
      return;
    }

    const key = mediaCandidateKey(absoluteUrl);
    const previous = mediaCandidates.get(key);
    const capturedAt = Number(extra.capturedAt || Date.now());
    const candidate = {
      ...previous,
      url: absoluteUrl,
      key,
      pageKey: extra.pageKey || currentPageKey(),
      mime: extra.mime || previous?.mime || "",
      ext: extra.ext || extensionOf(absoluteUrl) || previous?.ext || "",
      size: Number(extra.size || 0) > 0 ? Number(extra.size || 0) : Number(previous?.size || 0),
      width: Number(extra.width || 0) > 0 ? Number(extra.width || 0) : Number(previous?.width || 0),
      height: Number(extra.height || 0) > 0 ? Number(extra.height || 0) : Number(previous?.height || 0),
      sourceUrl: extra.sourceUrl || previous?.sourceUrl || "",
      sourceMime: extra.sourceMime || previous?.sourceMime || "",
      sourceSize: Number(extra.sourceSize || 0) > 0 ? Number(extra.sourceSize || 0) : Number(previous?.sourceSize || 0),
      source: extra.source || previous?.source || "content",
      capturedAt: Math.max(capturedAt, Number(previous?.capturedAt || 0))
    };
    mediaCandidates.set(key, candidate);
  }

  function pruneMediaCandidates() {
    const cutoff = Date.now() - MEDIA_CANDIDATE_TTL_MS;
    for (const [key, candidate] of mediaCandidates) {
      if (Number(candidate.capturedAt || 0) < cutoff) {
        mediaCandidates.delete(key);
      }
    }
  }

  function harvestPerformanceMediaCandidates(limit = 160) {
    if (!performance?.getEntriesByType) {
      return;
    }

    const entries = performance.getEntriesByType("resource").slice(-limit);
    for (const entry of entries) {
      const url = entry.name || "";
      const initiator = String(entry.initiatorType || "").toLowerCase();
      if (!isOverlayMediaResourceUrl(url) && !((initiator === "video" || initiator === "audio") && isInterestingUrl(url))) {
        continue;
      }

      trackMediaCandidate(url, {
        source: `performance:${initiator || "resource"}`,
        size: Math.max(0, Math.round(entry.transferSize || entry.encodedBodySize || 0)),
        capturedAt: performanceEntryCapturedAt(entry)
      });
    }
  }

  function recentMediaResourceCandidates(maxAgeMs = MEDIA_CANDIDATE_TTL_MS) {
    ensurePageContext();
    harvestPerformanceMediaCandidates();
    pruneMediaCandidates();
    const cutoff = Date.now() - maxAgeMs;
    const pageKey = currentPageKey();
    return [...mediaCandidates.values()]
      .filter((candidate) =>
        Number(candidate.capturedAt || 0) >= cutoff &&
        (!candidate.pageKey || candidate.pageKey === pageKey) &&
        isOverlayMediaResourceUrl(candidate.url, candidate) &&
        isUsableGoogleVideoUrl(candidate.url))
      .sort((left, right) => Number(right.capturedAt || 0) - Number(left.capturedAt || 0))
      .slice(0, 80);
  }

  function youtubeItagInfo(itag) {
    const numeric = Number(itag || 0);
    return Number.isFinite(numeric) ? YOUTUBE_ITAG_INFO[numeric] || null : null;
  }

  function decodedMimeFromUrl(url) {
    try {
      const params = new URL(url, location.href).searchParams;
      return (params.get("mime") || params.get("mime_type") || "").replace(/_/g, "/").split(";", 1)[0].toLowerCase();
    } catch {
      return "";
    }
  }

  function candidateMediaInfo(candidate) {
    let params;
    try {
      params = new URL(candidate?.url || "", location.href).searchParams;
    } catch {
      params = new URLSearchParams();
    }

    const itag = params.get("itag") || candidate?.itag || "";
    const itagInfo = youtubeItagInfo(itag);
    const mime = String(candidate?.mime || decodedMimeFromUrl(candidate?.url || "") || "").toLowerCase();
    const kind = candidate?.kind ||
      (mime.startsWith("audio/")
      ? "audio"
      : mime.startsWith("video/")
        ? "video"
        : itagInfo?.kind || "media");
    const resolution = String(candidate?.resolution || "");
    const height = Number(candidate?.height || itagInfo?.height || 0);
    const width = Number(candidate?.width || 0);
    const extension = String(candidate?.extension || candidate?.ext || "").replace(/^\./, "").toUpperCase();
    const format = candidate?.format ||
      itagInfo?.format ||
      extension ||
      (mime.includes("webm") ? "WebM" : mime.includes("mp4") ? "MP4" : mime.split("/", 2)[1]?.toUpperCase() || "");
    const size = Number(candidate?.size || params.get("clen") || 0);
    return {
      itag,
      mime,
      kind,
      resolution,
      height,
      width,
      format,
      size,
      progressive: Boolean(itagInfo?.progressive)
    };
  }

  function formatBytes(bytes) {
    const units = ["B", "KB", "MB", "GB"];
    let value = Math.max(0, Number(bytes || 0));
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return unit === 0 ? `${Math.round(value)} ${units[unit]}` : `${value.toFixed(value >= 10 ? 1 : 2)} ${units[unit]}`;
  }

  function normalizeChoiceKey(url) {
    try {
      const parsed = new URL(url, location.href);
      parsed.hash = "";
      return parsed.toString();
    } catch {
      return String(url || "").split("#", 1)[0];
    }
  }

  function rankResourceChoice(candidate) {
    const info = candidateMediaInfo(candidate);
    const kindScore = info.kind === "video" ? 2_000_000_000 : info.kind === "audio" ? 1_000_000_000 : 0;
    const completeScore = info.progressive ? 200_000_000 : 0;
    const heightScore = Math.min(info.height || 0, 4320) * 100_000;
    const sizeScore = Math.min(info.size || 0, 80_000_000) / 10;
    const timeScore = Number(candidate?.capturedAt || 0) / 100000;
    return kindScore + completeScore + heightScore + sizeScore + timeScore;
  }

  async function backgroundResourceChoiceCandidates(selected) {
    const selectedDimensions = mediaDimensions(selected?.media);
    const response = await runtimeMessage({
      type: "content_get_resource_choices",
      href: location.href,
      pageKey: currentPageKey(),
      width: selectedDimensions.width,
      height: selectedDimensions.height,
      resolution: selectedDimensions.width > 0 && selectedDimensions.height > 0 ? `${selectedDimensions.width}x${selectedDimensions.height}` : "",
      requestedAt: Date.now()
    });

    if (response.lastError || response.result?.ok === false || !Array.isArray(response.result?.choices)) {
      return [];
    }

    return response.result.choices;
  }

  async function resourceChoiceCandidates(selected = null) {
    const seen = new Set();
    const backgroundChoices = await backgroundResourceChoiceCandidates(selected);
    const localChoices = recentMediaResourceCandidates(100 * 1000);
    return [...backgroundChoices, ...localChoices]
      .filter((candidate) => !String(candidate.url || "").startsWith("blob:") || candidate.sourceUrl)
      .filter((candidate) => {
        const idKey = candidate.resourceId ? `id:${candidate.resourceId}` : "";
        const urlKey = normalizeChoiceKey(candidate.url);
        const normalizedUrlKey = urlKey ? `url:${urlKey}` : "";
        if ((!idKey && !normalizedUrlKey) || (idKey && seen.has(idKey)) || (normalizedUrlKey && seen.has(normalizedUrlKey))) {
          return false;
        }
        if (idKey) {
          seen.add(idKey);
        }
        if (normalizedUrlKey) {
          seen.add(normalizedUrlKey);
        }
        return true;
      })
      .sort((left, right) => rankResourceChoice(right) - rankResourceChoice(left))
      .slice(0, 12);
  }

  function resourceChoiceLabel(candidate, index) {
    const info = candidateMediaInfo(candidate);
    const kind = info.kind === "audio" ? "音频" : info.kind === "video" ? "视频" : "媒体";
    const quality = info.resolution || (info.height > 0 ? `${info.height}p` : info.width > 0 ? `${info.width}px` : `资源 ${index + 1}`);
    const format = info.format ? ` · ${info.format}` : "";
    const complete = info.progressive ? " · 音视频" : "";
    return candidate.label || `${quality} ${kind}${format}${complete}`;
  }

  function resourceChoiceMeta(candidate) {
    const info = candidateMediaInfo(candidate);
    const size = info.size > 0 ? formatBytes(info.size) : "";
    const itag = info.itag ? `itag ${info.itag}` : "";
    const fileName = candidate.filename && !/^videoplayback$/i.test(candidate.filename) ? candidate.filename : "";
    const parts = [];
    const pushUnique = (value) => {
      const text = String(value || "").trim();
      if (!text) {
        return;
      }

      const normalized = text.toLowerCase();
      if (parts.some((item) => {
        const existing = item.toLowerCase();
        return existing === normalized || existing.includes(normalized) || normalized.includes(existing);
      })) {
        return;
      }

      parts.push(text);
    };

    pushUnique(size);
    pushUnique(fileName);
    pushUnique(itag);
    String(candidate.meta || "")
      .split("·")
      .map((item) => item.trim())
      .forEach(pushUnique);
    pushUnique(info.progressive ? "可单文件播放" : info.kind === "video" ? "通常为仅视频轨" : "");
    return parts.join(" · ");
  }

  function resourceChoiceQuality(candidate, index) {
    const info = candidateMediaInfo(candidate);
    return info.resolution || (info.height > 0 ? `${info.height}p` : info.width > 0 ? `${info.width}px` : `资源 ${index + 1}`);
  }

  function resourceChoiceTypeText(candidate) {
    const info = candidateMediaInfo(candidate);
    const kind = info.kind === "audio" ? "音频" : info.kind === "video" ? "视频" : "媒体";
    return info.format ? `${kind} · ${info.format}` : kind;
  }

  function resourceChoiceSizeText(candidate) {
    const info = candidateMediaInfo(candidate);
    return info.size > 0 ? formatBytes(info.size) : "-";
  }

  function resourceChoiceSourceText(candidate) {
    const source = String(candidate?.source || candidate?.sourceType || "").trim();
    return source
      .replace(/^performance:/i, "性能:")
      .replace(/^webRequest-/i, "请求:")
      .replace(/xhr-response/ig, "XHR响应")
      .replace(/page-hook/ig, "hook")
      .replace(/request/ig, "请求");
  }

  function resourceChoiceFileText(candidate, index) {
    const fileName = candidate?.filename && !/^videoplayback(?:\.\w+)?$/i.test(candidate.filename)
      ? candidate.filename
      : filenameFromUrl(candidate?.url || "");
    return fileName || candidate?.label || `资源 ${index + 1}`;
  }

  function resourceChoiceTrackText(candidate) {
    const info = candidateMediaInfo(candidate);
    if (info.progressive) {
      return "音视频";
    }

    if (info.kind === "video") {
      return "仅视频轨";
    }

    if (info.kind === "audio") {
      return "仅音频轨";
    }

    return "";
  }

  function resourceChoiceMetaText(candidate) {
    const info = candidateMediaInfo(candidate);
    const source = resourceChoiceSourceText(candidate);
    const parts = [];
    if (source) {
      parts.push(`来源: ${source}`);
    }
    if (info.itag) {
      parts.push(`itag ${info.itag}`);
    }
    const mime = info.mime && info.mime !== "video/*" && info.mime !== "audio/*" ? info.mime : "";
    if (mime) {
      parts.push(mime);
    }
    const track = resourceChoiceTrackText(candidate);
    if (track) {
      parts.push(track);
    }
    return parts.join(" · ") || resourceChoiceMeta(candidate) || "点击发送到 NxTools";
  }

  function resourceChoiceDetail(candidate) {
    const fileName = resourceChoiceFileText(candidate, 0);
    const meta = resourceChoiceMetaText(candidate);
    return [fileName, meta].filter(Boolean).join(" · ") || "点击发送到 NxTools";
  }

  function sendResource(url, element, extra = {}) {
    ensurePageContext();
    let absoluteUrl;
    try {
      absoluteUrl = new URL(url, location.href).toString();
    } catch {
      return;
    }

    if (!isInterestingUrl(absoluteUrl, extra)) {
      return;
    }

    const previous = sent.get(absoluteUrl);
    const size = Number(extra.size || 0);
    const mime = String(extra.mime || mediaMimeFromElement(element) || "").toLowerCase();
    const dimensions = mediaDimensions(element);
    const resolvedWidth = Number(extra.width || dimensions.width || 0);
    const resolvedHeight = Number(extra.height || dimensions.height || 0);
    const elementKind = extra.elementKind || mediaElementKind(element);
    if (previous &&
        (!mime || previous.mime === mime) &&
        (!size || size <= previous.size) &&
        (!resolvedWidth || resolvedWidth <= previous.width) &&
        (!resolvedHeight || resolvedHeight <= previous.height)) {
      return;
    }

    sent.set(absoluteUrl, {
      mime: mime || previous?.mime || "",
      size: Math.max(size, previous?.size || 0),
      width: Math.max(resolvedWidth, previous?.width || 0),
      height: Math.max(resolvedHeight, previous?.height || 0)
    });
    trackMediaCandidate(absoluteUrl, {
      mime,
      ext: extra.ext || extensionOf(absoluteUrl),
      size,
      width: resolvedWidth,
      height: resolvedHeight,
      sourceUrl: extra.sourceUrl || "",
      sourceMime: extra.sourceMime || "",
      sourceSize: Number(extra.sourceSize || 0),
      elementKind,
      pageKey: currentPageKey(),
      source: extra.source || "content",
      capturedAt: Number(extra.capturedAt || Date.now())
    });
    detectedCount += 1;
    scheduleOverlayUpdate();

    if (!captureResourcesEnabled) {
      return;
    }

    safeRuntimeSendMessage({
      type: "content_media_resource",
      payload: {
        url: absoluteUrl,
        href: location.href,
        filename: element?.getAttribute?.("download") ||
          element?.getAttribute?.("title") ||
          element?.getAttribute?.("aria-label") ||
          extra.filename ||
          document.title ||
          filenameFromUrl(absoluteUrl),
        mime: mime || mediaMimeFromElement(element),
        ext: extra.ext || extensionOf(absoluteUrl),
        size,
        width: resolvedWidth,
        height: resolvedHeight,
        pageKey: currentPageKey(),
        sourceUrl: extra.sourceUrl || "",
        sourceMime: extra.sourceMime || "",
        sourceSize: Number(extra.sourceSize || 0),
        elementKind,
        capturedAt: Number(extra.capturedAt || Date.now()),
        source: extra.source || "content",
        blobUrl: absoluteUrl.startsWith("blob:") ? absoluteUrl : ""
      }
    });
  }

  let nxBlobRequestSeq = 0;
  function requestBlobBytesFromPage(url, timeoutMs = 3500) {
    return new Promise((resolve) => {
      const token = `nxblob-${Date.now()}-${++nxBlobRequestSeq}`;
      let settled = false;
      const cleanup = () => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        window.removeEventListener("message", onMessage, true);
      };
      const onMessage = (event) => {
        if (event.source !== window) {
          return;
        }
        const data = event.data;
        if (!data || data.__nxtoolsBlobBytesResponse !== true || data.token !== token) {
          return;
        }
        cleanup();
        resolve(data.ok
          ? { ok: true, base64: data.base64 || "", mime: data.mime || "", size: Number(data.size || 0) }
          : { ok: false, message: data.message || "页面脚本无法读取这个 Blob" });
      };
      const timer = setTimeout(() => {
        cleanup();
        resolve({ ok: false, message: "页面脚本读取 Blob 超时" });
      }, timeoutMs);
      window.addEventListener("message", onMessage, true);
      window.postMessage({ __nxtoolsBlobBytesRequest: true, token, url: String(url || "") }, "*");
    });
  }

  function requestBlobPageAction(requestFlag, responseFlag, payload, timeoutMs) {
    return new Promise((resolve) => {
      const token = `nxblob-${Date.now()}-${++nxBlobRequestSeq}`;
      let settled = false;
      const cleanup = () => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        window.removeEventListener("message", onMessage, true);
      };
      const onMessage = (event) => {
        if (event.source !== window) {
          return;
        }
        const data = event.data;
        if (!data || data[responseFlag] !== true || data.token !== token) {
          return;
        }
        cleanup();
        resolve({
          ok: Boolean(data.ok),
          message: String(data.message || ""),
          mime: String(data.mime || ""),
          size: Number(data.size || 0)
        });
      };
      const timer = setTimeout(() => {
        cleanup();
        resolve({ ok: false, message: "页面 Blob 操作超时" });
      }, timeoutMs);
      window.addEventListener("message", onMessage, true);
      window.postMessage({ [requestFlag]: true, token, ...payload }, "*");
    });
  }

  function blobInfoFromPage(url) {
    return requestBlobPageAction(
      "__nxtoolsBlobInfoRequest",
      "__nxtoolsBlobInfoResponse",
      { url: String(url || "") },
      3500
    );
  }

  function requestBlobChunkFromPage(url, expectedSize, offset, length, timeoutMs = 10000) {
    return new Promise((resolve) => {
      const token = `nxblob-${Date.now()}-${++nxBlobRequestSeq}`;
      let settled = false;
      const cleanup = () => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        window.removeEventListener("message", onMessage, true);
      };
      const onMessage = (event) => {
        if (event.source !== window) {
          return;
        }
        const data = event.data;
        if (!data || data.__nxtoolsBlobChunkResponse !== true || data.token !== token) {
          return;
        }
        cleanup();
        if (!data.ok || !(data.bytes instanceof ArrayBuffer) || data.bytes.byteLength !== length) {
          resolve({ ok: false, message: data.message || "Blob 分块读取失败" });
          return;
        }
        resolve({ ok: true, bytes: data.bytes });
      };
      const timer = setTimeout(() => {
        cleanup();
        resolve({ ok: false, message: "页面 Blob 分块读取超时" });
      }, timeoutMs);
      window.addEventListener("message", onMessage, true);
      window.postMessage({
        __nxtoolsBlobChunkRequest: true,
        token,
        url: String(url || ""),
        expectedSize,
        offset,
        length
      }, "*");
    });
  }

  async function uploadBlobFromPage(url, uploadUrl, expectedSize, chunkBytes) {
    if (!Number.isSafeInteger(expectedSize) || expectedSize <= 0
        || !Number.isSafeInteger(chunkBytes) || chunkBytes <= 0 || chunkBytes > 1024 * 1024
        || !/^http:\/\/127\.0\.0\.1:\d+\/nxtools\/blob\/[0-9a-f-]+\/[0-9a-f]{64}$/i.test(uploadUrl)) {
      return { ok: false, message: "Blob 上传参数无效" };
    }
    let offset = 0;
    while (offset < expectedSize) {
      const length = Math.min(chunkBytes, expectedSize - offset);
      const chunk = await requestBlobChunkFromPage(url, expectedSize, offset, length);
      if (!chunk.ok) {
        return chunk;
      }
      let response;
      try {
        response = await fetch(`${uploadUrl}?offset=${offset}`, {
          method: "POST",
          headers: { "Content-Type": "text/plain;charset=UTF-8" },
          body: chunk.bytes,
          cache: "no-store",
          credentials: "omit",
          redirect: "error"
        });
      } catch (error) {
        return { ok: false, message: error instanceof Error ? error.message : "Blob 分块上传失败" };
      }
      if (!response.ok) {
        return { ok: false, message: `Blob 分块上传失败：HTTP ${response.status}` };
      }
      offset += length;
    }
    return { ok: true, message: "Blob 已安全上传到 NxTools" };
  }

  function fallbackBlobDownloadFromPage(url, filename) {
    return requestBlobPageAction(
      "__nxtoolsBlobFallbackRequest",
      "__nxtoolsBlobFallbackResponse",
      { url: String(url || ""), filename: String(filename || "") },
      5000
    );
  }

  async function readBlobInPage(url, filename) {
    if (!String(url || "").startsWith("blob:")) {
      return { ok: false, message: "不是 Blob 链接" };
    }

    // Prefer the page hook (main world): it holds a strong reference to the original Blob captured at
    // createObjectURL time, so it still works after the page revoked the blob URL or when the blob
    // belongs to a frame this isolated-world fetch can't reach. Fall back to fetch() if it can't.
    const viaPage = await requestBlobBytesFromPage(url);
    if (viaPage.ok && viaPage.base64) {
      return {
        ok: true,
        filename: filename || filenameFromUrl(url) || `blob-${Date.now()}`,
        mime: viaPage.mime || "",
        size: viaPage.size || 0,
        base64: viaPage.base64
      };
    }

    let response;
    try {
      response = await fetch(url);
    } catch {
      return { ok: false, message: viaPage.message || "页面无法读取这个 Blob 链接，可能已失效或属于其他 frame" };
    }

    if (!response.ok) {
      return { ok: false, message: `Blob 读取失败：HTTP ${response.status}` };
    }

    const blob = await response.blob();
    if (!blob || blob.size <= 0) {
      return { ok: false, message: "Blob 内容为空" };
    }

    if (blob.size > 48 * 1024 * 1024) {
      return { ok: false, message: "Blob 超过 48 MB，当前只能等待真实网络源被捕获" };
    }

    const dataUrl = await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ""));
      reader.onerror = () => reject(new Error("Blob 读取失败"));
      reader.readAsDataURL(blob);
    });
    const base64 = String(dataUrl).split(",", 2)[1] || "";
    if (!base64) {
      return { ok: false, message: "Blob 编码失败" };
    }

    return {
      ok: true,
      filename: filename || filenameFromUrl(url) || `blob-${Date.now()}`,
      mime: blob.type || response.headers.get("content-type") || "",
      size: blob.size,
      base64
    };
  }

  async function downloadBlobInPage(url, filename) {
    const retainedFallback = await fallbackBlobDownloadFromPage(url, filename);
    if (retainedFallback.ok) {
      return retainedFallback;
    }
    const readResult = await readBlobInPage(url, filename);
    if (!readResult.ok) {
      return readResult;
    }

    const blob = await (await fetch(`data:${readResult.mime || "application/octet-stream"};base64,${readResult.base64}`)).blob();
    const objectUrl = URL.createObjectURL(blob);
    try {
      const anchor = document.createElement("a");
      anchor.href = objectUrl;
      anchor.download = readResult.filename || filename || filenameFromUrl(url) || `blob-${Date.now()}`;
      anchor.rel = "noopener";
      anchor.setAttribute("data-nxtools-pass-through", "1");
      anchor.style.display = "none";
      (document.body || document.documentElement).appendChild(anchor);
      anchor.click();
      anchor.remove();
      return { ok: true, message: `已在浏览器中下载 Blob 资源（${Math.round(blob.size / 1024)} KB）` };
    } finally {
      setTimeout(() => URL.revokeObjectURL(objectUrl), 30_000);
    }
  }

  function installPageHook() {
    if (!document.documentElement || document.getElementById("nxToolsPageHookScript")) {
      return;
    }

    const script = document.createElement("script");
    script.id = "nxToolsPageHookScript";
    script.src = chrome.runtime.getURL("page-hook.js");
    script.async = false;
    script.onload = () => script.remove();
    script.onerror = () => script.remove();
    (document.documentElement || document.head || document.body).appendChild(script);
  }

  function scanElement(element) {
    const tag = element.tagName.toLowerCase();
    const url = tag === "a"
      ? element.getAttribute("href")
      : element.currentSrc || element.getAttribute("src") || firstSrcsetUrl(element.getAttribute("srcset"));
    if (url) {
      sendResource(url, element, {
        elementKind: mediaElementKind(element),
        source: String(url).startsWith("blob:") ? "content-blob-media" : "content"
      });
    }
  }

  function scanMediaElementSources() {
    document.querySelectorAll("audio,video").forEach((element) => {
      const urls = [
        element.currentSrc,
        element.src,
        element.getAttribute("src"),
        ...[...element.querySelectorAll("source[src]")].map((source) => source.currentSrc || source.src || source.getAttribute("src"))
      ].filter(Boolean);
      for (const url of urls) {
        sendResource(url, element, {
          elementKind: mediaElementKind(element),
          source: String(url).startsWith("blob:") ? "content-blob-media" : "content-media-source"
        });
      }
    });
  }

  function scanCssBackgroundResources(limit = 240) {
    const urlPattern = /url\((['"]?)(.*?)\1\)/gi;
    for (const element of [...document.querySelectorAll("*")].slice(0, limit)) {
      let background = "";
      try {
        background = getComputedStyle(element).backgroundImage || "";
      } catch {
        background = "";
      }
      if (!background || background === "none") {
        continue;
      }

      urlPattern.lastIndex = 0;
      let match;
      while ((match = urlPattern.exec(background))) {
        const url = match[2] || "";
        if (!url || /^data:/i.test(url)) {
          continue;
        }
        sendResource(url, element, {
          mime: "image/*",
          source: "content-background"
        });
      }
    }
  }

  function scanPerformanceResources(limit = 120) {
    if (!performance?.getEntriesByType) {
      return;
    }

    for (const entry of performance.getEntriesByType("resource").slice(-limit)) {
      const url = entry.name || "";
      if (!isInterestingUrl(url)) {
        continue;
      }

      sendResource(url, null, {
        size: Math.max(0, Math.round(entry.transferSize || entry.encodedBodySize || 0)),
        source: `performance:${entry.initiatorType || "resource"}`,
        capturedAt: performanceEntryCapturedAt(entry)
      });
    }
  }

  function scanPage(deep = false) {
    if (!captureResourcesEnabled && !overlayEnabled) {
      return;
    }
    document.querySelectorAll(MEDIA_SELECTOR).forEach(scanElement);
    scanMediaElementSources();
    scanPerformanceResources();
    if (deep && captureResourcesEnabled) {
      scanCssBackgroundResources();
    }
  }

  function mediaHasPlayableSource(media) {
    return Boolean(media?.currentSrc || media?.src || media?.readyState > 0 || (!media?.paused && !media?.ended));
  }

  function viewportFallbackRect() {
    const top = Math.min(Math.max(72, OVERLAY_GAP), Math.max(OVERLAY_GAP, innerHeight - FLOATING_MEDIA_HEIGHT - OVERLAY_GAP));
    const left = Math.max(OVERLAY_GAP, innerWidth - FLOATING_MEDIA_WIDTH - OVERLAY_GAP);
    return {
      left,
      top,
      right: left + FLOATING_MEDIA_WIDTH,
      bottom: top + FLOATING_MEDIA_HEIGHT,
      width: FLOATING_MEDIA_WIDTH,
      height: FLOATING_MEDIA_HEIGHT,
      fallback: true
    };
  }

  function visibleElementRect(element, minWidth, minHeight) {
    const rect = element.getBoundingClientRect();
    const visibleWidth = Math.max(0, Math.min(rect.right, innerWidth) - Math.max(rect.left, 0));
    const visibleHeight = Math.max(0, Math.min(rect.bottom, innerHeight) - Math.max(rect.top, 0));

    if (visibleWidth < minWidth || visibleHeight < minHeight) {
      return null;
    }

    const style = getComputedStyle(element);
    if (style.display === "none" || style.visibility === "hidden" || Number(style.opacity || 1) <= 0.02) {
      return null;
    }

    return {
      left: rect.left,
      top: rect.top,
      right: rect.right,
      bottom: rect.bottom,
      width: rect.width,
      height: rect.height,
      visibleWidth,
      visibleHeight,
      visibleArea: visibleWidth * visibleHeight,
      fallback: false
    };
  }

  function visibleMediaRect(media) {
    const tag = media.tagName.toLowerCase();
    const rect = visibleElementRect(media, tag === "audio" ? 64 : 48, tag === "audio" ? 20 : 28);
    if (rect) {
      return rect;
    }

    if (tag === "audio" && mediaHasPlayableSource(media)) {
      return viewportFallbackRect();
    }

    if (tag === "video" && mediaHasPlayableSource(media) && !media.paused) {
      return viewportFallbackRect();
    }

    return null;
  }

  function hasMediaSurface(element) {
    if (element instanceof HTMLMediaElement || element instanceof HTMLCanvasElement) {
      return true;
    }

    if (element.matches?.("[data-bvid], [data-aid], [data-video-id]")) {
      return true;
    }

    if (element.querySelector?.("video,audio,canvas")) {
      return true;
    }

    return false;
  }

  function containerUrl(element) {
    if (!element) {
      return "";
    }

    if (element instanceof HTMLAnchorElement) {
      return element.href || "";
    }

    const anchor = element.closest?.("a[href]") || element.querySelector?.("a[href]");
    if (anchor instanceof HTMLAnchorElement) {
      return anchor.href || "";
    }

    return "";
  }

  function mediaContainerScore(element, rect) {
    const text = `${element.className || ""} ${element.id || ""} ${element.getAttribute?.("aria-label") || ""}`;
    const strong = /bili|video|player|media|播放|视频/i.test(text) ? 20_000_000 : 0;
    const direct = element.matches?.("[data-bvid], [data-aid], [data-video-id]") ? 50_000_000 : 0;
    const playing = [...element.querySelectorAll?.("video,audio") || []]
      .some((media) => media instanceof HTMLMediaElement && !media.paused && !media.ended)
      ? 1_000_000_000
      : 0;
    const viewportCenterY = innerHeight / 2;
    const elementCenterY = Math.max(0, Math.min(innerHeight, rect.top + rect.visibleHeight / 2));
    const centerPenalty = Math.abs(elementCenterY - viewportCenterY) * 1000;
    return playing + direct + strong + rect.visibleArea - centerPenalty;
  }

  function findVisibleMediaContainer() {
    let selected = null;
    const recentMediaUrls = recentMediaResourceUrls();
    const hasRecentMediaResource = recentMediaUrls.length > 0;
    const candidates = [...document.querySelectorAll(MEDIA_CONTAINER_SELECTOR)].slice(0, 180);
    for (const element of candidates) {
      if (!(element instanceof Element) ||
          element === overlayHost ||
          element.id === "nxToolsMediaDownload" ||
          element === document.documentElement ||
          element === document.body ||
          element.closest?.("#nxToolsMediaDownload")) {
        continue;
      }

      if (element instanceof HTMLMediaElement) {
        continue;
      }

      const rect = visibleElementRect(element, 120, 80);
      if (!rect || rect.visibleArea < 9600 || rect.visibleArea > innerWidth * innerHeight * 0.95) {
        continue;
      }

      if (!hasMediaSurface(element)) {
        continue;
      }

      const childMedia = [...element.querySelectorAll?.("video,audio") || []]
        .filter((media) => media instanceof HTMLMediaElement && mediaHasPlayableSource(media))
        .sort((left, right) => mediaScore(right, visibleMediaRect(right) || viewportFallbackRect()) - mediaScore(left, visibleMediaRect(left) || viewportFallbackRect()))[0] || null;
      if (!childMedia &&
          !hasRecentMediaResource &&
          !element.matches?.("[data-bvid], [data-aid], [data-video-id]")) {
        continue;
      }

      const score = mediaContainerScore(element, rect);
      if (!selected || score > selected.score) {
        selected = {
          element,
          media: childMedia,
          rect,
          score,
          kind: childMedia?.tagName?.toLowerCase?.() || "video",
          containerOnly: true
        };
      }
    }

    return selected;
  }

  function resourceOnlySelection() {
    if (recentMediaResourceCandidates(90 * 1000).length === 0) {
      return null;
    }

    return {
      media: null,
      rect: viewportFallbackRect(),
      score: 1,
      kind: "media",
      resourceOnly: true
    };
  }

  function mediaScore(media, rect) {
    const playing = !media.paused && !media.ended ? 1_000_000_000 : 0;
    const visible = rect.fallback ? 0 : 10_000_000;
    const source = mediaHasPlayableSource(media) ? 100_000 : 0;
    return playing + visible + media.readyState * 1_000_000 + source + rect.width * rect.height;
  }

  function mediaElements() {
    return [...document.querySelectorAll("video, audio")];
  }

  function findActiveMedia() {
    let selected = null;
    for (const media of mediaElements()) {
      const rect = visibleMediaRect(media);
      if (!rect) {
        continue;
      }
      const score = mediaScore(media, rect);
      if (!selected || score > selected.score) {
        selected = { media, rect, score, kind: media.tagName.toLowerCase() };
      }
    }

    const container = findVisibleMediaContainer();
    if (container && (!selected || container.score > selected.score * 0.8)) {
      return container;
    }

    return selected || container || resourceOnlySelection();
  }

  function recentMediaResourceUrls() {
    return recentMediaResourceCandidates().map((candidate) => candidate.url);
  }

  function selectionResolutionText(selected) {
    if (selected?.media instanceof HTMLVideoElement) {
      return mediaResolutionText(selected.media);
    }

    const activeVideo = mediaElements()
      .filter((media) => media instanceof HTMLVideoElement && mediaHasPlayableSource(media))
      .sort((left, right) => mediaScore(right, visibleMediaRect(right) || viewportFallbackRect()) - mediaScore(left, visibleMediaRect(left) || viewportFallbackRect()))[0];
    return activeVideo instanceof HTMLVideoElement ? mediaResolutionText(activeVideo) : "";
  }

  function sameResourceUrl(left, right) {
    try {
      const leftUrl = new URL(left, location.href);
      const rightUrl = new URL(right, location.href);
      leftUrl.hash = "";
      rightUrl.hash = "";
      return leftUrl.toString() === rightUrl.toString();
    } catch {
      return String(left || "").split("#", 1)[0] === String(right || "").split("#", 1)[0];
    }
  }

  function closeOverlayPanel(schedule = true) {
    if (!overlayHost?.shadowRoot) {
      return false;
    }

    const panel = overlayHost.shadowRoot.querySelector(".panel");
    if (!panel || panel.hidden) {
      return false;
    }

    panel.hidden = true;
    overlayHost.shadowRoot.querySelector("button")?.setAttribute("aria-expanded", "false");
    const label = overlayHost.shadowRoot.querySelector(".label");
    if (label) {
      label.textContent = "下载此媒体";
    }
    const status = overlayHost.shadowRoot.querySelector(".status");
    if (status) {
      status.textContent = "";
      status.className = "status";
    }
    if (schedule) {
      scheduleOverlayUpdate();
    }
    return true;
  }

  function isOverlayEvent(event) {
    if (!overlayHost) {
      return false;
    }

    const path = typeof event.composedPath === "function" ? event.composedPath() : [];
    return path.includes(overlayHost);
  }

  function handleOverlayOutsidePointer(event) {
    if (!isOverlayEvent(event)) {
      closeOverlayPanel();
    }
  }

  function handleOverlayKeydown(event) {
    if (event.key === "Escape" && closeOverlayPanel()) {
      event.stopPropagation();
    }
  }

  function handleOverlayViewportChange() {
    closeOverlayPanel(false);
    scheduleOverlayUpdate();
  }

  function ensureOverlay() {
    if (overlayHost || !document.documentElement) {
      return overlayHost;
    }

    overlayHost = document.createElement("div");
    overlayHost.id = "nxToolsMediaDownload";
    Object.assign(overlayHost.style, {
      display: "none",
      left: "0",
      position: "fixed",
      top: "0",
      zIndex: "2147483647"
    });

    const root = overlayHost.attachShadow({ mode: "open" });
    root.innerHTML = `
      <style>
        :host {
          /* Shared browser tokens (EX-11); EX-14 extracts these into the
           * shared style module. Values mirror popup-tokens.css. */
          --nt-canvas: #1e1f22;
          --nt-surface-raised: #2a2b2f;
          --nt-surface-hover: #303238;
          --nt-border: #44474f;
          --nt-text: #e0e0e0;
          --nt-accent: #4791bf;
          font: 13px/18px "Segoe UI", "Microsoft YaHei UI", Arial, sans-serif;
          height: 28px;
          overflow: visible;
          position: fixed;
          width: 28px;
        }
        .trigger {
          align-items: center;
          background: var(--nt-surface-raised, #2a2b2f);
          border: 1px solid var(--nt-border, #44474f);
          border-radius: 4px;
          box-shadow: 0 2px 8px rgba(0, 0, 0, 0.28);
          color: var(--nt-text, #e0e0e0);
          cursor: pointer;
          display: inline-flex;
          font: inherit;
          height: 28px;
          justify-content: center;
          padding: 0;
          width: 28px;
        }
        .trigger:hover {
          background: var(--nt-surface-hover, #303238);
          border-color: var(--nt-accent, #4791bf);
        }
        .trigger:disabled { cursor: default; opacity: .78; }
        .trigger[aria-expanded="true"] {
          border-color: var(--nt-accent, #4791bf);
        }
        .icon {
          align-items: center;
          color: var(--nt-text, #e0e0e0);
          display: inline-flex;
          height: 28px;
          justify-content: center;
          width: 28px;
        }
        .icon svg {
          height: 16px;
          width: 16px;
          fill: currentColor;
        }
        .status { display: none; }
        .label {
          height: 1px;
          opacity: 0;
          overflow: hidden;
          pointer-events: none;
          position: absolute;
          width: 1px;
        }
        .error { color: #ff9b91; }
        .panel {
          background: rgba(17, 19, 20, 0.96);
          border: 1px solid rgba(56, 214, 199, .36);
          border-radius: 9px;
          box-shadow: 0 18px 40px rgba(0,0,0,.34);
          display: grid;
          gap: 5px;
          max-width: min(430px, calc(100vw - 16px));
          max-height: min(386px, calc(100vh - 72px));
          overflow: auto;
          padding: 5px;
          position: absolute;
          right: 0;
          top: 34px;
          width: min(414px, calc(100vw - 16px));
          backdrop-filter: blur(16px);
        }
        .panel[hidden] { display: none; }
        .choice {
          align-items: start;
          background: rgba(255,255,255,.025);
          border: 1px solid rgba(255,255,255,.055);
          border-radius: 7px;
          box-shadow: none;
          display: grid;
          gap: 4px;
          grid-template-columns: minmax(0, 1fr);
          min-height: 66px;
          padding: 7px 8px;
          text-align: left;
          width: 100%;
          cursor: pointer;
        }
        .choice:hover { background: rgba(255,255,255,.08); border-color: rgba(56,214,199,.32); }
        .choice:focus-visible { outline: 2px solid rgba(56,214,199,.72); outline-offset: 1px; }
        .choice span {
          min-width: 0;
        }
        .choice-head {
          align-items: center;
          display: flex;
          flex-wrap: wrap;
          gap: 5px;
        }
        .quality,
        .kind,
        .size {
          border: 1px solid rgba(56,214,199,.16);
          border-radius: 999px;
          line-height: 17px;
          padding: 0 6px;
          white-space: nowrap;
        }
        .quality { color: #eafffb; font-size: 12px; font-weight: 750; }
        .kind { background: rgba(56,214,199,.10); color: #a9fff4; font-size: 11.5px; font-weight: 700; }
        .size { color: #d9e3de; font-size: 11px; }
        .choice-file {
          color: #f5f3ee;
          display: -webkit-box;
          font-size: 11.5px;
          font-weight: 620;
          line-height: 15.5px;
          max-height: 31px;
          overflow: hidden;
          overflow-wrap: anywhere;
          white-space: normal;
          -webkit-box-orient: vertical;
          -webkit-line-clamp: 2;
        }
        .choice-meta {
          color: #9aa7a3;
          display: block;
          font-size: 10.5px;
          line-height: 14px;
          overflow-wrap: anywhere;
          white-space: normal;
        }
      </style>
      <button class="trigger" type="button" title="选择媒体资源" aria-expanded="false">
        <span class="icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/></svg></span>
        <span class="label">下载此媒体</span>
        <span class="status"></span>
      </button>
      <div class="panel" hidden></div>
    `;

    const button = root.querySelector("button");
    const label = root.querySelector(".label");
    const status = root.querySelector(".status");
    const panel = root.querySelector(".panel");

    function setStatus(text, failed = false) {
      status.textContent = text || "";
      status.className = failed ? "status error" : "status";
      clearTimeout(resetTimer);
      if (text && !failed) {
        resetTimer = setTimeout(() => {
          status.textContent = "";
        }, 1600);
      }
    }

    function sendOverlayResource(selected, chosenCandidate = null) {
      button.disabled = true;
      label.textContent = "正在发送";
      setStatus("");
      if (panel) {
        panel.hidden = true;
      }
      button.setAttribute("aria-expanded", "false");
      const resourceCandidates = chosenCandidate ? [chosenCandidate] : recentMediaResourceCandidates();
      const resourceUrls = resourceCandidates.map((candidate) => candidate.url);
      const selectedUrl = selected.media?.currentSrc || selected.media?.src || containerUrl(selected.element) || "";
      const selectedUrlIsRecent = !selectedUrl || resourceUrls.some((url) => sameResourceUrl(url, selectedUrl));
      const selectedDimensions = mediaDimensions(selected.media);
      const chosenInfo = chosenCandidate ? candidateMediaInfo(chosenCandidate) : null;
      safeRuntimeSendMessage({
        type: "page_download_media",
        url: chosenCandidate?.url || (selectedUrlIsRecent ? selectedUrl : ""),
        selectedResourceId: chosenCandidate?.resourceId || "",
        selectedResourceUrl: chosenCandidate?.url || "",
        selectedResourceFileName: chosenCandidate?.filename || "",
        selectedResourceMime: chosenCandidate?.mime || "",
        selectedResourceSize: Number(chosenCandidate?.size || 0),
        selectedResourceWidth: Number(chosenCandidate?.width || 0),
        selectedResourceHeight: Number(chosenCandidate?.height || 0),
        selectedResourceResolution: chosenInfo?.resolution || "",
        forceDirectResource: Boolean(chosenCandidate),
        selectionUrl: selectedUrl,
        selectionKind: selected.kind || "",
        width: selectedDimensions.width,
        height: selectedDimensions.height,
        resolution: selectedDimensions.width > 0 && selectedDimensions.height > 0 ? `${selectedDimensions.width}x${selectedDimensions.height}` : "",
        preferRecentResources: Boolean(resourceUrls.length && selectedUrl && !selectedUrlIsRecent),
        requestedAt: Date.now(),
        href: location.href,
        filename: document.title,
        poster: selected.media?.poster || "",
        resourceUrls,
        resourceCandidates
      }, (result, lastError) => {
        const ok = !lastError && result?.ok !== false;
        setStatus(ok ? "已发送" : lastError?.message || result?.message || "发送失败", !ok);
        label.textContent = "下载此媒体";
        button.disabled = false;
        scheduleOverlayUpdate();
      });
    }

    function showResourceChoices(selected, choices) {
      if (!panel) {
        return false;
      }

      const rows = choices.map((candidate, index) => {
        const choice = document.createElement("button");
        choice.className = "choice";
        choice.type = "button";
        choice.dataset.index = String(index);
        choice.title = `${resourceChoiceLabel(candidate, index)}\n${resourceChoiceFileText(candidate, index)}\n${resourceChoiceMetaText(candidate)}`;

        const head = document.createElement("div");
        head.className = "choice-head";

        const quality = document.createElement("span");
        quality.className = "quality";
        quality.textContent = resourceChoiceQuality(candidate, index);

        const kind = document.createElement("span");
        kind.className = "kind";
        kind.textContent = resourceChoiceTypeText(candidate);

        const size = document.createElement("span");
        size.className = "size";
        size.textContent = resourceChoiceSizeText(candidate);

        head.append(quality, kind, size);

        const file = document.createElement("span");
        file.className = "choice-file";
        file.textContent = resourceChoiceFileText(candidate, index);

        const meta = document.createElement("span");
        meta.className = "choice-meta";
        meta.textContent = resourceChoiceMetaText(candidate);

        choice.append(head, file, meta);
        return choice;
      });
      panel.replaceChildren(...rows);
      panel.hidden = false;
      button.setAttribute("aria-expanded", "true");
      panel.querySelectorAll(".choice").forEach((choice) => {
        choice.addEventListener("click", (event) => {
          event.preventDefault();
          event.stopPropagation();
          const index = Number(choice.getAttribute("data-index") || 0);
          const candidate = choices[index];
          if (candidate) {
            sendOverlayResource(selected, candidate);
          }
        });
      });
      label.textContent = "选择资源";
      setStatus(`${choices.length} 项`);
      scheduleOverlayUpdate();
      return true;
    }

    button.addEventListener("click", async (event) => {
      event.preventDefault();
      event.stopPropagation();
      if (panel?.hidden === false) {
        panel.hidden = true;
        button.setAttribute("aria-expanded", "false");
        label.textContent = "下载此媒体";
        setStatus("");
        scheduleOverlayUpdate();
        return;
      }

      const selected = findActiveMedia();
      if (!selected) {
        setStatus("未检测到媒体", true);
        return;
      }

      button.disabled = true;
      setStatus("读取资源");
      const choices = await resourceChoiceCandidates(selected);
      button.disabled = false;
      if (choices.length > 0) {
        showResourceChoices(selected, choices);
        return;
      }

      if (panel) {
        panel.hidden = true;
        button.setAttribute("aria-expanded", "false");
      }
      sendOverlayResource(selected, null);
    });

    document.documentElement.appendChild(overlayHost);
    return overlayHost;
  }

  function updateOverlayPosition() {
    updateQueued = false;
    const host = ensureOverlay();
    if (!host || !overlayEnabled) {
      if (host) {
        closeOverlayPanel(false);
        host.style.display = "none";
      }
      return;
    }

    const selected = findActiveMedia();
    if (!selected) {
      closeOverlayPanel(false);
      host.style.display = "none";
      return;
    }

    host.style.display = "block";
    const shadowRoot = host.shadowRoot;
    const label = shadowRoot?.querySelector(".label");
    const button = shadowRoot?.querySelector("button");
    const panel = shadowRoot?.querySelector(".panel");
    if (label && !button?.disabled && panel?.hidden !== false) {
      const resolution = selectionResolutionText(selected);
      const baseLabel = selected.kind === "audio" ? "下载此音频" : selected.kind === "video" ? "下载此视频" : "下载此媒体";
      label.textContent = resolution ? `${baseLabel} · ${resolution}` : baseLabel;
    }

    const gap = OVERLAY_GAP;
    const buttonWidth = host.offsetWidth || 112;
    const buttonHeight = host.offsetHeight || 32;
    let left;
    let top;

    if (selected.rect.fallback) {
      left = innerWidth - buttonWidth - gap;
      top = Math.min(Math.max(72, gap), Math.max(gap, innerHeight - buttonHeight - gap));
    } else if ((selected.kind === "video" || selected.containerOnly) &&
               selected.rect.width >= buttonWidth + gap * 2 &&
               selected.rect.height >= buttonHeight + gap * 2) {
      left = selected.rect.right - buttonWidth - gap;
      top = selected.rect.top + gap;
    } else {
      left = selected.rect.right + gap;
      top = selected.rect.top + Math.max(0, (selected.rect.height - buttonHeight) / 2);

      if (left + buttonWidth > innerWidth - gap) {
        left = selected.rect.right - buttonWidth;
        top = selected.kind === "audio"
          ? selected.rect.top + Math.max(0, (selected.rect.height - buttonHeight) / 2)
          : selected.rect.top - buttonHeight - gap;
      }
      if (top < gap && selected.rect.left - buttonWidth - gap >= gap) {
        left = selected.rect.left - buttonWidth - gap;
        top = selected.rect.top;
      }
      if (top < gap) {
        top = selected.rect.bottom + gap;
      }
    }

    host.style.left = `${Math.max(gap, Math.min(left, innerWidth - buttonWidth - gap))}px`;
    host.style.top = `${Math.max(gap, Math.min(top, innerHeight - buttonHeight - gap))}px`;
  }

  function scheduleOverlayUpdate() {
    if (updateQueued) {
      return;
    }
    updateQueued = true;
    requestAnimationFrame(updateOverlayPosition);
  }

  function mediaAt(index) {
    const items = mediaElements();
    const resolvedIndex = index >= 0 && index < items.length ? index : 0;
    selectedMediaIndex = resolvedIndex;
    return items[resolvedIndex] || null;
  }

  function buildMediaState(index) {
    const items = mediaElements();
    const active = findActiveMedia();
    const media = mediaAt(index);
    if (!media) {
      return {
        count: 0,
        src: [],
        paused: true,
        duration: 0,
        currentTime: 0,
        time: 0,
        volume: 1,
        loop: false,
        muted: false,
        speed: 1,
        activeIndex: -1,
        activeScore: 0,
        detectedCount
      };
    }

    return {
      count: items.length,
      src: items.map((item) => item.currentSrc || item.src || ""),
      types: items.map((item) => item.tagName.toLowerCase()),
      widths: items.map((item) => mediaDimensions(item).width),
      heights: items.map((item) => mediaDimensions(item).height),
      resolutions: items.map((item) => mediaResolutionText(item)),
      paused: media.paused,
      duration: Number.isFinite(media.duration) ? media.duration : 0,
      currentTime: Number.isFinite(media.currentTime) ? media.currentTime : 0,
      time: media.duration > 0 ? media.currentTime / media.duration : 0,
      volume: media.volume,
      loop: media.loop,
      muted: media.muted,
      speed: media.playbackRate,
      activeIndex: active ? items.indexOf(active.media) : selectedMediaIndex,
      activeScore: active?.score || mediaScore(media, visibleMediaRect(media) || { width: 0, height: 0 }),
      detectedCount
    };
  }

  async function handleMediaCommand(message) {
    const index = Number(message.index ?? selectedMediaIndex ?? 0);
    const media = mediaAt(index);
    if (message.Message === "getVideoState") {
      return buildMediaState(index);
    }
    if (!media) {
      return { ok: false, message: "当前页面没有媒体元素" };
    }

    switch (message.Message) {
      case "play":
        await media.play();
        break;
      case "pause":
        media.pause();
        break;
      case "speed":
        media.playbackRate = Math.max(0.25, Math.min(4, Number(message.speed || 1)));
        break;
      case "pip":
        if (media.tagName.toLowerCase() !== "video") {
          return { ok: false, message: "只有视频元素支持画中画" };
        }
        if (document.pictureInPictureEnabled === false) {
          return { ok: false, message: "当前页面或浏览器不允许画中画" };
        }
        if (document.pictureInPictureElement) {
          await document.exitPictureInPicture();
        } else if (media.requestPictureInPicture) {
          if (media.readyState < 2) {
            await media.play().catch(() => undefined);
          }
          await media.requestPictureInPicture();
        } else {
          return { ok: false, message: "当前视频不支持画中画" };
        }
        break;
      case "loop":
        media.loop = Boolean(message.action);
        break;
      case "muted":
        media.muted = Boolean(message.action);
        break;
      case "setVolume":
        media.volume = Math.max(0, Math.min(1, Number(message.volume ?? media.volume)));
        break;
      case "setTime":
        if (Number.isFinite(media.duration) && media.duration > 0) {
          media.currentTime = Math.max(0, Math.min(media.duration, Number(message.time ?? media.currentTime)));
        }
        break;
      case "screenshot": {
        const canvas = document.createElement("canvas");
        canvas.width = media.videoWidth || Math.max(1, Math.round(media.clientWidth));
        canvas.height = media.videoHeight || Math.max(1, Math.round(media.clientHeight));
        const context = canvas.getContext("2d");
        context?.drawImage(media, 0, 0, canvas.width, canvas.height);
        const link = document.createElement("a");
        link.href = canvas.toDataURL("image/png");
        link.download = `${document.title || "video"}-${Date.now()}.png`;
        link.click();
        break;
      }
      default:
        return { ok: false, message: "不支持的媒体指令" };
    }

    return { ok: true, state: buildMediaState(index) };
  }

  function normalizeDropTargetSettings(settings = {}) {
    const position = String(settings.dropTargetPosition || settings.position || "right-bottom");
    return {
      enabled: settings.dropTargetEnabled !== false && settings.enabled !== false,
      position: /^(left|center|right)-(top|bottom)$/i.test(position) ? position.toLowerCase() : "right-bottom",
      width: Math.max(96, Math.min(360, Number(settings.dropTargetWidth || settings.width || 188))),
      height: Math.max(48, Math.min(220, Number(settings.dropTargetHeight || settings.height || 86)))
    };
  }

  function dragDataHasUrl(dataTransfer) {
    const types = [...(dataTransfer?.types || [])].map((type) => String(type).toLowerCase());
    return types.includes("text/uri-list") || types.includes("text/plain") || types.includes("text/x-moz-url");
  }

  function urlFromDragData(dataTransfer) {
    const candidates = [
      dataTransfer?.getData("text/uri-list"),
      dataTransfer?.getData("text/x-moz-url"),
      dataTransfer?.getData("text/plain")
    ];

    for (const candidate of candidates) {
      const line = String(candidate || "")
        .split(/\r?\n/)
        .map((item) => item.trim())
        .find((item) => /^https?:\/\//i.test(item));
      if (line) {
        return line;
      }
    }

    return "";
  }

  function ensureDropTarget() {
    if (dropTargetHost || !document.documentElement) {
      return dropTargetHost;
    }

    dropTargetHost = document.createElement("div");
    dropTargetHost.id = "nxToolsDropTarget";
    Object.assign(dropTargetHost.style, {
      display: "none",
      left: "0",
      position: "fixed",
      top: "0",
      zIndex: "2147483646"
    });

    const root = dropTargetHost.attachShadow({ mode: "open" });
    root.innerHTML = `
      <style>
        :host { font: 13px/18px "Segoe UI", "Microsoft YaHei UI", Arial, sans-serif; }
        .target {
          align-items: center;
          background: rgba(17, 19, 20, .88);
          border: 1px dashed rgba(56, 214, 199, .72);
          border-radius: 8px;
          box-shadow: 0 16px 36px rgba(0,0,0,.34);
          color: #eafffb;
          display: grid;
          gap: 4px;
          height: 100%;
          justify-items: center;
          padding: 10px;
          text-align: center;
          width: 100%;
          backdrop-filter: blur(14px);
        }
        .icon { color: #38d6c7; font-size: 20px; font-weight: 700; line-height: 20px; }
        strong { font-size: 13px; font-weight: 700; }
        small { color: #8ee6dc; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; width: 100%; }
        :host(.inside) .target { background: rgba(18, 52, 49, .94); border-style: solid; }
      </style>
      <div class="target">
        <span class="icon">↓</span>
        <strong>拖到这里下载</strong>
        <small>发送到 NxTools 队列</small>
      </div>
    `;

    document.documentElement.appendChild(dropTargetHost);
    return dropTargetHost;
  }

  function computeDropTargetRect() {
    const gap = 18;
    const width = dropTargetSettings.width;
    const height = dropTargetSettings.height;
    const [xPos, yPos] = dropTargetSettings.position.split("-");
    const left = xPos === "left"
      ? gap
      : xPos === "center"
        ? Math.max(gap, (innerWidth - width) / 2)
        : Math.max(gap, innerWidth - width - gap);
    const top = yPos === "top"
      ? gap
      : Math.max(gap, innerHeight - height - gap);

    return {
      left,
      top,
      right: left + width,
      bottom: top + height,
      width,
      height
    };
  }

  function pointInRect(event, rect) {
    return rect &&
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;
  }

  function showDropTarget(event) {
    if (!dropTargetSettings.enabled || !dragDataHasUrl(event.dataTransfer)) {
      return false;
    }

    clearTimeout(dropTargetHideTimer);
    dropTargetHideTimer = 0;

    const host = ensureDropTarget();
    if (!host) {
      return false;
    }

    const nextRect = computeDropTargetRect();
    const shouldUpdateLayout = !dropTargetVisible ||
      !dropTargetRect ||
      dropTargetRect.left !== nextRect.left ||
      dropTargetRect.top !== nextRect.top ||
      dropTargetRect.width !== nextRect.width ||
      dropTargetRect.height !== nextRect.height;

    dropTargetRect = nextRect;
    if (shouldUpdateLayout) {
      Object.assign(host.style, {
        display: "block",
        left: `${dropTargetRect.left}px`,
        top: `${dropTargetRect.top}px`,
        width: `${dropTargetRect.width}px`,
        height: `${dropTargetRect.height}px`
      });
    } else if (host.style.display !== "block") {
      host.style.display = "block";
    }
    dropTargetVisible = true;
    return true;
  }

  function hideDropTargetNow() {
    clearTimeout(dropTargetHideTimer);
    dropTargetHideTimer = 0;
    if (!dropTargetVisible || !dropTargetHost) {
      return;
    }
    dropTargetHost.style.display = "none";
    dropTargetHost.classList.remove("inside");
    dropTargetVisible = false;
    dropTargetRect = null;
  }

  function hideDropTargetSoon(delayMs = 180) {
    clearTimeout(dropTargetHideTimer);
    dropTargetHideTimer = setTimeout(hideDropTargetNow, delayMs);
  }

  function handleDragLeave(event) {
    if (!dropTargetVisible) {
      return;
    }

    const x = Number(event.clientX || 0);
    const y = Number(event.clientY || 0);
    const stillInsideViewport = x > 0 && y > 0 && x < innerWidth && y < innerHeight;
    if (event.relatedTarget || stillInsideViewport) {
      return;
    }

    hideDropTargetSoon();
  }

  function handleDragOver(event) {
    if (!showDropTarget(event)) {
      return;
    }

    const inside = pointInRect(event, dropTargetRect);
    dropTargetHost?.classList.toggle("inside", inside);
    if (inside) {
      event.preventDefault();
      event.stopPropagation();
      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "copy";
      }
    }
  }

  function handleDrop(event) {
    if (!dropTargetVisible || !pointInRect(event, dropTargetRect)) {
      hideDropTargetSoon();
      return;
    }

    const url = urlFromDragData(event.dataTransfer);
    event.preventDefault();
    event.stopPropagation();
    hideDropTargetNow();

    if (!url) {
      return;
    }

    safeRuntimeSendMessage({
      type: "drop_target_download",
      url,
      href: location.href,
      filename: document.title
    });
  }

  function closestDownloadAnchor(target) {
    for (let node = target; node && node !== document; node = node.parentElement) {
      if (node instanceof HTMLAnchorElement && node.href) {
        return node;
      }
    }

    return null;
  }

  function shouldInterceptDirectDownload(anchor, url) {
    if (!interceptDownloadsEnabled ||
        !(anchor instanceof HTMLAnchorElement) ||
        anchor.getAttribute("data-nxtools-pass-through") === "1" ||
        !/^https?:\/\//i.test(String(url || ""))) {
      return false;
    }

    if (anchor.hasAttribute("download")) {
      return true;
    }

    const extension = extensionOf(url);
    return DIRECT_DOWNLOAD_EXTENSIONS.has(extension);
  }

  function replayBrowserDownload(anchor) {
    try {
      anchor.setAttribute("data-nxtools-pass-through", "1");
      setTimeout(() => {
        try {
          anchor.removeAttribute("data-nxtools-pass-through");
        } catch {
          // Ignore cleanup failures on detached nodes.
        }
      }, 1500);
      anchor.click();
    } catch {
      try {
        location.href = anchor.href;
      } catch {
        // Nothing else to do.
      }
    }
  }

  function handleDirectDownloadClick(event) {
    if (event.defaultPrevented ||
        event.button !== 0 ||
        event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        event.shiftKey) {
      return;
    }

    const anchor = closestDownloadAnchor(event.target);
    if (!anchor) {
      return;
    }

    let absoluteUrl = "";
    try {
      absoluteUrl = new URL(anchor.href || anchor.getAttribute("href") || "", location.href).toString();
    } catch {
      return;
    }

    if (!shouldInterceptDirectDownload(anchor, absoluteUrl)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation?.();

    safeRuntimeSendMessage({
      type: "content_direct_download_request",
      url: absoluteUrl,
      href: location.href,
      filename: anchor.getAttribute("download") || anchor.getAttribute("title") || anchor.textContent || filenameFromUrl(absoluteUrl)
    }, (result, lastError) => {
      if (!lastError && result?.ok !== false) {
        return;
      }

      replayBrowserDownload(anchor);
    });
  }

  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === "attributes" && mutation.target instanceof Element) {
        scanElement(mutation.target);
        mutation.target.querySelectorAll?.("audio,video,source,img").forEach(scanElement);
        continue;
      }
      mutation.addedNodes.forEach((node) => {
        if (!(node instanceof Element)) {
          return;
        }
        if (node.matches?.(MEDIA_SELECTOR)) {
          scanElement(node);
        }
        node.querySelectorAll?.(MEDIA_SELECTOR).forEach(scanElement);
      });
    }
    scheduleOverlayUpdate();
  });

  document.addEventListener("loadedmetadata", (event) => {
    if (event.target instanceof Element) {
      scanElement(event.target);
    }
    scheduleOverlayUpdate();
  }, true);

  window.addEventListener("nxtools-media-candidate", (event) => {
    const detail = event.detail || {};
    sendResource(detail.url, null, {
      mime: detail.mime || "",
      ext: detail.ext || extensionOf(detail.url || ""),
      size: Number(detail.size || 0),
      width: Number(detail.width || 0),
      height: Number(detail.height || 0),
      sourceUrl: detail.sourceUrl || "",
      sourceMime: detail.sourceMime || "",
      sourceSize: Number(detail.sourceSize || 0),
      filename: detail.filename || "",
      capturedAt: Number(detail.capturedAt || Date.now()),
      source: detail.source || "page-hook"
    });
  }, true);

  window.addEventListener("nxtools-blob-download-request", (event) => {
    const detail = event.detail || {};
    if (!String(detail.url || "").startsWith("blob:")) {
      return;
    }

    safeRuntimeSendMessage({
      type: "content_blob_download_request",
      payload: {
        url: detail.url,
        href: detail.href || location.href,
        filename: detail.filename || document.title || "",
        mime: detail.mime || "",
        ext: detail.ext || extensionOf(detail.url || ""),
        size: Number(detail.size || 0),
        width: Number(detail.width || 0),
        height: Number(detail.height || 0),
        sourceUrl: detail.sourceUrl || "",
        sourceMime: detail.sourceMime || "",
        sourceSize: Number(detail.sourceSize || 0),
        capturedAt: Number(detail.capturedAt || Date.now()),
        source: detail.source || "blob-download-click"
      }
    }, (result, lastError) => {
      if (NxToolsBlobTransferPolicy.handoffAccepted(result, lastError)) {
        return;
      }

      void downloadBlobInPage(detail.url, detail.filename || document.title || "");
    });
  }, true);

  document.addEventListener("play", scheduleOverlayUpdate, true);
  document.addEventListener("pause", scheduleOverlayUpdate, true);
  document.addEventListener("yt-navigate-start", () => resetMediaCandidatesForNavigation("youtube-navigate-start"), true);
  document.addEventListener("yt-navigate-finish", () => {
    resetMediaCandidatesForNavigation("youtube-navigate-finish");
    setTimeout(() => {
      ensurePageContext();
      scanPage();
    }, 250);
  }, true);
  document.addEventListener("dragenter", handleDragOver, true);
  document.addEventListener("dragover", handleDragOver, true);
  document.addEventListener("dragleave", handleDragLeave, true);
  document.addEventListener("drop", handleDrop, true);
  document.addEventListener("dragend", hideDropTargetNow, true);
  document.addEventListener("click", handleDirectDownloadClick, true);
  document.addEventListener("pointerdown", handleOverlayOutsidePointer, true);
  document.addEventListener("keydown", handleOverlayKeydown, true);
  addEventListener("blur", () => closeOverlayPanel(false), true);
  addEventListener("resize", handleOverlayViewportChange, { passive: true });
  addEventListener("popstate", () => resetMediaCandidatesForNavigation("popstate"), { passive: true });
  addEventListener("hashchange", () => resetMediaCandidatesForNavigation("hashchange"), { passive: true });
  addEventListener("scroll", handleOverlayViewportChange, { passive: true });
  document.addEventListener("scroll", handleOverlayViewportChange, true);
  visualViewport?.addEventListener("resize", handleOverlayViewportChange, { passive: true });
  visualViewport?.addEventListener("scroll", handleOverlayViewportChange, { passive: true });

  if (typeof PerformanceObserver !== "undefined") {
    try {
      const performanceObserver = new PerformanceObserver((list) => {
        if (!captureResourcesEnabled && !overlayEnabled) {
          return;
        }
        for (const entry of list.getEntries()) {
          const url = entry.name || "";
          if (isInterestingUrl(url)) {
            sendResource(url, null, {
              size: Math.max(0, Math.round(entry.transferSize || entry.encodedBodySize || 0)),
              source: `performance:${entry.initiatorType || "resource"}`,
              capturedAt: performanceEntryCapturedAt(entry)
            });
          }
        }
      });
      performanceObserver.observe({ type: "resource", buffered: true });
    } catch {
      // Some pages disallow resource performance observation.
    }
  }

  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message?.type === "media_overlay_set_enabled") {
      overlayEnabled = message.enabled !== false;
      scheduleOverlayUpdate();
      sendResponse({ ok: true });
      return true;
    }

    if (message?.type === "content_scan_now") {
      scanPage(true);
      sendResponse({ ok: true, count: detectedCount });
      return true;
    }

    if (message?.type === "content_read_blob") {
      void readBlobInPage(message.url, message.filename).then(sendResponse, (error) => {
        sendResponse({
          ok: false,
          message: error instanceof Error ? error.message : "Blob 读取失败"
        });
      });
      return true;
    }

    if (message?.type === "content_download_blob") {
      void downloadBlobInPage(message.url, message.filename).then(sendResponse, (error) => {
        sendResponse({
          ok: false,
          message: error instanceof Error ? error.message : "Blob 下载失败"
        });
      });
      return true;
    }

    if (message?.Message) {
      void handleMediaCommand(message).then(sendResponse, (error) => {
        sendResponse({
          ok: false,
          message: error instanceof Error ? error.message : "媒体操作失败"
        });
      });
      return true;
    }

    return false;
  });

  chrome.storage.local.get({
    interceptDownloads: true,
    captureResources: true,
    mediaOverlayEnabled: true,
    dropTargetEnabled: true,
    dropTargetPosition: "right-bottom",
    dropTargetWidth: 188,
    dropTargetHeight: 86
  }, (settings) => {
    interceptDownloadsEnabled = settings.interceptDownloads !== false;
    captureResourcesEnabled = settings.captureResources !== false;
    overlayEnabled = settings.mediaOverlayEnabled !== false;
    dropTargetSettings = normalizeDropTargetSettings(settings);
    if (captureResourcesEnabled) {
      sent.clear();
      scanPage(false);
    }

    if (message?.type === "content_blob_info") {
      void blobInfoFromPage(message.url).then((result) => {
        sendResponse({
          ...result,
          filename: message.filename || filenameFromUrl(message.url) || `blob-${Date.now()}`
        });
      }, (error) => {
        sendResponse({
          ok: false,
          message: error instanceof Error ? error.message : "Blob 信息读取失败"
        });
      });
      return true;
    }

    if (message?.type === "content_upload_blob") {
      void uploadBlobFromPage(message.url, message.uploadUrl, message.expectedSize, message.chunkBytes)
        .then(sendResponse, (error) => {
          sendResponse({
            ok: false,
            message: error instanceof Error ? error.message : "Blob 上传失败"
          });
        });
      return true;
    }
    scheduleOverlayUpdate();
  });

  chrome.storage.onChanged.addListener((changes, areaName) => {
    if (areaName !== "local") {
      return;
    }

    if (changes.mediaOverlayEnabled) {
      overlayEnabled = changes.mediaOverlayEnabled.newValue !== false;
      scheduleOverlayUpdate();
    }

    if (changes.interceptDownloads) {
      interceptDownloadsEnabled = changes.interceptDownloads.newValue !== false;
    }

    if (changes.captureResources) {
      captureResourcesEnabled = changes.captureResources.newValue !== false;
      if (captureResourcesEnabled) {
        sent.clear();
        scanPage(true);
      }
    }

    if (changes.dropTargetEnabled || changes.dropTargetPosition || changes.dropTargetWidth || changes.dropTargetHeight) {
      dropTargetSettings = normalizeDropTargetSettings({
        ...dropTargetSettings,
        dropTargetEnabled: changes.dropTargetEnabled?.newValue ?? dropTargetSettings.enabled,
        dropTargetPosition: changes.dropTargetPosition?.newValue ?? dropTargetSettings.position,
        dropTargetWidth: changes.dropTargetWidth?.newValue ?? dropTargetSettings.width,
        dropTargetHeight: changes.dropTargetHeight?.newValue ?? dropTargetSettings.height
      });
      if (!dropTargetSettings.enabled && dropTargetHost) {
        dropTargetHost.style.display = "none";
        dropTargetVisible = false;
      }
    }
  });

  installPageHook();
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", installPageHook, { once: true });
  }
  ensurePageContext();
  scanPage(false);
  observer.observe(document.documentElement || document, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ["src", "href"]
  });
  setInterval(() => {
    if (document.visibilityState === "visible") {
      scanPage(false);
    }
  }, 7000);
  setInterval(() => ensurePageContext(), 1000);
  setInterval(scheduleOverlayUpdate, 1000);
})();
