"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { loadContent } = require("./harness/load-content.cjs");

function responseFrom(text, mime, options = {}) {
  const bytes = new TextEncoder().encode(text);
  const chunks = options.chunks || [bytes];
  let index = 0;
  const stats = { reads: 0, cancels: 0 };
  const reader = {
    read() {
      stats.reads += 1;
      return Promise.resolve(index < chunks.length ? { done: false, value: chunks[index++] } : { done: true });
    },
    cancel() { stats.cancels += 1; return Promise.resolve(); }
  };
  const body = {
    getReader() { return reader; },
    cancel() { stats.cancels += 1; return Promise.resolve(); }
  };
  return {
    response: {
      ok: options.ok !== false,
      status: options.status || 200,
      headers: { get(name) {
        if (String(name).toLowerCase() === "content-type") return mime;
        if (String(name).toLowerCase() === "content-length") return options.contentLength == null ? String(bytes.byteLength) : String(options.contentLength);
        return null;
      } },
      body
    },
    stats
  };
}

async function inspect(fixture, extra = {}) {
  return fixture.dispatchAsync({
    type: "content_inspect_media_resource",
    url: "https://cdn.test/media",
    pageKey: "https://example.test/watch",
    mime: "",
    includeCredentials: false,
    ...extra
  });
}

test("HLS distinguishes identity AES-128 from protected SAMPLE-AES and does not recurse", async () => {
  const bodies = [
    ["#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"secret.key\"\n#EXTINF:4,\nsegment.ts\n#EXT-X-ENDLIST", "ready", true],
    ["#EXTM3U\n#EXT-X-SESSION-KEY:METHOD=SAMPLE-AES,KEYFORMAT=\"com.apple.streamingkeydelivery\"\n#EXT-X-STREAM-INF:BANDWIDTH=1\nchild.m3u8", "protected", true]
  ];
  for (const [body, state, encrypted] of bodies) {
    const source = responseFrom(body, "application/vnd.apple.mpegurl");
    const requests = [];
    const f = loadContent({ fetch: async (url, options) => { requests.push({ url, options }); return source.response; } });
    const result = await inspect(f, { includeCredentials: true });
    assert.equal(result.inspection.state, state);
    assert.equal(result.inspection.kind, "hls");
    assert.equal(result.inspection.encrypted, encrypted);
    assert.equal(result.inspection.requiresMerge, false);
    assert.equal(requests.length, 1, "no key, child playlist, variant or segment request");
    assert.equal(requests[0].options.credentials, "include");
    assert.equal("body" in result, false);
  }
});

test("HLS live and DASH split/protected summaries come from bounded production parsing", async () => {
  const hls = responseFrom("#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXTINF:4,\na.ts", "application/x-mpegurl");
  const hlsFixture = loadContent({ fetch: async () => hls.response });
  const hlsResult = await inspect(hlsFixture);
  assert.equal(hlsResult.inspection.live, true);

  const dashBody = `<?xml version="1.0"?><MPD type="dynamic"><Period>
    <AdaptationSet contentType="video"><Representation mimeType="video/mp4"/></AdaptationSet>
    <AdaptationSet mimeType="audio/mp4"><ContentProtection schemeIdUri="urn:mpeg:dash:mp4protection:2011"/></AdaptationSet>
  </Period></MPD>`;
  const dash = responseFrom(dashBody, "application/dash+xml");
  const dashFixture = loadContent({ fetch: async () => dash.response });
  const dashResult = await inspect(dashFixture);
  assert.deepEqual(JSON.parse(JSON.stringify(dashResult.inspection)), {
    state: "protected", kind: "dash", requiresMerge: true, encrypted: true, live: true,
    message: "检测到受保护的 DASH 媒体", reason: "protected"
  });
});

test("plain audio/video responses classify without consuming the body and obey credential omission", async () => {
  for (const [mime, kind] of [["video/mp4", "video"], ["audio/mpeg", "audio"]]) {
    const source = responseFrom("unused payload", mime);
    let fetchOptions;
    const f = loadContent({ fetch: async (_url, options) => { fetchOptions = options; return source.response; } });
    const result = await inspect(f, { includeCredentials: false });
    assert.equal(result.inspection.kind, kind);
    assert.equal(result.inspection.requiresMerge, false, "video alone is not proof of an audio track");
    assert.equal(source.stats.reads, 0);
    assert.equal(source.stats.cancels, 1);
    assert.equal(fetchOptions.credentials, "omit");
  }
});

test("manifest reads stop at 2 MiB and timeout even when fetch ignores abort", async () => {
  const tooLarge = responseFrom("#EXTM3U", "application/vnd.apple.mpegurl", { contentLength: 2 * 1024 * 1024 + 1 });
  const largeFixture = loadContent({ fetch: async () => tooLarge.response });
  const large = await inspect(largeFixture);
  assert.equal(large.inspection.reason, "too_large");
  assert.equal(tooLarge.stats.reads, 0);
  assert.equal(tooLarge.stats.cancels, 1);

  const timeoutFixture = loadContent({ fetch: () => new Promise(() => {}) });
  const pending = inspect(timeoutFixture);
  timeoutFixture.runTimeout(5000);
  const timedOut = await pending;
  assert.equal(timedOut.inspection.reason, "timeout");
  assert.equal(timeoutFixture.timeouts.filter(timer => !timer.cleared).length, 0);
});

test("unknown responses sniff only a bounded prefix and cancel unused readers", async () => {
  const chunk = new Uint8Array(5000).fill(65);
  const source = responseFrom("", "application/octet-stream", { chunks: [chunk], contentLength: "" });
  const f = loadContent({ fetch: async () => source.response });
  const result = await inspect(f);
  assert.equal(result.inspection.kind, "file");
  assert.equal(source.stats.reads, 1);
  assert.equal(source.stats.cancels, 1);

  const html = responseFrom("<html>not media</html>", "text/html", { contentLength: "" });
  const htmlResult = await inspect(loadContent({ fetch: async () => html.response }));
  assert.equal(htmlResult.inspection.reason, "unsupported");
});

test("page changes during a delayed body read return page_changed", async () => {
  let release;
  const source = responseFrom("", "application/vnd.apple.mpegurl", { contentLength: "" });
  source.response.body.getReader = () => ({
    read: () => new Promise(resolve => { release = resolve; }),
    cancel: () => Promise.resolve()
  });
  const f = loadContent({ fetch: async () => source.response });
  const pending = inspect(f);
  while (!release) await new Promise(resolve => setImmediate(resolve));
  f.realm.location.href = "https://example.test/next";
  release({ done: true });
  const result = await pending;
  assert.equal(result.inspection.reason, "page_changed");
});

test("disabled capture plus overlay skips mutation subtree enumeration while timers remain", () => {
  const f = loadContent({ storage: { captureResources: false, mediaOverlayEnabled: false } });
  let enumerations = 0;
  const node = new f.Element("div");
  node.matches = () => false;
  node.querySelectorAll = () => { enumerations += 1; return []; };
  f.observers[0].fn([{ type: "childList", addedNodes: [node] }]);
  assert.equal(enumerations, 0);
  assert.deepEqual(f.intervals.map(timer => timer.ms), [7000, 1000, 1000]);
});
