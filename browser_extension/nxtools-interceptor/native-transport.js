"use strict";

(function initializeNativeTransport(global) {
  const HOST_NAME = "com.nxtools.next.browser";
  const PROTOCOL_VERSION = 1;
  const MAX_MESSAGE_BYTES = 768 * 1024;
  const BLOB_CHUNK_BASE64_CHARS = 384 * 1024;

  function randomId() {
    return crypto.randomUUID();
  }

  function encodedSize(value) {
    return new TextEncoder().encode(JSON.stringify(value)).byteLength;
  }

  function decodedBase64Size(base64) {
    const clean = String(base64 || "").replace(/\s+/g, "");
    if (!clean) {
      return 0;
    }
    const padding = clean.endsWith("==") ? 2 : clean.endsWith("=") ? 1 : 0;
    return Math.floor((clean.length * 3) / 4) - padding;
  }

  class NativeTransport {
    constructor(runtime = chrome.runtime, timeoutMs = 15000) {
      this.runtime = runtime;
      this.timeoutMs = timeoutMs;
      this.port = null;
      this.pending = new Map();
    }

    connect() {
      if (this.port) {
        return this.port;
      }
      const port = this.runtime.connectNative(HOST_NAME);
      this.port = port;
      port.onMessage.addListener((message) => this.handleMessage(message));
      port.onDisconnect.addListener(() => this.handleDisconnect());
      return port;
    }

    handleMessage(message) {
      const requestId = String(message?.requestId || "");
      const pending = this.pending.get(requestId);
      if (!pending) {
        return;
      }
      clearTimeout(pending.timeout);
      this.pending.delete(requestId);
      if (message?.protocolVersion !== PROTOCOL_VERSION) {
        pending.reject(new Error("NxTools Native Messaging 协议版本不兼容"));
      } else if (message?.ok === false) {
        const error = new Error(message?.error?.message || "NxTools Native Messaging 请求失败");
        error.code = message?.error?.code || "native_error";
        error.retryable = Boolean(message?.error?.retryable);
        pending.reject(error);
      } else {
        pending.resolve(message?.result ?? {});
      }
    }

    handleDisconnect() {
      const runtimeError = this.runtime.lastError?.message;
      this.port = null;
      const error = new Error(runtimeError || "NxTools Native Messaging 已断开");
      for (const pending of this.pending.values()) {
        clearTimeout(pending.timeout);
        pending.reject(error);
      }
      this.pending.clear();
    }

    request(action, payload = {}, options = {}) {
      const requestId = options.requestId || randomId();
      const message = {
        protocolVersion: PROTOCOL_VERSION,
        requestId,
        action,
        payload
      };
      if (options.idempotencyKey) {
        message.idempotencyKey = String(options.idempotencyKey);
      }
      if (encodedSize(message) > MAX_MESSAGE_BYTES) {
        return Promise.reject(new Error("发送给 NxTools 的消息超过安全上限"));
      }
      const port = this.connect();
      return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
          this.pending.delete(requestId);
          reject(new Error(`等待 NxTools 响应超时：${action}`));
        }, options.timeoutMs || this.timeoutMs);
        this.pending.set(requestId, { resolve, reject, timeout });
        try {
          port.postMessage(message);
        } catch (error) {
          clearTimeout(timeout);
          this.pending.delete(requestId);
          reject(error);
        }
      });
    }

    async stageBlob(download, blobBase64, idempotencyKey) {
      const transferId = randomId();
      const clean = String(blobBase64 || "").replace(/\s+/g, "");
      const size = decodedBase64Size(clean);
      await this.request("blob_begin", {
        transferId,
        size,
        download
      }, { idempotencyKey, timeoutMs: 15000 });
      let index = 0;
      for (let offset = 0; offset < clean.length; offset += BLOB_CHUNK_BASE64_CHARS) {
        await this.request("blob_chunk", {
          transferId,
          index,
          data: clean.slice(offset, offset + BLOB_CHUNK_BASE64_CHARS)
        }, { timeoutMs: 30000 });
        index += 1;
      }
      return this.request("blob_commit", { transferId }, {
        idempotencyKey,
        timeoutMs: 30000
      });
    }
  }

  const api = {
    HOST_NAME,
    PROTOCOL_VERSION,
    MAX_MESSAGE_BYTES,
    NativeTransport,
    decodedBase64Size
  };
  global.NxToolsNativeTransport = api;
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
})(typeof globalThis !== "undefined" ? globalThis : self);
