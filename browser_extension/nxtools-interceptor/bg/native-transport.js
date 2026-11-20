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

const CONNECTION_FAILURES = Object.freeze({
  host_not_registered: ["原生宿主未注册", "启动一次 NxTools 完成浏览器接管注册，然后重新加载扩展并测试连接。"],
  host_forbidden: ["扩展连接未获允许", "在 NxTools 的浏览器扩展设置中重新应用注册，然后重新加载扩展。"],
  host_start_failed: ["原生宿主启动失败", "确认 NxTools 安装完整；启动桌面端重新注册后，再测试连接。"],
  connection_timeout: ["连接 NxTools 超时", "等待桌面端启动完成，再点击测试连接；浏览器原下载会保留。"],
  connection_lost: ["与 NxTools 的连接已断开", "确认桌面端可用后点击测试连接，定时刷新不会自行重连。"],
  native_rejected: ["NxTools 拒绝了连接或请求", "确认桌面端与扩展均已更新，再重新加载扩展并测试连接。"],
  desktop_unavailable: ["NxTools 未运行", "启动 NxTools；若已在运行，请重新应用浏览器接管注册并测试连接。"]
});

function nativeConnectionCode(error, fallback = "desktop_unavailable") {
  if (Object.hasOwn(CONNECTION_FAILURES, error?.connectionCode)) return error.connectionCode;
  if (error?.handoffReason === "handoff_timeout") return "connection_timeout";
  if (error?.handoffReason === "handoff_disconnected") return "connection_lost";
  const message = String(error?.message || error || "").toLowerCase();
  if (/not found|no such native application|not registered/.test(message)) return "host_not_registered";
  if (/forbidden|not allowed|permission denied/.test(message)) return "host_forbidden";
  if (/failed to start|failed to launch|not executable/.test(message)) return "host_start_failed";
  if (/native host has exited|native application.*disconnected/.test(message)) return "connection_lost";
  return fallback;
}

function disconnectedStatus() {
  const [message, guidance] = CONNECTION_FAILURES[connectionFailureCode] || CONNECTION_FAILURES.desktop_unavailable;
  return { ok: false, code: connectionFailureCode, message, guidance, transport: "native_messaging" };
}

function desktopHandoffError(message, reason, boundaryCode, connectionCode = "") {
  const error = new Error(message);
  error.handoffReason = reason;
  error.boundaryCode = boundaryCode;
  if (connectionCode) error.connectionCode = connectionCode;
  return error;
}

