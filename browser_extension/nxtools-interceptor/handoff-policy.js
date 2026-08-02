"use strict";

(function initializeHandoffPolicy(global) {
  async function handoffBrowserDownload(operations) {
    const accepted = await operations.handoff();
    if (accepted?.ok === false) {
      throw new Error(accepted.message || "NxTools 未接受下载任务");
    }
    try {
      await operations.cancelBrowser();
    } catch (error) {
      await operations.compensate(accepted, error);
      return { state: "browser_retained", accepted, error };
    }
    if (operations.eraseBrowser) {
      await operations.eraseBrowser();
    }
    return { state: "intercepted", accepted };
  }

  const api = { handoffBrowserDownload };
  global.NxToolsHandoffPolicy = api;
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
})(typeof globalThis !== "undefined" ? globalThis : self);
