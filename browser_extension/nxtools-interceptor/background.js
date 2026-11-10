"use strict";

// Background dependencies. Chrome's MV3 service worker loads them here; Firefox's
// event page has no importScripts and loads the same list (dependencies first,
// this entry last) from manifest background.scripts. Keep both lists identical.
const BACKGROUND_DEPENDENCIES = [
  "handoff-policy.js",
  "capture-policy.js",
  "blob-transfer-policy.js",
  "batch-handoff-policy.js",
  "bg/constants.js",
  "bg/state.js",
  "bg/naming.js",
  "bg/request-context.js",
  "bg/settings.js",
  "bg/native-transport.js",
  "bg/site-adapters.js",
  "bg/resource-cache.js",
  "bg/media.js",
  "bg/tab-bridge.js",
  "bg/capture.js",
  "bg/handoff.js",
  "bg/resource-choices.js",
  "bg/popup-state.js",
  "bg/context-menu.js",
  "bg/router.js"
];
if (typeof importScripts === "function") {
  importScripts(...BACKGROUND_DEPENDENCIES);
}

chrome.runtime.onInstalled.addListener(() => {
  createContextMenus();
  setBadge("", "#138a72");
});

chrome.runtime.onStartup.addListener(() => {
  createContextMenus();
  void loadPersistentResources();
});

registerHeaderCapture();
void getSettings();
void getCapturePaused();
void loadPersistentResources();
void ensureDispositionNamesRestored();
void activeTab().then((tab) => {
  if (tab?.id && /^https?:/i.test(tab.url || "")) {
    void injectContentBridge(tab.id, tab.url || "");
  }
});

chrome.tabs.onActivated.addListener((activeInfo) => {
  lastActiveTabId = activeInfo.tabId;
  schedulePersistResources();
  void injectContentBridge(activeInfo.tabId);
});

chrome.windows.onFocusChanged.addListener((windowId) => {
  if (windowId !== chrome.windows.WINDOW_ID_NONE) {
    void activeTab().then((tab) => {
      if (tab?.id && /^https?:/i.test(tab.url || "")) {
        void injectContentBridge(tab.id, tab.url || "");
      }
    });
  }
});

chrome.tabs.onRemoved.addListener((tabId) => {
  clearTab(tabId);
});

chrome.tabs.onReplaced?.addListener((_addedTabId, removedTabId) => {
  clearTab(removedTabId);
});

chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName === "session" && changes[CAPTURE_PAUSE_STORAGE_KEY]) {
    capturePaused = changes[CAPTURE_PAUSE_STORAGE_KEY].newValue === true;
    capturePauseLoaded = true;
    capturePauseAvailable = true;
  }
  if (areaName !== "local") {
    return;
  }

  for (const settingName of Object.keys(DEFAULT_SETTINGS)) {
    if (changes[settingName]) {
      cachedSettings[settingName] = changes[settingName].newValue ?? DEFAULT_SETTINGS[settingName];
      settingsLoaded = true;
    }
  }
  cachedSettings = sanitizeSettings(cachedSettings);

  if (changes[STORAGE_DISABLED_SITES_KEY]) {
    loadDisabledSites(changes[STORAGE_DISABLED_SITES_KEY].newValue);
  }

  if (!changes.mediaOverlayEnabled) {
    return;
  }

  const enabled = changes.mediaOverlayEnabled.newValue !== false;
  chrome.tabs.query({}, (tabs) => {
    for (const tab of tabs) {
      if (!tab.id || !/^https?:/i.test(tab.url || "")) {
        continue;
      }
      void sendMessageToTab(tab.id, {
        type: "media_overlay_set_enabled",
        enabled
      }).then((result) => {
        if (!result.ok && enabled) {
          void injectContentBridge(tab.id, tab.url || "").then(() => sendMessageToTab(tab.id, {
            type: "media_overlay_set_enabled",
            enabled
          }));
        }
      });
    }
  });
});

if (chrome.webNavigation?.onCommitted) {
  chrome.webNavigation.onCommitted.addListener((details) => {
    if (details.frameId === 0) {
      clearTab(details.tabId);
      if (/^https?:\/\//i.test(details.url || "")) {
        void injectContentBridge(details.tabId, details.url);
      }
    }

    if (/^https?:\/\//i.test(details.url || "")) {
      void injectPageHook(details.tabId, details.frameId, details.url);
    }
  });
}

if (chrome.webNavigation?.onHistoryStateUpdated) {
  chrome.webNavigation.onHistoryStateUpdated.addListener((details) => {
    if (details.frameId === 0) {
      clearTabResources(details.tabId);
      if (/^https?:\/\//i.test(details.url || "")) {
        void injectContentBridge(details.tabId, details.url);
      }
    }

    if (/^https?:\/\//i.test(details.url || "")) {
      void injectPageHook(details.tabId, details.frameId, details.url);
    }
  });
}

const determiningFilenameEvent = chrome.downloads?.["onDetermining" + "Filename"];
if (determiningFilenameEvent?.addListener) {
  determiningFilenameEvent.addListener((downloadItem, suggest) => {
    void interceptBrowserDownload(downloadItem, { suggest });
    // suggest is resolved asynchronously after the handoff settles; Chrome
    // requires the listener to return true or it auto-suggests immediately.
    return true;
  });
} else if (chrome.downloads.onCreated?.addListener) {
  chrome.downloads.onCreated.addListener((downloadItem) => {
    void interceptBrowserDownload(downloadItem, { eraseBrowserHistory: true });
  });
}
