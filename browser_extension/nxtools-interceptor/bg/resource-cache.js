"use strict";

// Per-tab resource cache indexes, capacity/TTL and persistence projection.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

function bucketForTab(tabId) {
  const id = Number(tabId) || lastActiveTabId || 0;
  if (!resourcesByTab.has(id)) {
    resourcesByTab.set(id, new Map());
  }
  return resourcesByTab.get(id);
}

// Both maps are lookup indexes; aliases do not own another resource.
function removeCachedResource(tabId, resource) {
  const bucket = resourcesByTab.get(tabId);
  if (!bucket) {
    return;
  }
  for (const [key, cached] of bucket) {
    if (cached !== resource) {
      continue;
    }
    bucket.delete(key);
    if (resourcesById.get(key) === resource) {
      resourcesById.delete(key);
    }
  }
  if (bucket.size === 0) {
    resourcesByTab.delete(tabId);
  }
}

function indexCachedResource(resource) {
  const previous = resourcesByTab.get(resource.tabId)?.get(resource.id);
  if (previous && previous !== resource && previous.id === resource.id) {
    removeCachedResource(resource.tabId, previous);
  }
  const bucket = bucketForTab(resource.tabId);
  bucket.set(resource.id, resource);
  resourcesById.set(resource.id, resource);
  const normalizedId = `${resource.tabId}:${normalizeResourceUrlKey(resource.url)}`;
  const sameUrl = bucket.get(normalizedId);
  if (!sameUrl || (sameUrl.id !== normalizedId &&
      Number(resource.capturedAt || 0) >= Number(sameUrl.capturedAt || 0))) {
    bucket.set(normalizedId, resource);
    resourcesById.set(normalizedId, resource);
  }
}

function enforceResourceCapacity(tabId) {
  const bucket = resourcesByTab.get(tabId);
  const sorted = [...new Set(bucket?.values() || [])]
    .sort((left, right) => right.capturedAt - left.capturedAt);
  for (const stale of sorted.slice(RESOURCE_LIMIT_PER_TAB)) {
    removeCachedResource(tabId, stale);
  }
}

function cacheResource(resource) {
  const url = boundedCapturedUrl(resource?.url);
  if (!resource || !resource.id || !url) {
    return;
  }
  if (!isUsableGoogleVideoUrl(url)) {
    return;
  }

  resource = {
    ...resource,
    id: boundedCapturedText(resource.id, MAX_CAPTURED_URL_LENGTH + 32),
    url,
    blobUrl: boundedCapturedUrl(resource.blobUrl),
    pageKey: boundedCapturedText(resource.pageKey),
    pageTitle: boundedCapturedText(resource.pageTitle),
    pageUrl: boundedCapturedUrl(resource.pageUrl),
    filename: boundedCapturedText(resource.filename),
    filenameSource: boundedCapturedText(resource.filenameSource, 64),
    metadataSource: boundedCapturedText(resource.metadataSource, 64),
    headerSource: boundedCapturedText(resource.headerSource || resource.requestContextSource, 64),
    referrerSource: boundedCapturedText(resource.referrerSource || resource.requestContextSource, 64),
    mime: boundedCapturedText(resource.mime, 256).toLowerCase(),
    referer: boundedCapturedUrl(resource.referer),
    source: boundedCapturedText(resource.source, 128),
    size: boundedCapturedNumber(resource.size),
    width: boundedCapturedNumber(resource.width),
    height: boundedCapturedNumber(resource.height),
    requestHeaders: sanitizeCapturedHeaderObject(resource.requestHeaders),
    pendingBatchHandoffId: NxToolsBatchHandoffPolicy.normalizeHandoffId(resource.pendingBatchHandoffId),
    capturedAt: boundedCapturedNumber(resource.capturedAt) || Date.now()
  };
  if (!resource.id) {
    return;
  }

  const bucket = bucketForTab(resource.tabId);
  const indexed = bucket.get(resource.id);
  const existing = indexed?.id === resource.id ? indexed : null;
  const wasNew = !existing;
  const previousMime = existing?.mime;
  const previousSize = Number(existing?.size || 0);
  const merged = NxToolsCapturePolicy.mergeCapturedResource(existing, resource);
  const cachedResource = existing || merged;
  if (existing) {
    removeCachedResource(existing.tabId, existing);
    Object.assign(existing, merged);
  }
  indexCachedResource(cachedResource);

  if (wasNew ||
      (resource.mime && resource.mime !== previousMime) ||
      (Number(resource.size || 0) > previousSize)) {
    logResourceCaptured("resource_cached", merged, {
      source: resource.source || "cache"
    });
  }

  const mergedUrl = normalizeResourceUrlKey(merged.url);
  if (mergedUrl && (merged.size > 0 || merged.mime || merged.supportsRange)) {
    for (const cached of new Set(resourcesById.values())) {
      if (cached === cachedResource || normalizeResourceUrlKey(cached.url) !== mergedUrl) {
        continue;
      }
      const metadata = NxToolsCapturePolicy.mergeCapturedResource(cached, merged);
      // Sharing metadata must not move another capture's identity or handoff.
      for (const field of ["id", "tabId", "url", "capturedAt", "pendingBatchHandoffId", "sentToDesktopAt"]) {
        delete metadata[field];
      }
      Object.assign(cached, metadata);
    }
  }

  enforceResourceCapacity(cachedResource.tabId);
  schedulePersistResources();
}

