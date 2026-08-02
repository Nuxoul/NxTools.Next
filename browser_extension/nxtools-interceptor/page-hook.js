"use strict";

(function installNxToolsPageHook() {
  if (window.__nxToolsPageHookInstalled) {
    return;
  }
  window.__nxToolsPageHookInstalled = true;

  const sent = new Map();
  const blobSources = new WeakMap();
  const objectUrlMetadata = new Map();
  // Strong references to the Blob objects behind blob: URLs, keyed by the URL string. The page may
  // revoke a blob URL the instant after triggering its download (perplexity and many SPAs do), which
  // makes any later fetch(url) fail. Holding the Blob itself lets us still read the bytes. Bounded so
  // we never pin more than a handful of blobs.
  const objectUrlBlobs = new Map();
  const MAX_RETAINED_BLOBS = 48;
  const MAX_BLOB_READ_BYTES = 48 * 1024 * 1024;
  const nativeJsonParse = JSON.parse;
  const interestingExtensions = new Set([
    "avif", "bmp",
    "3g2", "3gp", "aac", "ac3", "adts", "aif", "aiff", "alac", "ape", "asf", "avi", "cmfa", "cmfv", "divx",
    "f4a", "f4b", "f4f", "f4m", "f4v", "flac", "flv", "gif", "hlv", "ismv", "m2t", "m2ts", "m4a", "m4s", "m4v",
    "jpeg", "jpg",
    "mkv", "mov", "mp2t", "mp3", "mp4", "mp4a", "mp4v", "mpa", "mpd", "mpeg", "mpg", "m3u", "m3u8", "oga",
    "ogg", "ogv", "opus", "png", "smil", "svg", "ts", "wav", "weba", "webm", "wmv", "webp"
  ]);
  const MAX_TEXT_INSPECT_LENGTH = 2_500_000;
  const MAX_WALK_DEPTH = 12;
  const MEDIA_URL_REGEX = /(?:(?:https?:)?\/\/|\/)[^\s"'<>`\\]+?\.(?:3g2|3gp|aac|ac3|adts|aif|aiff|alac|ape|asf|avi|cmfa|cmfv|divx|f4a|f4b|f4f|f4m|f4v|flac|flv|gif|hlv|ismv|m2t|m2ts|m4a|m4s|m4v|mkv|mov|mp2t|mp3|mp4|mp4a|mp4v|mpa|mpd|mpeg|mpg|m3u|m3u8|oga|ogg|ogv|opus|smil|ts|wav|weba|webm|wmv|webp)(?:\?[^"'\s<>`]*)?/gi;
  const JSON_MEDIA_KEY_HINT = /(^|[_-])(url|uri|src|source|play|playurl|play_url|backup|baseurl|base_url|dash|hls|m3u8|mpd|video|audio|media|segment|stream|manifest|playlist)([_-]|$)/i;

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

  function isMediaMime(mime) {
    const type = String(mime || "").toLowerCase();
    return type.startsWith("video/")
      || type.startsWith("audio/")
      || type.startsWith("image/")
      || type.includes("mpegurl")
      || type.includes("dash+xml")
      || type.includes("mp4")
      || type.includes("octet-stream");
  }

  function isInterestingUrl(url, mime = "", extHint = "") {
    const text = String(url || "");
    if (!/^(https?:|blob:)/i.test(text)) {
      return false;
    }
    if (isMediaMime(mime)) {
      return true;
    }

    const extension = String(extHint || extensionOf(text)).replace(/^\./, "").toLowerCase();
    return interestingExtensions.has(extension)
      || /\.(m3u8|mpd|mp4|m4s|webm|mkv|mov|flv|ts|m4a|mp3|aac|flac|wav|ogg|gif|webp|png|jpe?g|avif|bmp|svg)(?:[?#]|$)/i.test(text)
      || /(?:bilivideo|hdslb|upos-|mcdn|googlevideo|videoplayback|douyinvod|bytecdn|weibocdn|akamaized|cloudfront|dash|hls|segment|fragment|chunk|playback|stream|manifest|playlist).*?(?:m4s|mp4|audio|video|mime_type|mime=)/i.test(text);
  }

  function notify(url, extra = {}) {
    let absoluteUrl;
    try {
      absoluteUrl = new URL(url, location.href).toString();
    } catch {
      return;
    }

    const mime = String(extra.mime || "").split(";")[0].trim().toLowerCase();
    const size = Number(extra.size || 0);
    const ext = String(extra.ext || extensionOf(absoluteUrl) || "").replace(/^\./, "").toLowerCase();
    if (!isInterestingUrl(absoluteUrl, mime, ext)) {
      return;
    }

    const previous = sent.get(absoluteUrl);
    if (previous &&
        (!mime || previous.mime === mime) &&
        (!size || size <= previous.size) &&
        (!ext || previous.ext === ext) &&
        previous.source === extra.source) {
      return;
    }

    sent.set(absoluteUrl, {
      mime: mime || previous?.mime || "",
      size: Math.max(size, previous?.size || 0),
      ext: ext || previous?.ext || "",
      source: extra.source || ""
    });

    window.dispatchEvent(new CustomEvent("nxtools-media-candidate", {
      detail: {
        url: absoluteUrl,
        mime,
        size,
        ext,
        width: Number(extra.width || 0),
        height: Number(extra.height || 0),
        filename: extra.filename || "",
        sourceUrl: extra.sourceUrl || "",
        sourceMime: extra.sourceMime || "",
        sourceSize: Number(extra.sourceSize || 0),
        source: extra.source || "page-hook",
        capturedAt: Number(extra.capturedAt || Date.now()),
        statusCode: Number(extra.statusCode || 0)
      }
    }));
  }

  function rememberBlobSource(blob, source) {
    if (!(blob instanceof Blob) || !source?.url) {
      return;
    }

    blobSources.set(blob, {
      url: source.url,
      mime: String(source.mime || blob.type || "").split(";", 1)[0].trim().toLowerCase(),
      size: Number(source.size || blob.size || 0),
      filename: source.filename || "",
      statusCode: Number(source.statusCode || 0),
      capturedAt: Date.now(),
      source: source.source || "blob-source"
    });
  }

  function metadataFromBlobSource(blob) {
    if (!(blob instanceof Blob)) {
      return null;
    }

    return blobSources.get(blob) || null;
  }

  function metadataFromObjectUrl(url) {
    return objectUrlMetadata.get(String(url || "")) || null;
  }

  function mediaMimeFromTag(tagName, fallback = "") {
    const tag = String(tagName || "").toLowerCase();
    if (tag === "audio") {
      return "audio/*";
    }
    if (tag === "video") {
      return "video/*";
    }
    if (tag === "img" || tag === "image") {
      return "image/*";
    }
    return fallback;
  }

  function notifyElementBlobUrl(element, url, reason = "element-src") {
    const text = String(url || "");
    if (!text.startsWith("blob:")) {
      return;
    }

    const metadata = metadataFromObjectUrl(text) || {};
    const mime = metadata.mime || metadata.sourceMime || mediaMimeFromTag(element?.tagName, "");
    const payload = {
      ...metadata,
      source: `blob-${reason}`,
      mime,
      size: Number(metadata.size || metadata.sourceSize || 0),
      ext: metadata.ext || imageExtensionFromMime(mime),
      sourceMime: metadata.sourceMime || mime,
      sourceSize: Number(metadata.sourceSize || metadata.size || 0),
      filename: metadata.filename || filenameFromUrl(metadata.sourceUrl || ""),
      capturedAt: Date.now()
    };
    objectUrlMetadata.set(text, payload);
    notify(text, payload);
  }

  function imageExtensionFromMime(mime) {
    const type = String(mime || "").toLowerCase();
    if (type === "image/jpeg" || type === "image/jpg") {
      return "jpg";
    }
    if (type === "image/png") {
      return "png";
    }
    if (type === "image/gif") {
      return "gif";
    }
    if (type === "image/webp") {
      return "webp";
    }
    if (type === "image/bmp") {
      return "bmp";
    }
    if (type === "image/svg+xml") {
      return "svg";
    }
    return "";
  }

  function parseFilenameFromDisposition(value) {
    const header = String(value || "");
    const match = /filename\*\s*=\s*UTF-8''([^;]+)|filename\s*=\s*"?([^";]+)"?/i.exec(header);
    const raw = match?.[1] || match?.[2] || "";
    if (!raw) {
      return "";
    }

    try {
      return decodeURIComponent(raw).trim();
    } catch {
      return raw.trim();
    }
  }

  function isSameOriginResponse(url) {
    try {
      return new URL(url || location.href, location.href).origin === location.origin;
    } catch {
      return true;
    }
  }

  function safeResponseHeader(headers, name, responseUrl = location.href) {
    const headerName = String(name || "").toLowerCase();
    if (headerName === "content-disposition" && !isSameOriginResponse(responseUrl)) {
      return "";
    }

    try {
      return headers?.get?.(name) || "";
    } catch {
      return "";
    }
  }

  function safeXhrHeader(xhr, name, responseUrl = location.href) {
    const headerName = String(name || "").toLowerCase();
    if (headerName === "content-disposition" && !isSameOriginResponse(responseUrl)) {
      return "";
    }

    try {
      return xhr.getResponseHeader(name) || "";
    } catch {
      return "";
    }
  }

  function filenameFromUrl(url) {
    try {
      return decodeURIComponent(new URL(url, location.href).pathname.split("/").pop() || "").trim();
    } catch {
      return "";
    }
  }

  function imageBlobDimensions(blob, objectUrl) {
    if (!(blob instanceof Blob) || !String(blob.type || "").toLowerCase().startsWith("image/")) {
      return Promise.resolve(null);
    }

    return new Promise((resolve) => {
      const image = new Image();
      let settled = false;
      const timeout = setTimeout(() => finish(null), 2500);
      function finish(value) {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timeout);
        image.onload = null;
        image.onerror = null;
        resolve(value);
      }

      image.onload = () => {
        finish({
          width: Number(image.naturalWidth || image.width || 0),
          height: Number(image.naturalHeight || image.height || 0)
        });
      };
      image.onerror = () => finish(null);
      image.src = objectUrl;
    });
  }

  function extensionFromFilename(value) {
    const name = String(value || "").split(/[?#]/, 1)[0];
    const index = name.lastIndexOf(".");
    return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
  }

  function dispatchBlobDownloadRequest(anchor, reason, clickEvent) {
    if (!(anchor instanceof HTMLAnchorElement)) {
      return false;
    }
    if (anchor.getAttribute("data-nxtools-pass-through") === "1") {
      return false;
    }

    const href = anchor.href || anchor.getAttribute("href") || "";
    if (!String(href).startsWith("blob:")) {
      return false;
    }

    const metadata = metadataFromObjectUrl(href);
    if (!anchor.hasAttribute("download") && !metadata?.sourceUrl && !metadata?.mime) {
      return false;
    }

    const filename = anchor.getAttribute("download")
      || metadata?.filename
      || filenameFromUrl(metadata?.sourceUrl || "")
      || `blob-${Date.now()}`;
    const mime = metadata?.mime || metadata?.sourceMime || "";
    const detail = {
      url: href,
      filename,
      mime,
      size: Number(metadata?.size || metadata?.sourceSize || 0),
      ext: metadata?.ext || imageExtensionFromMime(mime) || extensionFromFilename(filename),
      width: Number(metadata?.width || 0),
      height: Number(metadata?.height || 0),
      sourceUrl: metadata?.sourceUrl || "",
      sourceMime: metadata?.sourceMime || metadata?.mime || "",
      sourceSize: Number(metadata?.sourceSize || metadata?.size || 0),
      href: location.href,
      source: `blob-download-${reason || "click"}`,
      capturedAt: Date.now()
    };

    window.dispatchEvent(new CustomEvent("nxtools-blob-download-request", { detail }));
    if (clickEvent) {
      clickEvent.preventDefault();
      clickEvent.stopPropagation();
      clickEvent.stopImmediatePropagation?.();
    }
    return true;
  }

  function requestUrl(input) {
    if (typeof input === "string") {
      return input;
    }
    if (input instanceof URL) {
      return input.toString();
    }
    return input?.url || "";
  }

  function unescapeCandidateText(value) {
    return String(value || "")
      .trim()
      .replace(/\\u0026/gi, "&")
      .replace(/\\u003d/gi, "=")
      .replace(/\\u002f/gi, "/")
      .replace(/\\\//g, "/");
  }

  function tryUrl(value, baseUrl = location.href) {
    const text = unescapeCandidateText(value);
    if (!text || /^(javascript:|mailto:|tel:|data:image\/)/i.test(text)) {
      return "";
    }
    try {
      return new URL(text, baseUrl).toString();
    } catch {
      return "";
    }
  }

  function extForPlaylistText(text, fallback = "") {
    const sample = String(text || "").slice(0, 1024);
    if (/^\s*#EXTM3U/i.test(sample)) {
      return "m3u8";
    }
    if (/<MPD[\s>]|urn:mpeg:dash:schema:mpd/i.test(sample)) {
      return "mpd";
    }
    return fallback;
  }

  function tryParseJson(text) {
    if (typeof text !== "string") {
      return null;
    }
    const trimmed = text.trim();
    if (!trimmed || !/^[{[]/.test(trimmed)) {
      return null;
    }
    try {
      return nativeJsonParse(trimmed);
    } catch {
      return null;
    }
  }

  function inspectMediaText(text, baseUrl = location.href, source = "text", mime = "") {
    let content = String(text || "");
    if (!content) {
      return;
    }
    if (content.length > MAX_TEXT_INSPECT_LENGTH) {
      content = content.slice(0, MAX_TEXT_INSPECT_LENGTH);
    }

    const manifestExt = extForPlaylistText(content);
    if (manifestExt) {
      notify(baseUrl, {
        source,
        mime: manifestExt === "mpd" ? "application/dash+xml" : "application/vnd.apple.mpegurl",
        ext: manifestExt
      });
    }

    if (/^data:(application|video|audio)\//i.test(content)) {
      const comma = content.indexOf(",");
      const header = content.slice(0, comma).toLowerCase();
      const data = comma >= 0 ? content.slice(comma + 1) : "";
      if (data && /mpegurl|dash\+xml|mpd|m3u8/i.test(header)) {
        try {
          inspectMediaText(header.includes(";base64") ? atob(data) : decodeURIComponent(data), baseUrl, `${source}-data-url`, mime);
        } catch {
          // Ignore malformed data URLs.
        }
      }
    }

    let match;
    MEDIA_URL_REGEX.lastIndex = 0;
    while ((match = MEDIA_URL_REGEX.exec(content)) !== null) {
      const candidate = tryUrl(match[0], baseUrl);
      if (candidate) {
        notify(candidate, {
          source,
          mime,
          ext: extensionOf(candidate)
        });
      }
    }
  }

  function parseSignatureCipher(value, baseUrl = location.href) {
    const text = String(value || "");
    if (!text) {
      return "";
    }

    try {
      const params = new URLSearchParams(text);
      return tryUrl(params.get("url") || "", baseUrl);
    } catch {
      return "";
    }
  }

  function notifyMediaObject(value, source, baseUrl) {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
      return false;
    }

    const rawUrl = value.url || value.baseUrl || parseSignatureCipher(value.signatureCipher || value.cipher, baseUrl);
    const candidate = tryUrl(rawUrl, baseUrl);
    if (!candidate) {
      return false;
    }

    const mime = String(value.mimeType || value.mime || "").split(";", 1)[0].trim().toLowerCase();
    const size = Number(value.contentLength || value.clen || value.size || 0);
    const width = Number(value.width || 0);
    const height = Number(value.height || 0);
    const itag = value.itag ? `itag-${value.itag}` : "";
    notify(candidate, {
      source: [source, itag].filter(Boolean).join("-"),
      mime,
      size,
      width,
      height,
      ext: extensionOf(candidate)
    });
    return true;
  }

  function inspectObject(value, source = "object", baseUrl = location.href, depth = 0, seen = new WeakSet(), parentKey = "") {
    if (value == null || depth > MAX_WALK_DEPTH) {
      return;
    }

    if (typeof value === "string") {
      const text = unescapeCandidateText(value);
      const candidate = tryUrl(text, baseUrl);
      const ext = candidate ? extensionOf(candidate) : "";
      if (candidate && (interestingExtensions.has(ext) || JSON_MEDIA_KEY_HINT.test(parentKey) || isInterestingUrl(candidate, "", ext))) {
        notify(candidate, {
          source,
          ext
        });
      }

      const likelyTextPayload = text.includes("#EXTM3U") || /<MPD[\s>]|urn:mpeg:dash:schema:mpd/i.test(text) || MEDIA_URL_REGEX.test(text);
      MEDIA_URL_REGEX.lastIndex = 0;
      if (likelyTextPayload) {
        inspectMediaText(text, candidate || baseUrl, source);
      }
      return;
    }

    if (typeof value !== "object") {
      return;
    }
    if (seen.has(value)) {
      return;
    }
    seen.add(value);

    if (value instanceof ArrayBuffer || ArrayBuffer.isView(value)) {
      return;
    }

    if (Array.isArray(value)) {
      for (const item of value.slice(0, 800)) {
        inspectObject(item, source, baseUrl, depth + 1, seen, parentKey);
      }
      return;
    }

    notifyMediaObject(value, source, baseUrl);

    for (const [key, child] of Object.entries(value).slice(0, 1200)) {
      inspectObject(child, source, baseUrl, depth + 1, seen, key);
    }
  }

  function inspectYoutubePlayerResponse(reason = "scan") {
    if (!/(^|\.)youtube\.com$/i.test(location.hostname)) {
      return;
    }

    const responses = [];
    try {
      if (window.ytInitialPlayerResponse) {
        responses.push(window.ytInitialPlayerResponse);
      }
    } catch {
      // Ignore blocked globals.
    }

    try {
      const raw = window.ytplayer?.config?.args?.player_response;
      if (typeof raw === "string") {
        const parsed = tryParseJson(raw);
        if (parsed) {
          responses.push(parsed);
        }
      }
    } catch {
      // Ignore legacy player config failures.
    }

    try {
      const player = document.querySelector("#movie_player");
      const response = player?.getPlayerResponse?.();
      if (response) {
        responses.push(response);
      }
    } catch {
      // Ignore player API failures.
    }

    const seenResponses = new WeakSet();
    for (const response of responses) {
      if (!response || typeof response !== "object" || seenResponses.has(response)) {
        continue;
      }
      seenResponses.add(response);
      inspectObject(response, `youtube-player-${reason}`, location.href);
    }
  }

  function shouldInspectResponseText(url, mime, length) {
    const type = String(mime || "").toLowerCase();
    const size = Number(length || 0);
    if (type.includes("mpegurl") || type.includes("dash+xml") || type.includes("json") || type.includes("xml") || type.startsWith("text/") || type.includes("javascript")) {
      return true;
    }
    if (/(playurl|play_url|manifest|playlist|m3u8|mpd|dash|hls|api|aweme|video|media)/i.test(String(url || "")) && (!size || size <= MAX_TEXT_INSPECT_LENGTH)) {
      return true;
    }
    return size > 0 && size <= 512 * 1024 && type.includes("octet-stream");
  }

  function inspectResponseClone(response, fallbackUrl, source) {
    try {
      const url = response.url || fallbackUrl || location.href;
      const mime = safeResponseHeader(response.headers, "content-type", url);
      const length = Number(safeResponseHeader(response.headers, "content-length", url) || 0);
      if (!shouldInspectResponseText(url, mime, length)) {
        return;
      }

      response.clone().text().then((text) => {
        inspectMediaText(text, url, source, mime);
        const json = tryParseJson(text);
        if (json) {
          inspectObject(json, `${source}-json`, url);
        }
      }).catch(() => {
        // Some response wrappers are not readable.
      });
    } catch {
      // Ignore unusual response/header wrappers.
    }
  }

  if (typeof window.fetch === "function") {
    const nativeFetch = window.fetch;
    window.fetch = async function nxToolsFetch(input, init) {
      const url = requestUrl(input);
      notify(url, { source: "fetch" });
      const response = await nativeFetch.apply(this, arguments);
      try {
        const responseUrl = response.url || url;
        const responseMime = safeResponseHeader(response.headers, "content-type", responseUrl);
        const responseSize = Number(safeResponseHeader(response.headers, "content-length", responseUrl) || 0);
        const responseFilename = parseFilenameFromDisposition(safeResponseHeader(response.headers, "content-disposition", responseUrl)) || filenameFromUrl(responseUrl);
        notify(response.url || url, {
          source: "fetch-response",
          mime: responseMime,
          size: responseSize,
          filename: responseFilename,
          statusCode: response.status
        });
        const nativeBlob = response.blob;
        if (typeof nativeBlob === "function") {
          response.blob = function nxToolsResponseBlob() {
            return nativeBlob.apply(this, arguments).then((blob) => {
              rememberBlobSource(blob, {
                url: responseUrl,
                mime: responseMime,
                size: responseSize || blob?.size || 0,
                filename: responseFilename,
                statusCode: response.status,
                source: "fetch-blob"
              });
              return blob;
            });
          };
        }
        inspectResponseClone(response, responseUrl, "fetch-response");
      } catch {
        // Ignore sites with unusual response/header wrappers.
      }
      return response;
    };
    window.fetch.toString = function () {
      return nativeFetch.toString();
    };
  }

  if (typeof window.XMLHttpRequest === "function") {
    const nativeOpen = XMLHttpRequest.prototype.open;
    const nativeSend = XMLHttpRequest.prototype.send;

    XMLHttpRequest.prototype.open = function nxToolsOpen(method, url) {
      this.__nxToolsMethod = String(method || "GET").toUpperCase();
      this.__nxToolsUrl = requestUrl(url);
      this.__nxToolsNotifiedHeaders = false;
      this.__nxToolsInspectedBody = false;
      return nativeOpen.apply(this, arguments);
    };

    XMLHttpRequest.prototype.send = function nxToolsSend() {
      notify(this.__nxToolsUrl, { source: "xhr" });
      this.addEventListener("readystatechange", () => {
        if (!this.__nxToolsNotifiedHeaders && this.readyState >= 2) {
          this.__nxToolsNotifiedHeaders = true;
          try {
            const responseUrl = this.responseURL || this.__nxToolsUrl;
            notify(this.responseURL || this.__nxToolsUrl, {
              source: "xhr-response",
              mime: safeXhrHeader(this, "content-type", responseUrl),
              size: Number(safeXhrHeader(this, "content-length", responseUrl) || 0),
              filename: parseFilenameFromDisposition(safeXhrHeader(this, "content-disposition", responseUrl)) || filenameFromUrl(this.responseURL || this.__nxToolsUrl),
              statusCode: this.status
            });
          } catch {
            notify(this.responseURL || this.__nxToolsUrl, {
              source: "xhr-response",
              statusCode: this.status
            });
          }
        }

        if (this.__nxToolsInspectedBody || this.readyState < 4 || this.status < 200 || this.status >= 400) {
          return;
        }
        this.__nxToolsInspectedBody = true;
        try {
          const responseUrl = this.responseURL || this.__nxToolsUrl || location.href;
          const responseMime = safeXhrHeader(this, "content-type", responseUrl);
          const responseLength = Number(safeXhrHeader(this, "content-length", responseUrl) || 0);
          if (!shouldInspectResponseText(responseUrl, responseMime, responseLength)) {
            return;
          }
          if (this.responseType === "" || this.responseType === "text") {
            const text = this.responseText || this.response || "";
            inspectMediaText(text, responseUrl, "xhr-response", responseMime);
            const json = tryParseJson(text);
            if (json) {
              inspectObject(json, "xhr-json", responseUrl);
            }
          } else if (this.responseType === "json") {
            inspectObject(this.response, "xhr-json", responseUrl);
          }
        } catch {
          // Binary/cross-origin response wrappers can throw on responseText.
        }
      });
      this.addEventListener("load", () => {
        try {
          const response = this.response;
          if (!(response instanceof Blob)) {
            return;
          }

          rememberBlobSource(response, {
            url: this.responseURL || this.__nxToolsUrl || location.href,
            mime: safeXhrHeader(this, "content-type", this.responseURL || this.__nxToolsUrl || location.href) || response.type || "",
            size: Number(safeXhrHeader(this, "content-length", this.responseURL || this.__nxToolsUrl || location.href) || response.size || 0),
            filename: parseFilenameFromDisposition(safeXhrHeader(this, "content-disposition", this.responseURL || this.__nxToolsUrl || location.href)) || filenameFromUrl(this.responseURL || this.__nxToolsUrl || ""),
            statusCode: this.status,
            source: "xhr-blob"
          });
        } catch {
          // Blob metadata is best-effort.
        }
      });
      return nativeSend.apply(this, arguments);
    };
    XMLHttpRequest.prototype.open.toString = function () {
      return nativeOpen.toString();
    };
    XMLHttpRequest.prototype.send.toString = function () {
      return nativeSend.toString();
    };
  }

  JSON.parse = function nxToolsJsonParse() {
    const parsed = nativeJsonParse.apply(this, arguments);
    try {
      inspectObject(parsed, "json-parse", location.href);
    } catch {
      // Keep page JSON parsing side-effect free.
    }
    return parsed;
  };
  JSON.parse.toString = function () {
    return nativeJsonParse.toString();
  };

  function rememberObjectUrlBlob(objectUrl, blob) {
    if (!objectUrl || !(blob instanceof Blob)) {
      return;
    }
    if (objectUrlBlobs.has(objectUrl)) {
      objectUrlBlobs.delete(objectUrl);
    }
    objectUrlBlobs.set(objectUrl, blob);
    while (objectUrlBlobs.size > MAX_RETAINED_BLOBS) {
      const oldest = objectUrlBlobs.keys().next().value;
      objectUrlBlobs.delete(oldest);
    }
  }

  function readBlobToBase64(blob) {
    return new Promise((resolve) => {
      try {
        if (!(blob instanceof Blob) || blob.size <= 0) {
          resolve({ ok: false, message: "Blob 内容为空" });
          return;
        }
        if (blob.size > MAX_BLOB_READ_BYTES) {
          resolve({ ok: false, message: "Blob 超过 48 MB，等待真实网络源被捕获" });
          return;
        }
        const reader = new FileReader();
        reader.onload = () => {
          const result = String(reader.result || "");
          const base64 = result.split(",", 2)[1] || "";
          resolve(base64
            ? { ok: true, base64, mime: blob.type || "", size: blob.size }
            : { ok: false, message: "Blob 编码失败" });
        };
        reader.onerror = () => resolve({ ok: false, message: "Blob 读取失败" });
        reader.readAsDataURL(blob);
      } catch (error) {
        resolve({ ok: false, message: error instanceof Error ? error.message : "Blob 读取异常" });
      }
    });
  }

  async function resolveBlobBytes(url) {
    const key = String(url || "");
    let blob = objectUrlBlobs.get(key) || null;
    if (!blob) {
      // Not retained (created before the hook, or evicted). Try a same-realm fetch: a main-world
      // fetch of a live blob URL is far more reliable than an isolated-world content-script fetch.
      try {
        const response = await fetch(key);
        if (response.ok) {
          blob = await response.blob();
        }
      } catch {
        blob = null;
      }
    }
    if (!blob) {
      return { ok: false, message: "页面已无法访问这个 Blob（可能已被回收）" };
    }
    return readBlobToBase64(blob);
  }

  // Read-on-demand channel for the content script (isolated world). It posts a request with a token;
  // we read the retained Blob here in the owning realm and post the bytes back.
  window.addEventListener("message", (event) => {
    if (event.source !== window) {
      return;
    }
    const data = event.data;
    if (!data || data.__nxtoolsBlobBytesRequest !== true || typeof data.token !== "string") {
      return;
    }
    const token = data.token;
    resolveBlobBytes(data.url).then((result) => {
      window.postMessage({
        __nxtoolsBlobBytesResponse: true,
        token,
        ok: Boolean(result.ok),
        base64: result.ok ? result.base64 : "",
        mime: result.ok ? (result.mime || "") : "",
        size: result.ok ? (result.size || 0) : 0,
        message: result.ok ? "" : (result.message || "Blob 读取失败")
      }, "*");
    });
  });

  if (URL?.createObjectURL) {
    const nativeCreateObjectURL = URL.createObjectURL;
    URL.createObjectURL = function nxToolsCreateObjectURL(value) {
      const objectUrl = nativeCreateObjectURL.apply(this, arguments);
      try {
        if (value instanceof Blob) {
          rememberObjectUrlBlob(objectUrl, value);
          const sourceMetadata = metadataFromBlobSource(value);
          const mime = value.type || sourceMetadata?.mime || "";
          const ext = mime.includes("mpegurl")
            ? "m3u8"
            : mime.includes("dash+xml")
              ? "mpd"
              : imageExtensionFromMime(mime);
          const blobPayload = {
            source: "blob-url",
            mime,
            size: value.size,
            ext,
            sourceUrl: sourceMetadata?.url || "",
            sourceMime: sourceMetadata?.mime || "",
            sourceSize: sourceMetadata?.size || 0,
            filename: sourceMetadata?.filename || filenameFromUrl(sourceMetadata?.url || ""),
            capturedAt: Date.now()
          };
          objectUrlMetadata.set(objectUrl, blobPayload);
          notify(objectUrl, blobPayload);
          imageBlobDimensions(value, objectUrl).then((dimensions) => {
            if (dimensions?.width > 0 && dimensions?.height > 0) {
              objectUrlMetadata.set(objectUrl, {
                ...blobPayload,
                width: dimensions.width,
                height: dimensions.height,
                source: "blob-image"
              });
              notify(objectUrl, {
                ...blobPayload,
                width: dimensions.width,
                height: dimensions.height,
                source: "blob-image"
              });
            }
          }).catch(() => {});
          if (sourceMetadata?.url) {
            notify(sourceMetadata.url, {
              source: `${sourceMetadata.source || "blob-source"}-source-url`,
              mime: sourceMetadata.mime || mime,
              size: sourceMetadata.size || value.size,
              ext: ext || extensionOf(sourceMetadata.url),
              filename: sourceMetadata.filename || filenameFromUrl(sourceMetadata.url),
              statusCode: sourceMetadata.statusCode || 0
            });
          }
          if (value.size <= MAX_TEXT_INSPECT_LENGTH && shouldInspectResponseText(objectUrl, mime, value.size)) {
            value.text().then((text) => inspectMediaText(text, location.href, "blob-text", mime)).catch(() => {});
          }
        }
      } catch {
        // Blob inspection is best-effort.
      }
      return objectUrl;
    };
    URL.createObjectURL.toString = function () {
      return nativeCreateObjectURL.toString();
    };

    if (typeof URL.revokeObjectURL === "function") {
      const nativeRevokeObjectURL = URL.revokeObjectURL;
      URL.revokeObjectURL = function nxToolsRevokeObjectURL(url) {
        objectUrlMetadata.delete(String(url || ""));
        return nativeRevokeObjectURL.apply(this, arguments);
      };
      URL.revokeObjectURL.toString = function () {
        return nativeRevokeObjectURL.toString();
      };
    }
  }

  if (typeof HTMLAnchorElement !== "undefined" && HTMLAnchorElement.prototype?.click) {
    const nativeAnchorClick = HTMLAnchorElement.prototype.click;
    HTMLAnchorElement.prototype.click = function nxToolsAnchorClick() {
      try {
        if (dispatchBlobDownloadRequest(this, "programmatic-click", null)) {
          return undefined;
        }
      } catch {
        // Keep anchor clicks side-effect compatible with the page.
      }

      return nativeAnchorClick.apply(this, arguments);
    };
    HTMLAnchorElement.prototype.click.toString = function () {
      return nativeAnchorClick.toString();
    };
  }

  function installBlobMediaSrcHook(ctor, tagName) {
    const descriptor = Object.getOwnPropertyDescriptor(ctor?.prototype || {}, "src");
    if (!descriptor?.set || !descriptor?.get) {
      return;
    }

    Object.defineProperty(ctor.prototype, "src", {
      configurable: true,
      enumerable: descriptor.enumerable,
      get: descriptor.get,
      set(value) {
        descriptor.set.call(this, value);
        try {
          notifyElementBlobUrl(this, value, `${tagName}-src`);
        } catch {
          // Keep native src assignment behavior.
        }
      }
    });
  }

  installBlobMediaSrcHook(globalThis.HTMLMediaElement, "media");
  installBlobMediaSrcHook(globalThis.HTMLImageElement, "image");

  const nativeSetAttribute = Element.prototype.setAttribute;
  Element.prototype.setAttribute = function nxToolsSetAttribute(name, value) {
    const result = nativeSetAttribute.apply(this, arguments);
    try {
      if (String(name || "").toLowerCase() === "src") {
        notifyElementBlobUrl(this, value, "set-attribute-src");
      }
    } catch {
      // Preserve native setAttribute behavior.
    }
    return result;
  };
  Element.prototype.setAttribute.toString = function () {
    return nativeSetAttribute.toString();
  };

  document.addEventListener("click", (event) => {
    try {
      const anchor = event.target?.closest?.("a[href]");
      dispatchBlobDownloadRequest(anchor, "click-event", event);
    } catch {
      // Click interception is best-effort.
    }
  }, true);

  function scanInlineScripts() {
    try {
      document.querySelectorAll("script:not([src])").forEach((script) => {
        if (script.textContent) {
          inspectMediaText(script.textContent, location.href, "inline-script");
        }
      });
    } catch {
      // Ignore locked-down documents.
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      scanInlineScripts();
      inspectYoutubePlayerResponse("dom-ready");
    }, { once: true });
  } else {
    scanInlineScripts();
    inspectYoutubePlayerResponse("initial");
  }

  document.addEventListener("yt-navigate-finish", () => {
    setTimeout(() => inspectYoutubePlayerResponse("navigate"), 400);
  }, true);
  setInterval(() => inspectYoutubePlayerResponse("interval"), 5000);
})();
