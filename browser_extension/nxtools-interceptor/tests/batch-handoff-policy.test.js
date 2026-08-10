"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");

const {
  MAX_BATCH_ITEMS,
  applyAcceptedHandoff,
  normalizeHandoffId,
  runBatchHandoffRequest,
  runSingleResourceHandoff,
  selectBatchResources,
  validateBatchResponse,
} = require("../batch-handoff-policy.js");

test("an older single-resource acceptance cannot clear a newer in-flight identity", async () => {
  const firstHandoffId = "12345678-1234-4abc-8def-1234567890ab";
  const secondHandoffId = "87654321-4321-4cba-9fed-ba0987654321";
  const resource = { id: "resource-1", pendingBatchHandoffId: firstHandoffId };
  const result = await runSingleResourceHandoff({
    resource,
    persistHandoff: async () => {},
    sendHandoff: async (handoffId) => {
      assert.equal(handoffId, firstHandoffId);
      resource.pendingBatchHandoffId = secondHandoffId;
      return { ok: true };
    },
    resolveCurrentResource: () => resource,
    acceptedAt: 1234,
  });

  assert.equal(result.ok, true);
  assert.equal(resource.pendingBatchHandoffId, secondHandoffId);
  assert.equal("sentToDesktopAt" in resource, false);
});

test("acceptance updates the current resource after an in-flight recapture replacement", () => {
  const handoffId = "12345678-1234-4abc-8def-1234567890ab";
  const observedResource = { id: "resource-1", pendingBatchHandoffId: handoffId };
  const currentResource = { id: "resource-1", pendingBatchHandoffId: handoffId };
  const updated = applyAcceptedHandoff({
    resourceId: "resource-1",
    handoffId,
    observedResource,
    resolveCurrentResource: () => currentResource,
    acceptedAt: 1234,
  });

  assert.equal(updated, true);
  for (const resource of [observedResource, currentResource]) {
    assert.equal(resource.sentToDesktopAt, 1234);
    assert.equal("pendingBatchHandoffId" in resource, false);
  }
});

test("batch request emits one correlated terminal when transport rejects", async () => {
  const events = [];
  await assert.rejects(
    runBatchHandoffRequest({
      correlationId: "correlation-a",
      handoffIds: ["handoff-a", "handoff-b"],
      sendRequest: async () => { throw new Error("native host timeout"); },
      emitLifecycle: (phase, details, correlationId) => events.push({ phase, details, correlationId }),
    }),
    /native host timeout/
  );
  assert.deepEqual(events, [
    { phase: "started", details: { count: 2 }, correlationId: "correlation-a" },
    {
      phase: "completed",
      details: { count: 2, acceptedCount: 0, failedCount: 2, duplicateCount: 0, transportFailed: true },
      correlationId: "correlation-a",
    },
  ]);
});

test("batch request validates identities and emits one correlated success terminal", async () => {
  const events = [];
  const result = await runBatchHandoffRequest({
    correlationId: "correlation-b",
    handoffIds: ["handoff-a"],
    sendRequest: async () => ({
      ok: true,
      data: {
        queued: true,
        items: [{ handoffId: "handoff-a", accepted: true, taskId: "task-a", duplicate: true }],
      },
    }),
    emitLifecycle: (phase, details, correlationId) => events.push({ phase, details, correlationId }),
  });
  assert.equal(result.validation.valid, true);
  assert.deepEqual(events, [
    { phase: "started", details: { count: 1 }, correlationId: "correlation-b" },
    {
      phase: "completed",
      details: { count: 1, acceptedCount: 1, failedCount: 0, duplicateCount: 1, transportFailed: false },
      correlationId: "correlation-b",
    },
  ]);
});

test("persisted batch handoff identity accepts only canonical UUIDs", () => {
  assert.equal(
    normalizeHandoffId("12345678-1234-4abc-8def-1234567890ab"),
    "12345678-1234-4abc-8def-1234567890ab"
  );
  assert.equal(normalizeHandoffId("not-a-uuid"), "");
  assert.equal(normalizeHandoffId("12345678-1234-4abc-8def-1234567890ab-extra"), "");
});

test("batch selection accepts at most 32 ordinary resources and keeps Blob items for single transfer", () => {
  const ordinary = Array.from({ length: 32 }, (_, index) => ({
    id: `resource-${index}`,
    url: `https://example.test/${index}.bin`,
  }));
  const selection = selectBatchResources([
    ...ordinary,
    { id: "blob-resource", url: "blob:https://example.test/blob-id" },
  ]);

  assert.equal(MAX_BATCH_ITEMS, 32);
  assert.deepEqual(selection.items.map((item) => item.id), ordinary.map((item) => item.id));
  assert.deepEqual(selection.unsupportedIds, ["blob-resource"]);
  assert.equal(selection.error, "");

  const oversized = selectBatchResources([
    ...ordinary,
    { id: "resource-32", url: "https://example.test/32.bin" },
  ]);
  assert.equal(oversized.items.length, 0);
  assert.match(oversized.error, /32/);
});

test("batch response binds every result to one expected handoff and preserves partial failures", () => {
  const result = validateBatchResponse({
    ok: true,
    data: {
      queued: true,
      items: [
        { handoffId: "handoff-a", accepted: true, duplicate: false, taskId: "task-a" },
        { handoffId: "handoff-b", accepted: false, duplicate: false, error: "rejected" },
      ],
    },
  }, ["handoff-a", "handoff-b"]);

  assert.equal(result.valid, true);
  assert.deepEqual(result.acceptedHandoffIds, ["handoff-a"]);
  assert.deepEqual(result.failedHandoffIds, ["handoff-b"]);
  assert.equal(result.duplicateCount, 0);
});

test("batch response rejects missing, duplicated or foreign handoff identities", () => {
  for (const items of [
    [{ handoffId: "handoff-a", accepted: true }],
    [
      { handoffId: "handoff-a", accepted: true },
      { handoffId: "handoff-a", accepted: true },
    ],
    [
      { handoffId: "handoff-a", accepted: true },
      { handoffId: "foreign", accepted: true },
    ],
  ]) {
    const result = validateBatchResponse({ ok: true, data: { queued: true, items } }, [
      "handoff-a",
      "handoff-b",
    ]);
    assert.equal(result.valid, false);
    assert.deepEqual(result.acceptedHandoffIds, []);
  }
});

test("batch response rejects acceptance without a queued task identity", () => {
  for (const response of [
    { ok: true, data: { items: [{ handoffId: "handoff-a", accepted: true, taskId: "task-a" }] } },
    { ok: true, data: { queued: true, items: [{ handoffId: "handoff-a", accepted: true }] } },
  ]) {
    const result = validateBatchResponse(response, ["handoff-a"]);
    assert.equal(result.valid, false);
  }
});
