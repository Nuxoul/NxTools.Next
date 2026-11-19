"use strict";

// Programmable WebExtension API double for behavior tests. Every event is a real
// listener registry the test can fire; every call is recorded. No network, no
// native host, no browser profile.

function createEvent() {
  const listeners = [];
  const registrations = [];
  return {
    listeners,
    registrations,
    addListener(fn, filter, extra) { listeners.push(fn); registrations.push({ filter, extra }); },
    removeListener(fn) {
      const index = listeners.indexOf(fn);
      if (index >= 0) listeners.splice(index, 1);
    },
    hasListeners() { return listeners.length > 0; },
    fire(...args) { return listeners.map((fn) => fn(...args)); }
  };
}

function createNativePort(name, registry) {
  const port = {
    name,
    sent: [],
    closed: false,
    onMessage: createEvent(),
    onDisconnect: createEvent(),
    postMessage(message) {
      if (port.closed) throw new Error("port closed");
      port.sent.push(JSON.parse(JSON.stringify(message)));
      registry.onPost?.(port, message);
    },
    receive(message) { port.onMessage.fire(message); },
    disconnect() {
      if (port.closed) return;
      port.closed = true;
      port.onDisconnect.fire(port);
    },
    lastRequest(type) {
      return [...port.sent].reverse().find((message) => !type || message.type === type);
    }
  };
  return port;
}

