use std::path::Path;

use gpui::SharedString;
use gpui_component::IconNamed;

pub(crate) fn shell_icon_path_is_renderable(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        && imagesize::size(path)
            .is_ok_and(|dimensions| dimensions.width > 0 && dimensions.height > 0)
}

#[cfg(test)]
pub(crate) const ONE_PIXEL_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xb5, 0x1c, 0x0c,
    0x02, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x64, 0xf8, 0x0f, 0x00,
    0x01, 0x05, 0x01, 0x01, 0x27, 0x18, 0xe3, 0x66, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl IconNamed for CaptionIcon {
    fn path(self) -> SharedString {
        match self {
            Self::Minimize => "caption/minimize.svg",
            Self::Maximize => "caption/maximize.svg",
            Self::Restore => "caption/restore.svg",
            Self::Close => "caption/close.svg",
        }
        .into()
    }
}

#[cfg(test)]
impl CaptionIcon {
    pub(crate) const ALL: &'static [Self] =
        &[Self::Minimize, Self::Maximize, Self::Restore, Self::Close];
}

macro_rules! material_icons {
    ($( $variant:ident => $path:literal ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum MaterialIcon {
            $( $variant, )+
        }

        impl IconNamed for MaterialIcon {
            fn path(self) -> SharedString {
                match self {
                    $( Self::$variant => $path, )+
                }
                .into()
            }
        }

        #[cfg(test)]
        impl MaterialIcon {
            pub(crate) const ALL: &'static [Self] = &[
                $( Self::$variant, )+
            ];
        }
    };
}

material_icons! {
    Calculate => "material-symbols/calculate.svg",
    DataObject => "material-symbols/data-object.svg",
    Language => "material-symbols/language.svg",
    Terminal => "material-symbols/terminal.svg",
    Dashboard => "material-symbols/dashboard.svg",
    Star => "material-symbols/star.svg",
    TextFields => "material-symbols/text-fields.svg",
    Image => "material-symbols/image.svg",
    Folder => "material-symbols/folder.svg",
    Add => "material-symbols/add.svg",
    Edit => "material-symbols/edit.svg",
    Keep => "material-symbols/keep.svg",
    Delete => "material-symbols/delete.svg",
    ArrowUpward => "material-symbols/arrow-upward.svg",
    Undo => "material-symbols/undo.svg",
    OpenInFull => "material-symbols/open-in-full.svg",
    Search => "material-symbols/search.svg",
    Settings => "material-symbols/settings.svg",
    History => "material-symbols/history.svg",
    ManageSearch => "material-symbols/manage-search.svg",
    OpenInNew => "material-symbols/open-in-new.svg",
    Download => "material-symbols/download.svg",
    ContentCopy => "material-symbols/content-copy.svg",
    ContentPaste => "material-symbols/content-paste.svg",
    Check => "material-symbols/check.svg",
    Close => "material-symbols/close.svg",
    Info => "material-symbols/info.svg",
    ProgressActivity => "material-symbols/progress-activity.svg",
    CalendarMonth => "material-symbols/calendar-month.svg",
    ExpandMore => "material-symbols/expand-more.svg",
    ExpandLess => "material-symbols/expand-less.svg",
    ChevronLeft => "material-symbols/chevron-left.svg",
    ChevronRight => "material-symbols/chevron-right.svg",
    Cancel => "material-symbols/cancel.svg",
}