function pruneResources() {
  const now = Date.now();
  for (const [tabId, bucket] of resourcesByTab) {
    for (const resource of new Set(bucket.values())) {
      if (now - resource.capturedAt > RESOURCE_TTL_MS || !isUsableGoogleVideoUrl(resource.url)) {
        removeCachedResource(tabId, resource);
      }
    }
    if (bucket.size === 0) {
      resourcesByTab.delete(tabId);
    }
  }
}

function serializeResources() {
  pruneResources();
  const result = {};
  for (const [tabId, bucket] of resourcesByTab) {
    result[String(tabId)] = NxToolsCapturePolicy.uniqueCapturedResources([...bucket.values()])
      .sort((left, right) => right.capturedAt - left.capturedAt)
      .slice(0, RESOURCE_LIMIT_PER_TAB)
      .map((resource) => NxToolsCapturePolicy.projectResourceForStorage(resource));
  }
  return result;
}

function schedulePersistResources() {
  if (persistTimer) {
    return;
  }

  persistTimer = setTimeout(() => {
    persistTimer = 0;
    chrome.storage.local.set({
      [STORAGE_RESOURCE_KEY]: serializeResources(),
      [STORAGE_LAST_TAB_KEY]: lastActiveTabId
    });
  }, 600);
}

async function persistResourcesNow() {
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = 0;
  }
  await chrome.storage.local.set({
    [STORAGE_RESOURCE_KEY]: serializeResources(),
    [STORAGE_LAST_TAB_KEY]: lastActiveTabId
  });
}

async function loadPersistentResources() {
  const stored = await chrome.storage.local.get({
    [STORAGE_RESOURCE_KEY]: {},
    [STORAGE_LAST_TAB_KEY]: 0,
    [STORAGE_MOBILE_UA_TABS_KEY]: [],
    [STORAGE_DISABLED_SITES_KEY]: []
  });

  lastActiveTabId = Number(stored[STORAGE_LAST_TAB_KEY] || 0);
  loadDisabledSites(stored[STORAGE_DISABLED_SITES_KEY]);
  mobileUserAgentTabs.clear();
  for (const tabId of stored[STORAGE_MOBILE_UA_TABS_KEY] || []) {
    if (Number.isInteger(tabId) && tabId > 0) {
      mobileUserAgentTabs.add(tabId);
      void updateMobileUserAgentRule(tabId, true);
    }
  }

  resourcesByTab.clear();
  resourcesById.clear();
  for (const [tabIdText, resources] of Object.entries(stored[STORAGE_RESOURCE_KEY] || {})) {
    const tabId = Number(tabIdText);
    if (!Number.isInteger(tabId) || tabId <= 0 || !Array.isArray(resources)) {
      continue;
    }
    for (const resource of resources) {
      if (!resource?.id || !resource?.url || !isUsableGoogleVideoUrl(resource.url)) {
        continue;
      }
      indexCachedResource({ ...resource, tabId });
    }
    enforceResourceCapacity(tabId);
  }
  pruneResources();
  await chrome.storage.local.set({
    [STORAGE_RESOURCE_KEY]: serializeResources(),
    [STORAGE_LAST_TAB_KEY]: lastActiveTabId
  });
}

function clearTabResources(tabId) {
  const bucket = resourcesByTab.get(tabId);
  if (bucket) {
    for (const resource of new Set(bucket.values())) {
      removeCachedResource(tabId, resource);
    }
    schedulePersistResources();
  }
}

function clearTab(tabId) {
  clearTabResources(tabId);
  if (mobileUserAgentTabs.delete(tabId)) {
    void updateMobileUserAgentRule(tabId, false);
    void persistFeatureState();
  }
}
