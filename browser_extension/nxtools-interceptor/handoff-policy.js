(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) {
    module.exports = api;
  }
  root.NxToolsHandoffPolicy = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";

  function acceptedHandoffResponse(response, expectedHandoffId) {
    return Boolean(
      response &&
      response.ok === true &&
      response.accepted === true &&
      (!response.data?.handoffState || response.data.handoffState === "persisted") &&
      expectedHandoffId &&
      response.handoffId === expectedHandoffId
    );
  }

  function handoffOutcome(response, expectedHandoffId) {
    const awaitingConfirmation = response?.data?.handoffState === "awaiting_confirmation";
    if ((response?.handoffId && response.handoffId !== expectedHandoffId) ||
        ((response?.accepted === true || awaitingConfirmation) && (!expectedHandoffId || !response.handoffId)) ||
        response?.data?.reason === "identity_mismatch") {
      return { state: "rejected", reason: "identity_mismatch" };
    }
    if (awaitingConfirmation) {
      return response.ok === true && response.accepted === false && !response.taskId &&
          response.data.reason === "confirmation_pending" && response.data.queued !== true
        ? { state: "awaiting_confirmation", reason: "confirmation_pending" }
        : { state: "rejected", reason: "desktop_refused" };
    }
    if (acceptedHandoffResponse(response, expectedHandoffId)) {
      return { state: "persisted", reason: "desktop_accepted", queued: response.data?.queued === true };
    }
    if ((response?.ok === true || response?.data?.handoffState === "queued") && response?.data?.queued === true) {
      return { state: "queued", reason: "queued_not_accepted" };
    }
    return { state: "rejected", reason: "desktop_refused" };
  }

  function isWorkflowLogEvent(event) {
    return ["link_collection", "media_inspection", "connection_check"].some(prefix =>
      ["started", "completed", "failed"].some(phase => event === `${prefix}_${phase}`));
  }

  function buildExtensionLogRequest({
    protocolVersion,
    level,
    event,
    message,
    correlationId = "",
    handoffState,
    reason,
    elapsedMs,
    boundaryCode,
    compensation,
    allocateRequestId,
    isRequestIdPending = () => false,
  }) {
    for (let attempt = 0; attempt < 4; attempt += 1) {
      const requestId = String(allocateRequestId() || "");
      if (requestId && requestId !== correlationId && !isRequestIdPending(requestId)) {
        return {
          protocolVersion,
          type: "extension_log",
          requestId,
          payload: {
            level,
            event,
            message,
            correlationId: correlationId || undefined,
            ...(event === "handoff_state" ? { handoffState, reason, elapsedMs, boundaryCode, compensation } : {}),
            ...(["capture_pause_started", "capture_pause_applied", "capture_pause_failed"].includes(event) || isWorkflowLogEvent(event) ? { reason, elapsedMs } : {}),
          },
        };
      }
    }
    throw new Error("unable to allocate an isolated extension log request ID");
  }

  async function completeAcceptedHandoff({
    response,
    expectedHandoffId,
    cancelBrowserDownload,
    eraseBrowserDownload,
    rollbackDesktopTask,
  }) {
    if (!acceptedHandoffResponse(response, expectedHandoffId)) {
      return {
        transferred: false,
        error: response?.accepted === true && response?.handoffId !== expectedHandoffId
          ? "handoff response identity mismatch"
          : response && response.error ? String(response.error) : "handoff rejected",
      };
    }

    try {
      await cancelBrowserDownload();
    } catch (error) {
      let compensation = "not_available";
      if (response.taskId && typeof rollbackDesktopTask === "function") {
        try {
          await rollbackDesktopTask(response.taskId);
          compensation = "succeeded";
        } catch {
          // Retry uses the original handoff ID; the browser item stays authoritative.
          compensation = "failed";
        }
      }
      error.handoffReason = "browser_cancel_failed";
      error.compensation = compensation;
      throw error;
    }
    // History cleanup does not change ownership after browser cancellation.
    try { await eraseBrowserDownload(); } catch { /* Browser already canceled. */ }
    return {
      transferred: true,
      taskId: response.taskId || null,
      duplicate: response.duplicate === true,
    };
  }

  return Object.freeze({
    acceptedHandoffResponse,
    handoffOutcome,
    buildExtensionLogRequest,
    isWorkflowLogEvent,
    completeAcceptedHandoff,
  });
});
