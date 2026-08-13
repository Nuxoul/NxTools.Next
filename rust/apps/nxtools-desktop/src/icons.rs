use gpui::SharedString;
use gpui_component::IconNamed;

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
    ChevronLeft => "material-symbols/chevron-left.svg",
    ChevronRight => "material-symbols/chevron-right.svg",
    Cancel => "material-symbols/cancel.svg",
}
