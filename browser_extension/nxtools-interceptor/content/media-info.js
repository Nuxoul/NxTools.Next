"use strict";

// URL/MIME/filename classification and media element metadata (pure helpers).
// Isolated-world content module: registered once on the extension-private
// NxToolsContent namespace (re-injection is a no-op). Holds no shared mutable
// page state; content-script.js owns all overlay/drop/capture state.
(function installNxToolsContentMedia(content) {
  if (content.media) {
    return;
  }

  const { GOOGLEVIDEO_MIN_REMAINING_MS, GOOGLEVIDEO_MAX_FUTURE_MS, YOUTUBE_ITAG_INFO, INTERESTING_EXTENSIONS, OVERLAY_MEDIA_EXTENSIONS } = content.constants;

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

  function performanceEntryCapturedAt(entry) {
    const origin = Number(performance?.timeOrigin || 0);
    const start = Number(entry?.startTime || 0);
    return origin > 0 && start >= 0 ? Math.round(origin + start) : Date.now();
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

  // Outline glyph for the page media chooser rows (same 24px grid as the popup).
  const MEDIA_KIND_ICON_PATHS = {
    video: ["M4 6.5h11a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 16V8A1.5 1.5 0 0 1 4 6.5z", "M16.5 10.5l5-3v9l-5-3"],
    audio: ["M9 17.5V6l11-2v11.5", "M9 17.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z", "M20 15.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z"]
  };

  function mediaKindIcon(kind) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");
    for (const d of MEDIA_KIND_ICON_PATHS[kind] || MEDIA_KIND_ICON_PATHS.video) {
      const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
      path.setAttribute("d", d);
      svg.append(path);
    }
    return svg;
  }
  content.media = Object.freeze({
    extensionOf,
    isInterestingMime,
    isOverlayMediaMime,
    isInterestingUrl,
    isOverlayMediaResourceUrl,
    mediaElementKind,
    isGoogleVideoUrl,
    isUsableGoogleVideoUrl,
    filenameFromUrl,
    firstSrcsetUrl,
    mediaMimeFromElement,
    mediaDimensions,
    mediaResolutionText,
    mediaCandidateKey,
    escapeHtml,
    currentYoutubeVideoId,
    currentPageKey,
    performanceEntryCapturedAt,
    youtubeItagInfo,
    decodedMimeFromUrl,
    candidateMediaInfo,
    formatBytes,
    mediaKindIcon
  });
})(globalThis.NxToolsContent || (globalThis.NxToolsContent = {}));
