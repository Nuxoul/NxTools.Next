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
  // EX-01: the popup mirrors the desktop zed_dark semantic palette through the
  // shared browser token file instead of an independent cyan-on-black scheme.
  const tokens = fs.readFileSync(path.join(root, "popup-tokens.css"), "utf8");
  for (const [name, value] of [
    ["--nt-canvas", "#1e1f22"],
    ["--nt-surface", "#242529"],
    ["--nt-surface-raised", "#2a2b2f"],
    ["--nt-surface-hover", "#303238"],
    ["--nt-border-subtle", "#34363b"],
    ["--nt-border", "#44474f"],
    ["--nt-text", "#e0e0e0"],
    ["--nt-text-secondary", "#b8bac0"],
    ["--nt-text-muted", "#858991"],
    ["--nt-accent", "#4791bf"],
    ["--nt-accent-soft", "#273744"],
    ["--nt-accent-text", "#d6e8f5"],
    ["--nt-warning", "#d6a85f"],
    ["--nt-danger", "#ff6b78"],
  ]) {
    assert.match(tokens, new RegExp(`${name.replace(/-/g, "\\-")}:\\s*${value}`, "i"));
  }
  assert.match(css, /--accent:\s*var\(--nt-accent\)/);
  assert.doesNotMatch(css, /#53c7d8/i);
  assert.doesNotMatch(css, /#0d0f11/i);
  assert.doesNotMatch(css, /rgba\(83,\s*199,\s*216/);
  assert.ok(
    html.indexOf("popup-tokens.css") >= 0 &&
      html.indexOf("popup-tokens.css") < html.indexOf("popup.css"),
    "popup-tokens.css must load before popup.css",
  );
  assert.match(css, /:focus-visible/);
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(css, /signal-sweep/);
  assert.doesNotMatch(css, /width:\s*520px/);
});

test("popup surfaces stay flat, opaque and grid-free (EX-02)", () => {
  // The shell canvas is one continuous surface; decorations, grid meshes and
  // translucent panels are retired. The signal sweep overlay keeps its
  // gradient until EX-09 removes the whole animation.
  const bodyRule = cssRule("body");
  const shellRule = cssRule(".shell");
  assert.doesNotMatch(bodyRule, /gradient/);
  assert.match(shellRule, /border:\s*1px solid var\(--nt-border-subtle\)/);
  assert.doesNotMatch(css, /repeating-linear-gradient/);
  assert.doesNotMatch(css, /linear-gradient\(90deg,\s*var\(--accent-soft\)/);
  assert.doesNotMatch(css, /rgba\(21,\s*25,\s*29/);
  assert.doesNotMatch(css, /rgba\(30,\s*36,\s*41/);
  assert.doesNotMatch(css, /rgba\(255,\s*255,\s*255/);
  assert.doesNotMatch(css, /#101316/i);
  assert.doesNotMatch(css, /#242b31/i);
});

test("popup typography keeps the 10px metadata floor and 11px Chinese body (EX-03)", () => {
  const sizes = [...css.matchAll(/font-size:\s*([0-9.]+)px/g)].map((match) =>
    parseFloat(match[1]),
  );
  assert.ok(sizes.length > 0, "popup should declare font sizes");
  for (const size of sizes) {
    assert.ok(
      size >= 10,
      `font-size ${size}px is below the 10px metadata floor`,
    );
    assert.ok(
      Number.isInteger(size),
      `font-size ${size}px must be an integer token value`,
    );
  }
  assert.match(css, /font-size:\s*11px/);
  assert.match(css, /font-size:\s*12px/);
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
