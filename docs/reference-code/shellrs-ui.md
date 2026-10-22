# ShellRS UI composition reference

Verified2026-10-10 from user-supplied `D:\Code\shell-rs-main` source snapshot. This is visual/component precedent, not NxTools domain/native behavior authority. No reference or product executable built/launched; no SSH/PTY/keyring/filesystem transfer/updater/analytics operation. Checked-in screenshots were located, not directly visually inspected or accepted. No Git metadata in supplied directory; exact entry hashes below anchor this snapshot.

## Stack and reuse boundary
`Cargo.toml`: shellrs0.1.6, gpui-kit0.6.1; project documentation identifies gpui-base/gpui-component0.6.1 and gpui-pre0.3.2 family. `rust-toolchain.toml` pins1.98.1. NxTools pinned GPUI0.2.2/vendor API is different: Settings/Field/Radio/focus_visible/accessibility APIs are not assumed available. No dependency upgrade authorized by reference review.

Project license is **GPL-3.0-only**, confirmed Cargo.toml/LICENSE. Learn design principles and independently implement in NxTools; copying implementation/assets or adding dependency requires separate licensing/compatibility assessment. Do not treat supplied repository as permissive code.

## Verified source entries
All paths below relative to the supplied root.

| Entry | Verified responsibility / useful precedent |
| --- | --- |
| `.agents/skills/gpui-kit-design-guides/references/design-guides.md` | Read full890-line design guide: task-first hierarchy, semantic tokens, alignment spines, context density, relative-scale zoom, distinct complete states, real-window review; guide is authoritative in ShellRS, reference only here |
| `src/settings/app_theme.rs:1–200,237–267,284–395` | Default theme neutral-gray remapping preserves role relationships; SOFT_DARK includes background1e1e1f/chrome2f2f30/border-hover3b3b3d/focus7b7b7f/texte0e0e4. Recolor considers foreground pairing/readable contrast; tests examine multiple terminal themes and muted text readability |
| `src/settings/apply.rs:1–109` | Applies theme/locale/terminal options only when changed; theme change reapplies shared hover mapping. Application and terminal typography remain separate |
| `src/app/mod.rs:51–139` | One initialization and semantic list/table hover mapping; dark hover half the selected treatment; test verifies role distinction in light/dark. Not a reason to add another Next global theme mutation owner |
| `src/settings/settings_panel.rs:196–323` | `Settings → SettingPage → SettingGroup → SettingItem/SettingField`; small local context,200px sidebar API boundary, category recovery on remount, coherent page/group/control hierarchy. Standard compound components rather than per-page ad hoc tile layout |
| `src/host/host_dialog.rs:1154–1256,1384–1448` | Four-column Form/Field: name full row, related address/port3:1, distinct auth choices and own descriptions; bounded dialog body max90% viewport and top5%; footer test action left, Cancel/commit right, shared dialog Confirm/Cancel path |
| `src/shared/dialog.rs:1–127` | Repeated commit footer and deletion decision copy; keyed replaceable form-error notification dismissed when form closes. Error-notification policy is ShellRS-specific, not automatic authority to replace Next inline errors |
| `src/shared/segmented_control.rs:1–184` | Presentation wraps one RadioGroup/Radio behavior owner: equal flex shares/min_w_0, border reserved for every segment, checked distinct from hover, focus_visible, disabled semantics/tooltip, pointer focus policy. Shape changes never alter hit geometry |
| `src/workspace/title_bar.rs:1–190` | TitleBar/ButtonGroup and small ghost commands, selected mode mapped to owner; fixed local icon/label slots, no-drag boundary; state-colored update action is exceptional rather than all ordinary actions primary |
| `src/workspace/status_bar.rs:1–236` | Status typed by active object; connection states use icon+text, technical detail separate; StatusBar/Separator/tooltip composition, not redundant badge wall |
| `src/workspace/workspace_view.rs:91–94,2187–2206,2648–2686` | UI font scale12–20/default16/step2; changes theme font axis and rem size; stable title/body/status bands, shrinking content, centralized Root sheet/dialog/notification layers. Document Dock belongs to SSH product, not a Next shell replacement |
| `src/explorer/file_listing.rs:124–160,434–553` | DataTable column owner and per-cell rendering; name/icon stable lane, right-aligned sizes; selected/hover/cursor separate. Name-cell-only selection is WinSCP behavior and must not replace Next full task-row selection |
| `tests/ui/settings.rs:17–144` | Production Workspace theme toggle, one settings tab, category navigation, focus, restored category, repeated open and close; test source inspected, not executed |
| `tests/ui/{host_dialog,workspace,sftp,transfer}.rs` | Related production UI fixture entries located; not claiming all tests read/executed. No zoom-acceptance claim inferred solely from zoom action implementation |

