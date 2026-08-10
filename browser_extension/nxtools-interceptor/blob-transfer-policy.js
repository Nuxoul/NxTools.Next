"use strict";

(function installBlobTransferPolicy(root, factory) {
  const policy = factory();
  if (typeof module === "object" && module.exports) {
    module.exports = policy;
  }
  root.NxToolsBlobTransferPolicy = policy;
})(typeof globalThis === "object" ? globalThis : this, function createBlobTransferPolicy() {
  const MAX_BLOB_BYTES = 2 * 1024 * 1024 * 1024;
  const MAX_CHUNK_BYTES = 1024 * 1024;
  const UPLOAD_URL = /^http:\/\/127\.0\.0\.1:\d+\/nxtools\/blob\/[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\/[0-9a-f]{64}$/i;
  const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

  function boundedMetadata(size, fileName, mediaType) {
    const resolvedSize = Number(size || 0);
    const resolvedFileName = String(fileName || "").trim();
    const resolvedMediaType = String(mediaType || "application/octet-stream").trim();
    if (!Number.isSafeInteger(resolvedSize) || resolvedSize <= 0 || resolvedSize > MAX_BLOB_BYTES) {
      return null;
    }
    if (!resolvedFileName || new TextEncoder().encode(resolvedFileName).byteLength > 512) {
      return null;
    }
    if (resolvedMediaType.length > 256 || /[\r\n]/.test(resolvedMediaType)) {
      return null;
    }
    return { size: resolvedSize, fileName: resolvedFileName, mediaType: resolvedMediaType };
  }

  function uploadCapability(response, expectedSize) {
    const uploadId = String(response?.data?.uploadId || "");
    const uploadUrl = String(response?.data?.uploadUrl || "");
    const maxBytes = Number(response?.data?.maxBytes || 0);
    const chunkBytes = Number(response?.data?.chunkBytes || 0);
    if (!UUID.test(uploadId) || !UPLOAD_URL.test(uploadUrl)) {
      return null;
    }
    if (!Number.isSafeInteger(maxBytes) || expectedSize > maxBytes || maxBytes > MAX_BLOB_BYTES) {
      return null;
    }
    if (!Number.isSafeInteger(chunkBytes) || chunkBytes <= 0 || chunkBytes > MAX_CHUNK_BYTES) {
      return null;
    }
    return { uploadId, uploadUrl, chunkBytes };
  }

  function handoffAccepted(result, runtimeError = null) {
    return !runtimeError && result?.ok === true;
  }

  function browserFallback(result, expectedHandoffId, defaultMessage) {
    if (result?.ok === true && result?.accepted === true
        && expectedHandoffId && result?.handoffId === expectedHandoffId) {
      return result;
    }
    const identityMismatch = result?.accepted === true && result?.handoffId !== expectedHandoffId;
    return {
      ok: false,
      fallbackToBrowser: true,
      message: identityMismatch
        ? "NxTools Blob 关联响应不匹配"
        : String(result?.message || defaultMessage || "NxTools 未接受 Blob 下载")
    };
  }

  return Object.freeze({
    MAX_BLOB_BYTES,
    MAX_CHUNK_BYTES,
    boundedMetadata,
    uploadCapability,
    handoffAccepted,
    browserFallback
  });
});
