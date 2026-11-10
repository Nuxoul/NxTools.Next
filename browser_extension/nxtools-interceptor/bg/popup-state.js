"use strict";

// Connection status and the secret-free popup state projection.
// Classic script sharing the background global scope; loaded by background.js
// (Chrome importScripts) and manifest background.scripts (Firefox).

async function checkApiStatus(explicitRetry = false) {
  if (popupPortOwners > 0 && !explicitRetry && !desktopPort && !desktopConnectPromise) {
    return { ok: false, transport: "native_messaging", message: "NxTools 未运行", data: lastSnapshot };
  }
  try {
    const response = await sendDesktopRequest("get_snapshot", {}, 2500);
    lastSnapshot = response.data || lastSnapshot;
    return {
      ok: true,
      message: "已连接",
      transport: "native_messaging",
      data: response.data || lastSnapshot
    };
  } catch (error) {
    return {
      ok: false,
      transport: "native_messaging",
      message: "NxTools 未运行",
      data: lastSnapshot
    };
  }
}

function resourcesForPopup(activeTabId) {
  pruneResources();
  const current = activeTabId
    ? NxToolsCapturePolicy.uniqueCapturedResources([...(resourcesByTab.get(activeTabId)?.values() || [])].filter(resource => !NxToolsCapturePolicy.discoveryCandidateReason(resource)))
    : [];
  const other = [];
  for (const [tabId, bucket] of resourcesByTab) {
    if (tabId === activeTabId) {
      continue;
    }
    other.push(...bucket.values());
  }

  const enrich = (resource) => ({
    ...NxToolsCapturePolicy.projectResourceForPopup(resource),
    domain: domainFromUrl(resource.url),
    description: describeResource(resource)
  });

  const bySizeThenTime = (left, right) => {
    const leftSize = Number(left.size || left.sourceSize || 0);
    const rightSize = Number(right.size || right.sourceSize || 0);
    if (leftSize !== rightSize) {
      return rightSize - leftSize;
    }
    return Number(right.capturedAt || 0) - Number(left.capturedAt || 0);
  };

  const uniqueOther = NxToolsCapturePolicy.uniqueCapturedResources(other.filter(resource => !NxToolsCapturePolicy.discoveryCandidateReason(resource)));
  const currentResources = current.sort(bySizeThenTime).map(enrich);
  const otherResources = uniqueOther.sort(bySizeThenTime).map(enrich);
  const allResources = [...currentResources, ...otherResources];
  const latestCapturedAt = allResources.reduce(
    (latest, resource) => Math.max(latest, Number(resource.capturedAt || 0)),
    0);
  return {
    currentResources,
    otherResources,
    captureSummary: {
      currentCount: currentResources.length,
      otherCount: otherResources.length,
      totalCount: allResources.length,
      recentCount: allResources.filter((resource) => Date.now() - Number(resource.capturedAt || 0) <= 60_000).length,
      latestCapturedAt
    }
  };
}

async function buildPopupState(preferredMediaIndex = null, currentView = "tasks") {
  const settings = await getSettings();
  const status = await checkApiStatus();
  const tab = await activeTab();
  const tabId = tab?.id || lastActiveTabId || 0;
  const shouldScanPage = currentView === "resources" || currentView === "advanced";
  const shouldReadMedia = currentView === "advanced";
  if (settings.captureResources !== false && tabId > 0 && shouldScanPage) {
    await scanTabForResources(tabId);
  }

  const resources = resourcesForPopup(tabId);
  const savedIndex = selectedMediaIndexByTab.get(tabId) ?? 0;
  const requestedIndex = Number.isInteger(preferredMediaIndex) ? preferredMediaIndex : savedIndex;
  const mediaState = shouldReadMedia
    ? await getMediaState(tabId, requestedIndex)
    : {
        mediaItems: [],
        mediaPlaybackState: {
          available: false,
          message: "切换到高级页后检测页面媒体",
          tabId: tabId || null,
          frameId: 0,
          mediaIndex: -1,
          currentTime: 0,
          duration: 0,
          progress: 0,
          volume: 1,
          paused: true,
          loop: false,
          muted: false,
          speed: 1
        }
      };
  const nextIndex = mediaState?.mediaPlaybackState?.mediaIndex;
  if (Number.isInteger(nextIndex) && nextIndex >= 0) {
    selectedMediaIndexByTab.set(tabId, nextIndex);
  }

  if (mediaState?.mediaPlaybackState?.available === false &&
      resources.currentResources.some(canSendOneClickResource)) {
    sendExtensionLog("warn", "page_media_dom_empty_resources_present", "页面媒体控件未检测到 DOM 媒体，但当前页已有可下载资源", {
      tabId,
      pageUrl: tab?.url || "",
      resourceCount: resources.currentResources.length,
      sampleUrls: resourceSamples(resources.currentResources, 5)
    }, 30000);
  }

  const activeHost = siteHostKey(tab?.url || "");
  return {
    settings,
    capturePaused: await getCapturePaused(),
    capturePauseAvailable,
    status,
    tasks: Array.isArray(status.data?.tasks) ? status.data.tasks : [],
    lastHandoff: lastHandoff ? { ...lastHandoff } : null,
    featureStates: featureStatesForTab(tabId, settings),
    siteControl: {
      host: activeHost,
      supported: isHttpUrl(tab?.url || ""),
      enabled: activeHost ? !disabledSites.has(activeHost) : true
    },
    ...mediaState,
    activeTab: tab
      ? {
          id: tab.id,
          title: tab.title || "",
          url: tab.url || "",
          domain: domainFromUrl(tab.url || ""),
          thumbnail: pageThumbnailByTab.get(tab.id) || ""
        }
      : null,
    ...resources
  };
}
