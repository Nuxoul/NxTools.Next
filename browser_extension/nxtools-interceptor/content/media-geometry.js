"use strict";

// Viewport/media rectangle geometry and media element scoring (pure helpers).
// Isolated-world content module: registered once on the extension-private
// NxToolsContent namespace (re-injection is a no-op). Holds no shared mutable
// page state; content-script.js owns all overlay/drop/capture state.
(function installNxToolsContentGeometry(content) {
  if (content.geometry) {
    return;
  }

  const { OVERLAY_GAP, FLOATING_MEDIA_WIDTH, FLOATING_MEDIA_HEIGHT } = content.constants;
  const { mediaResolutionText } = content.media;

  function mediaHasPlayableSource(media) {
    return Boolean(media?.currentSrc || media?.src || media?.readyState > 0 || (!media?.paused && !media?.ended));
  }

  function viewportFallbackRect() {
    const top = Math.min(Math.max(72, OVERLAY_GAP), Math.max(OVERLAY_GAP, innerHeight - FLOATING_MEDIA_HEIGHT - OVERLAY_GAP));
    const left = Math.max(OVERLAY_GAP, innerWidth - FLOATING_MEDIA_WIDTH - OVERLAY_GAP);
    return {
      left,
      top,
      right: left + FLOATING_MEDIA_WIDTH,
      bottom: top + FLOATING_MEDIA_HEIGHT,
      width: FLOATING_MEDIA_WIDTH,
      height: FLOATING_MEDIA_HEIGHT,
      fallback: true
    };
  }

  function visibleElementRect(element, minWidth, minHeight) {
    const rect = element.getBoundingClientRect();
    const visibleWidth = Math.max(0, Math.min(rect.right, innerWidth) - Math.max(rect.left, 0));
    const visibleHeight = Math.max(0, Math.min(rect.bottom, innerHeight) - Math.max(rect.top, 0));

    if (visibleWidth < minWidth || visibleHeight < minHeight) {
      return null;
    }

    const style = getComputedStyle(element);
    if (style.display === "none" || style.visibility === "hidden" || Number(style.opacity || 1) <= 0.02) {
      return null;
    }

    return {
      left: rect.left,
      top: rect.top,
      right: rect.right,
      bottom: rect.bottom,
      width: rect.width,
      height: rect.height,
      visibleWidth,
      visibleHeight,
      visibleArea: visibleWidth * visibleHeight,
      fallback: false
    };
  }

  function visibleMediaRect(media) {
    const tag = media.tagName.toLowerCase();
    const rect = visibleElementRect(media, tag === "audio" ? 64 : 48, tag === "audio" ? 20 : 28);
    if (rect) {
      return rect;
    }

    if (tag === "audio" && mediaHasPlayableSource(media)) {
      return viewportFallbackRect();
    }

    if (tag === "video" && mediaHasPlayableSource(media) && !media.paused) {
      return viewportFallbackRect();
    }

    return null;
  }

  function hasMediaSurface(element) {
    if (element instanceof HTMLMediaElement || element instanceof HTMLCanvasElement) {
      return true;
    }

    if (element.matches?.("[data-bvid], [data-aid], [data-video-id]")) {
      return true;
    }

    if (element.querySelector?.("video,audio,canvas")) {
      return true;
    }

    return false;
  }

  function containerUrl(element) {
    if (!element) {
      return "";
    }

    if (element instanceof HTMLAnchorElement) {
      return element.href || "";
    }

    const anchor = element.closest?.("a[href]") || element.querySelector?.("a[href]");
    if (anchor instanceof HTMLAnchorElement) {
      return anchor.href || "";
    }

    return "";
  }

  function mediaContainerScore(element, rect) {
    const text = `${element.className || ""} ${element.id || ""} ${element.getAttribute?.("aria-label") || ""}`;
    const strong = /bili|video|player|media|播放|视频/i.test(text) ? 20_000_000 : 0;
    const direct = element.matches?.("[data-bvid], [data-aid], [data-video-id]") ? 50_000_000 : 0;
    const playing = [...element.querySelectorAll?.("video,audio") || []]
      .some((media) => media instanceof HTMLMediaElement && !media.paused && !media.ended)
      ? 1_000_000_000
      : 0;
    const viewportCenterY = innerHeight / 2;
    const elementCenterY = Math.max(0, Math.min(innerHeight, rect.top + rect.visibleHeight / 2));
    const centerPenalty = Math.abs(elementCenterY - viewportCenterY) * 1000;
    return playing + direct + strong + rect.visibleArea - centerPenalty;
  }

  function mediaScore(media, rect) {
    const playing = !media.paused && !media.ended ? 1_000_000_000 : 0;
    const visible = rect.fallback ? 0 : 10_000_000;
    const source = mediaHasPlayableSource(media) ? 100_000 : 0;
    return playing + visible + media.readyState * 1_000_000 + source + rect.width * rect.height;
  }

  function mediaElements() {
    return [...document.querySelectorAll("video, audio")];
  }

  function selectionResolutionText(selected) {
    if (selected?.media instanceof HTMLVideoElement) {
      return mediaResolutionText(selected.media);
    }

    const activeVideo = mediaElements()
      .filter((media) => media instanceof HTMLVideoElement && mediaHasPlayableSource(media))
      .sort((left, right) => mediaScore(right, visibleMediaRect(right) || viewportFallbackRect()) - mediaScore(left, visibleMediaRect(left) || viewportFallbackRect()))[0];
    return activeVideo instanceof HTMLVideoElement ? mediaResolutionText(activeVideo) : "";
  }

  function sameResourceUrl(left, right) {
    try {
      const leftUrl = new URL(left, location.href);
      const rightUrl = new URL(right, location.href);
      leftUrl.hash = "";
      rightUrl.hash = "";
      return leftUrl.toString() === rightUrl.toString();
    } catch {
      return String(left || "").split("#", 1)[0] === String(right || "").split("#", 1)[0];
    }
  }

  content.geometry = Object.freeze({
    mediaHasPlayableSource,
    viewportFallbackRect,
    visibleElementRect,
    visibleMediaRect,
    hasMediaSurface,
    containerUrl,
    mediaContainerScore,
    mediaScore,
    mediaElements,
    selectionResolutionText,
    sameResourceUrl
  });
})(globalThis.NxToolsContent || (globalThis.NxToolsContent = {}));
