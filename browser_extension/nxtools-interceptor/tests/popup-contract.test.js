"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const html = fs.readFileSync(path.join(root, "popup.html"), "utf8");
const css = fs.readFileSync(path.join(root, "popup.css"), "utf8");
const script = fs.readFileSync(path.join(root, "popup.js"), "utf8");
const contentScript = fs.readFileSync(path.join(root, "content-script.js"), "utf8");
const shadowStyles = require(path.join(root, "shadow-styles.js"));

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
  assert.doesNotMatch(css, /width:\s*520px/);
});

test("popup surfaces stay flat, opaque and grid-free (EX-02)", () => {
  // The shell canvas is one continuous surface; decorations, grid meshes and
  // translucent panels are retired.
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

test("popup brand uses the official extension icon asset (EX-04)", () => {
  assert.ok(
    fs.existsSync(path.join(root, "icons", "icon-32.png")),
    "the official icon-32.png asset must exist",
  );
  assert.match(html, /<img class="brand-mark" src="icons\/icon-32\.png"/);
  assert.doesNotMatch(html, /<i><\/i><i><\/i><i><\/i>/);
  assert.doesNotMatch(css, /\.brand-mark i\b/);
  assert.doesNotMatch(css, /background:\s*var\(--accent-soft\)[^;]*\n\}\n\n\.brand-mark i/);
});

test("popup tabs keep text plus a 2px accent underline and quiet counts (EX-05)", () => {
  const activeTabRule = cssRule(".tab.active");
  assert.doesNotMatch(activeTabRule, /background/);
  assert.doesNotMatch(activeTabRule, /border/);
  assert.match(cssRule(".tab.active::after"), /background:\s*var\(--accent\)/);
  const countRule = cssRule(".tab-count");
  assert.doesNotMatch(countRule, /border/);
  assert.doesNotMatch(countRule, /border-radius/);
  assert.doesNotMatch(css, /\.tab\.active \.tab-count/);
});

test("resource rows keep three stable meta chips and move the rest to details (EX-06)", () => {
  // Exactly resolution, kind and size stay as chips, in that order.
  assert.match(
    script,
    /const chips = \[\s*resolution \? \{ text: resolution, important: true \} : null,\s*\{ text: resourceKindLabel\(info\.kind\), important: true \},\s*resource\.size \? \{ text: formatBytes\(resource\.size\) \} : null,?\s*\]\.filter\(Boolean\);/,
  );
  assert.doesNotMatch(script, /\{ text: info\.badge \|\| "资源", important: true \}/);
  assert.doesNotMatch(script, /resource\.source \? \{ text: sourceLabel\(resource\.source\) \} : null,/);
  assert.doesNotMatch(script, /trackHint \? \{ text: trackHint \}/);
  assert.doesNotMatch(script, /blobNote \? \{ text: blobNote \}/);
  // The moved context stays reachable: badge remains in the title tooltip and
  // track/blob hints join the title tooltip as well.
  assert.match(script, /info\.badge \|\| ""/);
  assert.match(script, /trackHint,\s*blobNote/);
});

test("capture tool toggles stay flat command rows, not a card wall (EX-07)", () => {
  const cardRule = cssRule(".feature-card");
  // Row grammar: flat, full-width, separated by one hairline, no card chrome.
  assert.match(cardRule, /border-radius:\s*0/);
  assert.match(cardRule, /border-bottom:\s*1px solid var\(--line-soft\)/);
  assert.match(cardRule, /background:\s*transparent/);
  assert.doesNotMatch(css, /\.feature-grid\s*\{[^}]*grid-template-columns:\s*repeat/);
  // The active state uses the desktop 2px left accent edge, not a card fill.
  assert.match(cssRule(".feature-card.active"), /inset 2px 0 var\(--accent\)/);
  // Index numerals are metadata and must use token colors.
  assert.match(cssRule(".feature-index"), /color:\s*var\(--muted\)/);
  assert.doesNotMatch(css, /#59646e/i);
});

test("popup settings reuse the desktop row, toggle and footer grammar (EX-08)", () => {
  const switchRowRule = cssRule(".switch-row");
  assert.match(switchRowRule, /grid-template-columns:\s*minmax\(0, 1fr\) auto/);
  const toggleRule = cssRule('input[type="checkbox"]');
  assert.doesNotMatch(toggleRule, /#[0-9a-fA-F]{6}/);
  assert.match(toggleRule, /border:\s*1px solid var\(--border\)/);
  assert.match(toggleRule, /background:\s*var\(--surface-raised\)/);
  const checkedRule = cssRule('input[type="checkbox"]:checked');
  assert.match(checkedRule, /background:\s*var\(--nt-accent\)/);
  const knobCheckedRule = cssRule('input[type="checkbox"]:checked::after');
  assert.match(knobCheckedRule, /background:\s*var\(--nt-canvas\)/);
  const smallRule = cssRule(".switch-row small");
  assert.match(smallRule, /font-size:\s*10px/);
  assert.doesNotMatch(css, /#3a444d/i);
  assert.doesNotMatch(css, /#1a1f24/i);
  assert.doesNotMatch(css, /#87919a/i);
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

test("signal rail pulses once per state change and never loops (EX-09)", () => {
  // The continuous capture sweep is retired; the rail plays a one-shot pulse
  // only when its state class actually changes.
  assert.doesNotMatch(css, /signal-sweep/);
  assert.doesNotMatch(css, /infinite/);
  assert.doesNotMatch(css, /#b5f4fb/i);
  const pulseRule = cssRule(".signal-rail.pulse .signal-track i");
  assert.match(pulseRule, /animation:\s*signal-pulse\s+240ms ease-out 1/);
  const railRule = cssRule(".signal-rail");
  assert.doesNotMatch(railRule, /rgba\(17,\s*20,\s*23/);
  assert.match(railRule, /background:\s*var\(--surface-0\)/);
  assert.match(cssRule(".signal-track"), /background:\s*var\(--surface-2\)/);
  // JS restarts the pulse only when the rail state string changes.
  assert.match(script, /dataset\.signalState/);
  assert.match(script, /classList\.remove\("pulse"\)/);
  assert.match(script, /classList\.add\("pulse"\)/);
});

test("popup css is token-pure with visible focus and reduced-motion gates (EX-10)", () => {
  // Every color resolves through popup-tokens.css (or color-mix over those
  // tokens); no raw hex or rgba literals remain anywhere in popup.css.
  assert.doesNotMatch(css, /#[0-9a-fA-F]{3,8}\b/);
  assert.doesNotMatch(css, /rgba?\(/);
  // Interactive elements keep a visible focus outline.
  assert.match(cssRule(":focus-visible"), /outline:\s*2px solid var\(--accent\)/);
  // Reduced motion clamps every animation and transition.
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(
    css,
    /prefers-reduced-motion:\s*reduce[\s\S]*animation-duration:\s*0\.01ms[\s\S]*transition-duration:\s*0\.01ms/,
  );
});

test("media entry is a 28px graphite icon button with the real download icon (EX-11)", () => {
  // EX-11 owns the media ENTRY's shadow style (since EX-14 it lives in the
  // shared shadow-styles module); the panel (EX-12) and the drop target
  // (EX-13) keep their own sections.
  const entryStyle = shadowStyles.mediaEntry;
  const entryTriggerStyle = entryStyle.split(".panel {")[0];
  // The shadow root defines the shared browser tokens locally until EX-14
  // extracts the style module.
  assert.match(entryStyle, /--nt-surface-raised:\s*#2a2b2f/);
  assert.match(entryStyle, /--nt-accent:\s*#4791bf/);
  // The cyan gradient ball grammar is retired from the trigger: teal gradients,
  // frosted blur, heavy shadows, cyan rings and the pill radius are all gone.
  assert.doesNotMatch(entryTriggerStyle, /rgba\(56,\s*214,\s*199/);
  assert.doesNotMatch(entryTriggerStyle, /rgba\(20,\s*138,\s*125/);
  assert.doesNotMatch(entryTriggerStyle, /#061412/i);
  assert.doesNotMatch(entryTriggerStyle, /backdrop-filter/);
  assert.doesNotMatch(entryTriggerStyle, /border-radius:\s*999px/);
  // The trigger is a 28px graphite control with the desktop 4px control radius.
  assert.match(entryTriggerStyle, /background:\s*var\(--nt-surface-raised,\s*#2a2b2f\)/);
  assert.match(entryTriggerStyle, /border:\s*1px solid var\(--nt-border,\s*#44474f\)/);
  assert.match(entryTriggerStyle, /border-radius:\s*4px/);
  // The real download glyph replaces the text arrow inside the entry button.
  assert.match(
    contentScript,
    /class="trigger"[^]*?M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z/,
  );
});

test("media panel is a 6px token popover with standard two-level resource rows (EX-12)", () => {
  // EX-12 owns the media PANEL style section (from `.panel {` to the end of
  // the entry style module) plus the choice-row builder. The drop target is
  // EX-13's separate shadow root and stays out of scope here.
  const panelStyle = shadowStyles.mediaEntry
    .split(".panel {")[1];
  // The teal panel grammar is retired: cyan border/accents, frosted blur,
  // the 9px panel radius, the 7px row radius and the 999px pill capsules.
  assert.doesNotMatch(panelStyle, /rgba\(56,\s*214,\s*199/);
  assert.doesNotMatch(panelStyle, /backdrop-filter/);
  assert.doesNotMatch(panelStyle, /border-radius:\s*9px/);
  assert.doesNotMatch(panelStyle, /border-radius:\s*7px/);
  assert.doesNotMatch(panelStyle, /border-radius:\s*999px/);
  assert.doesNotMatch(panelStyle, /0 18px 40px/);
  assert.doesNotMatch(panelStyle, /\.quality\b|\.kind\b|\.size\b/);
  // The panel is the standard 6px popover on the raised surface with the
  // shared border and one restrained single-layer shadow.
  assert.match(panelStyle, /background:\s*var\(--nt-surface-raised,\s*#2a2b2f\)/);
  assert.match(panelStyle, /border:\s*1px solid var\(--nt-border,\s*#44474f\)/);
  assert.match(panelStyle, /border-radius:\s*6px/);
  assert.match(panelStyle, /box-shadow:\s*0 10px 26px rgba\(0,\s*0,\s*0,\s*0\.28\)/);
  // Resource rows follow the two-level standard: hover lifts the background
  // only, the title line ellipses like the popup resource rows, and the meta
  // line is one muted mono string.
  assert.match(panelStyle, /\.choice:hover\s*{\s*background:\s*var\(--nt-surface-hover/);
  assert.match(panelStyle, /\.choice-file\s*{[^}]*white-space:\s*nowrap/);
  assert.match(panelStyle, /\.choice-meta\s*{[^}]*Cascadia Mono/);
  assert.match(panelStyle, /\.choice-meta\s*{[^}]*var\(--nt-text-muted,\s*#858991\)/);
  // The builder emits exactly two levels per row: title plus one
  // `quality · kind · size` meta line; the pill head is gone.
  const builder = contentScript
    .split("function showResourceChoices")[1]
    .split("panel.replaceChildren")[0];
  assert.doesNotMatch(builder, /choice-head|className = "quality"|className = "kind"|className = "size"/);
  assert.match(builder, /className = "choice-file"/);
  assert.match(builder, /className = "choice-meta"/);
  assert.match(builder, /resourceChoiceQuality[\s\S]*·[\s\S]*resourceChoiceTypeText[\s\S]*·[\s\S]*resourceChoiceSizeText/);
  // The shadow tokens gain the muted text color the meta line needs (mirrors
  // popup-tokens.css; EX-14 moved them into the shared module).
  const hostTokens = shadowStyles.mediaEntry
    .split(":host {")[1]
    .split(".trigger {")[0];
  assert.match(hostTokens, /--nt-text-muted:\s*#858991/);
});

test("drop target uses the real download icon on a quiet token surface (EX-13)", () => {
  // EX-13 owns the drop-target shadow style (since EX-14 it lives in the
  // shared shadow-styles module); the media panel above is EX-12's scope.
  const dropStyle = shadowStyles.dropTarget
    .split(".target {")[1];
  const dropHost = shadowStyles.dropTarget
    .split(".target {")[0];
  // The teal drop grammar is retired: dashed cyan border, frosted blur, the
  // 8px radius, the heavy two-digit shadow and the text arrow are all gone.
  assert.doesNotMatch(dropStyle, /rgba\(56,\s*214,\s*199/);
  assert.doesNotMatch(dropStyle, /backdrop-filter/);
  assert.doesNotMatch(dropStyle, /border-radius:\s*8px/);
  assert.doesNotMatch(dropStyle, /0 16px 36px/);
  assert.doesNotMatch(dropStyle, /#38d6c7|#8ee6dc|#eafffb/i);
  assert.doesNotMatch(contentScript, /class="icon">↓</);
  // Quiet token surface: opaque --nt-surface with a dashed shared border, the
  // 6px popover radius and one restrained single-layer shadow.
  assert.match(dropStyle, /background:\s*var\(--nt-surface,\s*#242529\)/);
  assert.match(dropStyle, /border:\s*1px dashed var\(--nt-border,\s*#44474f\)/);
  assert.match(dropStyle, /border-radius:\s*6px/);
  assert.match(dropStyle, /box-shadow:\s*0 10px 26px rgba\(0,\s*0,\s*0,\s*0\.28\)/);
  assert.match(dropStyle, /color:\s*var\(--nt-text,\s*#e0e0e0\)/);
  assert.match(dropStyle, /small\s*{[^}]*var\(--nt-text-secondary,\s*#b8bac0\)/);
  // Entering the target only enhances border and background: accent-soft fill
  // with a solid accent border; the icon and text stay untouched.
  const insideStyle = shadowStyles.dropTarget.split(":host(.inside)")[1];
  assert.match(
    insideStyle,
    /{\s*background:\s*var\(--nt-accent-soft,\s*#273744\);\s*border-color:\s*var\(--nt-accent,\s*#4791bf\);\s*border-style:\s*solid;\s*}/,
  );
  // The real download glyph replaces the text arrow inside the drop target
  // markup (the content-script region after the style-module interpolation).
  const dropMarkup = contentScript.split("NxToolsShadowStyles.dropTarget")[1];
  assert.match(
    dropMarkup,
    /class="icon"><svg viewBox="0 0 24 24"[^]*?M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z/,
  );
  // The drop-target shadow defines the shared tokens locally until EX-14
  // extracts the shared style module.
  assert.match(dropHost, /--nt-text-secondary:\s*#b8bac0/);
  assert.match(dropHost, /--nt-accent-soft:\s*#273744/);
});

test("media overlay exposes aria wiring, keyboard navigation and focus return (EX-15)", () => {
  // The trigger announces the popover relationship; the panel carries an
  // accessible name; the status line is a live region instead of a
  // display:none string invisible to assistive tech.
  assert.match(contentScript, /class="trigger"[^>]*aria-haspopup="true"/);
  assert.match(contentScript, /class="trigger"[^>]*aria-controls="nxtools-media-panel"/);
  assert.match(contentScript, /class="panel" id="nxtools-media-panel" aria-label="媒体资源列表" role="group" hidden/);
  assert.match(contentScript, /class="status" role="status" aria-live="polite"/);
  // The drop target is a pointer-only decoration for assistive tech.
  assert.match(contentScript, /dropTargetHost\.setAttribute\("aria-hidden", "true"\)/);
  // Choice rows carry a programmatic label, not only a mouse tooltip.
  assert.match(contentScript, /choice\.setAttribute\(\s*"aria-label",/);
  // Panel rows navigate with ArrowUp/ArrowDown (wrapping) and Home/End.
  const nav = contentScript
    .split('panel.addEventListener("keydown"')[1]
    .split('button.addEventListener("click"')[0];
  assert.match(nav, /"ArrowDown"/);
  assert.match(nav, /"ArrowUp"/);
  assert.match(nav, /"Home"/);
  assert.match(nav, /"End"/);
  assert.match(nav, /\.focus\(\)/);
  // Escape close returns focus to the trigger when the panel held it.
  const closer = contentScript
    .split("function closeOverlayPanel")[1]
    .split("function isOverlayEvent")[0];
  assert.match(closer, /panel\.contains\(activeElement\)/);
  assert.match(closer, /\.focus\(\)/);
  // The status line stays visually hidden but reachable by assistive tech.
  assert.match(shadowStyles.mediaEntry, /\.status\s*{[^}]*position:\s*absolute/);
});

console.log("popup contract tests passed");
