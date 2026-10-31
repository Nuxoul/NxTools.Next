"use strict";

// Structural contract for the content-script modules: one ordered list shared by
// the manifest and background re-injection, idempotent namespace modules with no
// shared page state, and a bounded main closure that owns all mutable state.

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { loadBackground, readManifest, EXTENSION_ROOT } = require("./harness/load-background.cjs");

const MAIN_LINE_LIMIT = 1600;
const MODULE_LINE_LIMIT = 400;

function read(file) {
  return fs.readFileSync(path.join(EXTENSION_ROOT, file), "utf8");
}

test("background re-injection uses exactly the manifest content script list", () => {
  const manifest = readManifest();
  const bg = loadBackground();
  const injected = bg.evaluate("CONTENT_SCRIPT_FILES");
  assert.deepEqual([...injected], manifest.content_scripts[0].js);
  for (const file of injected) assert.ok(fs.existsSync(path.join(EXTENSION_ROOT, file)), `${file} exists`);
});

test("content modules load before the main closure and after their dependencies", () => {
  const list = readManifest().content_scripts[0].js;
  const main = list.indexOf("content-script.js");
  assert.equal(main, list.length - 1, "main closure loads last");
  for (const file of list.filter((name) => name.startsWith("content/"))) {
    assert.ok(list.indexOf(file) < main);
  }
  assert.ok(list.indexOf("shadow-styles.js") < main && list.indexOf("blob-transfer-policy.js") < main);
});

test("content modules are idempotent, isolated and frozen", () => {
  const list = readManifest().content_scripts[0].js.filter((name) => name.startsWith("content/"));
  const realm = { window: {}, location: { href: "https://example.test/" }, chrome: { runtime: {} } };
  realm.globalThis = realm;
  const context = vm.createContext(realm);
  for (const file of list) vm.runInContext(read(file), context, { filename: file });
  const first = { ...realm.NxToolsContent };
  for (const file of list) vm.runInContext(read(file), context, { filename: file });
  for (const [key, api] of Object.entries(first)) {
    assert.equal(realm.NxToolsContent[key], api, `${key} not replaced by re-injection`);
    assert.ok(Object.isFrozen(api), `${key} exports are frozen`);
  }
  assert.deepEqual(Object.keys(first).sort(), ["blob", "choices", "constants", "geometry", "media", "runtime"]);
});

test("content modules hold no shared page state and stay bounded", () => {
  const shared = ["sent", "mediaCandidates", "detectedCount", "overlayHost", "overlayEnabled", "captureResourcesEnabled",
    "dropTargetHost", "dropTargetVisible", "dropTargetRect", "dropTargetHideTimer", "dropTargetSettings",
    "interceptDownloadsEnabled", "updateQueued", "selectedMediaIndex", "resetTimer", "lastPageKey"];
  for (const file of readManifest().content_scripts[0].js.filter((name) => name.startsWith("content/"))) {
    const source = read(file);
    for (const name of shared) {
      assert.doesNotMatch(source, new RegExp(`\\b${name}\\b`), `${file} must not reference shared ${name}`);
    }
    assert.ok(source.split(/\r?\n/).length <= MODULE_LINE_LIMIT, `${file} line budget`);
  }
  assert.ok(read("content-script.js").split(/\r?\n/).length <= MAIN_LINE_LIMIT, "main closure line budget");
});
