"use strict";

// Mutable worker state shared by background modules (caches, transport, settings).
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

const recentRequestsByUrl = new Map();
const recentRequestsByRequestId = new Map();
const redirectHintsByRequestId = new Map();
const redirectHintsByUrl = new Map();
// Authoritative Content-Disposition filenames remembered per normalized URL, plus
// a redirect target→source map so a disposition seen on the final hop (e.g. the
// codeload.github.com 200) can be applied back to the URL the user actually clicked.
const contentDispositionByUrl = new Map();
const redirectSourceByTarget = new Map();
const resourcesByTab = new Map();
const resourcesById = new Map();
let lastActiveTabId = 0;
let persistTimer = 0;
let desktopPort = null;
let desktopConnectPromise = null;
let desktopConnectStartedAt = 0;
let desktopIdleTimer = 0;
let desktopPortOwners = 0;
const DESKTOP_IDLE_RELEASE_MS = 1000;
let connectionState = "disconnected";
let connectionMessage = "未连接 NxTools";
const DESKTOP_CONNECT_TIMEOUT_MS = 1800;
let lastSnapshot = null;
const pendingDesktopRequests = new Map();
const mobileUserAgentTabs = new Set();
const extensionLogTimes = new Map();
const selectedMediaIndexByTab = new Map();
const lastFrameScanByTab = new Map();
const disabledSites = new Set();
let cachedSettings = { ...DEFAULT_SETTINGS };
let settingsLoaded = false;
let settingsLoadPromise = null;
