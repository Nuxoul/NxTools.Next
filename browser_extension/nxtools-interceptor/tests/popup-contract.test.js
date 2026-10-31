"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const root = path.resolve(__dirname, "..");
const html = fs.readFileSync(path.join(root, "popup.html"), "utf8");
const css = fs.readFileSync(path.join(root, "popup.css"), "utf8");
const popupScripts = [...html.matchAll(/<script src="([^"]+)"><\/script>/g)].map(m => m[1]);
const script = popupScripts.map(f => fs.readFileSync(path.join(root, f), "utf8")).join("\n");
const contentScript = fs.readFileSync(path.join(root, "content-script.js"), "utf8");
const shadowStyles = require(path.join(root, "shadow-styles.js"));
function cssRule(selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escaped}\\s*\\{([\\s\\S]*?)\\}`));
  assert.ok(match, `${selector} rule should exist`); return match[1];
}
function test(name, callback) { callback(); console.log(`ok - ${name}`); }

test("production popup retains all semantic pages and original controls", () => {
  for (const landmark of ["<header", "<nav", "<main", "<footer"]) assert.match(html, new RegExp(landmark));
  assert.equal((html.match(/role="tab"/g) || []).length, 4);
  assert.equal((html.match(/role="tabpanel"/g) || []).length, 4);
  assert.match(script, /event\.key === "ArrowRight"/); assert.match(script, /event\.key === "Home"/);
  for (const text of ["弹出下载确认窗口", "连接 NxTools 桌面端", "批量加入队列", "更改会自动保存", "携带 Cookie", "不会自动启动桌面端"]) assert.ok(html.includes(text));
  assert.doesNotMatch(html, /mergeResourcesButton|合并 · 后续/, "disabled merge placeholder is not offered");
  assert.match(script, /type:\s*"popup_send_resources"/);
  assert.doesNotMatch(html, /WebSocket 地址|交接后等待手动开始/);
});
test("every production script id appears exactly once", () => {
  const ids = [...script.matchAll(/getElementById\("([^"]+)"\)/g)].map(m => m[1]); assert.ok(ids.length > 30);
  for (const id of ids) assert.equal([...html.matchAll(new RegExp(`id=["']${id}["']`, "g"))].length, 1, id);
});
test("neutral browser token vocabulary does not silently change desktop", () => {
  const tokens = fs.readFileSync(path.join(root, "popup-tokens.css"), "utf8");
  for (const [name,value] of [["canvas","#18181b"],["surface","#1f1f23"],["text","#f4f4f5"],["accent","#bfc0c4"],["danger","#ff6b78"]]) assert.match(tokens,new RegExp(`--nt-${name}:\\s*${value}`));
  assert.ok(html.indexOf("popup-tokens.css") < html.indexOf("popup.css"));
  assert.doesNotMatch(css, /#[0-9a-fA-F]{3,8}\b|rgba?\(/);
});
test("continuous opaque workbench has no decorative ticker or looping animation", () => {
  assert.match(cssRule(".shell"), /border:\s*1px solid var\(--nt-border-subtle\)/);
  assert.doesNotMatch(css,/gradient|infinite|signal-pulse|signal-sweep/);
  assert.doesNotMatch(html,/signal-track|feature-index|eyebrow/);
  assert.match(cssRule(".capture-summary"),/font-size:\s*12px/);
});
test("readable typography retains integer scale and complete feedback", () => {
  for (const match of css.matchAll(/font-size:\s*([0-9.]+)px/g)) assert.ok(Number.isInteger(Number(match[1])) && Number(match[1]) >= 11);
  assert.match(cssRule(":root"),/font-size:\s*13px/); assert.match(cssRule(":root"),/line-height:\s*20px/);
  assert.match(cssRule(".switch-row small"),/font-size:\s*12px/);
  assert.match(cssRule(".message"),/white-space:\s*normal/); assert.match(cssRule(".message"),/overflow-y:\s*auto/);
});
test("brand uses official asset; page context stays outside hidden panels", () => {
  assert.match(html, /<img class="brand-mark" src="icons\/icon-32\.png"/);
  assert.ok(html.indexOf('id="activeTabTitle"') < html.indexOf('id="tasksView"'));
  assert.match(script,/elements\.activeTabTitle\.title/);
});
test("tabs have stable underline and filter pressed state", () => {
  assert.match(cssRule(".tab.active::after"),/height:\s*2px/);
  assert.match(cssRule(".tab.active::after"),/background:\s*var\(--accent\)/);
  assert.match(script,/item\.setAttribute\("aria-pressed", String\(item === button\)\)/);
  assert.match(script,/button\.setAttribute\("aria-pressed", String\(Boolean\(enabled\)\)\)/);
});
test("resource context and three metadata fields remain available", () => {
  assert.match(script,/function resourceMetaText\(resource\)/);
  for (const field of ["resourceKindLabel", "info.resolution", "formatBytes(resource.size)"]) assert.ok(script.includes(field), field);
  assert.match(script,/info\.badge \|\| ""/); assert.match(script,/trackHint, blobNote/);
  assert.match(script,/resourcePreview: resource\.id/); assert.match(script,/`选择 \$\{name\}`/);
  assert.doesNotMatch(script,/innerHTML|escapeHtml/, "popup renders with DOM APIs only");
  assert.match(cssRule("input.resource-check"),/width:\s*16px/);
  assert.doesNotMatch(cssRule("input.resource-check"),/display:\s*none/);
});
test("task and batch ownership, caps and partial retry remain", () => {
  assert.match(script,/tasks\.slice\(0, 8\)/); assert.match(script,/visibleResources\.slice\(0, 24\)/);
  assert.match(script,/selectedResourceIds\.size > 32/); assert.match(script,/result\.acceptedResourceIds \|\| \[\]/);
  assert.match(script,/selectedResourceIds\.delete\(resourceId\)/);
  assert.match(script,/isQueuedDesktopResult\(result\)/);
});
test("media tools are flat named action rows not indexed cards", () => {
  assert.match(cssRule(".feature-card"),/border-radius:\s*0/);
  assert.match(cssRule(".feature-card"),/border-bottom:\s*1px solid var\(--line-soft\)/);
  assert.match(cssRule(".feature-card"),/background:\s*transparent/);
  assert.match(html,/aria-pressed="false"/);
  assert.match(script,/mediaActionInFlight/);
});
test("settings switches and original auto-save policy are retained", () => {
  assert.match(cssRule(".switch-row"),/grid-template-columns:\s*minmax\(0, 1fr\) auto/);
  assert.match(cssRule('input[type="checkbox"]'),/border:\s*1px solid var\(--border\)/);
  assert.match(cssRule('input[type="checkbox"]:checked'),/background:\s*var\(--nt-accent\)/);
  assert.match(script,/input\.addEventListener\("change", async/);
  assert.match(script,/type:\s*"popup_save_settings"/); assert.match(script,/collectSettings\(\)/);
});
test("popup intrinsic budget remains456 with bounded original scroll", () => {
  assert.match(cssRule("html"),/width:\s*456px/); assert.match(cssRule("body"),/min-width:\s*456px/);
  assert.match(cssRule(".workspace"),/min-height:\s*420px/); assert.match(cssRule(".workspace"),/max-height:\s*445px/);
  assert.match(cssRule(".batch-toolbar"),/position:\s*sticky/);
});
test("focus and reduced motion remain explicit in popup and both shadow roots", () => {
  assert.match(cssRule(":focus-visible"),/outline:\s*2px solid var\(--accent\)/);
  assert.match(css,/prefers-reduced-motion:\s*reduce[\s\S]*transition-duration:\s*0\.01ms/);
  for(const style of [shadowStyles.mediaEntry,shadowStyles.dropTarget]) assert.match(style,/prefers-reduced-motion:\s*reduce/);
});
test("media entry retains real glyph and original28px placement", () => {
  assert.match(shadowStyles.mediaEntry,/width:\s*28px; height:\s*28px/);
  assert.match(contentScript,/class="trigger"[^]*?M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z/);
  assert.match(shadowStyles.mediaEntry,/\.status\s*{[^}]*position:\s*fixed/);
  assert.doesNotMatch(shadowStyles.mediaEntry.split('.status {')[1].split('}')[0],/height:\s*1px/);
});
test("media chooser remains two-level and bounded", () => {
  assert.match(shadowStyles.mediaEntry,/\.panel\s*{[^}]*max-height:\s*min\(386px/);
  assert.match(shadowStyles.mediaEntry,/\.choice-file\s*{[^}]*white-space:\s*nowrap/);
  assert.match(shadowStyles.mediaEntry,/\.choice-meta\s*{[^}]*font-size:\s*12px/);
  const builder=contentScript.split("function showResourceChoices")[1].split("panel.replaceChildren")[0];
  assert.match(builder,/className = "choice-file"/);assert.match(builder,/className = "choice-meta"/);
  assert.match(builder,/choice\.setAttribute\([\s\S]*"aria-label"/);
});
test("drop target retains declared pointer boundary and configured minimum", () => {
  assert.match(contentScript,/dropTargetHost\.setAttribute\("aria-hidden", "true"\)/);
  assert.match(shadowStyles.dropTarget,/border:\s*1px dashed var\(--nt-border\)/);
  assert.match(shadowStyles.dropTarget,/@container \(max-height:\s*65px\)/);
  assert.match(contentScript.split("NxToolsShadowStyles.dropTarget")[1],/M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z/);
});
test("original overlay keyboard navigation and focus return stay live", () => {
  assert.match(contentScript,/aria-controls="nxtools-media-panel"/);
  assert.match(contentScript,/class="panel" id="nxtools-media-panel" aria-label="媒体资源列表" role="group" hidden/);
  assert.match(contentScript,/class="status" role="status" aria-live="polite"/);
  const nav=contentScript.split('panel.addEventListener("keydown"')[1].split('button.addEventListener("click"')[0];
  for(const key of ['ArrowDown','ArrowUp','Home','End']) assert.ok(nav.includes(key));
  const closer=contentScript.split("function closeOverlayPanel")[1].split("function isOverlayEvent")[0];
  assert.match(closer,/panel\.contains\(activeElement\)/);assert.match(closer,/\.focus\(\)/);
});
console.log("popup contract tests passed");