## Why this is relevant to current Next feedback
1. **Coherent composition:** nav, groups, labels, help, controls, footer come from a small family of standard structures. Next's historical intrinsic-width nav/background and per-state foreground issues show why no individual control can be accepted independently of its whole surface.
2. **Role relationships, not color swapping:** selected plane stronger than hover; focus is keyboard presentation, not selected state. Status color is scarce; ordinary toolbar commands stay quiet. Foreground/background are tested together.
3. **Whole local density:** Size affects label/icon/padding/frame, not only an externally forced height. Consistent relative scale exists across ordinary workbench UI, with explicit pixel exceptions for external window/pane/table API geometry.
4. **Stable alignment lanes:** form fields share a grid, related fields alone share rows; table identity/numeric/action columns are stable; borders exist before selection, so state changes do not shift content.
5. **One behavior owner:** standard Button/Radio/Dialog/Action owns activation/focus/disabled/dismissal; product only composes appearance/meaning. Do not rebuild a competing keyboard framework to imitate the new stack.
6. **Evidence reflects real task:** production Workspace tests navigate, open/close/reopen and verify state, not just static token strings. Source test inspection is not a passing compiled/native gate.

## NxTools adaptation map (proposed, not implemented by this review)
- First: Global/Download settings and main Quick/Clipboard configuration. Establish one reusable navigation/section/field/footer grammar; retain every field/control and immediate-versus-draft owner. Long/many/empty/busy/error and actual minimum clients must fit without inflating windows.
- Shared buttons: state background+label+icon pairing; one primary commit per decision, quiet normal commands; normal/hover/pressed/keyboard/disabled/loading and single-dispatch production tests.
- Downloads list: stable header/data/numeric/action columns, full-row selection/multiselect/context/queue/five details unchanged. Do not import WinSCP name-only selection or Dock interaction.
- Quick/Clipboard popup: use role/alignment principles within620×44/410 and300×440 plus current previews; no automatic16px baseline/rem conversion of fixed-client surfaces or new preview native owner.
- Shared overlays: coherent title/body/footer and region-owned scrolling; keep current inline failure/draft/retry semantics and passive preview/noactivate/lease boundaries.

Canonical implementation order remains Foundation → Downloads → Quick Find → Clipboard + Tools → Global settings + supporting surfaces. Reference-first behavioral source remains legacy NxTools/ABDM; ShellRS only supplies visual/component precedent. No control deletion, unified mega-save, theme chooser/interface zoom feature, provider injection, copiedGPL code, vendor patch or service/install scope implied.

## Snapshot anchors (SHA256)
- Cargo.toml: AF534EA64A61BCC73FB9243C3130AE61596C81C6F1F63AAF9093B4009C9C8847
- src/settings/app_theme.rs:9DC68AA11DE2456FA98E90DCEB7D35FC4DFC712C272993ABEF25F337D02076B9
- src/settings/settings_panel.rs:25D1DC5D51A2AAFE360286E27206070414FC428C63379D1A75DF1C241BFAF464
- src/shared/segmented_control.rs:C7617D5459EC896E81C292B6279BDDE8E1A750C5B19F03F467C8088CBCD0BF65
- src/workspace/title_bar.rs:8CC7B2AE8BB07C897CD95AF9D14CA0CA1F92267727E0A708ED51DB6C788BA6F6
- src/host/host_dialog.rs:22BED3A1E958EF35CE6504277528047ADCEC39657363CDBF71072A46E4760A29

## Verification limits
Source entries/guide/theme/form/table/status/test implementation inspected; screenshots exist in docs/screenshots, not newly eye-inspected. No ShellRS compiled checks/live window, no Next product changes, Cargo, service, package/install, clipboard/history access or copied code this task. The reference study establishes actionable direction, not a new approved numeric/color matrix or aesthetic certification.

## Continued Kit migration reference (2026-10-10)
User explicitly requests source-backed ShellRS comparison during implementation. Revisited src/explorer/pane_menu.rs directory_menu/path_menu/refresh_items: capability-disabled entries, stable icon menu structure, separators for task groups and owned submenu snapshots. src/history/history_panel.rs build_context_menu separates insert/run from copy without dropping commands. Revisited src/shared/segmented_control.rs RenderOnce and src/app/mod.rs deepen_list_hover/selected_row_color: constant geometry, one underlying behavior owner, checked independent from hover and dark hover weaker than selected. Those0.6.1 APIs/implementation are not copied into Kit0.7.1 Next.

Application boundary for this slice: independently implement stable14px leading marker/icon and14px trailing arrow columns, flexible clipped label with full tooltip, neutral selected/hover/pressed pairing, unchanged28px menu row and240px maximum width within300px Clipboard client. Preserve all Clipboard actions/source-category checks/deferred lease boundaries and flat shell; ShellRS's SSH/WinSCP actions, color literals, rem scale and shadow treatment are not product authority. Actual production bounds and callback tests, not ShellRS source inspection, establish acceptance.
