# Design

Keep Kit/Base/Component/Assets0.8.0 with gpui-pre0.3.9. Retain Graphite Light/Dark, existing fonts and semantic success/warning/danger colors.

## Surfaces

- Archive settings: match Download settings section panels and aligned label/help/control rows. Reuse the actual Download layout composition, official Settings/Form/Field/Switch/Checkbox/Select/NumberInput/Button. Header commands remain outlined; Toolbar.child must not erase their boundaries.
- Archive form: bounded source viewport above compact output/format fields; password and advanced disclosure retain original draft and validation; fixed primary/cancel footer.
- Download progress: larger filename, clear progress/bytes and speed/ETA, quiet short route and colored Nms. Original expansion owns technical details; official DescriptionList/Progress/Button retain interaction.
- Category rules: horizontal selected-category field and rule-heading/add action; actual rules below. No second category state or saver.
- Quick Launch: animate row content transform/opacity only. Tween ease-out, finite duration, no native-bound interpolation. Original generation/result identity records first entry; existing virtualized list, input, selection and preview owners remain. Reduced motion and disabled mode render final values immediately. Stable expanded bounds replace count-driven height settling; empty/query close retains compact lifecycle.

## Ownership

Rust owns settings, draft/output plans, list identities, animation decisions and layout. GPUI renders. C# owns Shell/COM selection, Windows icons and native bounds. Complete selection forwarding and original job admission/publication are reused; no timing-based intent aggregation or competing Shell drag framework.

New quick-create operations carry correlation through existing launch/job/native boundaries. Icon diagnostics expose fixed stage/category, exception type/HRESULT/native codes and elapsed time only. No path, filename, query, URL, IP, payload, cookies or credentials are logged.
