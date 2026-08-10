(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) {
    module.exports = api;
  }
  root.NxToolsBatchHandoffPolicy = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";

  const MAX_BATCH_ITEMS = 32;
  const HANDOFF_ID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

  function normalizeHandoffId(value) {
    const handoffId = String(value || "").trim();
    return HANDOFF_ID_PATTERN.test(handoffId) ? handoffId : "";
  }

  function applyAcceptedHandoff({
    resourceId,
    handoffId,
    observedResource,
    resolveCurrentResource,
    acceptedAt = Date.now(),
  }) {
    const candidates = new Set([
      observedResource,
      typeof resolveCurrentResource === "function"
        ? resolveCurrentResource(resourceId)
        : null,
    ]);
    let updated = false;
    for (const resource of candidates) {
      if (!resource || resource.pendingBatchHandoffId !== handoffId) {
        continue;
      }
      resource.sentToDesktopAt = acceptedAt;
      delete resource.pendingBatchHandoffId;
      updated = true;
    }
    return updated;
  }

  async function runSingleResourceHandoff({
    resource,
    persistHandoff,
    sendHandoff,
    resolveCurrentResource,
    acceptedAt,
  }) {
    const resourceId = String(resource?.id || "");
    const handoffId = String(resource?.pendingBatchHandoffId || "");
    await persistHandoff();
    const result = await sendHandoff(handoffId);
    if (result?.ok === true) {
      applyAcceptedHandoff({
        resourceId,
        handoffId,
        observedResource: resource,
        resolveCurrentResource,
        acceptedAt: acceptedAt ?? Date.now(),
      });
    }
    return result;
  }

  function isOrdinaryDownloadUrl(value) {
    try {
      const url = new URL(String(value || ""));
      return url.protocol === "http:" || url.protocol === "https:";
    } catch {
      return false;
    }
  }

  function selectBatchResources(resources) {
    const candidates = Array.isArray(resources) ? resources : [];
    const items = candidates.filter((resource) => isOrdinaryDownloadUrl(resource?.url));
    const unsupportedIds = candidates
      .filter((resource) => !isOrdinaryDownloadUrl(resource?.url))
      .map((resource) => String(resource?.id || ""))
      .filter(Boolean);
    if (items.length > MAX_BATCH_ITEMS) {
      return {
        items: [],
        unsupportedIds,
        error: `每次最多批量提交 ${MAX_BATCH_ITEMS} 个资源`,
      };
    }
    return { items, unsupportedIds, error: "" };
  }

  function invalidBatchResponse(message) {
    return {
      valid: false,
      acceptedHandoffIds: [],
      failedHandoffIds: [],
      duplicateCount: 0,
      error: message,
    };
  }

  function validateBatchResponse(response, expectedHandoffIds) {
    const expected = Array.isArray(expectedHandoffIds)
      ? expectedHandoffIds.map(String)
      : [];
    const items = response?.ok === true && response?.data?.queued === true && Array.isArray(response?.data?.items)
      ? response.data.items
      : null;
    if (!items || items.length !== expected.length || expected.length === 0) {
      return invalidBatchResponse("batch handoff response shape mismatch");
    }
    const expectedSet = new Set(expected);
    const observed = new Set();
    for (const item of items) {
      const handoffId = String(item?.handoffId || "");
      if (!expectedSet.has(handoffId) || observed.has(handoffId)) {
        return invalidBatchResponse("batch handoff response identity mismatch");
      }
      if (item?.accepted === true && !String(item?.taskId || "").trim()) {
        return invalidBatchResponse("batch handoff response task identity missing");
      }
      observed.add(handoffId);
    }
    if (observed.size !== expectedSet.size) {
      return invalidBatchResponse("batch handoff response identity mismatch");
    }

    return {
      valid: true,
      acceptedHandoffIds: items
        .filter((item) => item.accepted === true)
        .map((item) => String(item.handoffId)),
      failedHandoffIds: items
        .filter((item) => item.accepted !== true)
        .map((item) => String(item.handoffId)),
      duplicateCount: items.filter((item) => item.accepted === true && item.duplicate === true).length,
      error: "",
    };
  }

  async function runBatchHandoffRequest({
    correlationId,
    handoffIds,
    sendRequest,
    emitLifecycle,
  }) {
    const expectedHandoffIds = Array.isArray(handoffIds) ? handoffIds.map(String) : [];
    emitLifecycle("started", { count: expectedHandoffIds.length }, correlationId);
    try {
      const response = await sendRequest();
      const validation = validateBatchResponse(response, expectedHandoffIds);
      const acceptedCount = validation.valid ? validation.acceptedHandoffIds.length : 0;
      emitLifecycle("completed", {
        count: expectedHandoffIds.length,
        acceptedCount,
        failedCount: validation.valid
          ? validation.failedHandoffIds.length
          : expectedHandoffIds.length,
        duplicateCount: validation.valid ? validation.duplicateCount : 0,
        transportFailed: false,
      }, correlationId);
      return { response, validation };
    } catch (error) {
      emitLifecycle("completed", {
        count: expectedHandoffIds.length,
        acceptedCount: 0,
        failedCount: expectedHandoffIds.length,
        duplicateCount: 0,
        transportFailed: true,
      }, correlationId);
      throw error;
    }
  }

  return Object.freeze({
    MAX_BATCH_ITEMS,
    applyAcceptedHandoff,
    normalizeHandoffId,
    runBatchHandoffRequest,
    runSingleResourceHandoff,
    selectBatchResources,
    validateBatchResponse,
  });
});
