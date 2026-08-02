const assert = require("node:assert/strict");
const test = require("node:test");

global.crypto = require("node:crypto").webcrypto;
const {
  HOST_NAME,
  MAX_MESSAGE_BYTES,
  NativeTransport,
  decodedBase64Size
} = require("../native-transport.js");

function eventHook() {
  const listeners = [];
  return {
    addListener(listener) { listeners.push(listener); },
    emit(value) { for (const listener of listeners) listener(value); }
  };
}

function fakeRuntime() {
  const onMessage = eventHook();
  const onDisconnect = eventHook();
  const messages = [];
  return {
    runtime: {
      lastError: null,
      connectNative(name) {
        assert.equal(name, HOST_NAME);
        return {
          onMessage,
          onDisconnect,
          postMessage(message) { messages.push(message); }
        };
      }
    },
    messages,
    respond(result, ok = true, requestId = messages.at(-1)?.requestId) {
      onMessage.emit({
        protocolVersion: 1,
        requestId,
        ok,
        ...(ok ? { result } : { error: result })
      });
    }
  };
}

test("native requests correlate out-of-order responses", async () => {
  const fake = fakeRuntime();
  const transport = new NativeTransport(fake.runtime, 1000);
  const first = transport.request("ping", {}, { requestId: "first" });
  const second = transport.request("ping", {}, { requestId: "second" });
  assert.equal(fake.messages.length, 2);
  fake.respond({ value: 2 }, true, "second");
  fake.respond({ value: 1 }, true, "first");
  assert.deepEqual(await first, { value: 1 });
  assert.deepEqual(await second, { value: 2 });
});

test("oversized control messages are rejected before native dispatch", async () => {
  const fake = fakeRuntime();
  const transport = new NativeTransport(fake.runtime, 1000);
  await assert.rejects(
    transport.request("create_task", { value: "x".repeat(MAX_MESSAGE_BYTES) }),
    /超过安全上限/
  );
  assert.equal(fake.messages.length, 0);
});

test("blob staging is sequential, bounded and retains one idempotency key", async () => {
  const fake = fakeRuntime();
  const transport = new NativeTransport(fake.runtime, 1000);
  const raw = Buffer.alloc(400 * 1024, 7);
  const operation = transport.stageBlob({ url: "blob:test", fileName: "test.bin" }, raw.toString("base64"), "blob-key");
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(fake.messages[0].action, "blob_begin");
  assert.equal(fake.messages[0].payload.size, raw.length);
  fake.respond({ accepted: true });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(fake.messages.at(-1).action, "blob_chunk");
  assert.equal(fake.messages.at(-1).payload.index, 0);
  fake.respond({ accepted: true });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(fake.messages.at(-1).action, "blob_chunk");
  assert.equal(fake.messages.at(-1).payload.index, 1);
  fake.respond({ accepted: true });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(fake.messages.at(-1).action, "blob_commit");
  fake.respond({ ok: true });
  await operation;
  assert.equal(decodedBase64Size(raw.toString("base64")), raw.length);
});
