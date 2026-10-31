"use strict";

// Loads the real production background scripts into an isolated vm realm with a
// programmable chrome API and a scripted fake desktop host. Chrome mode exposes
// importScripts (service worker); Firefox mode has no importScripts and loads the
// manifest background.scripts list in order (event page).

const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { webcrypto } = require("node:crypto");
const { createChromeApi } = require("./chrome-api.cjs");

const EXTENSION_ROOT = path.resolve(__dirname, "..", "..");

function readManifest() {
  return JSON.parse(fs.readFileSync(path.join(EXTENSION_ROOT, "manifest.json"), "utf8"));
}

function createClock() {
  let now = 1_800_000_000_000;
  let serial = 0;
  const timers = new Map();
  return {
    timers,
    now: () => now,
    setTimeout(fn, ms = 0, ...args) {
      const id = ++serial;
      timers.set(id, { fn, at: now + Number(ms || 0), ms: Number(ms || 0), args, repeat: false });
      return id;
    },
    setInterval(fn, ms = 0, ...args) {
      const id = ++serial;
      timers.set(id, { fn, at: now + Number(ms || 0), ms: Number(ms || 0), args, repeat: true });
      return id;
    },
    clearTimeout(id) { timers.delete(id); },
    // Advance virtual time, firing due timers in order (intervals are re-armed).
    advance(ms) {
      const target = now + ms;
      for (;;) {
        const due = [...timers.entries()]
          .filter(([, timer]) => timer.at <= target)
          .sort((left, right) => left[1].at - right[1].at)[0];
        if (!due) break;
        const [id, timer] = due;
        now = Math.max(now, timer.at);
        if (timer.repeat) timer.at = now + Math.max(1, timer.ms);
        else timers.delete(id);
        timer.fn(...timer.args);
      }
      now = target;
    }
  };
}

// Fake desktop: answers hello automatically and lets a test decide every request.
function attachFakeDesktop(chrome, options = {}) {
  const requests = [];
  let responder = options.respond || null;
  chrome.nativeRegistry.onPost = (port, message) => {
    if (message.type === "hello") {
      if (options.helloFails) return;
      queueMicrotask(() => port.receive({ requestId: "native-hello", ok: true, type: "hello_ack" }));
      return;
    }
    if (message.type === "extension_log") return;
    requests.push({ port, message });
    if (responder) {
      const reply = responder(message);
      if (reply !== undefined) {
        Promise.resolve(reply).then((value) => {
          if (value) port.receive({ requestId: message.requestId, ...value });
        });
      }
    }
  };
  return {
    requests,
    respondWith(fn) { responder = fn; },
    reply(message, value) { message.port.receive({ requestId: message.message.requestId, ...value }); },
    requestsOfType(type) { return requests.filter((entry) => entry.message.type === type); }
  };
}

function loadBackground(options = {}) {
  const manifest = readManifest();
  const firefox = options.browser === "firefox";
  const chrome = createChromeApi({ ...options.chrome, firefox });
  const clock = createClock();
  const desktop = attachFakeDesktop(chrome, options.desktop || {});
  const consoleSink = { warn() {}, error() {}, info() {}, log() {}, debug() {} };
  const loaded = [];

  const realm = {
    chrome,
    console: options.console || consoleSink,
    URL, URLSearchParams, TextEncoder, TextDecoder, AbortController, Blob,
    crypto: webcrypto,
    navigator: { language: "zh-CN", languages: ["zh-CN", "en"], userAgent: "fixture" },
    fetch: async () => { throw new Error("network blocked in behavior tests"); },
    queueMicrotask,
    setTimeout: clock.setTimeout,
    clearTimeout: clock.clearTimeout,
    setInterval: clock.setInterval,
    clearInterval: clock.clearTimeout,
    Date: class extends Date {
      constructor(...args) { super(...(args.length ? args : [clock.now()])); }
      static now() { return clock.now(); }
    }
  };
  realm.chrome = chrome.api;
  realm.globalThis = realm;
  realm.self = realm;
  const context = vm.createContext(realm);

  function runFile(relativePath) {
    const file = path.join(EXTENSION_ROOT, relativePath);
    loaded.push(relativePath);
    vm.runInContext(fs.readFileSync(file, "utf8"), context, { filename: file });
  }

  if (!firefox) {
    realm.importScripts = (...files) => files.forEach(runFile);
    runFile(manifest.background.service_worker);
  } else {
    const scripts = options.firefoxScripts || manifest.background.scripts || [manifest.background.service_worker];
    scripts.forEach(runFile);
  }

  async function settle(rounds = 12) {
    for (let index = 0; index < rounds; index += 1) {
      await new Promise((resolve) => setImmediate(resolve));
    }
  }

  // Deliver a runtime message the way Chrome does: listener returns true to keep
  // sendResponse alive; resolves with the response (or undefined if none).
  function sendRuntimeMessage(message, sender) {
    return new Promise((resolve) => {
      let responded = false;
      const results = chrome.api.runtime.onMessage.fire(message, sender, (value) => {
        responded = true;
        resolve(value);
      });
      if (!results.some((value) => value === true) && !responded) resolve(undefined);
    });
  }

  const senders = {
    popup: () => ({ id: chrome.extensionId, url: chrome.api.runtime.getURL("popup.html") }),
    content: (tab = { id: 7, url: "https://example.test/page" }, frameId = 0) => ({
      id: chrome.extensionId,
      tab,
      frameId,
      url: tab.url
    }),
    foreign: () => ({ id: "someotherextension", url: "chrome-extension://someotherextension/x.html" })
  };

  // Read script-scoped (const/let) background state, which vm does not expose
  // as context properties. Test-only; never used to mutate production state.
  function evaluate(expression) {
    return vm.runInContext(expression, context);
  }

  return { chrome, clock, desktop, context, realm, loaded, settle, sendRuntimeMessage, senders, manifest, evaluate };
}

module.exports = { loadBackground, readManifest, EXTENSION_ROOT };
