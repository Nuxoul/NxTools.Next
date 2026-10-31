"use strict";

// Demand-only Native Messaging lifetime, exercised on the real loaded background
// scripts (harness realm, fake native port, virtual clock). No source slicing.

const test = require("node:test");
const assert = require("node:assert/strict");
const { loadBackground } = require("./harness/load-background.cjs");
const { backgroundSource, extractDeclaration } = require("./harness/background-source.cjs");

const IDLE_RELEASE_MS = 1000;
const CONNECT_TIMEOUT_MS = 1800;

async function fixture(desktop = {}) {
  const bg = loadBackground({ desktop });
  await bg.settle();
  return {
    bg,
    api: bg.context,
    ports: bg.chrome.nativePorts,
    async flush() { await bg.settle(6); },
    idle() { bg.clock.advance(IDLE_RELEASE_MS + 10); },
    pendingCount() { return bg.evaluate("pendingDesktopRequests.size"); }
  };
}

test("browser startup and settling never open the native host", async () => {
  const f = await fixture();
  f.idle();
  await f.flush();
  assert.equal(f.ports.length, 0, "startup must not create a resident host");
});

test("native port releases when idle and never reconnects without a request", async () => {
  const f = await fixture({ respond: () => ({ ok: true, data: {} }) });
  await f.api.sendDesktopRequest("get_snapshot");
  assert.equal(f.ports.length, 1);
  f.idle();
  await f.flush();
  assert.equal(f.ports[0].closed, true, "idle host receives EOF through port disconnect");
  f.idle();
  await f.flush();
  assert.equal(f.ports.length, 1, "disconnect must not relaunch an idle host");
});

test("diagnostics do not launch an idle host", async () => {
  const f = await fixture();
  f.api.sendExtensionLog("info", "fixture_event", "fixture", {}, 0);
  await f.flush();
  assert.equal(f.ports.length, 0);
});

test("concurrent requests and a Blob lease keep the port until the last owner releases", async () => {
  const f = await fixture();
  const release = f.api.retainDesktopPort();
  const first = f.api.sendDesktopRequest("prepare_blob_upload");
  await f.flush();
  const second = f.api.sendDesktopRequest("get_snapshot");
  await f.flush();
  const [firstReq, secondReq] = f.bg.desktop.requests;
  f.bg.desktop.reply(firstReq, { ok: true });
  await first;
  f.idle();
  assert.equal(f.ports[0].closed, false, "pending second request keeps the host");
  f.bg.desktop.reply(secondReq, { ok: true });
  await second;
  f.idle();
  assert.equal(f.ports[0].closed, false, "Blob capability/upload/commit owner retains the host");
  release();
  await f.flush();
  f.idle();
  assert.equal(f.ports[0].closed, true);
});

test("disconnect rejects once, timeout releases, and a stale port cannot affect its replacement", async () => {
  const f = await fixture();
  const pending = f.api.sendDesktopRequest("task_action");
  await f.flush();
  f.ports[0].disconnect();
  await assert.rejects(pending, /断开/);
  assert.equal(f.pendingCount(), 0);
  f.idle();
  assert.equal(f.ports.length, 1);

  const next = f.api.sendDesktopRequest("get_snapshot", {}, 2200);
  await f.flush();
  assert.equal(f.ports.length, 2);
  f.ports[0].receive({ requestId: "native-hello", ok: true });
  assert.equal(f.bg.evaluate("connectionState"), "connected");
  f.bg.clock.advance(2200);
  await assert.rejects(next, /超时/);
  await f.flush();
  f.idle();
  assert.equal(f.ports[1].closed, true);
});

test("failed handshake and failed send release ownership without reconnecting", async () => {
  const silent = await fixture({ helloFails: true });
  const opening = silent.api.sendDesktopRequest("get_snapshot");
  await silent.flush();
  silent.bg.clock.advance(CONNECT_TIMEOUT_MS + 10);
  await assert.rejects(opening);
  await silent.flush();
  assert.equal(silent.ports[0].closed, true);
  assert.equal(silent.pendingCount(), 0);
  silent.idle();
  assert.equal(silent.ports.length, 1);

  const f = await fixture();
  f.bg.chrome.nativeRegistry.onPost = (port, message) => {
    if (message.type === "hello") {
      queueMicrotask(() => {
        port.postMessage = () => { throw new Error("synthetic post failure"); };
        port.receive({ requestId: "native-hello", ok: true });
      });
    }
  };
  await assert.rejects(f.api.sendDesktopRequest("get_snapshot"), /发送失败/);
  await f.flush();
  assert.equal(f.pendingCount(), 0);
  f.idle();
  assert.equal(f.ports[0].closed, true);
});

test("the multi-step Blob flow retains and releases an explicit lease", () => {
  const blob = extractDeclaration(backgroundSource(), "sendBlobResourceViaContentScript");
  assert.ok(blob.includes("retainDesktopPort()") && blob.includes("finally"),
    "multi-step Blob operation owns and retires an explicit lease");
});
