"use strict";

// Popup pure formatting and classification helpers. No DOM, no messaging.

function formatBytes(value) {
  const bytes = Number(value || 0);
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "--";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = bytes;
  let index = 0;
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index += 1;
  }
  return `${size.toFixed(size >= 100 || index === 0 ? 0 : 1)} ${units[index]}`;
}

function relativeCaptureAge(value) {
  const timestamp = Number(value || 0);
  if (!timestamp) {
    return "";
  }
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 8) return "刚刚";
  if (seconds < 60) return `${seconds} 秒前`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} 分钟前`;
  return `${Math.floor(minutes / 60)} 小时前`;
}

function statusLabel(status) {
  const text = String(status || "").toLowerCase();
  if (text.includes("downloading")) return "下载中";
  if (text.includes("created") || text.includes("waiting")) return "等待";
  if (text.includes("queued")) return "排队";
  if (text.includes("paused")) return "暂停";
  if (text.includes("completed")) return "完成";
  if (text.includes("failed")) return "失败";
  if (text.includes("verifying")) return "校验";
  if (text.includes("finalizing")) return "收尾";
  if (text.includes("cancelled")) return "已取消";
  return status || "-";
}

// Semantic tone for the task status text: active, idle, error or done.
function statusTone(status) {
  const text = String(status || "").toLowerCase();
  if (text.includes("failed")) return "error";
  if (text.includes("completed")) return "done";
  if (text.includes("paused") || text.includes("cancelled") || text.includes("created") || text.includes("waiting")) return "idle";
  return "active";
}

function isCompletedStatus(status) {
  return String(status || "").toLowerCase().includes("completed");
}

function isActiveStatus(status) {
  const text = String(status || "").toLowerCase();
  return text.includes("downloading") || text.includes("waiting") || text.includes("queued") ||
    text.includes("finalizing") || text.includes("verifying") || text.includes("probing");
}

function resourceKind(resource) {
  return resource?.description?.kind || "download";
}

function isPreviewableResource(resource) {
  const kind = resourceKind(resource);
  const mime = String(resource?.mime || "").toLowerCase();
  return kind === "image" || kind === "audio" || kind === "video" ||
    mime.startsWith("image/") || mime.startsWith("audio/") || mime.startsWith("video/");
}

// Preview element kind for a previewable resource: image, audio or video.
function previewKind(resource) {
  const kind = resourceKind(resource);
  const mime = String(resource?.mime || "").toLowerCase();
  if (kind === "image" || mime.startsWith("image/")) return "image";
  if (kind === "audio" || mime.startsWith("audio/")) return "audio";
  if (kind === "video" || mime.startsWith("video/")) return "video";
  return "";
}

function isBlobResource(resource) {
  return String(resource?.url || "").startsWith("blob:");
}

function sourceLabel(source) {
  const text = String(source || "").trim();
  if (!text) {
    return "来源未知";
  }

  return text
    .replace(/^webRequest-/, "网络-")
    .replace(/^content-/, "页面-")
    .replace(/^performance:/, "性能:")
    .replace(/^xhr-response$/, "XHR 响应")
    .replace(/^fetch-response$/, "Fetch 响应");
}

function resourceKindLabel(kind) {
  switch (kind) {
    case "streaming":
      return "流媒体";
    case "video":
      return "视频";
    case "audio":
      return "音频";
    case "image":
      return "图片";
    case "download":
      return "文件";
    default:
      return kind || "资源";
  }
}

function resourceTrackHint(resource) {
  const text = [
    resource?.description?.kind,
    resource?.description?.badge,
    resource?.mime,
    resource?.url,
    resource?.filename
  ].join(" ").toLowerCase();

  if (text.includes("mime_type=audio") || text.includes("mime=audio") || text.includes("audio/") || text.includes("media-audio")) {
    return "通常为音频轨";
  }

  if (text.includes("mime_type=video") || text.includes("mime=video") || text.includes("video/") || text.includes("media-video")) {
    return "通常为仅视频轨";
  }

  if (text.includes(".m3u8") || text.includes("mpegurl")) {
    return "播放列表";
  }

  return "";
}

function compactUrl(url) {
  const text = String(url || "");
  if (!text) {
    return "";
  }

  if (text.startsWith("blob:")) {
    try {
      const inner = new URL(text.slice(5));
      const shortPath = inner.pathname.length > 54 ? `${inner.pathname.slice(0, 51)}...` : inner.pathname;
      return `blob:${inner.hostname}${shortPath}`;
    } catch {
      return text.length > 110 ? `${text.slice(0, 107)}...` : text;
    }
  }

  try {
    const parsed = new URL(text);
    return `${parsed.hostname}${parsed.pathname}`;
  } catch {
    return text.length > 110 ? `${text.slice(0, 107)}...` : text;
  }
}

function formatTime(value) {
  const seconds = Math.max(0, Math.floor(Number(value || 0)));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  if (hours > 0) {
    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
  }
  return `${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
}

function progressValue(task) {
  if (isCompletedStatus(task.status)) {
    return 100;
  }

  const total = Number(task.totalBytes || 0);
  const done = Number(task.downloadedBytes || 0);
  let value = 0;
  if (Number.isFinite(total) && total > 0 && Number.isFinite(done) && done >= 0) {
    value = (Math.min(done, total) / total) * 100;
  } else {
    const progress = Number(task.progress || 0);
    if (Number.isFinite(progress) && progress > 0) {
      value = progress <= 1 ? progress * 100 : progress;
    }
  }

  value = Math.max(0, Math.min(100, value));
  return isActiveStatus(task.status) && value >= 100 ? 99.5 : value;
}

function progressLabel(value, task) {
  if (isCompletedStatus(task.status)) {
    return "100%";
  }
  return `${Math.floor(Math.max(0, Math.min(99, value)))}%`;
}

function taskActionLabel(task) {
  const text = String(task.status || "").toLowerCase();
  if (text.includes("created") || text.includes("waiting") || text.includes("paused") || text.includes("failed")) {
    return { action: "resume", label: "继续" };
  }
  if (text.includes("downloading") || text.includes("queued") || text.includes("probing")) {
    return { action: "pause", label: "暂停" };
  }
  return null;
}

function isQueuedDesktopResult(result) {
  return Boolean(result?.data?.queued);
}
