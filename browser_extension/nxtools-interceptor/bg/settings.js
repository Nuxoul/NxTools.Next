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
  return next;
}

async function getSettings() {
  if (settingsLoaded) {
    return cachedSettings;
  }
  if (!settingsLoadPromise) {
    settingsLoadPromise = storageGet(DEFAULT_SETTINGS).then((stored) => {
      cachedSettings = {
        ...DEFAULT_SETTINGS,
        ...stored
      };
      settingsLoaded = true;
      return cachedSettings;
    }).finally(() => {
      settingsLoadPromise = null;
    });
  }
  return settingsLoadPromise;
}

function setBadge(text, color) {
  const action = actionApi();
  action?.setBadgeText?.({ text });
  action?.setBadgeBackgroundColor?.({ color });
}
