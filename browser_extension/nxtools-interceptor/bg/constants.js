"use strict";

// Static protocol limits, default settings and media/MIME tables.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

const NATIVE_HOST_NAME = "com.nxtools.next.browser";
const BROWSER_PROTOCOL_VERSION = 2;
const MAX_NATIVE_REQUEST_BYTES = 256 * 1024;

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
const EXTENSION_LOG_THROTTLE_LIMIT = 256;
const MAX_CAPTURED_URL_LENGTH = 16 * 1024;
const MAX_CAPTURED_TEXT_LENGTH = 1024;
const MAX_CAPTURED_HEADER_NAME_LENGTH = 128;
const MAX_CAPTURED_HEADER_VALUE_LENGTH = 16 * 1024;
const MAX_CAPTURED_HEADER_COUNT = 64;
const GOOGLEVIDEO_MIN_REMAINING_MS = 30 * 1000;
const GOOGLEVIDEO_MAX_FUTURE_MS = 14 * 24 * 60 * 60 * 1000;
const STORAGE_RESOURCE_KEY = "nxtools_resource_cache";
const STORAGE_LAST_TAB_KEY = "nxtools_last_active_tab";
const STORAGE_MOBILE_UA_TABS_KEY = "nxtools_mobile_ua_tabs";
const STORAGE_DISABLED_SITES_KEY = "nxtools_disabled_sites";

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

const LIKELY_MEDIA_URL_PATTERN = /(?:bilivideo|hdslb|upos-|mcdn|googlevideo|videoplayback|douyinvod|bytecdn|weibocdn|akamaized|cloudfront|dash|hls|segment|fragment|frag|chunk|playurl|play_url|playback|stream|manifest|playlist|media-video|media-audio|mime=video|mime=audio|mime_type=video|mime_type=audio|video_mp4|audio_mp4)/i;
