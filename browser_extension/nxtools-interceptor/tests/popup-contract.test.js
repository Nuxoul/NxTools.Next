"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const html = fs.readFileSync(path.join(root, "popup.html"), "utf8");
const css = fs.readFileSync(path.join(root, "popup.css"), "utf8");
const script = fs.readFileSync(path.join(root, "popup.js"), "utf8");

function cssRule(selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escapedSelector}\\s*\\{([\\s\\S]*?)\\}`));
  assert.ok(match, `${selector} rule should exist`);
  return match[1];
}

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
  assert.match(html, /id="batchResourcesButton"/);
  assert.match(html, /批量加入队列/);
  assert.match(script, /type:\s*"popup_send_resources"/);
  assert.match(html, /role="tablist"/);
  assert.equal((html.match(/role="tab"/g) || []).length, 4);
  assert.equal((html.match(/role="tabpanel"/g) || []).length, 4);
  assert.match(script, /event\.key === "ArrowRight"/);
  assert.match(script, /event\.key === "Home"/);
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
  assert.match(css, /:focus-visible/);
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(css, /signal-sweep/);
  assert.doesNotMatch(css, /width:\s*520px/);
});

test("popup root uses deterministic intrinsic dimensions", () => {
  const htmlRule = cssRule("html");
  const bodyRule = cssRule("body");
  const workspaceRule = cssRule(".workspace");

  assert.match(htmlRule, /width:\s*456px/);
  assert.match(bodyRule, /width:\s*456px/);
  assert.match(bodyRule, /min-width:\s*456px/);
  assert.doesNotMatch(bodyRule, /(?:width|min-width|max-width):[^;]*(?:vw|vh)/);
  assert.match(workspaceRule, /min-height:\s*420px/);
  assert.match(workspaceRule, /max-height:\s*445px/);
  assert.doesNotMatch(workspaceRule, /(?:vw|vh)/);
});

console.log("popup contract tests passed");
