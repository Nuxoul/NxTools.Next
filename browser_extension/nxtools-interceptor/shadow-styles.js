(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  root.NxToolsShadowStyles = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";
  // Same semantic vocabulary as popup-tokens.css, scoped inside injected roots.
  const tokens = `
    --nt-canvas: #18181b;
    --nt-surface: #1f1f23;
    --nt-surface-raised: #27272a;
    --nt-surface-hover: #303034;
    --nt-surface-pressed: #353539;
    --nt-border-subtle: #343438;
    --nt-border: #48484f;
    --nt-text: #f4f4f5;
    --nt-text-secondary: #a1a1aa;
    --nt-text-muted: #92929c;
    --nt-text-disabled: #71717a;
    --nt-accent: #bfc0c4;
    --nt-accent-soft: #353539;
    --nt-accent-text: #18181b;
    --nt-success: #7fbf9a;
    --nt-success-soft: #22302a;
    --nt-warning: #d6a85f;
    --nt-danger: #ff6b78;
    --nt-danger-soft: #35252a;`;
  const mediaEntry = `:host {
    ${tokens}
    font: 13px/20px "Segoe UI", "Microsoft YaHei UI", sans-serif;
    position: fixed; width: 28px; height: 28px; overflow: visible;
  }
  *, *::before, *::after { box-sizing: border-box; }
  .trigger {
    display: inline-flex; align-items: center; justify-content: center;
    position: relative; width: 28px; height: 28px; padding: 0;
    border: 1px solid var(--nt-border); border-radius: 6px;
    background: var(--nt-surface); color: var(--nt-text); cursor: pointer; font: inherit;
    box-shadow: 0 2px 8px rgba(0,0,0,.2);
  }
  .trigger:hover { background: var(--nt-surface-hover); border-color: var(--nt-accent); }
  .trigger[aria-expanded="true"] { border-color: var(--nt-accent); background: var(--nt-accent-soft); }
  .trigger:disabled { cursor: wait; opacity: .7; }
  .status { position: fixed; left: 8px; bottom: 8px; max-width: min(360px,calc(100vw - 16px)); padding: 6px 10px; background: var(--nt-surface); border: 1px solid var(--nt-border); border-radius: 6px; color: var(--nt-text-secondary); font-size: 12px; line-height: 18px; text-align: left; overflow-wrap: anywhere; pointer-events: none; }
  :focus-visible { outline: 2px solid var(--nt-accent); outline-offset: 2px; }
  .icon { display: inline-flex; align-items: center; justify-content: center; flex: none; width: 24px; height: 24px; }
  .icon svg { width: 16px; height: 16px; fill: currentColor; }
  .label { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  .status:empty { display: none; }
  .status.error { color: var(--nt-danger); border-color: var(--nt-danger); }
  .panel {
    position: absolute; right: 0; top: 34px; width: min(414px,calc(100vw - 16px));
    max-width: min(430px,calc(100vw - 16px)); max-height: min(386px,calc(100vh - 72px));
    padding: 6px; overflow: auto; display: grid; gap: 0;
    background: var(--nt-surface); border: 1px solid var(--nt-border); border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,.22);
  }
  .panel[hidden] { display: none; }
  .choice { display: grid; grid-template-columns: 28px minmax(0,1fr); align-items: center; gap: 10px; width: 100%; min-height: 52px; padding: 8px 10px; text-align: left; font: inherit; cursor: pointer; background: transparent; border: 0; border-bottom: 1px solid var(--nt-border-subtle); color: var(--nt-text); border-radius: 4px; }
  .choice-kind { display: inline-flex; align-items: center; justify-content: center; width: 28px; height: 28px; border: 1px solid var(--nt-border-subtle); border-radius: 6px; background: var(--nt-surface-raised); color: var(--nt-text-secondary); }
  .choice-kind svg { width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .choice-text { display: grid; gap: 2px; min-width: 0; }
  .choice:last-child { border-bottom-color: transparent; }
  .choice:hover { background: var(--nt-surface-hover); }
  .choice:active { background: var(--nt-surface-pressed); }
  .choice-file { display: block; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; font-size: 13px; line-height: 20px; font-weight: 600; }
  .choice-meta { display: block; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--nt-text-secondary); font-family: inherit; font-size: 12px; line-height: 18px; }
  @media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation: none !important; transition: none !important; } }`;
  const dropTarget = `:host {
    ${tokens}
    font: 13px/18px "Segoe UI", "Microsoft YaHei UI", sans-serif;
    container-type: size;
  }
  *, *::before, *::after { box-sizing: border-box; }
  .target { display: grid; place-content: center; justify-items: center; gap: 4px; width: 100%; height: 100%; padding: 6px; text-align: center; background: var(--nt-surface); color: var(--nt-text); border: 1px dashed var(--nt-border); border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,.22); }
  .icon { display: inline-flex; align-items: center; justify-content: center; width: 20px; height: 20px; color: var(--nt-text-secondary); }
  .icon svg { width: 20px; height: 20px; fill: currentColor; }
  strong { font-size: 13px; line-height: 18px; font-weight: 600; white-space: nowrap; }
  small { font-size: 12px; line-height: 18px; color: var(--nt-text-secondary); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  :host(.inside) .target { background: var(--nt-accent-soft); border-color: var(--nt-accent); border-style: solid; }
  @container (max-height: 65px) { .icon, small { display: none; } }
  @media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation: none !important; transition: none !important; } }`;
  return { tokens, mediaEntry, dropTarget };
});
