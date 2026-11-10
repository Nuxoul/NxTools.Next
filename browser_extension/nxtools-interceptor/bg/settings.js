"use strict";

// Site toggles, settings sanitization/loading and action badge.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

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

const DROP_TARGET_POSITIONS = new Set(["left-top", "left-bottom", "right-top", "right-bottom", "center-top", "center-bottom"]);

function clampSettingNumber(value, min, max, fallback) {
  const number = Number(value);
  return Number.isFinite(number) ? Math.round(Math.max(min, Math.min(max, number))) : fallback;
}

function captureRuleTokens(value, separator = /[\s,;]+/) {
  return [...new Set((typeof value === "string" ? value : "").slice(0, MAX_CAPTURED_TEXT_LENGTH)
    .split(separator).map(token => token.trim().toLowerCase()).filter(Boolean))];
}

// Settings written from the popup: only known keys, each with its declared type
// and range. Unknown keys and malformed values fall back to the defaults.
function sanitizeSettings(input) {
  const source = input && typeof input === "object" ? input : {};
  const next = { ...DEFAULT_SETTINGS };
  for (const [key, fallback] of Object.entries(DEFAULT_SETTINGS)) {
    if (typeof fallback === "boolean" && typeof source[key] === "boolean") {
      next[key] = source[key];
    }
  }
  const position = String(source.dropTargetPosition || "").toLowerCase();
  next.dropTargetPosition = DROP_TARGET_POSITIONS.has(position) ? position : DEFAULT_SETTINGS.dropTargetPosition;
  next.dropTargetWidth = clampSettingNumber(source.dropTargetWidth, 96, 360, DEFAULT_SETTINGS.dropTargetWidth);
  next.dropTargetHeight = clampSettingNumber(source.dropTargetHeight, 48, 220, DEFAULT_SETTINGS.dropTargetHeight);
  next.captureFileTypes = captureRuleTokens(source.captureFileTypes)
    .map(type => type.replace(/^\*?\./, "")).filter(Boolean).join(", ");
  next.captureMinimumKiB = clampSettingNumber(source.captureMinimumKiB, 0,
    Math.floor(Number.MAX_SAFE_INTEGER / 1024), DEFAULT_SETTINGS.captureMinimumKiB);
  next.captureExcludedSites = [...new Set(captureRuleTokens(source.captureExcludedSites)
    .map(site => siteHostKey(site.includes("://") ? site : `https://${site}`)).filter(Boolean))].join("\n");
  next.captureExcludedAddresses = captureRuleTokens(source.captureExcludedAddresses, /[\r\n]+/).join("\n");
  return next;
}

async function getSettings() {
  if (settingsLoaded) {
    return cachedSettings;
  }
  if (!settingsLoadPromise) {
    settingsLoadPromise = storageGet(DEFAULT_SETTINGS).then((stored) => {
      cachedSettings = sanitizeSettings({ ...DEFAULT_SETTINGS, ...stored });
      settingsLoaded = true;
      return cachedSettings;
    }).finally(() => {
      settingsLoadPromise = null;
    });
  }
  return settingsLoadPromise;
}

async function getCapturePaused() {
  if (capturePauseLoaded) return capturePaused;
  if (!chrome.storage?.session?.get) {
    capturePauseAvailable = false;
    return false;
  }
  if (!capturePauseLoadPromise) {
    capturePauseLoadPromise = chrome.storage.session.get({ [CAPTURE_PAUSE_STORAGE_KEY]: false })
      .then(stored => {
        capturePaused = stored[CAPTURE_PAUSE_STORAGE_KEY] === true;
        capturePauseLoaded = true;
        capturePauseAvailable = true;
        return capturePaused;
      }, () => {
        capturePauseAvailable = false;
        return true;
      }).finally(() => { capturePauseLoadPromise = null; });
  }
  return capturePauseLoadPromise;
}

async function setCapturePaused(paused) {
  await getCapturePaused();
  if (!chrome.storage?.session?.set) throw new Error("capture_pause_unavailable");
  await chrome.storage.session.set({ [CAPTURE_PAUSE_STORAGE_KEY]: paused });
  capturePaused = paused;
  capturePauseLoaded = true;
  capturePauseAvailable = true;
  return paused;
}

async function automaticCaptureDecision(item, settings) {
  if (!settings.interceptDownloads || await getCapturePaused()) {
    return { state: "skipped", reason: "temporarily_paused" };
  }
  const urls = [item.referrer, item.url, item.finalUrl].filter(value => typeof value === "string" && value);
  const sites = captureRuleTokens(settings.captureExcludedSites);
  const addresses = captureRuleTokens(settings.captureExcludedAddresses, /[\r\n]+/);
  if (!isSiteEnabled(item.referrer, item.finalUrl, item.url) ||
      urls.some(value => sites.includes(siteHostKey(value))) ||
      urls.some(value => addresses.some(part => value.toLowerCase().includes(part)))) {
    return { state: "skipped", reason: "rule_excluded" };
  }
  const types = captureRuleTokens(settings.captureFileTypes);
  if (types.length) {
    await ensureDispositionNamesRestored();
    const url = item.finalUrl || item.url;
    const name = dispositionFileNameForUrl(url) || dispositionFileNameForUrl(item.url) ||
      getFileNameFromPath(item.filename) || getFileNameFromUrl(url);
    const extension = getFileExtension(name) || extensionFromMime(item.mime || "");
    if (!types.some(type => type === extension || name.toLowerCase().endsWith(`.${type}`))) {
      return { state: "skipped", reason: "rule_excluded" };
    }
  }
  const size = Number(item.totalBytes);
  if (Number.isFinite(size) && size > 0 && size < settings.captureMinimumKiB * 1024) {
    return { state: "skipped", reason: "rule_excluded" };
  }
  return null;
}

function automaticCaptureFallback(outcome) {
  recordBrowserHandoff({ correlationId: nextRequestId(), startedAt: Date.now() }, outcome);
  return { ok: false, accepted: false, fallbackToBrowser: true,
    data: { handoffState: outcome.state, reason: outcome.reason },
    message: outcome.reason === "temporarily_paused"
      ? "自动接管已暂停，交还浏览器下载" : "自动接管规则已排除，交还浏览器下载" };
}

function setBadge(text, color) {
  const action = actionApi();
  action?.setBadgeText?.({ text });
  action?.setBadgeBackgroundColor?.({ color });
}
