use gpui::SharedString;
use gpui_component::IconNamed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialIcon {
    Calculate,
    DataObject,
    Language,
    Terminal,
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
    History,
    ManageSearch,
    OpenInNew,
    Download,
    ContentCopy,
    ContentPaste,
    Check,
    Close,
    Info,
    ProgressActivity,
    ChevronRight,
    Cancel,
}

impl IconNamed for MaterialIcon {
    fn path(self) -> SharedString {
        match self {
            Self::Calculate => "material-symbols/calculate.svg",
            Self::DataObject => "material-symbols/data-object.svg",
            Self::Language => "material-symbols/language.svg",
            Self::Terminal => "material-symbols/terminal.svg",
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
            Self::History => "material-symbols/history.svg",
            Self::ManageSearch => "material-symbols/manage-search.svg",
            Self::OpenInNew => "material-symbols/open-in-new.svg",
            Self::Download => "material-symbols/download.svg",
            Self::ContentCopy => "material-symbols/content-copy.svg",
            Self::ContentPaste => "material-symbols/content-paste.svg",
            Self::Check => "material-symbols/check.svg",
            Self::Close => "material-symbols/close.svg",
            Self::Info => "material-symbols/info.svg",
            Self::ProgressActivity => "material-symbols/progress-activity.svg",
            Self::ChevronRight => "material-symbols/chevron-right.svg",
            Self::Cancel => "material-symbols/cancel.svg",
        }
        .into()
    }
}