function createChromeApi(options = {}) {
  const calls = [];
  const record = (name, args) => calls.push({ name, args });
  const nativePorts = [];
  const nativeRegistry = { onPost: null };
  const storageAreas = { local: { ...(options.storage || {}) }, session: { ...(options.sessionStorage || {}) } };
  const tabs = new Map((options.tabs || []).map((tab) => [tab.id, { ...tab }]));
  const tabMessages = [];
  let tabMessageHandler = options.tabMessageHandler || (() => ({ ok: true }));
  const extensionId = options.extensionId || "fixtureextensionid";
  const downloadItems = new Map((options.downloadItems || []).map((item) => [item.id, {
    state: "in_progress", paused: false, ...item
  }]));
  const downloadCreated = createEvent();
  const downloadDeterminingFilename = createEvent();
  const recordDownloadItem = (item) => {
    if (!item || !Number.isInteger(item.id)) return;
    downloadItems.set(item.id, { state: "in_progress", paused: false, ...item });
  };
  const fireCreated = downloadCreated.fire;
  downloadCreated.fire = (item, ...args) => {
    recordDownloadItem(item);
    return fireCreated(item, ...args);
  };
  const fireDeterminingFilename = downloadDeterminingFilename.fire;
  downloadDeterminingFilename.fire = (item, ...args) => {
    recordDownloadItem(item);
    return fireDeterminingFilename(item, ...args);
  };

  function storageArea(areaName) {
    const values = storageAreas[areaName];
    return {
      async get(keys) {
        record(`storage.${areaName}.get`, [keys]);
        if (areaName === "session" && options.sessionGetFails) throw new Error("synthetic session read failure");
        if (keys == null) return { ...values };
        if (typeof keys === "string") return keys in values ? { [keys]: values[keys] } : {};
        if (Array.isArray(keys)) {
          return Object.fromEntries(keys.filter((key) => key in values).map((key) => [key, values[key]]));
        }
        return Object.fromEntries(Object.entries(keys).map(([key, fallback]) => [key, key in values ? values[key] : fallback]));
      },
      async set(items) {
        record(`storage.${areaName}.set`, [items]);
        const changes = {};
        for (const [key, value] of Object.entries(items)) {
          changes[key] = { oldValue: values[key], newValue: value };
          values[key] = value;
        }
        api.storage.onChanged.fire(changes, areaName);
      },
      async remove(keys) {
        for (const key of [].concat(keys)) delete values[key];
      }
    };
  }

  const api = {
    runtime: {
      id: extensionId,
      lastError: null,
      getManifest: () => ({ version: "fixture", ...(options.manifest || {}) }),
      getURL: (file) => `chrome-extension://${extensionId}/${file}`,
      connectNative(name) {
        record("runtime.connectNative", [name]);
        const port = createNativePort(name, nativeRegistry);
        nativePorts.push(port);
        options.onNativeConnect?.(port);
        return port;
      },
      onMessage: createEvent(),
      onConnect: createEvent(),
      onInstalled: createEvent(),
      onStartup: createEvent()
    },
    storage: {
      local: storageArea("local"),
      session: storageArea("session"),
      onChanged: createEvent()
    },
    downloads: {
      onCreated: downloadCreated,
      onDeterminingFilename: downloadDeterminingFilename,
      async search(query = {}) {
        record("downloads.search", [query]);
        return [...downloadItems.values()]
          .filter((item) => query.id == null || item.id === query.id)
          .map((item) => ({ ...item }));
      },
      async pause(id) {
        record("downloads.pause", [id]);
        const item = downloadItems.get(id);
        if (!item) throw new Error(`No download with id: ${id}`);
        item.paused = true;
        // Firefox pause calls Download.cancel() while retaining partial data.
        if (options.firefox) Object.assign(item, { state: "interrupted", error: "USER_CANCELED" });
      },
      async resume(id) {
        record("downloads.resume", [id]);
        const item = downloadItems.get(id);
        if (!item) throw new Error(`No download with id: ${id}`);
        item.paused = false;
        item.state = "in_progress";
        delete item.error;
      },
      async cancel(id) {
        record("downloads.cancel", [id]);
        if (options.cancelFails) throw new Error("synthetic cancel failure");
        const item = downloadItems.get(id);
        if (!item) throw new Error(`No download with id: ${id}`);
        Object.assign(item, { state: "interrupted", error: "USER_CANCELED", paused: false });
      },
      async erase(query) {
        record("downloads.erase", [query]);
        const ids = [...downloadItems.values()]
          .filter((item) => query?.id == null || item.id === query.id)
          .map((item) => item.id);
        ids.forEach((id) => downloadItems.delete(id));
        return ids;
      },
      async download(details) { record("downloads.download", [details]); return 1; },
      ...(options.downloads || {})
    },
    tabs: {
      onActivated: createEvent(),
      onRemoved: createEvent(),
      onReplaced: createEvent(),
      query(queryInfo, callback) {
        record("tabs.query", [queryInfo]);
        const all = [...tabs.values()];
        const result = queryInfo?.active ? all.filter(tab => tab.active) : all;
        if (callback) { callback(result); return undefined; }
        return Promise.resolve(result);
      },
      async get(tabId) {
        const tab = tabs.get(tabId);
        if (!tab) throw new Error(`No tab with id: ${tabId}`);
        return { ...tab };
      },
      sendMessage(tabId, message, optionsOrCallback, maybeCallback) {
        const callback = typeof optionsOrCallback === "function" ? optionsOrCallback : maybeCallback;
        tabMessages.push({ tabId, message });
        const response = tabMessageHandler(tabId, message);
        if (callback) {
          Promise.resolve(response).then((value) => callback(value));
          return undefined;
        }
        return Promise.resolve(response);
      },
      async captureVisibleTab() { return "data:image/png;base64,AA=="; }
    },
    windows: { WINDOW_ID_NONE: -1, onFocusChanged: createEvent() },
    webNavigation: {
      onCommitted: createEvent(),
      onHistoryStateUpdated: createEvent(),
      getFrame({ tabId, frameId }, callback) {
        callback({ frameId, url: tabs.get(tabId)?.url || "", documentId: `${tabId}:${frameId}:document` });
      },
      getAllFrames(details, callback) {
        const result = [{ frameId: 0 }];
        if (callback) { callback(result); return undefined; }
        return Promise.resolve(result);
      }
    },
    webRequest: {
      onBeforeSendHeaders: createEvent(),
      onHeadersReceived: createEvent(),
      onErrorOccurred: createEvent(),
      onResponseStarted: createEvent(),
      onBeforeRedirect: createEvent()
    },
    contextMenus: {
      onClicked: createEvent(),
      removeAll(callback) { record("contextMenus.removeAll", []); callback?.(); },
      create(item) { record("contextMenus.create", [item]); }
    },
    cookies: { async getAll() { return []; } },
    scripting: {
      executeScript(details, callback) {
        record("scripting.executeScript", [details]);
        const result = [{ result: true }];
        if (callback) { callback(result); return undefined; }
        return Promise.resolve(result);
      }
    },
    declarativeNetRequest: {
      ResourceType: { MAIN_FRAME: "main_frame", XMLHTTPREQUEST: "xmlhttprequest" },
      async updateSessionRules(details) { record("declarativeNetRequest.updateSessionRules", [details]); }
    },
    action: {
      setBadgeText(details) { record("action.setBadgeText", [details]); },
      setBadgeBackgroundColor() {},
      openPopup() { record("action.openPopup", []); }
    }
  };

  if (options.firefox) {
    delete api.downloads.onDeterminingFilename;
    delete api.declarativeNetRequest;
  }

  return {
    api,
    calls,
    nativePorts,
    nativeRegistry,
    storage: storageAreas,
    tabs,
    tabMessages,
    setTabMessageHandler(handler) { tabMessageHandler = handler; },
    callsNamed(name) { return calls.filter((call) => call.name === name); },
    downloadItems,
    extensionId
  };
}

module.exports = { createChromeApi, createEvent };
