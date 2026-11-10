"use strict";
// Shared by the browser fixture and production-script element/API prechecks.
(function(root) {
  function holdNextRequests(runtime, types) {
    const original = runtime.sendMessage, remaining = new Set(types), requests = [];
    runtime.sendMessage = message => {
      if (!remaining.delete(message.type)) return original.call(runtime, message);
      const request = { id: requests.length + 1, type: message.type, state: "held", message: structuredClone(message) };
      requests.push(request);
      if (!remaining.size) runtime.sendMessage = original;
      return new Promise((resolve, reject) => { request.resolve = resolve; request.reject = reject; });
    };
    return {
      inspect: () => requests.map(({ id, type, state }) => ({ id, type, state })),
      release(id) {
        const request = requests.find(row => row.id === id);
        if (!request || request.state !== "held") throw new Error("held request is absent or already released");
        request.state = "released";
        Promise.resolve().then(() => original.call(runtime, request.message)).then(
          value => { request.state = "settled"; request.resolve(value); },
          error => { request.state = "failed"; request.reject(error); });
      }
    };
  }
  if (typeof module !== "undefined") module.exports = { holdNextRequests };
  else root.NxToolsHeldRequests = { holdNextRequests };
})(globalThis);
