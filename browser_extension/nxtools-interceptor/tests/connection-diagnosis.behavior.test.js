"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");

for (const [nativeError, code, label] of [
  ["Specified native messaging host not found. private-payload", "host_not_registered", "原生宿主未注册"],
  ["Access to the specified native messaging host is forbidden. private-payload", "host_forbidden", "扩展连接未获允许"],
  ["Failed to start native messaging host. C:\\private-payload", "host_start_failed", "原生宿主启动失败"],
  ["No such native application com.private-payload", "host_not_registered", "原生宿主未注册"]
]) test(`native failure ${code} has fixed actionable guidance without raw errors`, async () => {
  const bg = loadBackground();
  await bg.settle();
  bg.chrome.api.runtime.connectNative = () => { throw new Error(nativeError); };
  const result = await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  assert.equal(result.ok, false);
  assert.equal(result.code, code);
  assert.equal(result.message, label);
  assert.ok(result.guidance.length > 5);
  assert.ok(!JSON.stringify(result).includes("private-payload"));
});

test("connection timeout retains a classified diagnosis through passive refresh and allows explicit retry", async () => {
  const bg = loadBackground({ desktop: { helloFails: true } });
  await bg.settle();
  const checking = bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  await bg.waitFor(() => bg.chrome.nativePorts.length === 1);
  bg.clock.advance(1800);
  const result = await checking;
  assert.equal(result.code, "connection_timeout");
  const owner = { name: "nxtools-popup", sender: bg.senders.popup(), onDisconnect: { addListener() {} } };
  bg.chrome.api.runtime.onConnect.fire(owner);
  await bg.settle();
  bg.clock.advance(1800);
  await bg.settle();
  const count = bg.chrome.nativePorts.length;
  const refreshed = await bg.sendRuntimeMessage({ type: "popup_get_state", currentView: "settings" }, bg.senders.popup());
  assert.equal(refreshed.status.code, "connection_timeout");
  assert.equal(bg.chrome.nativePorts.length, count, "passive state does not reconnect after loss");
});

test("negative native hello is rejected immediately rather than mislabeled timeout", async () => {
  const bg = loadBackground({ desktop: { helloFails: true } });
  await bg.settle();
  const pending = bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  await bg.waitFor(() => bg.chrome.nativePorts.length === 1);
  bg.chrome.nativePorts[0].receive({ requestId: "native-hello", ok: false, message: "private-payload", data: {} });
  bg.clock.advance(1800);
  const result = await pending;
  assert.equal(result.code, "native_rejected");
  assert.ok(!JSON.stringify(result).includes("private-payload"));
});

test("connected native host distinguishes an unavailable desktop from protocol rejection", async () => {
  const bg = loadBackground({ desktop: { respond: () => ({ ok: false, error: "desktop_unavailable",
    message: "private-payload", data: { reason: "desktop_unavailable" } }) } });
  await bg.settle();
  const reply = await bg.sendRuntimeMessage({ type: "popup_test_connection" }, bg.senders.popup());
  assert.equal(reply.code, "desktop_unavailable");
  assert.match(reply.guidance, /启动 NxTools/);
  assert.ok(!JSON.stringify(reply).includes("private-payload"));
});
