# Extension UI: target, observed states and gaps

Evidence: `repo:browser_extension/nxtools-interceptor/popup.html`, `repo:browser_extension/nxtools-interceptor/popup.js#setView`, `repo:browser_extension/nxtools-interceptor/popup.css`, `repo:browser_extension/nxtools-interceptor/shadow-styles.js`, `repo:browser_extension/nxtools-interceptor/tests/popup-contract.test.js`.

Required target uses [shared semantic UI/state contract](../shared/ui-design.md) while respecting native browser inputs, CSP and popup/content lifetimes. Popup tab/panel ARIA-selected/hidden and roving tabIndex plus Arrow/Home/End focus are observed code. beginBusy/refreshInFlight prevent duplicate busy updates, failures stay observable. Real browser rendering/keyboard/assistive tech isn't proven by source tests.

Popup CSS has prefers-reduced-motion override1014–1023; do not generalize it to shadow overlay. Resource filter group toggles .active in popup.js826–835 but does not synchronize selected ARIA like tabs; observed gap, not fixed here. shadow-styles choice focus exists; equivalent reduced-motion rule not found in collected source. Content overlay has keyboard/Escape paths, but all controls/provider names still need actual browser matrix.

Acceptance for changed surfaces: default/hover/pressed/selected/focus/disabled/loading/empty/error/success, semantic HTML labels/role, keyboard equivalence, no layout shift, CSS contrast and reduced-motion at popup+shadow independently. No infinite decorative animation. Header/media context never enters UI storage. Update source contract tests and add real DOM interaction evidence when authorized; don't mark static ARIA string presence as accessibility certification. [Gap ledger](../shared/foundation-backlog.md).
