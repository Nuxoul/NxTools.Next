"use strict";

// Popup entry. Modules (loaded in order by popup.html):
//   popup/format.js  pure formatting/classification
//   popup/dom.js     state, element references, safe DOM builders
//   popup/render.js  page rendering
//   popup/actions.js messaging, busy guards, feedback, event wiring
// Opening the popup reads a passive snapshot only; it never starts the desktop app.

const popupTransportPort = chrome.runtime.connect({ name: "nxtools-popup" });
window.addEventListener("pagehide", () => popupTransportPort.disconnect(), { once: true });

bindPopupEvents();
setView(currentView);
const initialRefresh = refresh().catch((error) => {
  setMessage(error instanceof Error ? error.message : "扩展初始化失败");
  return currentState;
});
restoreViewPreference().then(async (restored) => {
  if (!restored) return;
  await initialRefresh;
  await refresh({ force: true }).catch(() => {});
});
startRefreshLoop();
