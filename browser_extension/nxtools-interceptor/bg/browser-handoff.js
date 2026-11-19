"use strict";

// Automatic browser download ownership. The session journal contains identities only.
const BROWSER_HANDOFF_SESSION_KEY = "nxtools_browser_handoffs";
const browserHandoffOperations = new Map();
const browserHandoffJournal = new Map();
let browserHandoffStorageWrite = Promise.resolve();
let browserHandoffRestore = null;
let browserHandoffJournalReady = false;
const BROWSER_CONFIRMATION_WAIT_MS = 10 * 60 * 1000;
const BROWSER_HANDOFF_LIMIT = 32;

function browserDownloadIsPaused(item) {
  return item?.paused === true && ["in_progress", "interrupted"].includes(item.state);
}

function browserDownloadIsCancelled(item) {
  return item?.state === "interrupted" && item.error === "USER_CANCELED" && item.paused !== true;
}

function persistBrowserHandoffs() {
  const records = [...browserHandoffJournal.values()].map(record => ({ ...record }));
  const write = browserHandoffStorageWrite.catch(() => {}).then(() => {
    if (!chrome.storage?.session?.set) throw new Error("handoff_session_unavailable");
    return chrome.storage.session.set({ [BROWSER_HANDOFF_SESSION_KEY]: records });
  });
  browserHandoffStorageWrite = write;
  return write;
}

async function updateBrowserHandoff(record, phase, fields = {}) {
  Object.assign(record, fields, { phase });
  browserHandoffJournal.set(record.downloadId, record);
  await persistBrowserHandoffs();
}

async function forgetBrowserHandoff(record) {
  if (browserHandoffJournal.get(record.downloadId) !== record) return;
  browserHandoffJournal.delete(record.downloadId);
  await persistBrowserHandoffs();
}

async function currentBrowserDownload(record) {
  const [item] = await chrome.downloads.search({ id: record.downloadId });
  if (!item || (record.browserStartedAt && item.startTime !== record.browserStartedAt)) return null;
  return item;
}

function controlledHandoffState(response, record) {
  if (!response || response.ok !== true || response.handoffId !== record.handoffId) {
    throw desktopHandoffError("浏览器交接回复身份不匹配", "identity_mismatch", "native_rejected");
  }
  const state = response.data?.browserHandoffState;
  if (!["pending", "held", "committed", "aborted", "expired", "missing"].includes(state) ||
      (["held", "committed"].includes(state) && (!response.accepted || !response.taskId ||
        (record.taskId && response.taskId !== record.taskId)))) {
    throw desktopHandoffError("浏览器交接回复无效", "identity_mismatch", "native_rejected");
  }
  return state;
}

function browserHandoffRequest(type, record) {
  return sendDesktopRequest(type, { payload: {
    handoffId: record.handoffId, ...(record.taskId ? { taskId: record.taskId } : {})
  } }, 12000, record.handoffId);
}

function controlledHandoffStatus(record, state, reason, details = {}) {
  recordBrowserHandoff({ correlationId: record.handoffId, startedAt: record.startedAt }, { state, reason, controlled: true, ...details });
}

async function abortControlledHandoff(record, reason) {
  try { await updateBrowserHandoff(record, "aborting"); } catch { record.phase = "aborting"; }
  let ended = !record.prepared;
  if (record.prepared) try {
    const response = await browserHandoffRequest("abort_browser_handoff", record);
    const state = controlledHandoffState(response, record);
    ended = ["aborted", "expired", "missing"].includes(state);
    if (state === "committed") {
      controlledHandoffStatus(record, "transferred", "browser_transferred");
      await forgetBrowserHandoff(record);
      return;
    }
  } catch { /* Uncommitted held tasks cannot transfer; a pending lease expires. */ }
  const item = record.kind === "response" ? null : await currentBrowserDownload(record);
  if (browserDownloadIsPaused(item)) {
    try { await chrome.downloads.resume(record.downloadId); }
    catch {
      controlledHandoffStatus(record, "failed", "browser_resume_failed");
      return;
    }
  }
  controlledHandoffStatus(record, "failed", reason, { compensation: ended ? "succeeded" : "failed" });
  if (ended || !record.prepared) await forgetBrowserHandoff(record);
}

