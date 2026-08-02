const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const extensionRoot = path.resolve(__dirname, "..");

function chromiumId(publicKeyBase64) {
  const digest = crypto.createHash("sha256").update(Buffer.from(publicKeyBase64, "base64")).digest();
  const alphabet = "abcdefghijklmnop";
  return [...digest.subarray(0, 16)]
    .map((byte) => alphabet[byte >> 4] + alphabet[byte & 15])
    .join("");
}

test("migrated extension preserves its signed identity and uses Native Messaging only", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(extensionRoot, "manifest.json"), "utf8"));
  assert.equal(chromiumId(manifest.key), "ajcjlgdnfcagknjcajhbiikagimgdnaf");
  assert.equal(
    manifest.browser_specific_settings.gecko.id,
    "nxtools-download-interceptor@nxtools.local"
  );
  assert.ok(manifest.permissions.includes("nativeMessaging"));
  assert.ok(!manifest.host_permissions.some((value) => value.includes("localhost") || value.includes("127.0.0.1")));
  assert.ok(!manifest.content_security_policy.extension_pages.includes("ws://"));
});

test("production extension source loads the tested handoff and transport boundaries", () => {
  const background = fs.readFileSync(path.join(extensionRoot, "background.js"), "utf8");
  const hostCargo = fs.readFileSync(
    path.resolve(extensionRoot, "..", "..", "rust", "apps", "nxtools-cli", "Cargo.toml"),
    "utf8"
  );
  assert.match(background, /importScripts\("native-transport\.js", "handoff-policy\.js"\)/);
  assert.match(background, /NxToolsHandoffPolicy\.handoffBrowserDownload/);
  assert.match(background, /id: "nxtools-download-link"/);
  assert.match(background, /title: "使用 NxTools 解析下载"/);
  assert.match(background, /chrome\.contextMenus\.onClicked\.addListener/);
  assert.match(background, /void sendUrl\(url,/);
  assert.doesNotMatch(background, /chrome\.downloads\.download\(\{ url: downloadItem/);
  assert.doesNotMatch(background, /new WebSocket\(/);
  assert.doesNotMatch(hostCargo, /rusqlite/i);
});

test("ordinary Manifest V3 installs never register blocking webRequest listeners", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(extensionRoot, "manifest.json"), "utf8"));
  const background = fs.readFileSync(path.join(extensionRoot, "background.js"), "utf8");

  assert.equal(manifest.manifest_version, 3);
  assert.ok(!manifest.permissions.includes("webRequestBlocking"));
  assert.doesNotMatch(
    background,
    /\[[^\]]*["']blocking["'][^\]]*\]/s,
    "MV3 extensions installed by users cannot register blocking webRequest listeners"
  );
});

test("extension popup does not rely on CSP-blocked inline styles", () => {
  const popupHtml = fs.readFileSync(path.join(extensionRoot, "popup.html"), "utf8");
  const popupScript = fs.readFileSync(path.join(extensionRoot, "popup.js"), "utf8");

  assert.doesNotMatch(popupHtml, /\sstyle\s*=/i);
  assert.doesNotMatch(popupScript, /\sstyle\s*=/i);
});
