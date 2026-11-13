"use strict";

// Popup shared state, element references and safe DOM builders. All rendering
// uses createElement/textContent; no HTML string is ever parsed (extension CSP).

let currentState = null;
let currentView = "tasks";
let currentResourceFilter = "all";
let selectedResourceIds = new Set();
let previewResourceId = "";
let currentResourceSearch = "";
let viewSelectionRevision = 0;
let lastHandledLinkReviewId = "";
let localLinkReviewStatus = null;
const inspectingResourceIds = new Set();
let selectedMediaIndex = 0;
let refreshTimer = null;
let refreshInFlight = null;
let busyActionCount = 0;
let mediaActionInFlight = false;
let renderRevision = 0;
const captureRuleKeys = ["captureFileTypes", "captureMinimumKiB", "captureExcludedSites", "captureExcludedAddresses"];
const captureRuleDrafts = new Set();

const elements = {
  interceptDownloads: document.getElementById("interceptDownloads"),
  captureFileTypes: document.getElementById("captureFileTypes"),
  captureMinimumKiB: document.getElementById("captureMinimumKiB"),
  captureExcludedSites: document.getElementById("captureExcludedSites"),
  captureExcludedAddresses: document.getElementById("captureExcludedAddresses"),
  capturePauseButton: document.getElementById("capturePauseButton"),
  homeCapturePauseButton: document.getElementById("homeCapturePauseButton"),
  homeCaptureStatus: document.getElementById("homeCaptureStatus"),
  homeCaptureHint: document.getElementById("homeCaptureHint"),
  capturePauseHint: document.getElementById("capturePauseHint"),
  openDialog: document.getElementById("openDialog"),
  captureResources: document.getElementById("captureResources"),
  mediaOverlayEnabled: document.getElementById("mediaOverlayEnabled"),
  dropTargetEnabled: document.getElementById("dropTargetEnabled"),
  dropTargetPosition: document.getElementById("dropTargetPosition"),
  dropTargetWidth: document.getElementById("dropTargetWidth"),
  dropTargetHeight: document.getElementById("dropTargetHeight"),
  eraseBrowserHistory: document.getElementById("eraseBrowserHistory"),
  includeRequestHeaders: document.getElementById("includeRequestHeaders"),
  includeCookies: document.getElementById("includeCookies"),
  saveButton: document.getElementById("saveButton"),
  testButton: document.getElementById("testButton"),
  sendTabButton: document.getElementById("sendTabButton"),
  homeTestButton: document.getElementById("homeTestButton"),
  homeConnectionPanel: document.getElementById("homeConnectionPanel"),
  homeConnectionMessage: document.getElementById("homeConnectionMessage"),
  homeConnectionGuidance: document.getElementById("homeConnectionGuidance"),
  extractLinksButton: document.getElementById("extractLinksButton"),
  resourceSearchInput: document.getElementById("resourceSearchInput"),
  linkReviewStatus: document.getElementById("linkReviewStatus"),
  clearResourcesButton: document.getElementById("clearResourcesButton"),
  clearSelectionButton: document.getElementById("clearSelectionButton"),
  batchResourcesButton: document.getElementById("batchResourcesButton"),
  statusText: document.getElementById("statusText"),
  statusDot: document.getElementById("statusDot"),
  signalRail: document.getElementById("signalRail"),
  captureSummaryText: document.getElementById("captureSummaryText"),
  captureAgeText: document.getElementById("captureAgeText"),
  taskTabCount: document.getElementById("taskTabCount"),
  resourceTabCount: document.getElementById("resourceTabCount"),
  activeTabTitle: document.getElementById("activeTabTitle"),
  activeTabDomain: document.getElementById("activeTabDomain"),
  siteEnabledToggle: document.getElementById("siteEnabledToggle"),
  siteToggleHint: document.getElementById("siteToggleHint"),
  siteToggleRow: document.getElementById("siteToggleRow"),
  handoffStatus: document.getElementById("handoffStatus"),
  taskCountText: document.getElementById("taskCountText"),
  taskList: document.getElementById("taskList"),
  currentResourceList: document.getElementById("currentResourceList"),
  currentResourceCountText: document.getElementById("currentResourceCountText"),
  otherResourceList: document.getElementById("otherResourceList"),
  mediaStateText: document.getElementById("mediaStateText"),
  mediaSelect: document.getElementById("mediaSelect"),
  mediaPlayButton: document.getElementById("mediaPlayButton"),
  mediaPipButton: document.getElementById("mediaPipButton"),
  mediaScreenshotButton: document.getElementById("mediaScreenshotButton"),
  otherResourcesDetails: document.getElementById("otherResourcesDetails"),
  otherResourceCountText: document.getElementById("otherResourceCountText"),
  mediaTimeRange: document.getElementById("mediaTimeRange"),
  mediaVolumeRange: document.getElementById("mediaVolumeRange"),
  mediaMuteButton: document.getElementById("mediaMuteButton"),
  mediaLoopButton: document.getElementById("mediaLoopButton"),
  mediaSpeedInput: document.getElementById("mediaSpeedInput"),
  mediaSpeedButton: document.getElementById("mediaSpeedButton"),
  featureOverlayButton: document.getElementById("featureOverlayButton"),
  featureDeepScanButton: document.getElementById("featureDeepScanButton"),
  featureMobileUaButton: document.getElementById("featureMobileUaButton"),
  messageText: document.getElementById("messageText"),
  dismissMessageButton: document.getElementById("dismissMessageButton")
};
const batchToolbar = document.querySelector(".batch-toolbar");
const statusFooter = document.querySelector(".status-footer");

