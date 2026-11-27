//! Reader-local document typography and article geometry.
use gpui_kit::base::text::{CodeBlock, TextViewStyle};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    AnyElement, ClipboardItem, FontWeight, HighlightStyle, Overflow, Pixels, StyleRefinement,
    relative, rems,
};

use super::*;

pub(super) const STANDARD_MEASURE: f32 = 880.;
pub(super) const WIDE_MEASURE: f32 = 1240.;
pub(super) const SIDEBAR_WIDTH: f32 = 194.;
pub(super) const RAIL_WIDTH: f32 = 26.;
const RAIL_GAP: f32 = 32.;
const PIN_GAP: f32 = 8.;

pub(super) fn can_pin_outline(width: Pixels) -> bool {
    width >= px(560. + SIDEBAR_WIDTH + PIN_GAP + 44.)
}

pub(super) struct ArticleLayout {
    pub(super) left: Pixels,
    pub(super) width: Pixels,
    pub(super) inset: Pixels,
    pub(super) outline_left: Pixels,
    pub(super) pinned: bool,
}

pub(super) fn article_layout(
    width: Pixels,
    measure: f32,
    outline: bool,
    pin: bool,
) -> ArticleLayout {
    let inset = if width >= px(600.) { px(32.) } else { px(20.) };
    let desired = (width - inset * 2.).clamp(px(1.), px(measure));
    let pinned = outline && pin && can_pin_outline(width);
    let clearance = if pinned {
        px(12. + SIDEBAR_WIDTH + PIN_GAP)
    } else if outline {
        px(10. + RAIL_WIDTH)
    } else {
        inset
    };
    let left = ((width - desired) / 2.).max(clearance.min((width - inset - px(1.)).max(px(0.))));
    let outline_left = if pinned {
        left - px(SIDEBAR_WIDTH + PIN_GAP)
    } else {
        (left - px(RAIL_WIDTH + RAIL_GAP)).max(px(6.))
    };
    ArticleLayout {
        left,
        width: desired.min((width - inset - left).max(px(1.))),
        inset,
        outline_left,
        pinned,
    }
}

pub(super) struct Paper {
    pub(super) background: gpui_kit::Hsla,
    pub(super) ink: gpui_kit::Hsla,
    pub(super) muted: gpui_kit::Hsla,
    border: gpui_kit::Hsla,
    code: gpui_kit::Hsla,
    table_head: gpui_kit::Hsla,
}

/// Long-form text keeps a nearly solid plane on a solid palette.
const READING_FLOOR: f32 = 0.9;

/// Solid palettes read on the table plane. A frosted window's Root already paints the
/// content layer over the veil, so the page stays clear and the reader shows that material;
/// the cleared table roles must not be floored into a dark wash.
pub(super) fn paper(cx: &App) -> Paper {
    let colors = &gpui_kit::component::Theme::global(cx).colors;
    let frosted = cx
        .try_global::<crate::theme::UiThemeState>()
        .is_some_and(|state| state.material_style.is_frosted());
    let (background, table_head) = if frosted {
        (gpui_kit::transparent_black(), colors.muted)
    } else {
        let mut background = colors.table;
        background.a = background.a.max(READING_FLOOR);
        (background, colors.table_head)
    };
    Paper {
        background,
        ink: colors.foreground,
        muted: colors.muted_foreground,
        border: colors.border,
        code: colors.secondary,
        table_head,
    }
}

