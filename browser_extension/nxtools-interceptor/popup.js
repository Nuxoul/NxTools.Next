"use strict";

// Popup entry. Modules (loaded in order by popup.html):
//   popup/format.js  pure formatting/classification
//   popup/dom.js     state, element references, safe DOM builders
//   popup/render.js  page rendering
//   popup/actions.js messaging, busy guards, feedback, event wiring
// Opening the popup reads a passive snapshot only; it never starts the desktop app.

bindPopupEvents();
setView(currentView);
refresh().catch((error) => {
  setMessage(error instanceof Error ? error.message : "扩展初始化失败");
});
startRefreshLoop();
