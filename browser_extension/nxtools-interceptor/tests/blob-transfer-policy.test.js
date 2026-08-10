"use strict";

const assert = require("node:assert/strict");
const policy = require("../blob-transfer-policy.js");

const metadata = policy.boundedMetadata(12, "report.bin", "application/octet-stream");
assert.deepEqual(metadata, {
  size: 12,
  fileName: "report.bin",
  mediaType: "application/octet-stream"
});
assert.equal(policy.boundedMetadata(policy.MAX_BLOB_BYTES + 1, "report.bin", "text/plain"), null);

const capability = policy.uploadCapability({
  data: {
    uploadId: "123e4567-e89b-42d3-a456-426614174000",
    uploadUrl: "http://127.0.0.1:49152/nxtools/blob/123e4567-e89b-42d3-a456-426614174000/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    maxBytes: policy.MAX_BLOB_BYTES,
    chunkBytes: policy.MAX_CHUNK_BYTES
  }
}, 12);
assert.ok(capability);
assert.equal(policy.uploadCapability({
  data: { ...capability, uploadUrl: `${capability.uploadUrl}00`, maxBytes: policy.MAX_BLOB_BYTES, chunkBytes: policy.MAX_CHUNK_BYTES }
}, 12), null);

assert.equal(policy.handoffAccepted({ ok: true }), true);
assert.equal(policy.handoffAccepted(undefined), false);
assert.equal(policy.handoffAccepted({ ok: true }, new Error("extension context invalidated")), false);
assert.deepEqual(policy.browserFallback({ ok: false, message: "desktop rejected" }, "handoff-1", "fallback"), {
  ok: false,
  fallbackToBrowser: true,
  message: "desktop rejected"
});
assert.equal(policy.browserFallback({
  ok: true,
  accepted: true,
  handoffId: "handoff-1"
}, "handoff-1", "fallback").ok, true);
assert.deepEqual(policy.browserFallback({
  ok: true,
  accepted: true,
  handoffId: "other"
}, "handoff-1", "fallback"), {
  ok: false,
  fallbackToBrowser: true,
  message: "NxTools Blob 关联响应不匹配"
});
assert.equal(policy.uploadCapability({
  data: {
    uploadId: "123e4567-e89b-42d3-a456-426614174000",
    uploadUrl: "http://localhost:49152/nxtools/blob/123e4567-e89b-42d3-a456-426614174000/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    maxBytes: policy.MAX_BLOB_BYTES,
    chunkBytes: policy.MAX_CHUNK_BYTES
  }
}, 12), null);

console.log("Blob transfer policy tests passed.");