async function commitControlledHandoff(record) {
  await updateBrowserHandoff(record, "browser_cancelled");
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const response = await browserHandoffRequest("commit_browser_handoff", record);
      if (controlledHandoffState(response, record) !== "committed" || response.ok !== true) {
        controlledHandoffStatus(record, "failed", "desktop_activation_pending");
        return;
      }
      controlledHandoffStatus(record, "transferred", "browser_transferred", { queued: response.data?.queued });
      if (record.eraseBrowserHistory) {
        try { await chrome.downloads.erase({ id: record.downloadId }); } catch { /* Ownership already transferred. */ }
      }
      await forgetBrowserHandoff(record);
      setBadge("OK", "#138a72");
      return;
    } catch {
      if (attempt < 2) await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  // The original is cancelled; retain the durable held identity for the next explicit demand.
  controlledHandoffStatus(record, "failed", "desktop_activation_pending");
}

async function advanceControlledHandoff(record, response = null) {
  const reply = response || await browserHandoffRequest("get_browser_handoff", record);
  const state = controlledHandoffState(reply, record);
  if (state === "committed") {
    controlledHandoffStatus(record, "transferred", "browser_transferred", { queued: reply.data?.queued });
    await forgetBrowserHandoff(record);
    return true;
  }
  if (["aborted", "expired", "missing"].includes(state)) {
    await abortControlledHandoff(record, state === "expired" ? "confirmation_timeout" : "confirmation_cancelled");
    return true;
  }
  // A resolved cancel is durable ownership evidence even if browser history was erased.
  if (record.phase === "browser_cancelled" && state === "held") {
    record.taskId = reply.taskId;
    await commitControlledHandoff(record);
    return true;
  }
  const item = await currentBrowserDownload(record);
  if (record.phase === "cancelling") {
    if (browserDownloadIsCancelled(item) && state === "held") {
      record.taskId = reply.taskId;
      await commitControlledHandoff(record);
      return true;
    }
    // Without a cancellation acknowledgment or an observable original, keep Held recoverable.
    if (!item) {
      controlledHandoffStatus(record, "failed", "browser_cancel_unconfirmed");
      return true;
    }
  }
  if (!item || !browserDownloadIsPaused(item)) {
    await abortControlledHandoff(record, "browser_control_failed");
    return true;
  }
  if (state === "pending") {
    if (record.phase !== "waiting_confirmation") {
      await updateBrowserHandoff(record, "waiting_confirmation");
      controlledHandoffStatus(record, "awaiting_confirmation", "confirmation_pending");
    }
    return false;
  }
  await updateBrowserHandoff(record, "cancelling", { taskId: reply.taskId });
  controlledHandoffStatus(record, "persisted", "desktop_accepted");
  try {
    await chrome.downloads.cancel(record.downloadId);
  } catch {
    // Resolve ownership from the observed original; an API error alone is not cancellation evidence.
  }
  const cancelled = await currentBrowserDownload(record);
  if (!cancelled) {
    controlledHandoffStatus(record, "failed", "browser_cancel_unconfirmed");
    return true;
  }
  if (!browserDownloadIsCancelled(cancelled)) {
    await abortControlledHandoff(record, "browser_cancel_failed");
    return true;
  }
  await commitControlledHandoff(record);
  return true;
}

