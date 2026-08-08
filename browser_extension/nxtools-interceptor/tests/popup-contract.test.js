"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const html = fs.readFileSync(path.join(root, "popup.html"), "utf8");
const css = fs.readFileSync(path.join(root, "popup.css"), "utf8");
const script = fs.readFileSync(path.join(root, "popup.js"), "utf8");

function test(name, callback) {
  try {
    callback();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

test("popup uses semantic landmarks and an honest browser handoff label", () => {
  for (const landmark of ["<header", "<nav", "<main", "<footer"]) {
    assert.match(html, new RegExp(landmark, "i"));
  }
  assert.match(html, /弹出下载确认窗口/);
  assert.doesNotMatch(html, /交接后等待手动开始/);
  assert.match(html, /Native Messaging/);
  assert.match(html, /合并 · 后续/);
});

test("every popup script element id exists exactly once", () => {
  const referencedIds = [...script.matchAll(/getElementById\("([^"]+)"\)/g)].map((match) => match[1]);
  assert.ok(referencedIds.length > 30);
  for (const id of referencedIds) {
    const occurrences = [...html.matchAll(new RegExp(`id=["']${id}["']`, "g"))].length;
    assert.equal(occurrences, 1, `${id} should exist exactly once`);
  }
});

test("popup visual system keeps compact tokens, focus and reduced-motion gates", () => {
  assert.match(css, /--accent:\s*#53c7d8/i);
  assert.match(css, /width:\s*456px/);
  assert.match(css, /:focus-visible/);
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(css, /signal-sweep/);
  assert.doesNotMatch(css, /width:\s*520px/);
});

console.log("popup contract tests passed");
