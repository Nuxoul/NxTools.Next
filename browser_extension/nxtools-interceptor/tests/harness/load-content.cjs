"use strict";
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const root = path.resolve(__dirname, "../..");

function loadContent(options = {}) {
  const listeners = new Map(), messages = [], intervals = [], timeouts = [], observers = [], performanceObservers = [];
  let runtimeResponse = () => ({ ok: true, choices: [] });
  let timeoutSerial = 0;
  const elements = [], entries = [], runtimeListeners = [];
  class Element {
    constructor(tag = "div", attributes = {}) { this.tagName = tag.toUpperCase(); this.attributes = { ...attributes }; this.style = {}; this.children = []; }
    getAttribute(name) { return this.attributes[name] ?? null; }
    hasAttribute(name) { return name in this.attributes; }
    setAttribute(name, value) { this.attributes[name] = String(value); }
    querySelectorAll() { return this.children; }
    closest(selector) { return selector === "audio,video" ? this.parentElement || null : null; }
    getBoundingClientRect() { return { width: 320, height: 180, left: 0, top: 0, right: 320, bottom: 180 }; }
    appendChild(child) { this.children.push(child); }
    remove() {}
  }
  class HTMLMediaElement extends Element {}
  class HTMLAudioElement extends HTMLMediaElement { constructor(attributes) { super("audio", attributes); } }
  class HTMLVideoElement extends HTMLMediaElement { constructor(attributes) { super("video", attributes); } }
  class HTMLImageElement extends Element {}
  const addEventListener = (name, fn) => { if (!listeners.has(name)) listeners.set(name, []); listeners.get(name).push(fn); };
  const document = { title: "fixture", readyState: "complete", visibilityState: "visible", documentElement: new Element("html"),
    addEventListener, getElementById: () => null, querySelector: () => null, createElement: tag => new Element(tag),
    querySelectorAll(selector) { return selector === "*" ? elements : elements.filter(element => selector.includes(element.tagName.toLowerCase())); } };
  const realm = { console, URL, TextDecoder, TextEncoder, AbortController, DOMParser: options.DOMParser,
    Element, HTMLMediaElement, HTMLAudioElement, HTMLVideoElement, HTMLImageElement,
    innerWidth: 900, innerHeight: 900,
    location: { href: "https://example.test/watch", hostname: "example.test" }, document, addEventListener,
    performance: { timeOrigin: Date.now(), getEntriesByType: () => entries },
    MutationObserver: class { constructor(fn) { this.fn = fn; observers.push(this); } observe(target, options) { this.options = options; } },
    PerformanceObserver: class { constructor(fn) { this.fn = fn; performanceObservers.push(this); } observe(options) { this.options = options; } },
    visualViewport: null, requestAnimationFrame: () => 1,
    setTimeout(fn, ms = 0) { const timer = { id: ++timeoutSerial, fn, ms, cleared: false }; timeouts.push(timer); return timer.id; },
    clearTimeout(id) { const timer = timeouts.find(item => item.id === id); if (timer) timer.cleared = true; },
    setInterval(fn, ms) { intervals.push({ fn, ms }); return intervals.length; },
    getComputedStyle: element => ({ backgroundImage: element.backgroundImage || "none" }),
    fetch: options.fetch || (async () => { throw new Error("network blocked in content behavior tests"); }),
    chrome: { runtime: { id: "fixture", lastError: null, getURL: () => "data:text/javascript,",
      sendMessage(message, callback) { messages.push(message); Promise.resolve(runtimeResponse(message)).then(value => callback?.(value)); },
      onMessage: { addListener(fn) { runtimeListeners.push(fn); } } },
      storage: { local: { get(defaults, callback) { callback({ ...defaults, ...(options.storage || {}) }); } }, onChanged: { addListener() {} } } }
  };
  realm.window = realm;
  const context = vm.createContext(realm);
  const contentFiles = [...JSON.parse(fs.readFileSync(path.join(root, "manifest.json"), "utf8")).content_scripts[0].js];
  if (!contentFiles.includes("content/media-inspection.js")) contentFiles.splice(contentFiles.indexOf("content-script.js"), 0, "content/media-inspection.js");
  for (const file of contentFiles) {
    vm.runInContext(fs.readFileSync(path.join(root, file), "utf8"), context, { filename: file });
  }
  return { realm, messages, elements, entries, intervals, timeouts, observers, performanceObservers, Element, HTMLAudioElement, HTMLVideoElement,
    setRuntimeResponse(handler) { runtimeResponse = handler; },
    fire(name, event = {}) { for (const fn of listeners.get(name) || []) fn(event); },
    mutate(element, attributeName) { for (const observer of observers) if (observer.options.attributeFilter.includes(attributeName)) observer.fn([{ type: "attributes", target: element }]); },
    runTimeout(ms) { for (const timer of timeouts.filter(item => !item.cleared && item.ms === ms)) { timer.cleared = true; timer.fn(); } },
    dispatch(message) { let result; for (const fn of runtimeListeners) fn(message, {}, value => { result = value; }); return result; },
    dispatchAsync(message) { return new Promise(resolve => { let pending = false, responded = false; for (const fn of runtimeListeners) pending = fn(message, {}, value => { responded = true; resolve(value); }) === true || pending; if (!pending && !responded) resolve(undefined); }); }
  };
}
module.exports = { loadContent };
