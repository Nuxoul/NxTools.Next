"use strict";

// Page-hook Blob bridge: info, bounded chunked upload, read and browser fallback.
// Isolated-world content module: registered once on the extension-private
// NxToolsContent namespace (re-injection is a no-op). Holds no shared mutable
// page state; content-script.js owns all overlay/drop/capture state.
(function installNxToolsContentBlob(content) {
  if (content.blob) {
    return;
  }

  const { filenameFromUrl } = content.media;

  let nxBlobRequestSeq = 0;

  function requestBlobBytesFromPage(url, timeoutMs = 3500) {
    return new Promise((resolve) => {
      const token = `nxblob-${Date.now()}-${++nxBlobRequestSeq}`;
      let settled = false;
      const cleanup = () => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        window.removeEventListener("message", onMessage, true);
      };
      const onMessage = (event) => {
        if (event.source !== window) {
          return;
        }
        const data = event.data;
        if (!data || data.__nxtoolsBlobBytesResponse !== true || data.token !== token) {
          return;
        }
        cleanup();
        resolve(data.ok
          ? { ok: true, base64: data.base64 || "", mime: data.mime || "", size: Number(data.size || 0) }
          : { ok: false, message: data.message || "页面脚本无法读取这个 Blob" });
      };
      const timer = setTimeout(() => {
        cleanup();
        resolve({ ok: false, message: "页面脚本读取 Blob 超时" });
      }, timeoutMs);
      window.addEventListener("message", onMessage, true);
      window.postMessage({ __nxtoolsBlobBytesRequest: true, token, url: String(url || "") }, "*");
    });
  }

  function requestBlobPageAction(requestFlag, responseFlag, payload, timeoutMs) {
    return new Promise((resolve) => {
      const token = `nxblob-${Date.now()}-${++nxBlobRequestSeq}`;
      let settled = false;
      const cleanup = () => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        window.removeEventListener("message", onMessage, true);
      };
      const onMessage = (event) => {
        if (event.source !== window) {
          return;
        }
        const data = event.data;
        if (!data || data[responseFlag] !== true || data.token !== token) {
          return;
        }
        cleanup();
        resolve({
          ok: Boolean(data.ok),
          message: String(data.message || ""),
          mime: String(data.mime || ""),
          size: Number(data.size || 0)
        });
      };
      const timer = setTimeout(() => {
        cleanup();
        resolve({ ok: false, message: "页面 Blob 操作超时" });
      }, timeoutMs);
      window.addEventListener("message", onMessage, true);
      window.postMessage({ [requestFlag]: true, token, ...payload }, "*");
    });
  }

  function blobInfoFromPage(url) {
    return requestBlobPageAction(
      "__nxtoolsBlobInfoRequest",
      "__nxtoolsBlobInfoResponse",
      { url: String(url || "") },
      3500
    );
  }

  function requestBlobChunkFromPage(url, expectedSize, offset, length, timeoutMs = 10000) {
    return new Promise((resolve) => {
      const token = `nxblob-${Date.now()}-${++nxBlobRequestSeq}`;
      let settled = false;
      const cleanup = () => {
        if (settled) {
          return;
        }
        settled = true;
        clearTimeout(timer);
        window.removeEventListener("message", onMessage, true);
      };
      const onMessage = (event) => {
        if (event.source !== window) {
          return;
        }
        const data = event.data;
        if (!data || data.__nxtoolsBlobChunkResponse !== true || data.token !== token) {
          return;
        }
        cleanup();
        if (!data.ok || !(data.bytes instanceof ArrayBuffer) || data.bytes.byteLength !== length) {
          resolve({ ok: false, message: data.message || "Blob 分块读取失败" });
          return;
        }
        resolve({ ok: true, bytes: data.bytes });
      };
      const timer = setTimeout(() => {
        cleanup();
        resolve({ ok: false, message: "页面 Blob 分块读取超时" });
      }, timeoutMs);
      window.addEventListener("message", onMessage, true);
      window.postMessage({
        __nxtoolsBlobChunkRequest: true,
        token,
        url: String(url || ""),
        expectedSize,
        offset,
        length
      }, "*");
    });
  }

  async function uploadBlobFromPage(url, uploadUrl, expectedSize, chunkBytes) {
    if (!Number.isSafeInteger(expectedSize) || expectedSize <= 0
        || !Number.isSafeInteger(chunkBytes) || chunkBytes <= 0 || chunkBytes > 1024 * 1024
        || !/^http:\/\/127\.0\.0\.1:\d+\/nxtools\/blob\/[0-9a-f-]+\/[0-9a-f]{64}$/i.test(uploadUrl)) {
      return { ok: false, message: "Blob 上传参数无效" };
    }
    let offset = 0;
    while (offset < expectedSize) {
      const length = Math.min(chunkBytes, expectedSize - offset);
      const chunk = await requestBlobChunkFromPage(url, expectedSize, offset, length);
      if (!chunk.ok) {
        return chunk;
      }
      let response;
      try {
        response = await fetch(`${uploadUrl}?offset=${offset}`, {
          method: "POST",
          headers: { "Content-Type": "text/plain;charset=UTF-8" },
          body: chunk.bytes,
          cache: "no-store",
          credentials: "omit",
          redirect: "error"
        });
      } catch (error) {
        return { ok: false, message: error instanceof Error ? error.message : "Blob 分块上传失败" };
      }
      if (!response.ok) {
        return { ok: false, message: `Blob 分块上传失败：HTTP ${response.status}` };
      }
      offset += length;
    }
    return { ok: true, message: "Blob 已安全上传到 NxTools" };
  }

  function fallbackBlobDownloadFromPage(url, filename) {
    return requestBlobPageAction(
      "__nxtoolsBlobFallbackRequest",
      "__nxtoolsBlobFallbackResponse",
      { url: String(url || ""), filename: String(filename || "") },
      5000
    );
  }

  async function readBlobInPage(url, filename) {
    if (!String(url || "").startsWith("blob:")) {
      return { ok: false, message: "不是 Blob 链接" };
    }

    // Prefer the page hook (main world): it holds a strong reference to the original Blob captured at
    // createObjectURL time, so it still works after the page revoked the blob URL or when the blob
    // belongs to a frame this isolated-world fetch can't reach. Fall back to fetch() if it can't.
    const viaPage = await requestBlobBytesFromPage(url);
    if (viaPage.ok && viaPage.base64) {
      return {
        ok: true,
        filename: filename || filenameFromUrl(url) || `blob-${Date.now()}`,
        mime: viaPage.mime || "",
        size: viaPage.size || 0,
        base64: viaPage.base64
      };
    }

    let response;
    try {
      response = await fetch(url);
    } catch {
      return { ok: false, message: viaPage.message || "页面无法读取这个 Blob 链接，可能已失效或属于其他 frame" };
    }

    if (!response.ok) {
      return { ok: false, message: `Blob 读取失败：HTTP ${response.status}` };
    }

    const blob = await response.blob();
    if (!blob || blob.size <= 0) {
      return { ok: false, message: "Blob 内容为空" };
    }

    if (blob.size > 48 * 1024 * 1024) {
      return { ok: false, message: "Blob 超过 48 MB，当前只能等待真实网络源被捕获" };
    }

    const dataUrl = await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ""));
      reader.onerror = () => reject(new Error("Blob 读取失败"));
      reader.readAsDataURL(blob);
    });
    const base64 = String(dataUrl).split(",", 2)[1] || "";
    if (!base64) {
      return { ok: false, message: "Blob 编码失败" };
    }

    return {
      ok: true,
      filename: filename || filenameFromUrl(url) || `blob-${Date.now()}`,
      mime: blob.type || response.headers.get("content-type") || "",
      size: blob.size,
      base64
    };
  }

  async function downloadBlobInPage(url, filename) {
    const retainedFallback = await fallbackBlobDownloadFromPage(url, filename);
    if (retainedFallback.ok) {
      return retainedFallback;
    }
    const readResult = await readBlobInPage(url, filename);
    if (!readResult.ok) {
      return readResult;
    }

    const blob = await (await fetch(`data:${readResult.mime || "application/octet-stream"};base64,${readResult.base64}`)).blob();
    const objectUrl = URL.createObjectURL(blob);
    try {
      const anchor = document.createElement("a");
      anchor.href = objectUrl;
      anchor.download = readResult.filename || filename || filenameFromUrl(url) || `blob-${Date.now()}`;
      anchor.rel = "noopener";
      anchor.setAttribute("data-nxtools-pass-through", "1");
      anchor.style.display = "none";
      (document.body || document.documentElement).appendChild(anchor);
      anchor.click();
      anchor.remove();
      return { ok: true, message: `已在浏览器中下载 Blob 资源（${Math.round(blob.size / 1024)} KB）` };
    } finally {
      setTimeout(() => URL.revokeObjectURL(objectUrl), 30_000);
    }
  }

  content.blob = Object.freeze({
    requestBlobBytesFromPage,
    requestBlobPageAction,
    blobInfoFromPage,
    requestBlobChunkFromPage,
    uploadBlobFromPage,
    fallbackBlobDownloadFromPage,
    readBlobInPage,
    downloadBlobInPage
  });
})(globalThis.NxToolsContent || (globalThis.NxToolsContent = {}));
