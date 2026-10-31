"use strict";

// Static selectors, extension sets, YouTube itag table and overlay geometry constants.
// Isolated-world content module: registered once on the extension-private
// NxToolsContent namespace (re-injection is a no-op). Holds no shared mutable
// page state; content-script.js owns all overlay/drop/capture state.
(function installNxToolsContentConstants(content) {
  if (content.constants) {
    return;
  }

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

  content.constants = Object.freeze({
    OVERLAY_GAP,
    FLOATING_MEDIA_WIDTH,
    FLOATING_MEDIA_HEIGHT,
    MEDIA_CANDIDATE_TTL_MS,
    GOOGLEVIDEO_MIN_REMAINING_MS,
    GOOGLEVIDEO_MAX_FUTURE_MS,
    YOUTUBE_ITAG_INFO,
    INTERESTING_EXTENSIONS,
    DIRECT_DOWNLOAD_EXTENSIONS,
    OVERLAY_MEDIA_EXTENSIONS,
    MEDIA_SELECTOR,
    MEDIA_CONTAINER_SELECTOR
  });
})(globalThis.NxToolsContent || (globalThis.NxToolsContent = {}));
