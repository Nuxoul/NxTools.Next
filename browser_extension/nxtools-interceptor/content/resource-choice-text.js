"use strict";

// Ranking and display text for the page media resource chooser.
// Isolated-world content module: registered once on the extension-private
// NxToolsContent namespace (re-injection is a no-op). Holds no shared mutable
// page state; content-script.js owns all overlay/drop/capture state.
(function installNxToolsContentChoices(content) {
  if (content.choices) {
    return;
  }

  const { runtimeMessage } = content.runtime;
  const { filenameFromUrl, mediaDimensions, currentPageKey, candidateMediaInfo, formatBytes } = content.media;

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

  content.choices = Object.freeze({
    normalizeChoiceKey,
    rankResourceChoice,
    backgroundResourceChoiceCandidates,
    resourceChoiceLabel,
    resourceChoiceMeta,
    resourceChoiceQuality,
    resourceChoiceTypeText,
    resourceChoiceSizeText,
    resourceChoiceSourceText,
    resourceChoiceFileText,
    resourceChoiceTrackText,
    resourceChoiceMetaText,
    resourceChoiceDetail
  });
})(globalThis.NxToolsContent || (globalThis.NxToolsContent = {}));