// el("button", { className: "x", dataset: { a: "1" }, attrs: { "aria-label": "y" }, text: "z" }, ...children)
function el(tag, options = {}, ...children) {
  const node = document.createElement(tag);
  if (options.className) node.className = options.className;
  if (options.text !== undefined) node.textContent = String(options.text);
  if (options.title) node.title = String(options.title);
  if (options.type) node.type = options.type;
  for (const [key, value] of Object.entries(options.dataset || {})) node.dataset[key] = String(value);
  for (const [key, value] of Object.entries(options.attrs || {})) node.setAttribute(key, String(value));
  for (const child of children) if (child) node.append(child);
  return node;
}

const SVG_NS = "http://www.w3.org/2000/svg";
// Outline glyphs (24px grid, 1.6 stroke) for the resource type marker.
const KIND_ICON_PATHS = {
  video: ["M4 6.5h11a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H4A1.5 1.5 0 0 1 2.5 16V8A1.5 1.5 0 0 1 4 6.5z", "M16.5 10.5l5-3v9l-5-3"],
  audio: ["M9 17.5V6l11-2v11.5", "M9 17.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z", "M20 15.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z"],
  image: ["M4.5 4.5h15a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-15a1 1 0 0 1-1-1v-13a1 1 0 0 1 1-1z", "M3.5 16l5-5 4 4 3-3 5 5", "M15.5 8.5h.01"],
  streaming: ["M4 7h12", "M4 12h16", "M4 17h9", "M18 15l3 2-3 2z"],
  download: ["M6.5 3.5h7l4 4v13h-11z", "M13.5 3.5v4h4", "M12 11v6", "M9.5 14.5L12 17l2.5-2.5"]
};

function kindIcon(kind) {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  for (const d of KIND_ICON_PATHS[kind] || KIND_ICON_PATHS.download) {
    const path = document.createElementNS(SVG_NS, "path");
    path.setAttribute("d", d);
    svg.append(path);
  }
  return svg;
}

function emptyState(text, hint) {
  return el("div", { className: "empty" }, el("strong", { text }), hint ? el("small", { text: hint }) : null);
}

function allResourcesFromState(state = currentState) {
  return [
    ...(Array.isArray(state?.currentResources) ? state.currentResources : []),
    ...(Array.isArray(state?.otherResources) ? state.otherResources : [])
  ];
}

function findResourceById(resourceId, state = currentState) {
  return allResourcesFromState(state).find((resource) => resource.id === resourceId) || null;
}

function resourceSearchText(resource) {
  return [resource?.filename, resource?.domain, resource?.url].filter(Boolean).join(" ").toLocaleLowerCase();
}

function filterResourceList(resources) {
  const query = currentResourceSearch.trim().toLocaleLowerCase();
  return resources.filter((resource) => {
    const matchesKind = currentResourceFilter === "all" || resourceKind(resource) === currentResourceFilter;
    return matchesKind && (!query || resourceSearchText(resource).includes(query));
  });
}

function capturePauseButtons() {
  return [elements.capturePauseButton, elements.homeCapturePauseButton].filter(Boolean);
}

function connectionTestButtons() {
  return [elements.testButton, elements.homeTestButton].filter(Boolean);
}

function collectSettings() {
  const minimumText = elements.captureMinimumKiB.value.trim();
  const minimumKiB = Number(minimumText);
  if (!/^\d+$/.test(minimumText) || !Number.isSafeInteger(minimumKiB) || minimumKiB > Math.floor(Number.MAX_SAFE_INTEGER / 1024)) {
    throw new Error("最小文件大小必须是非负整数 KiB；0 表示不限制，请更正后重试");
  }
  return {
    interceptDownloads: elements.interceptDownloads.checked,
    captureFileTypes: elements.captureFileTypes.value,
    captureMinimumKiB: minimumKiB,
    captureExcludedSites: elements.captureExcludedSites.value,
    captureExcludedAddresses: elements.captureExcludedAddresses.value,
    openDialog: elements.openDialog.checked,
    captureResources: elements.captureResources.checked,
    mediaOverlayEnabled: elements.mediaOverlayEnabled.checked,
    dropTargetEnabled: elements.dropTargetEnabled.checked,
    dropTargetPosition: elements.dropTargetPosition.value,
    dropTargetWidth: Math.max(96, Math.min(360, Number(elements.dropTargetWidth.value || 188))),
    dropTargetHeight: Math.max(48, Math.min(220, Number(elements.dropTargetHeight.value || 86))),
    eraseBrowserHistory: elements.eraseBrowserHistory.checked,
    includeRequestHeaders: elements.includeRequestHeaders.checked,
    includeCookies: elements.includeCookies.checked
  };
}
