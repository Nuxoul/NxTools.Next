"use strict";

// Explicit, page-context media inspection. The caller supplies one captured URL;
// this module never follows manifest children, keys, variants or segments.
(function installNxToolsContentMediaInspection(content) {
  if (content.mediaInspection) return;

  const MAX_BODY_BYTES = 2 * 1024 * 1024;
  const UNKNOWN_SNIFF_BYTES = 4096;
  const INSPECTION_TIMEOUT_MS = 5000;
  const TIMEOUT = Object.freeze({ timeout: true });

  function fixedResult(state, kind, values = {}) {
    const reason = state === "ready" ? "ready" : state === "protected" ? "protected" : values.reason || "invalid_response";
    const messages = {
      ready: kind === "hls" ? "HLS 清单已检查" : kind === "dash" ? "DASH 清单已检查" :
        kind === "audio" ? "音频资源可用" : kind === "video" ? "视频资源可用" : "文件资源可用",
      protected: kind === "hls" ? "检测到受保护的 HLS 媒体" : "检测到受保护的 DASH 媒体",
      page_changed: "页面已变化，请重新选择资源",
      timeout: "媒体检查超时，请重试",
      too_large: "媒体清单超过 2 MiB，未继续读取",
      unavailable: "当前页面无法检查此资源",
      invalid_response: "页面返回的媒体信息无效",
      unsupported: "无法识别此媒体资源"
    };
    const message = state === "ready" ? messages.ready : state === "protected" ? messages.protected : messages[reason];
    return { ok: state === "ready" || state === "protected", inspection: {
      state, kind, requiresMerge: Boolean(values.requiresMerge), encrypted: Boolean(values.encrypted),
      live: Boolean(values.live), message, reason
    }, message };
  }

  function manifestKind(mime, url, prefix = "") {
    const type = String(mime || "").split(";", 1)[0].trim().toLowerCase();
    const start = String(prefix || "").replace(/^\uFEFF/, "").trimStart();
    if (type.includes("mpegurl") || /\.m3u8(?:[?#]|$)/i.test(url) || start.startsWith("#EXTM3U")) return "hls";
    if (type.includes("dash+xml") || /\.mpd(?:[?#]|$)/i.test(url) || /^<\?xml[\s\S]*?<MPD\b|^<MPD\b/i.test(start)) return "dash";
    return "";
  }

  function hlsAttribute(line, name) {
    const match = String(line || "").match(new RegExp(`(?:^|,)\\s*${name}=((?:\"[^\"]*\")|[^,]*)`, "i"));
    return String(match?.[1] || "").trim().replace(/^\"|\"$/g, "");
  }

  function parseHlsInspection(text) {
    const body = String(text || "").replace(/^\uFEFF/, "").trimStart();
    if (!body.startsWith("#EXTM3U")) return fixedResult("failed", "hls", { reason: "invalid_response" });
    const lines = body.split(/\r?\n/).map(line => line.trim());
    const master = lines.some(line => /^#EXT-X-(?:STREAM-INF|I-FRAME-STREAM-INF|MEDIA):/i.test(line));
    let encrypted = false;
    let protectedMedia = false;
    for (const line of lines.filter(value => /^#EXT-X-(?:KEY|SESSION-KEY):/i.test(value))) {
      const attributes = line.slice(line.indexOf(":") + 1);
      const method = hlsAttribute(attributes, "METHOD").toUpperCase();
      const keyFormat = hlsAttribute(attributes, "KEYFORMAT").toLowerCase();
      if (method && method !== "NONE") encrypted = true;
      if (method.startsWith("SAMPLE-AES") || (keyFormat && keyFormat !== "identity")) protectedMedia = true;
    }
    return fixedResult(protectedMedia ? "protected" : "ready", "hls", {
      encrypted, live: !master && !lines.some(line => /^#EXT-X-ENDLIST\b/i.test(line)), requiresMerge: false
    });
  }

  function dashFactsFromDom(text) {
    if (typeof DOMParser !== "function") return null;
    const xml = new DOMParser().parseFromString(text, "application/xml");
    const errors = xml.getElementsByTagName?.("parsererror") || [];
    if (errors.length > 0) return { invalid: true };
    const root = xml.documentElement;
    if (!root || String(root.localName || root.nodeName).toLowerCase() !== "mpd") return { invalid: true };
    const all = [...(xml.getElementsByTagName?.("*") || [])];
    const adaptations = all.filter(node => String(node.localName || node.nodeName).toLowerCase() === "adaptationset");
    const protectedMedia = all.some(node => String(node.localName || node.nodeName).toLowerCase() === "contentprotection");
    let video = false, audio = false;
    for (const adaptation of adaptations) {
      const descendants = [adaptation, ...(adaptation.getElementsByTagName?.("*") || [])];
      const types = descendants.map(node => `${node.getAttribute?.("contentType") || ""} ${node.getAttribute?.("mimeType") || ""}`.toLowerCase());
      video ||= types.some(value => /(?:^|\s)video(?:\/|\s|$)/.test(value));
      audio ||= types.some(value => /(?:^|\s)audio(?:\/|\s|$)/.test(value));
    }
    return { protectedMedia, video, audio, live: String(root.getAttribute?.("type") || "").toLowerCase() === "dynamic" };
  }

  function dashFactsFallback(text) {
    const root = String(text || "").match(/<MPD\b([^>]*)>/i);
    if (!root) return { invalid: true };
    const adaptations = [...String(text).matchAll(/<AdaptationSet\b([^>]*)>([\s\S]*?)<\/AdaptationSet\s*>|<AdaptationSet\b([^>]*)\/>/gi)];
    let video = false, audio = false;
    for (const match of adaptations) {
      const section = `${match[1] || match[3] || ""} ${match[2] || ""}`.toLowerCase();
      video ||= /(?:contenttype|mimetype)\s*=\s*["']video(?:\/[^"']*)?["']/.test(section);
      audio ||= /(?:contenttype|mimetype)\s*=\s*["']audio(?:\/[^"']*)?["']/.test(section);
    }
    return { protectedMedia: /<ContentProtection\b/i.test(text), video, audio,
      live: /\btype\s*=\s*["']dynamic["']/i.test(root[1]) };
  }

  function parseDashInspection(text) {
    const facts = dashFactsFromDom(text) || dashFactsFallback(text);
    if (facts.invalid) return fixedResult("failed", "dash", { reason: "invalid_response" });
    return fixedResult(facts.protectedMedia ? "protected" : "ready", "dash", {
      requiresMerge: facts.video && facts.audio, encrypted: facts.protectedMedia, live: facts.live
    });
  }

  function cancelBody(body, reader) {
    try {
      const pending = reader?.cancel ? reader.cancel() : body?.cancel?.();
      pending?.catch?.(() => {});
    } catch { /* Release is best effort after a terminal summary. */ }
  }

  async function inspectMediaResource(url, options = {}) {
    const expectedPageKey = String(options.pageKey || "");
    const pageMatches = () => !expectedPageKey || content.media.currentPageKey() === expectedPageKey;
    if (!/^https?:\/\//i.test(String(url || ""))) return fixedResult("failed", "unknown", { reason: "unsupported" });
    if (!pageMatches()) return fixedResult("failed", "unknown", { reason: "page_changed" });

    const controller = new AbortController();
    let timeoutId = 0, response = null, reader = null, readerDone = false;
    let timedOut = false;
    const timeoutPromise = new Promise(resolve => {
      timeoutId = setTimeout(() => { timedOut = true; controller.abort(); resolve(TIMEOUT); }, INSPECTION_TIMEOUT_MS);
    });
    const withinDeadline = promise => Promise.race([Promise.resolve(promise), timeoutPromise]);
    try {
      response = await withinDeadline(fetch(String(url), {
        method: "GET", credentials: options.includeCredentials === true ? "include" : "omit",
        cache: "no-store", redirect: "follow", signal: controller.signal
      }));
      if (response === TIMEOUT || timedOut) return fixedResult("failed", "unknown", { reason: "timeout" });
      if (!pageMatches()) return fixedResult("failed", "unknown", { reason: "page_changed" });
      if (!response || response.ok !== true || !response.headers?.get) return fixedResult("failed", "unknown", { reason: "unavailable" });

      const responseMime = String(response.headers.get("content-type") || "").split(";", 1)[0].trim().toLowerCase();
      const effectiveMime = responseMime || String(options.mime || "").split(";", 1)[0].trim().toLowerCase();
      const initialKind = manifestKind(effectiveMime, url);
      if (!initialKind && (effectiveMime.startsWith("video/") || effectiveMime.startsWith("audio/"))) {
        cancelBody(response.body, null);
        response = null;
        return fixedResult("ready", effectiveMime.startsWith("audio/") ? "audio" : "video");
      }

      const lengthHeader = response.headers.get("content-length");
      const contentLength = lengthHeader == null || lengthHeader === "" ? 0 : Number(lengthHeader);
      if (Number.isFinite(contentLength) && contentLength > MAX_BODY_BYTES) {
        cancelBody(response.body, null);
        response = null;
        return fixedResult("failed", initialKind || "unknown", { reason: "too_large" });
      }
      if (!response.body?.getReader) return fixedResult("failed", initialKind || "unknown", { reason: "invalid_response" });

      reader = response.body.getReader();
      const chunks = [];
      let bytesRead = 0, kind = initialKind;
      while (true) {
        const item = await withinDeadline(reader.read());
        if (item === TIMEOUT || timedOut) return fixedResult("failed", kind || "unknown", { reason: "timeout" });
        if (!pageMatches()) return fixedResult("failed", kind || "unknown", { reason: "page_changed" });
        if (!item || item.done) { readerDone = true; break; }
        const chunk = item.value instanceof Uint8Array ? item.value : new Uint8Array(item.value || 0);
        bytesRead += chunk.byteLength;
        if (bytesRead > MAX_BODY_BYTES) return fixedResult("failed", kind || "unknown", { reason: "too_large" });
        chunks.push(chunk);
        if (!kind) {
          let sniffRemaining = UNKNOWN_SNIFF_BYTES;
          const sniffChunks = [];
          for (const value of chunks) {
            if (sniffRemaining <= 0) break;
            sniffChunks.push(value.subarray(0, sniffRemaining));
            sniffRemaining -= value.byteLength;
          }
          const sniff = new TextDecoder().decode(sniffChunks.length === 1 ? sniffChunks[0] :
            Uint8Array.from(sniffChunks.flatMap(value => [...value])));
          kind = manifestKind(effectiveMime, url, sniff);
          if (!kind && bytesRead >= UNKNOWN_SNIFF_BYTES) return fixedResult("ready", "file");
        }
      }
      if (!kind) return effectiveMime && !effectiveMime.startsWith("text/")
        ? fixedResult("ready", "file")
        : fixedResult("failed", "unknown", { reason: "unsupported" });
      const bodyBytes = new Uint8Array(bytesRead);
      let offset = 0;
      for (const chunk of chunks) { bodyBytes.set(chunk, offset); offset += chunk.byteLength; }
      const text = new TextDecoder().decode(bodyBytes);
      return kind === "hls" ? parseHlsInspection(text) : parseDashInspection(text);
    } catch {
      return fixedResult("failed", "unknown", { reason: timedOut ? "timeout" : "unavailable" });
    } finally {
      clearTimeout(timeoutId);
      if (reader && !readerDone) cancelBody(null, reader);
      else if (!reader && response?.body) cancelBody(response.body, null);
    }
  }

  content.mediaInspection = Object.freeze({ inspectMediaResource, parseHlsInspection, parseDashInspection });
})(globalThis.NxToolsContent || (globalThis.NxToolsContent = {}));