async function followControlledHandoff(record, response = null) {
  let reply = response;
  while (true) {
    if (await advanceControlledHandoff(record, reply)) return;
    reply = null;
    if (Date.now() - record.startedAt >= BROWSER_CONFIRMATION_WAIT_MS) {
      await abortControlledHandoff(record, "confirmation_timeout");
      return;
    }
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
}

async function runControlledBrowserHandoff(downloadItem, context, requestContext, settings, options) {
  if (!browserHandoffJournalReady || browserHandoffJournal.size >= BROWSER_HANDOFF_LIMIT) {
    controlledHandoffStatus({ handoffId: context.correlationId, startedAt: context.startedAt }, "skipped", "browser_control_failed");
    return;
  }
  const release = retainDesktopPort();
  const record = { downloadId: downloadItem.id, handoffId: context.correlationId,
    browserStartedAt: downloadItem.startTime || "", phase: "pausing", taskId: "", prepared: false,
    startedAt: context.startedAt, eraseBrowserHistory: Boolean(settings.eraseBrowserHistory || options.eraseBrowserHistory) };
  try {
    await updateBrowserHandoff(record, "pausing");
    const original = await currentBrowserDownload(record);
    if (!original || original.state !== "in_progress" || original.paused) {
      await forgetBrowserHandoff(record);
      controlledHandoffStatus(record, "skipped", "browser_control_failed");
      return;
    }
    try {
      await chrome.downloads.pause(record.downloadId);
      const paused = await currentBrowserDownload(record);
      if (!paused || !browserDownloadIsPaused(paused)) throw new Error("not_paused");
    } catch {
      await abortControlledHandoff(record, "browser_control_failed");
      return;
    }
    try { await updateBrowserHandoff(record, "waiting", { prepared: true }); }
    catch (error) { record.prepared = false; throw error; }
    controlledHandoffStatus(record, "submitting", "handoff_started");
    const { response } = await sendDownloadItem(downloadItem, record.handoffId, requestContext, true);
    await followControlledHandoff(record, response);
  } catch (error) {
    if (["connect_failed", "connect_timeout", "request_too_large"].includes(error?.boundaryCode) && record.phase === "waiting") {
      record.prepared = false;
    }
    if (["browser_cancelled", "cancelling"].includes(record.phase)) {
      controlledHandoffStatus(record, "failed", record.phase === "browser_cancelled" ? "desktop_activation_pending" : "browser_cancel_unconfirmed");
      return;
    }
    const reason = ["desktop_unavailable", "desktop_refused", "handoff_timeout", "handoff_disconnected", "identity_mismatch"]
      .includes(error?.handoffReason) ? error.handoffReason : "desktop_unavailable";
    await abortControlledHandoff(record, reason).catch(() => {
      controlledHandoffStatus(record, "failed", "browser_resume_failed");
    });
  } finally { release(); }
}

async function restoreBrowserHandoffs() {
  if (browserHandoffRestore) return browserHandoffRestore;
  browserHandoffRestore = (async () => {
    if (!chrome.storage?.session?.get) return;
    const stored = await chrome.storage.session.get(BROWSER_HANDOFF_SESSION_KEY);
    browserHandoffJournalReady = true;
    for (const value of Array.isArray(stored[BROWSER_HANDOFF_SESSION_KEY]) ? stored[BROWSER_HANDOFF_SESSION_KEY] : []) {
      const responseRecord = value?.kind === "response" && typeof value.requestId === "string" &&
        value.downloadId === `response:${value.requestId}`;
      if ((!Number.isInteger(value?.downloadId) && !responseRecord) || typeof value.handoffId !== "string" ||
          !/^[0-9a-f-]{36}$/i.test(value.handoffId) || browserHandoffOperations.has(value.downloadId)) continue;
      const record = { downloadId: value.downloadId, handoffId: value.handoffId,
        ...(responseRecord ? { kind: "response", requestId: value.requestId } : {}),
        browserStartedAt: String(value.browserStartedAt || ""), phase: value.phase,
        taskId: String(value.taskId || ""), prepared: value.prepared !== false,
        startedAt: Number(value.startedAt) || Date.now(), eraseBrowserHistory: value.eraseBrowserHistory === true };
      browserHandoffJournal.set(record.downloadId, record);
      const operation = (async () => {
        const release = retainDesktopPort();
        try {
          if (responseRecord) await recoverFirefoxResponseHandoff(record);
          else if (["pausing", "aborting"].includes(record.phase)) await abortControlledHandoff(record, "desktop_unavailable");
          else await followControlledHandoff(record);
        } catch {
          if (!["browser_cancelled", "cancelling"].includes(record.phase)) {
            await abortControlledHandoff(record, "handoff_disconnected").catch(() => {});
          }
        } finally { release(); }
      })().finally(() => browserHandoffOperations.delete(record.downloadId));
      browserHandoffOperations.set(record.downloadId, operation);
    }
  })().catch(() => {}).finally(() => { browserHandoffRestore = null; });
  return browserHandoffRestore;
}

// Firefox can retain a frame response before a DownloadItem (and its UI) exists.
const firefoxResponseRequests = new Map();
const firefoxResponseDecisions = new Map();
const FIREFOX_RESPONSE_DECISION_TTL_MS = 120000;

function firefoxResponseDownload(details) {
  if (!isHttpUrl(details.url) || !["main_frame", "sub_frame"].includes(details.type) ||
      details.method !== "GET" || details.statusCode < 200 || details.statusCode >= 300 ||
      [204, 205, 206].includes(details.statusCode)) return null;
  const disposition = responseHeaderValue(details.responseHeaders, "content-disposition");
  const mime = responseHeaderValue(details.responseHeaders, "content-type").split(";")[0].toLowerCase();
  const attachment = /^attachment(?:\s*;|\s*$)/i.test(disposition);
  // Inline documents/media and unknown types remain the browser's; do not infer a download from the URL.
  const inline = /^(?:text\/|image\/|audio\/|video\/|multipart\/)|^application\/(?:pdf|xml|xhtml\+xml|json|javascript|x-javascript|ecmascript|ogg|x-xpinstall|[\w.+-]+\+(?:xml|json))$/i;
  if (!attachment && (!mime || inline.test(mime))) return null;
  const length = responseHeaderValue(details.responseHeaders, "content-encoding") ? 0
    : Number(responseHeaderValue(details.responseHeaders, "content-length")) || 0;
  const redirect = resolveRedirectHint(details);
  const filename = parseContentDispositionFilename(disposition);
  if (filename) rememberContentDisposition(details.url, filename);
  return { ...details, url: redirect?.sourceUrl || details.url, finalUrl: details.url, mime,
    filename: filename || getFileNameFromUrl(details.url),
    totalBytes: length > 0 ? length : 0,
    canResume: /bytes/i.test(responseHeaderValue(details.responseHeaders, "accept-ranges")) };
}

function firefoxResponseWasDecided(requestContext, downloadItem) {
  const snapshot = requestContext.headerSnapshot;
  const url = normalizeUrlKey(downloadItem.finalUrl || downloadItem.url);
  const source = { ...downloadItem, ...(requestContext.downloadTabId > 0 ? { tabId: requestContext.downloadTabId } : {}) };
  let decision = snapshot?.url === url ? firefoxResponseDecisions.get(snapshot.requestId) : null;
  if (!decision && snapshot?.url === url) return false; // A newer actual request is not a released fallback.
  if (!decision) {
    const candidates = [...firefoxResponseDecisions.values()].filter(value =>
      (value.url === url || value.originalUrl === normalizeUrlKey(downloadItem.url)) && matchesRequestContext(value, source));
    if (candidates.length !== 1) return false;
    [decision] = candidates;
  }
  if (!decision || (!firefoxResponseRequests.has(decision.requestId) &&
      Date.now() - decision.time > FIREFOX_RESPONSE_DECISION_TTL_MS)) return false;
  // An actual DownloadItem proves that this response was not stopped by our cancellation result.
  noteFirefoxResponseTerminal(decision, "started");
  return true;
}

function noteFirefoxResponseTerminal(details, terminal) {
  const active = firefoxResponseRequests.get(details.requestId);
  if (!active || active.terminal) return;
  active.terminal = terminal;
  active.resolveTerminal(terminal);
}

async function recoverFirefoxResponseHandoff(record) {
  if (record.phase === "browser_cancelled") {
    const reply = await browserHandoffRequest("get_browser_handoff", record);
    const state = controlledHandoffState(reply, record);
    if (state === "held") await commitControlledHandoff(record);
    else if (state === "committed") await forgetBrowserHandoff(record);
    else await abortControlledHandoff(record, "confirmation_cancelled");
  } else if (record.phase === "cancelling") {
    // A restarted event page cannot reconstruct a lost request cancellation acknowledgment.
    controlledHandoffStatus(record, "failed", "browser_cancel_unconfirmed");
  } else {
    await abortControlledHandoff(record, "browser_control_failed");
  }
}

function releaseFirefoxResponse(active) {
  if (firefoxResponseRequests.get(active.record.requestId) !== active) return;
  const decision = firefoxResponseDecisions.get(active.record.requestId);
  if (decision) decision.time = Date.now();
  firefoxResponseRequests.delete(active.record.requestId);
  browserHandoffOperations.delete(active.record.downloadId);
  active.release?.();
}

async function finishFirefoxResponseCancellation(active) {
  const record = active.record;
  let timer;
  try {
    const terminal = await Promise.race([active.terminalPromise,
      new Promise(resolve => { timer = setTimeout(() => resolve("unknown"), 12000); })]);
    if (terminal === "failed") {
      // Firefox's matching onErrorOccurred is the native boundary proving the response cannot transfer.
      await commitControlledHandoff(record);
    } else if (terminal === "started") {
      await abortControlledHandoff(record, "browser_cancel_failed");
    } else {
      controlledHandoffStatus(record, "failed", "browser_cancel_unconfirmed");
    }
  } catch {
    controlledHandoffStatus(record, "failed", record.phase === "browser_cancelled"
      ? "desktop_activation_pending" : "browser_cancel_unconfirmed");
  } finally {
    clearTimeout(timer);
    releaseFirefoxResponse(active);
  }
}

async function runFirefoxResponseHandoff(active, item) {
  const record = active.record;
  try {
    const settings = await getSettings();
    const requestContext = await resolveDownloadRequestContext(item);
    item.referrer = requestContext.headerSnapshot?.headers?.referer || "";
    const excluded = await automaticCaptureDecision(item, settings);
    if (excluded) { controlledHandoffStatus(record, excluded.state, excluded.reason); return {}; }
    if (requestContext.method && requestContext.method !== "GET") {
      controlledHandoffStatus(record, "skipped", "unsupported_request_method");
      return {};
    }
    if (!browserHandoffJournalReady) await restoreBrowserHandoffs();
    if (!browserHandoffJournalReady || browserHandoffJournal.size >= BROWSER_HANDOFF_LIMIT || active.terminal) {
      controlledHandoffStatus(record, "skipped", "browser_control_failed");
      return {};
    }
    await updateBrowserHandoff(record, "waiting");
    active.release = retainDesktopPort();
    record.prepared = true;
    await updateBrowserHandoff(record, "waiting");
    controlledHandoffStatus(record, "submitting", "handoff_started");
    const result = await sendDownloadItem(item, record.handoffId, requestContext, true);
    const port = desktopPort;
    let reply = result.response;
    while (true) {
      if (!port || desktopPort !== port) throw desktopHandoffError("原生连接已断开", "handoff_disconnected", "port_disconnected");
      if (active.terminal) {
        await abortControlledHandoff(record, "browser_control_failed");
        return {};
      }
      const state = controlledHandoffState(reply, record);
      if (state === "held") {
        controlledHandoffStatus(record, "persisted", "desktop_accepted");
        await updateBrowserHandoff(record, "cancelling", { taskId: reply.taskId });
        if (active.terminal) {
          await abortControlledHandoff(record, "browser_control_failed");
          return {};
        }
        active.cancelReturned = true;
        void finishFirefoxResponseCancellation(active);
        return { cancel: true };
      }
      if (state !== "pending") {
        await abortControlledHandoff(record, state === "expired" ? "confirmation_timeout" : "confirmation_cancelled");
        return {};
      }
      if (record.phase !== "waiting_confirmation") {
        await updateBrowserHandoff(record, "waiting_confirmation");
        controlledHandoffStatus(record, "awaiting_confirmation", "confirmation_pending");
      }
      if (Date.now() - record.startedAt >= BROWSER_CONFIRMATION_WAIT_MS) {
        await abortControlledHandoff(record, "confirmation_timeout");
        return {};
      }
      await new Promise(resolve => setTimeout(resolve, 1000));
      if (!port || desktopPort !== port) throw desktopHandoffError("原生连接已断开", "handoff_disconnected", "port_disconnected");
      reply = await browserHandoffRequest("get_browser_handoff", record);
    }
  } catch (error) {
    if (["connect_failed", "connect_timeout", "request_too_large"].includes(error?.boundaryCode)) record.prepared = false;
    const reason = ["desktop_unavailable", "desktop_refused", "handoff_timeout", "handoff_disconnected", "identity_mismatch"]
      .includes(error?.handoffReason) ? error.handoffReason : "desktop_unavailable";
    await abortControlledHandoff(record, reason).catch(() => {
      controlledHandoffStatus(record, "failed", reason);
    });
    return {};
  } finally {
    if (!active.cancelReturned) releaseFirefoxResponse(active);
  }
}

function interceptFirefoxResponse(details) {
  const existing = firefoxResponseRequests.get(details.requestId);
  if (existing) return existing.decision;
  const item = firefoxResponseDownload(details);
  if (!item || firefoxResponseRequests.size >= BROWSER_HANDOFF_LIMIT) return Promise.resolve({});
  // Keep method/source identity available even when no request-header event preceded this response.
  if (!recentRequestsByRequestId.has(details.requestId)) rememberRequestHeaders({ ...details, requestHeaders: [] });
  for (const [id, decision] of firefoxResponseDecisions) {
    if (!firefoxResponseRequests.has(id) && Date.now() - decision.time > FIREFOX_RESPONSE_DECISION_TTL_MS) firefoxResponseDecisions.delete(id);
  }
  firefoxResponseDecisions.set(details.requestId, { requestId: details.requestId, time: Date.now(),
    url: normalizeUrlKey(details.url), originalUrl: normalizeUrlKey(item.url), tabId: details.tabId,
    tabIdObserved: details.tabId > 0, ...cookieSourceDetails(details) });
  while (firefoxResponseDecisions.size > REQUEST_CACHE_LIMIT) firefoxResponseDecisions.delete(firefoxResponseDecisions.keys().next().value);
  const record = { kind: "response", requestId: details.requestId, downloadId: `response:${details.requestId}`,
    handoffId: nextRequestId(), phase: "waiting", taskId: "", prepared: false, startedAt: Date.now(), eraseBrowserHistory: false };
  const active = { record, terminal: "", cancelReturned: false, release: null };
  active.terminalPromise = new Promise(resolve => { active.resolveTerminal = resolve; });
  firefoxResponseRequests.set(details.requestId, active);
  active.decision = runFirefoxResponseHandoff(active, item);
  browserHandoffOperations.set(record.downloadId, active.decision);
  return active.decision;
}

function registerFirefoxResponseHandoff() {
  if (!chrome.runtime.getManifest().permissions?.includes("webRequestBlocking") ||
      !chrome.webRequest?.onHeadersReceived?.addListener || !chrome.webRequest?.onErrorOccurred?.addListener) return;
  const filter = { urls: ["http://*/*", "https://*/*"], types: ["main_frame", "sub_frame"] };
  chrome.webRequest.onHeadersReceived.addListener(interceptFirefoxResponse, filter, ["blocking", "responseHeaders"]);
  chrome.webRequest.onErrorOccurred.addListener(details => noteFirefoxResponseTerminal(details, "failed"), filter);
}
