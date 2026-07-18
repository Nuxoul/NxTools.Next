use gpui::SharedString;
use gpui_component::IconNamed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialIcon {
    Dashboard,
    Star,
    TextFields,
    Image,
    Folder,
    Add,
    Edit,
    Keep,
    Delete,
    ArrowUpward,
    Undo,
    OpenInFull,
    Search,
}

impl IconNamed for MaterialIcon {
    fn path(self) -> SharedString {
        match self {
            Self::Dashboard => "material-symbols/dashboard.svg",
            Self::Star => "material-symbols/star.svg",
            Self::TextFields => "material-symbols/text-fields.svg",
            Self::Image => "material-symbols/image.svg",
            Self::Folder => "material-symbols/folder.svg",
            Self::Add => "material-symbols/add.svg",
            Self::Edit => "material-symbols/edit.svg",
            Self::Keep => "material-symbols/keep.svg",
            Self::Delete => "material-symbols/delete.svg",
            Self::ArrowUpward => "material-symbols/arrow-upward.svg",
            Self::Undo => "material-symbols/undo.svg",
            Self::OpenInFull => "material-symbols/open-in-full.svg",
            Self::Search => "material-symbols/search.svg",
        }
        .into()
    }
}
