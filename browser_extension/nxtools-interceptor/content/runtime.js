"use strict";

// Disconnect-safe runtime messaging to the background worker.
// Isolated-world content module: registered once on the extension-private
// NxToolsContent namespace (re-injection is a no-op). Holds no shared mutable
// page state; content-script.js owns all overlay/drop/capture state.
(function installNxToolsContentRuntime(content) {
  if (content.runtime) {
    return;
  }

  function runtimeLastError() {
    try {
      return chrome.runtime.lastError || null;
    } catch {
      return { message: "Extension context invalidated." };
    }
  }

  function safeRuntimeSendMessage(message, callback) {
    try {
      if (!globalThis.chrome?.runtime?.id || typeof chrome.runtime.sendMessage !== "function") {
        callback?.(undefined, { message: "Extension context invalidated." });
        return false;
      }

      chrome.runtime.sendMessage(message, (result) => {
        callback?.(result, runtimeLastError());
      });
      return true;
    } catch (error) {
      callback?.(undefined, {
        message: error instanceof Error ? error.message : "Extension message failed."
      });
      return false;
    }
  }

  function runtimeMessage(message) {
    return new Promise((resolve) => {
      safeRuntimeSendMessage(message, (result, lastError) => {
        resolve({ result, lastError });
      });
    });
  }

  content.runtime = Object.freeze({
    runtimeLastError,
    safeRuntimeSendMessage,
    runtimeMessage
  });
})(globalThis.NxToolsContent || (globalThis.NxToolsContent = {}));