pub(super) fn prose_style(body: f32, cx: &App) -> TextViewStyle {
    let theme = cx.theme();
    let Paper {
        background,
        ink,
        muted,
        border,
        code,
        table_head,
    } = paper(cx);
    let code_box = StyleRefinement::default()
        .rounded(px(6.))
        .text_size(px(body * 0.85))
        .line_height(relative(1.45))
        .border_0()
        .p(px(body));
    let mut table = StyleRefinement::default().rounded(px(6.)).bg(background);
    table.overflow.x = Some(Overflow::Scroll);
    let table_text = (body * 0.875).round();
    TextViewStyle::default()
        .with_dark(theme.is_dark())
        .with_foreground(ink)
        .with_muted_foreground(muted)
        .with_selection(theme.selection)
        .with_link(theme.link)
        .with_border(border)
        .with_paragraph_gap(rems(body / 16.))
        .with_heading(move |level| {
            let scale = [2., 1.5, 1.25, 1.0, 0.875, 0.85][usize::from(level.clamp(1, 6) - 1)];
            let style = StyleRefinement::default()
                .text_size(px(body * scale))
                .line_height(relative(1.25))
                .font_weight(FontWeight::BOLD)
                .mt(px(if level > 1 { body * 0.5 } else { 0. }))
                .text_color(if level == 6 { muted } else { ink });
            if level <= 2 {
                style
                    .border_b_1()
                    .border_color(border)
                    .pb(px(body * scale * 0.3))
            } else {
                style
            }
        })
        .with_code_background(code)
        .with_inline_code(HighlightStyle {
            background_color: Some(code),
            ..Default::default()
        })
        .with_code_block(code_box)
        .with_table(table)
        .with_table_head(
            StyleRefinement::default()
                .bg(table_head)
                .text_color(ink)
                .font_weight(FontWeight::SEMIBOLD),
        )
        .with_table_cell(
            StyleRefinement::default()
                .text_size(px(table_text))
                .line_height(relative(1.5))
                .px(px((table_text * 0.8125).round()))
                .py(px((table_text * 0.5).round()))
                .border_r_0(),
        )
}

pub(super) fn code_actions(
    block: &CodeBlock,
    hovered: Option<usize>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let text = block.code();
    let source = block.span.map(|span| span.start);
    let visible = source.is_some() && source == hovered;
    let focus = window
        .use_keyed_state(
            ("markdown-code-overlay", source.unwrap_or(usize::MAX)),
            cx,
            |_, cx| cx.focus_handle(),
        )
        .read(cx)
        .clone();
    div()
        .id("markdown-code-action")
        .track_focus(&focus)
        .in_focus(|style| style.opacity(1.))
        .opacity(if visible { 1. } else { 0. })
        .child(
            Button::new("markdown-code-copy")
                .debug_selector(|| "markdown-code-copy".into())
                .icon(IconName::Copy)
                .tooltip("复制代码")
                .accessibility_label("复制代码")
                .xsmall()
                .ghost()
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()))
                }),
        )
        .into_any_element()
}

#[cfg(test)]
mod material_tests {
    use super::*;

    #[gpui_kit::test]
    fn material_reader_paper_tracks_public_reading_roles(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        for preference in [
            nxtools_core::ThemePreference::Light,
            nxtools_core::ThemePreference::Dark,
        ] {
            for style in [
                nxtools_core::MaterialStyle::Solid,
                nxtools_core::MaterialStyle::Acrylic,
            ] {
                cx.update(|cx| crate::theme::set_appearance_preferences(preference, style, cx));
                cx.read(|cx| {
                    let page = paper(cx);
                    let colors = &gpui_kit::component::Theme::global(cx).colors;
                    assert_eq!(page.ink, colors.foreground);
                    assert_eq!(page.muted, colors.muted_foreground);
                    assert_eq!(page.border, colors.border);
                    if style.is_frosted() {
                        // The window's frosted Root shows through; never a floored dark wash.
                        assert_eq!(page.background.a, 0.0);
                        assert_eq!(page.table_head, colors.muted);
                    } else {
                        assert_eq!(
                            (page.background.h, page.background.s, page.background.l),
                            (colors.table.h, colors.table.s, colors.table.l)
                        );
                        assert!(
                            page.background.a >= READING_FLOOR,
                            "reading remains a solid-enough plane"
                        );
                    }
                });
            }
        }
    }
}
