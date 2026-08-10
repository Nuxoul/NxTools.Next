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
      expectedHandoffId &&
      response.handoffId === expectedHandoffId
    );
  }

  function buildExtensionLogRequest({
    protocolVersion,
    level,
    event,
    message,
    correlationId = "",
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
      if (response.taskId && typeof rollbackDesktopTask === "function") {
        try {
          await rollbackDesktopTask(response.taskId);
        } catch {
          // The browser item is still authoritative. A later idempotent retry uses the same
          // handoff ID, so rollback failure cannot create a second desktop task.
        }
      }
      throw error;
    }
    await eraseBrowserDownload();
    return {
      transferred: true,
      taskId: response.taskId || null,
      duplicate: response.duplicate === true,
    };
  }

  return Object.freeze({
    acceptedHandoffResponse,
    buildExtensionLogRequest,
    completeAcceptedHandoff,
  });
});
