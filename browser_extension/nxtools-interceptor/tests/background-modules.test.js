"use strict";

// Structural contract for the split background: one ordered script list shared
// by Chrome (importScripts) and Firefox (manifest background.scripts), a thin
// entry, bounded modules, and every declaration owned by exactly one module.

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { loadBackground, readManifest, EXTENSION_ROOT } = require("./harness/load-background.cjs");

const ENTRY_LINE_LIMIT = 200;
const MODULE_LINE_LIMIT = 900;

function lineCount(file) {
  return fs.readFileSync(path.join(EXTENSION_ROOT, file), "utf8").split(/\r?\n/).length;
}

test("Chrome importScripts order equals Firefox background.scripts order", () => {
  const manifest = readManifest();
  const bg = loadBackground();
  assert.deepEqual(bg.loaded.slice(1), manifest.background.scripts.slice(0, -1));
  assert.equal(manifest.background.scripts.at(-1), manifest.background.service_worker);
});

test("Firefox realm loads the same modules and registers the same listeners", async () => {
  const chrome = loadBackground();
  const firefox = loadBackground({ browser: "firefox" });
  await chrome.settle();
  await firefox.settle();
  assert.deepEqual(firefox.loaded, readManifest().background.scripts);
  assert.equal(firefox.chrome.api.runtime.onMessage.listeners.length, 1);
  assert.equal(chrome.chrome.api.runtime.onMessage.listeners.length, 1);
  assert.equal(firefox.chrome.api.webRequest.onResponseStarted.listeners.length, 1);
});

test("entry stays thin and each background module stays bounded", () => {
  const manifest = readManifest();
  assert.ok(lineCount("background.js") <= ENTRY_LINE_LIMIT, "background.js entry line budget");
  for (const file of manifest.background.scripts.filter((name) => name.startsWith("bg/"))) {
    assert.ok(lineCount(file) <= MODULE_LINE_LIMIT, `${file} exceeds ${MODULE_LINE_LIMIT} lines`);
  }
});

test("every top-level background declaration has exactly one owner", () => {
  const manifest = readManifest();
  const owners = new Map();
  for (const file of manifest.background.scripts.filter((name) => name === "background.js" || name.startsWith("bg/"))) {
    const source = fs.readFileSync(path.join(EXTENSION_ROOT, file), "utf8");
    for (const match of source.matchAll(/^(?:async\s+)?function\s+(\w+)|^(?:const|let)\s+(\w+)/gm)) {
      const name = match[1] || match[2];
      assert.ok(!owners.has(name), `${name} declared in both ${owners.get(name)} and ${file}`);
      owners.set(name, file);
    }
  }
  assert.ok(owners.size > 150, "declarations discovered");
});

test("every packaged background file exists on disk", () => {
  for (const file of readManifest().background.scripts) {
    assert.ok(fs.existsSync(path.join(EXTENSION_ROOT, file)), `${file} exists`);
  }
});
