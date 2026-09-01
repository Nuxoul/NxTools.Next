(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) {
    module.exports = api;
  }
  root.NxToolsShadowStyles = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";

  /* Shared browser UI tokens for the injected Shadow DOM overlays (EX-14).
   *
   * These values mirror popup-tokens.css, which mirrors the desktop semantic
   * palette in rust/apps/nxtools-desktop/src/theme.rs (Theme::zed_dark). Keep
   * the three files in sync: shadow-styles.test.js locks this block against
   * popup-tokens.css declaration by declaration, so a desktop theme change
   * must update both files in the same slice. The content-script overlays
   * interpolate this block into their :host rules instead of carrying their
   * own copies. */

  const tokens = `
          --nt-canvas: #1e1f22;
          --nt-surface: #242529;
          --nt-surface-raised: #2a2b2f;
          --nt-surface-hover: #303238;
          --nt-surface-pressed: #35373d;
          --nt-border-subtle: #34363b;
          --nt-border: #44474f;
          --nt-text: #e0e0e0;
          --nt-text-secondary: #b8bac0;
          --nt-text-muted: #858991;
          --nt-text-disabled: #64676e;
          --nt-accent: #4791bf;
          --nt-accent-soft: #273744;
          --nt-accent-text: #d6e8f5;
          --nt-warning: #d6a85f;
          --nt-danger: #ff6b78;
          --nt-danger-soft: #35252a;`;

  const sharedComment = `/* Shared browser tokens (EX-14); values mirror popup-tokens.css. */`;

  const mediaEntry = `:host {
          ${sharedComment}
${tokens}
          font: 13px/18px "Segoe UI", "Microsoft YaHei UI", Arial, sans-serif;
          height: 28px;
          overflow: visible;
          position: fixed;
          width: 28px;
        }
        .trigger {
          align-items: center;
          background: var(--nt-surface-raised, #2a2b2f);
          border: 1px solid var(--nt-border, #44474f);
          border-radius: 4px;
          box-shadow: 0 2px 8px rgba(0, 0, 0, 0.28);
          color: var(--nt-text, #e0e0e0);
          cursor: pointer;
          display: inline-flex;
          font: inherit;
          height: 28px;
          justify-content: center;
          padding: 0;
          width: 28px;
        }
        .trigger:hover {
          background: var(--nt-surface-hover, #303238);
          border-color: var(--nt-accent, #4791bf);
        }
        .trigger:disabled { cursor: default; opacity: .78; }
        .trigger[aria-expanded="true"] {
          border-color: var(--nt-accent, #4791bf);
        }
        .icon {
          align-items: center;
          color: var(--nt-text, #e0e0e0);
          display: inline-flex;
          height: 28px;
          justify-content: center;
          width: 28px;
        }
        .icon svg {
          height: 16px;
          width: 16px;
          fill: currentColor;
        }
        .status {
          height: 1px;
          opacity: 0;
          overflow: hidden;
          pointer-events: none;
          position: absolute;
          width: 1px;
        }
        .label {
          height: 1px;
          opacity: 0;
          overflow: hidden;
          pointer-events: none;
          position: absolute;
          width: 1px;
        }
        .error { color: #ff9b91; }
        .panel {
          background: var(--nt-surface-raised, #2a2b2f);
          border: 1px solid var(--nt-border, #44474f);
          border-radius: 6px;
          box-shadow: 0 10px 26px rgba(0, 0, 0, 0.28);
          display: grid;
          gap: 3px;
          max-width: min(430px, calc(100vw - 16px));
          max-height: min(386px, calc(100vh - 72px));
          overflow: auto;
          padding: 4px;
          position: absolute;
          right: 0;
          top: 34px;
          width: min(414px, calc(100vw - 16px));
        }
        .panel[hidden] { display: none; }
        .choice {
          align-items: start;
          background: transparent;
          border: 1px solid transparent;
          border-radius: 4px;
          display: grid;
          gap: 2px;
          grid-template-columns: minmax(0, 1fr);
          min-height: 44px;
          padding: 6px 8px;
          text-align: left;
          width: 100%;
          cursor: pointer;
        }
        .choice:hover {
          background: var(--nt-surface-hover, #303238);
        }
        .choice:focus-visible { outline: 2px solid var(--nt-accent, #4791bf); outline-offset: 1px; }
        .choice span {
          min-width: 0;
        }
        .choice-file {
          color: var(--nt-text, #e0e0e0);
          display: block;
          font-size: 11px;
          font-weight: 600;
          line-height: 15px;
          overflow: hidden;
          overflow-wrap: anywhere;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .choice-meta {
          color: var(--nt-text-muted, #858991);
          display: block;
          font-family: "Cascadia Mono", "SFMono-Regular", Consolas, monospace;
          font-size: 10px;
          line-height: 14px;
          overflow: hidden;
          overflow-wrap: anywhere;
          text-overflow: ellipsis;
          white-space: nowrap;
        }`;

  const dropTarget = `:host {
          ${sharedComment}
${tokens}
          font: 13px/18px "Segoe UI", "Microsoft YaHei UI", Arial, sans-serif;
        }
        .target {
          align-items: center;
          background: var(--nt-surface, #242529);
          border: 1px dashed var(--nt-border, #44474f);
          border-radius: 6px;
          box-shadow: 0 10px 26px rgba(0, 0, 0, 0.28);
          color: var(--nt-text, #e0e0e0);
          display: grid;
          gap: 4px;
          height: 100%;
          justify-items: center;
          padding: 10px;
          text-align: center;
          width: 100%;
        }
        .icon {
          align-items: center;
          color: var(--nt-text-secondary, #b8bac0);
          display: inline-flex;
          height: 24px;
          justify-content: center;
          width: 24px;
        }
        .icon svg {
          fill: currentColor;
          height: 22px;
          width: 22px;
        }
        strong { font-size: 13px; font-weight: 700; }
        small {
          color: var(--nt-text-secondary, #b8bac0);
          font-size: 11px;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          width: 100%;
        }
        :host(.inside) .target {
          background: var(--nt-accent-soft, #273744);
          border-color: var(--nt-accent, #4791bf);
          border-style: solid;
        }`;

  return { tokens, mediaEntry, dropTarget };
});