function rejectPendingDesktopRequests(message) {
  for (const [requestId, pending] of pendingDesktopRequests) {
    clearTimeout(pending.timeoutId);
    pending.reject(desktopHandoffError(message, "handoff_disconnected", "port_disconnected"));
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

  // Only fixed handoff metadata crosses the diagnostics boundary; resource payloads do not.

  // Diagnostics never launch or retain an otherwise idle native host.
  if (!desktopPort || connectionState !== "connected") return;
  try {
    desktopPort.postMessage(NxToolsHandoffPolicy.buildExtensionLogRequest({
      protocolVersion: BROWSER_PROTOCOL_VERSION,
      level,
      event,
      message: trimLogText(message, 240),
      ...(event === "handoff_state" ? {
        handoffState: details.state,
        reason: details.reason,
        elapsedMs: details.elapsedMs,
        boundaryCode: details.boundaryCode,
        compensation: details.compensation
      } : {}),
      ...(["capture_pause_started", "capture_pause_applied", "capture_pause_failed"].includes(event) || NxToolsHandoffPolicy.isWorkflowLogEvent(event)
        ? { reason: details.reason, elapsedMs: details.elapsedMs } : {}),
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

  const requestId = String(message.requestId || "");
  if (!requestId || !pendingDesktopRequests.has(requestId)) {
    return;
  }

  const pending = pendingDesktopRequests.get(requestId);
  clearTimeout(pending.timeoutId);
  pendingDesktopRequests.delete(requestId);

  if (message.ok === false) {
    const reason = ["desktop_unavailable", "handoff_timeout", "handoff_disconnected", "identity_mismatch"]
      .includes(message.data?.reason) ? message.data.reason : "desktop_refused";
    const error = desktopHandoffError("NxTools 拒绝了连接或请求", reason, "native_rejected",
      reason === "desktop_unavailable" ? "desktop_unavailable" : "native_rejected");
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
    let port;
    try { port = chrome.runtime.connectNative(NATIVE_HOST_NAME); } catch (error) {
      connectionState = "disconnected";
      connectionFailureCode = nativeConnectionCode(error);
      reject(desktopHandoffError(disconnectedStatus().message, "desktop_unavailable", "connect_failed", connectionFailureCode));
      return;
    }
    desktopPort = port;
    let settled = false;

    const fail = (message, reason = "desktop_unavailable", boundaryCode = "connect_failed", code = "desktop_unavailable") => {
      if (desktopPort !== port) return;
      clearTimeout(openTimeout);
      desktopPort = null;
      connectionState = "disconnected";
      connectionFailureCode = code;
      connectionMessage = disconnectedStatus().message;
      desktopConnectPromise = null;
      if (!settled) {
        settled = true;
        reject(desktopHandoffError(connectionMessage, reason, boundaryCode, code));
      }
      try { port.disconnect(); } catch { /* Already disconnected. */ }
    };

    const openTimeout = setTimeout(() => {
      fail("连接 NxTools Native Messaging 超时", "handoff_timeout", "connect_timeout", "connection_timeout");
    }, DESKTOP_CONNECT_TIMEOUT_MS);

    port.onMessage.addListener((message) => {
      if (desktopPort !== port) return;
      if (!settled && message?.requestId === "native-hello" && message?.ok === false) {
        fail("NxTools 拒绝了连接或请求", "desktop_refused", "native_rejected", "native_rejected");
        return;
      }
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
      connectionFailureCode = nativeConnectionCode(runtimeError, settled ? "connection_lost" : "desktop_unavailable");
      connectionMessage = disconnectedStatus().message;
      rejectPendingDesktopRequests("与 NxTools 的 Native Messaging 连接已断开");
      if (!settled) {
        settled = true;
        reject(desktopHandoffError(connectionMessage, "desktop_unavailable", "connect_failed", connectionFailureCode));
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
      fail("无法连接 NxTools Native Messaging", "desktop_unavailable", "connect_failed", nativeConnectionCode(error));
    }
  });

  const connecting = desktopConnectPromise;
  try { return await connecting; } catch (error) {
    if (desktopConnectPromise === connecting) desktopConnectPromise = null;
    throw error;
  }
}

async function sendDesktopRequest(type, payload = {}, timeoutMs = 12000, requestId = nextRequestId()) {
  const releasePort = retainDesktopPort();
  try {
  const port = await ensureDesktopConnection();
  if (!port) {
    throw desktopHandoffError("NxTools Native Messaging 未连接", "desktop_unavailable", "connect_failed");
  }

  const message = {
    protocolVersion: BROWSER_PROTOCOL_VERSION,
    type,
    requestId,
    ...payload
  };
  if (new TextEncoder().encode(JSON.stringify(message)).byteLength > MAX_NATIVE_REQUEST_BYTES) {
    throw desktopHandoffError("发送到 NxTools 的消息超过 256 KiB 上限", "desktop_refused", "request_too_large");
  }
  if (type === "create_task" && lastHandoff?.correlationId === requestId && lastHandoff.state === "submitting") {
    emitBrowserHandoffState(lastHandoff);
  }

  return await new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      pendingDesktopRequests.delete(requestId);
      reject(desktopHandoffError("等待 NxTools Native Messaging 响应超时", "handoff_timeout", "request_timeout"));
    }, timeoutMs);

    pendingDesktopRequests.set(requestId, { resolve, reject, timeoutId });
    try { port.postMessage(message); } catch {
      clearTimeout(timeoutId);
      pendingDesktopRequests.delete(requestId);
      reject(desktopHandoffError("NxTools Native Messaging 发送失败", "handoff_disconnected", "send_failed"));
    }
  });
  } finally {
    releasePort();
  }
}
