# Design delivery verification record

## Scope
Approved documentation + interactive local prototype after the user's whole-app layout replan. Existing sources/working tree retained; no production code migration, build, package, service or user-data operation. Existing staged task/spec updated with design-only scope. Typography/rail candidates await user visual approval. Original19:30 installed layout rejection remains recorded.

## Checks performed
- Five design documents local links and23 core production entry files: passed through session scratch `check-layout-docs.py`.
- `python tests/architecture/spec-contracts.py`:87 documents/12 owners navigation/contracts pass. Checks docs, not product behavior. Final diff check passes.
- `node --check artifacts/ui-layout-quality/prototype/prototype.js` and `design-checks.cjs`:passed.
- `node artifacts/ui-layout-quality/prototype/design-checks.cjs`:final305 passed,0 failed.125 applicable page/state/size audits +148 subsection/state/size audits +32 interaction/keyboard/alignment cases. Fresh headless Chrome profile in scratch, file URL and CDP anonymous pipes; no product HTTP server.
- Browser tests include default/min workbench, fixed windows, long/empty/loading/error/busy, rule grid including scrollbar gutter/many rows, independent scopes, failed/successful memory Apply/Cancel, command drafts/error, paused task intake, Clipboard immediate-setting/edit failure/inert background, Tab/Space/arrows/Enter/Escape and focus return, disabled no dispatch, reduced motion and explicit fit vs actual scrolling at600px viewport.

## Concrete convergence fixes
Delegate reported233 checks. Parent found initial Quick popup640×420 assumption conflicted with verified620×410. Corrected dimensions/search42/result rows40, retained complete names in title and explicit ellipsis rails. First corrected run failed exactly Quick/long horizontal overflow; fixed flexible-name min-width/ellipsis, reran234/234. Expanded subsection checks to default/minimum,302/302. Added actual existing collapsed620×44/search-input expansion with focus regression,305/305. Parent initial event order was corrected before final run. Early screenshot showed unresolved path interpolation; delegate repaired final rendering. No relaxed footer/bounds assertions.

PowerShell initially read UTF8 JSON using default encoding and emitted a parsing error; report bytes/Node checks were valid. Explicit UTF8/Node reading confirmed final counts. The tool browser disallowed Emulation.setDeviceMetricsOverride; no workaround/approval bypass used. Browser captures from the exact tested standalone Chrome frame used for full-window comparison instead. One live guest screenshot did not answer; later gallery screenshots succeeded.

## Visual evidence
Eight final browser frame PNGs in `screenshots/`:downloads1060×650,global720×480,quickSettings1060×650,clipboardSettings1060×650,quickPopup620×410,clipboardPopup300×440,confirm560×278,progress440×184.1:1 CSS/deviceScaleFactor1 captures; gallery fit labeling prevents resized image from becoming type/DPI proof. Parent inspected download/global/command/clipboard popup captures via tool screenshots; other captures acquired, full state visual matrix/user approval pending. No claim screenshots prove noactivate/native focus/OLE/provider.

Full machine report remains session scratch `prototype-design-checks.json`. Screenshot hashes/sizes are recorded in `evidence.json`. Prototype and screenshot artifacts are ignored by current Git rules; no force-add/ignore modification.

## Remaining gates
- User visual approval of workbench type/rail/form and compact density.
- Fully interactive native-sized download settings/schedule/delete/shutdown/tool panel/attached search/preview/extension:currently source-backed blueprints/supporting wireframes, not complete render acceptance.
- Production GPUI representative/native screenshot comparison and regression migration in canonical order.
- Native100/125/150/mixed DPI, physical pointer/keyboard lifecycle and installed candidate, when separately authorized.
- Pinned GPUI reader-provider gap unchanged; web ARIA is not native support.

This is a completed first design/prototype deliverable, not whole-app UI implementation or release acceptance.

Final review addendum: Quick popup620×410 was also inspected via gallery tool screenshot after convergence. Five docs/23 source-entry check, explicit static no-network/storage/clipboard API/remote-assets scan, spec87/12 and diff check passed at final closeout. Screenshot hashes recorded in evidence.json. This inspection still does not approve complete states, native fonts or user direction.
