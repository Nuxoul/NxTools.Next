"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const policy = require("../capture-policy.js");

function test(name, callback) {
  try {
    callback();
    console.log(`ok - ${name}`);
  } catch (error) {
    console.error(`not ok - ${name}`);
    throw error;
  }
}

test("response metadata upgrades a request-stage resource", () => {
  const merged = policy.mergeCapturedResource({
    id: "9:https://cdn.example.test/file",
    url: "https://cdn.example.test/file",
    filename: "file",
    filenameSource: "url",
    metadataSource: "request",
    requestContextSource: "browser",
    mime: "",
    size: 0,
    pageTitle: "Release page",
    pageUrl: "https://example.test/releases",
    referer: "https://example.test/releases",
    requestHeaders: { referer: "https://example.test/releases" },
    source: "webRequest-request",
    capturedAt: 100
  }, {
    id: "9:https://cdn.example.test/file",
    url: "https://cdn.example.test/file",
    filename: "nxtools-0.6.0.zip",
    filenameSource: "content-disposition",
    metadataSource: "response",
    requestContextSource: "browser",
    mime: "application/zip",
    size: 1048576,
    pageTitle: "",
    pageUrl: "",
    referer: "",
    requestHeaders: { range: "bytes=0-" },
    source: "webRequest-response",
    capturedAt: 200
  });

  assert.equal(merged.filename, "nxtools-0.6.0.zip");
  assert.equal(merged.filenameSource, "content-disposition");
  assert.equal(merged.mime, "application/zip");
  assert.equal(merged.size, 1048576);
  assert.equal(merged.pageTitle, "Release page");
  assert.equal(merged.pageUrl, "https://example.test/releases");
  assert.equal(merged.referer, "https://example.test/releases");
  assert.deepEqual(merged.requestHeaders, {
    referer: "https://example.test/releases",
    range: "bytes=0-"
  });
});

test("later weak page observations cannot erase authoritative metadata", () => {
  const merged = policy.mergeCapturedResource({
    filename: "archive.zip",
    filenameSource: "content-disposition",
    metadataSource: "response",
    requestContextSource: "browser",
    mime: "application/zip",
    size: 8192,
    pageTitle: "Builds",
    pageUrl: "https://example.test/builds",
    referer: "https://example.test/builds",
    requestHeaders: { referer: "https://example.test/builds", authorization: "browser value" },
    source: "webRequest-response",
    capturedAt: 100
  }, {
    filename: "resource.bin",
    filenameSource: "page",
    metadataSource: "page",
    requestContextSource: "page",
    mime: "video/mp4",
    size: 999999,
    pageTitle: "",
    pageUrl: "",
    referer: "https://untrusted.example.test/",
    requestHeaders: { referer: "https://untrusted.example.test/", authorization: "page value" },
    source: "performance:fetch",
    capturedAt: 300
  });

  assert.equal(merged.filename, "archive.zip");
  assert.equal(merged.mime, "application/zip");
  assert.equal(merged.size, 8192);
  assert.equal(merged.pageTitle, "Builds");
  assert.equal(merged.referer, "https://example.test/builds");
  assert.equal(merged.requestHeaders.authorization, "browser value");
  assert.equal(merged.requestHeaders.referer, "https://example.test/builds");
  assert.equal(merged.capturedAt, 300);
});

test("popup projection omits privileged request context", () => {
  const projected = policy.projectResourceForPopup({
    id: "resource-1",
    url: "https://cdn.example.test/archive.zip",
    filename: "archive.zip",
    requestHeaders: {
      authorization: "Bearer secret",
      cookie: "session=secret"
    },
    referer: "https://private.example.test/account",
    pageUrl: "https://private.example.test/account",
    supportsRange: true,
    capturedAt: 100
  });

  assert.equal(projected.id, "resource-1");
  assert.equal(projected.url, "https://cdn.example.test/archive.zip");
  assert.equal(projected.hasRequestContext, true);
  assert.equal("requestHeaders" in projected, false);
  assert.equal("referer" in projected, false);
  assert.equal("pageUrl" in projected, false);
});

test("storage projection never persists page or request context", () => {
  const projected = policy.projectResourceForStorage({
    id: "resource-1",
    url: "https://cdn.example.test/archive.zip",
    pageTitle: "Private account",
    pageUrl: "https://private.example.test/account",
    referer: "https://private.example.test/account",
    requestHeaders: { cookie: "session=secret" },
    filename: "archive.zip"
  });

  assert.equal(projected.filename, "archive.zip");
  for (const field of ["pageTitle", "pageUrl", "referer", "requestHeaders"]) {
    assert.equal(field in projected, false, `${field} must remain worker-only`);
  }
});

test("popup resources are deduplicated by stable id", () => {
  const shared = { id: "stable", url: "https://cdn.example.test/file", capturedAt: 100 };
  const resources = policy.uniqueCapturedResources([
    shared,
    { ...shared, capturedAt: 200 },
    { id: "other", url: "https://cdn.example.test/other", capturedAt: 150 }
  ]);

  assert.deepEqual(resources.map((resource) => resource.id), ["stable", "other"]);
  assert.equal(resources[0].capturedAt, 200);
});

test("worker capture never trusts page-supplied request headers or persists context", () => {
  const background = fs.readFileSync(path.resolve(__dirname, "..", "background.js"), "utf8");
  assert.doesNotMatch(background, /payload\.requestHeaders/);
  assert.match(background, /projectResourceForStorage\(resource\)/);
  assert.match(background, /mergeCapturedResource\(cached, merged\)/);
});

console.log("capture policy tests passed");
