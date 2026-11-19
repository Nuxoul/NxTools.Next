"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
function render(status) {
  const target = { hidden: true, textContent: "", dataset: {} };
  const context = vm.createContext({ elements: { handoffStatus: target } });
  vm.runInContext(fs.readFileSync(path.join(__dirname, "../popup/render.js"), "utf8"), context);
  context.renderHandoff(status);
  assert.equal(target.hidden, false);
  return target.textContent;
}
test("controlled pending says paused while ordinary manual confirmation retains its own wording", () => {
  assert.match(render({ controlled: true, reason: "confirmation_pending" }), /原下载已暂停/);
  assert.match(render({ reason: "confirmation_pending" }), /尚未创建任务/);
});
test("post-cancel activation failure describes the same held task and explicit retry", () => {
  const text = render({ controlled: true, reason: "desktop_activation_pending", state: "failed" });
  assert.match(text, /原任务已取消/);
  assert.match(text, /尚未启动/);
  assert.match(text, /重试同一任务/);
  assert.doesNotMatch(text, /仍由浏览器处理/);
});
test("unknown cancellation and resume failure expose the correct recovery action", () => {
  assert.match(render({ reason: "browser_cancel_unconfirmed" }), /尚未确认.*保持暂停/);
  assert.match(render({ reason: "browser_resume_failed" }), /浏览器下载页恢复/);
});
