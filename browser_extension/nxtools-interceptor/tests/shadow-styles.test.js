"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const contentScript = fs.readFileSync(path.join(root, "content-script.js"), "utf8");
const manifest = JSON.parse(fs.readFileSync(path.join(root, "manifest.json"), "utf8"));
const styles = require(path.join(root, "shadow-styles.js"));

function test(name, callback) {
  try {
    callback();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

function parseTokenDeclarations(css) {
  const map = new Map();
  for (const match of css.matchAll(/(--nt-[a-z-]+):\s*(#[0-9a-fA-F]{6})\s*;/g)) {
    map.set(match[1], match[2].toLowerCase());
  }
  return map;
}

test("shadow tokens mirror popup-tokens.css declaration for declaration", () => {
  const popupTokens = fs.readFileSync(path.join(root, "popup-tokens.css"), "utf8");
  assert.deepEqual(
    parseTokenDeclarations(styles.tokens),
    parseTokenDeclarations(popupTokens),
    "shadow-styles tokens and popup-tokens.css must stay one vocabulary",
  );
});

test("both shadow roots interpolate the shared module (EX-14)", () => {
  assert.match(
    contentScript,
    /<style>\s*\$\{NxToolsShadowStyles\.mediaEntry\}\s*<\/style>/,
  );
  assert.match(
    contentScript,
    /<style>\s*\$\{NxToolsShadowStyles\.dropTarget\}\s*<\/style>/,
  );
  // Page isolation: the content script carries no inline style copies
  // anymore - tokens, radii and surfaces live only in the module.
  assert.doesNotMatch(contentScript, /backdrop-filter/);
  assert.doesNotMatch(contentScript, /rgba\(56,\s*214,\s*199/);
  assert.doesNotMatch(contentScript, /:host\s*\{/);
  assert.doesNotMatch(contentScript, /--nt-[a-z-]+:\s*#/);
  // The manifest loads the style module before the content script.
  const js = manifest.content_scripts[0].js;
  assert.ok(
    js.indexOf("shadow-styles.js") !== -1 &&
      js.indexOf("shadow-styles.js") < js.indexOf("content-script.js"),
    "shadow-styles.js must be injected before content-script.js",
  );
});

test("module keeps the slice contracts reachable (EX-11/12/13)", () => {
  // EX-11 trigger: 28px graphite control on the desktop 4px control radius.
  assert.match(styles.mediaEntry, /border-radius:\s*4px/);
  // EX-12 panel: standard 6px popover, two-level rows with the muted mono meta.
  assert.match(styles.mediaEntry, /\.panel\s*{[^}]*border-radius:\s*6px/);
  assert.match(styles.mediaEntry, /\.choice-meta\s*{[^}]*var\(--nt-text-muted,\s*#858991\)/);
  // EX-13 drop target: quiet surface, dashed shared border, accent entry.
  assert.match(styles.dropTarget, /border:\s*1px dashed var\(--nt-border,\s*#44474f\)/);
  assert.match(
    styles.dropTarget,
    /:host\(\.inside\)\s+\.target\s*{[^}]*var\(--nt-accent-soft,\s*#273744\)/,
  );
  // Neither section reintroduces the retired teal grammar.
  const both = `${styles.mediaEntry}\n${styles.dropTarget}`;
  assert.doesNotMatch(both, /backdrop-filter|rgba\(56,\s*214,\s*199|border-radius:\s*9px|border-radius:\s*999px/);
});

console.log("shadow styles tests passed");
