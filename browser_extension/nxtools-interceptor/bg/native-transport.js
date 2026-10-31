"use strict";

// Demand-only Native Messaging transport, pending requests and diagnostics.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

function nextRequestId() {
  if (crypto.randomUUID) {
    return crypto.randomUUID();
  }
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = [...bytes].map((value) => value.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

async function browserDownloadHandoffId(downloadItem) {
  const identity = [
    String(downloadItem.id ?? ""),
    String(downloadItem.startTime || ""),
    String(downloadItem.finalUrl || downloadItem.url || "")
  ].join("\u0000");
  const digest = new Uint8Array(await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(identity)
  ));
  digest[6] = (digest[6] & 0x0f) | 0x50;
  digest[8] = (digest[8] & 0x3f) | 0x80;
  const hex = [...digest.slice(0, 16)]
    .map((value) => value.toString(16).padStart(2, "0"))
    .join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function rejectPendingDesktopRequests(message) {
  for (const [requestId, pending] of pendingDesktopRequests) {
    clearTimeout(pending.timeoutId);
    pending.reject(new Error(message));
    pendingDesktopRequests.delete(requestId);
  }
}

function cancelDesktopIdleRelease() {
  clearTimeout(desktopIdleTimer);
  desktopIdleTimer = 0;
}

function scheduleDesktopIdleRelease() {
  cancelDesktopIdleRelease();
  if (!desktopPort || desktopConnectPromise || desktopPortOwners || pendingDesktopRequests.size) return;
  const port = desktopPort;
  desktopIdleTimer = setTimeout(() => {
    desktopIdleTimer = 0;
    if (desktopPort === port && !desktopConnectPromise && !desktopPortOwners && !pendingDesktopRequests.size) {
      closeDesktopPort();
    }
  }, DESKTOP_IDLE_RELEASE_MS);
}

function retainDesktopPort() {
  cancelDesktopIdleRelease();
  desktopPortOwners += 1;
  let released = false;
  return () => {
    if (released) return;
    released = true;
    desktopPortOwners -= 1;
    scheduleDesktopIdleRelease();
  };
}

function closeDesktopPort() {
  cancelDesktopIdleRelease();
  const port = desktopPort;
  desktopPort = null;
  connectionState = "disconnected";
  connectionMessage = "NxTools 连接已空闲释放";
  if (port) {
    try { port.disconnect(); } catch { /* Already disconnected. */ }
  }
}

function trimLogText(value, maxLength = 1200) {
  const text = String(value ?? "").replace(/\s+/g, " ").trim();
  return text.length <= maxLength ? text : `${text.slice(0, Math.max(0, maxLength - 3))}...`;
}

function sendExtensionLog(
  level,
  event,
  message,
  details = {},
  throttleMs = EXTENSION_LOG_THROTTLE_MS,
  correlationId = ""
) {
  const key = `${trimLogText(level, 16)}:${trimLogText(event, 128)}:${trimLogText(details.reason, 128)}`;
  const now = Date.now();
  const last = extensionLogTimes.get(key) || 0;
  if (throttleMs > 0 && now - last < throttleMs) {
    return;
  }
  if (!extensionLogTimes.has(key) && extensionLogTimes.size >= EXTENSION_LOG_THROTTLE_LIMIT) {
    extensionLogTimes.delete(extensionLogTimes.keys().next().value);
  }
  extensionLogTimes.set(key, now);

  const payload = {
    level,
    area: "BrowserExtension",
    event,
    message,
    ...details
  };

  for (const name of ["url", "pageUrl", "resourceUrl"]) {
    if (payload[name]) {
      payload[name] = trimLogText(payload[name], 1200);
    }
  }

  // Diagnostics never launch or retain an otherwise idle native host.
  if (!desktopPort || connectionState !== "connected") return;
  try {
    desktopPort.postMessage(NxToolsHandoffPolicy.buildExtensionLogRequest({
      protocolVersion: BROWSER_PROTOCOL_VERSION,
      level: payload.level,
      event: payload.event,
      message: trimLogText(payload.message, 240),
      correlationId,
      allocateRequestId: nextRequestId,
      isRequestIdPending: (requestId) => pendingDesktopRequests.has(requestId)
    }));
  } catch { /* Best-effort diagnostics do not change feature outcome. */ }
  scheduleDesktopIdleRelease();
}

function resourceSamples(resources, limit = 5) {
  return (resources || [])
    .filter((resource) => isUsableGoogleVideoUrl(resource?.url || ""))
    .slice(0, limit)
    .map((resource) => resource.url || "")
    .filter(Boolean);
}

function logResourceCaptured(event, resource, extra = {}) {
  const info = describeResource(resource);
  sendExtensionLog("info", event, "捕获浏览器媒体/下载资源", {
    source: resource.source || extra.source || "",
    url: resource.url,
    pageUrl: resource.pageUrl || "",
    tabId: resource.tabId || 0,
    kind: info.kind,
    mime: resource.mime || "",
    extension: info.extension || "",
    count: resourcesByTab.get(resource.tabId)?.size || 0,
    statusCode: extra.statusCode || 0
  });
}

function handleDesktopMessage(message) {

  if (message.type === "hello_ack") {
    connectionState = "connected";
    connectionMessage = message.message || "Native Messaging 已连接 NxTools";
  }

  if (message.type === "snapshot") {
    lastSnapshot = message.data || null;
  }

  const requestId = String(message.requestId || "");
  if (!requestId || !pendingDesktopRequests.has(requestId)) {
    return;
  }

  const pending = pendingDesktopRequests.get(requestId);
  clearTimeout(pending.timeoutId);
  pendingDesktopRequests.delete(requestId);

  if (message.ok === false) {
    const error = new Error(message.message || "NxTools 返回失败");
    error.nativeRejected = true;
    pending.reject(error);
    return;
  }

  pending.resolve(message);
}

async function ensureDesktopConnection(force = false) {
  cancelDesktopIdleRelease();
  if (!force && desktopPort && connectionState === "connected") {
    return desktopPort;
  }

  if (!force && desktopConnectPromise) {
    if (Date.now() - desktopConnectStartedAt < DESKTOP_CONNECT_TIMEOUT_MS) {
      return desktopConnectPromise;
    }
    closeDesktopPort();
    desktopConnectPromise = null;
  }

  if (force) {
    closeDesktopPort();
  }

  connectionState = "connecting";
  connectionMessage = "正在连接 NxTools Native Messaging";
  desktopConnectStartedAt = Date.now();

  desktopConnectPromise = new Promise((resolve, reject) => {
    const port = chrome.runtime.connectNative(NATIVE_HOST_NAME);
    desktopPort = port;
    let settled = false;

    const fail = (message) => {
      if (desktopPort !== port) return;
      clearTimeout(openTimeout);
      try { port.disconnect(); } catch { /* Already disconnected. */ }
      if (desktopPort === port) {
        desktopPort = null;
      }
      connectionState = "disconnected";
      connectionMessage = message;
      desktopConnectPromise = null;
      if (!settled) {
        settled = true;
        reject(new Error(message));
      }
    };

    const openTimeout = setTimeout(() => {
      try {
        port.disconnect();
      } catch {
        // Ignore close failures.
      }
      fail("连接 NxTools Native Messaging 超时");
    }, DESKTOP_CONNECT_TIMEOUT_MS);

    port.onMessage.addListener((message) => {
      if (desktopPort !== port) return;
      handleDesktopMessage(message);
      if (!settled && message?.requestId === "native-hello" && message?.ok === true) {
        settled = true;
        clearTimeout(openTimeout);
        connectionState = "connected";
        connectionMessage = message.message || "Native Messaging 已连接 NxTools";
        desktopConnectPromise = null;
        resolve(port);
        scheduleDesktopIdleRelease();
      }
    });

    port.onDisconnect.addListener(() => {
      clearTimeout(openTimeout);
      if (desktopPort !== port) return;
      cancelDesktopIdleRelease();
      const runtimeError = chrome.runtime.lastError?.message || "";
      if (desktopPort === port) {
        desktopPort = null;
      }
      desktopConnectPromise = null;
      connectionState = "disconnected";
      connectionMessage = runtimeError || "Native Messaging 已断开";
      rejectPendingDesktopRequests("与 NxTools 的 Native Messaging 连接已断开");
      if (!settled) {
        settled = true;
        reject(new Error(connectionMessage));
      }
      // A future user request may reconnect; idle disconnect never restarts a host.
    });

    try {
      port.postMessage({
        protocolVersion: BROWSER_PROTOCOL_VERSION,
        type: "hello",
        requestId: "native-hello",
        extensionVersion: chrome.runtime.getManifest().version,
        clientKind: "browser_extension"
      });
    } catch (error) {
      clearTimeout(openTimeout);
      fail(error instanceof Error ? error.message : "无法连接 NxTools Native Messaging");
    }
  });

  return desktopConnectPromise;
}

async function sendDesktopRequest(type, payload = {}, timeoutMs = 12000, requestId = nextRequestId()) {
  const releasePort = retainDesktopPort();
  try {
  const port = await ensureDesktopConnection();
  if (!port) {
    throw new Error("NxTools Native Messaging 未连接");
  }

  const message = {
    protocolVersion: BROWSER_PROTOCOL_VERSION,
    type,
    requestId,
    ...payload
  };
  if (new TextEncoder().encode(JSON.stringify(message)).byteLength > MAX_NATIVE_REQUEST_BYTES) {
    throw new Error("发送到 NxTools 的消息超过 256 KiB 上限");
  }

  return await new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      pendingDesktopRequests.delete(requestId);
      reject(new Error("等待 NxTools Native Messaging 响应超时"));
    }, timeoutMs);

    pendingDesktopRequests.set(requestId, { resolve, reject, timeoutId });
    try { port.postMessage(message); } catch {
      clearTimeout(timeoutId);
      pendingDesktopRequests.delete(requestId);
      reject(new Error("NxTools Native Messaging 发送失败"));
    }
  });
  } finally {
    releasePort();
  }
}
