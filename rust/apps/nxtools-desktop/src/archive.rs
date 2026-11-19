//! Keyed browsers and bounded jobs over the original archive transport/worker.
mod application;
mod batch;
mod browser;
#[cfg(test)]
pub(crate) use browser::ArchiveBrowser;
mod draft;
mod forms;
mod model;
pub(crate) mod preview;
mod progress;
mod queue;
mod space;
#[cfg(debug_assertions)]
pub(crate) mod visual_preview;
#[cfg(test)]
pub(crate) use application::settings_updated;
pub(crate) use application::shell_integration;
pub(crate) use application::{
    ArchiveApplication, clear_recent, extract_from_window, handle, new_from_window, open_internal,
    open_recent, pick_from_window, pick_temp_directory, recent_owner, recent_snapshot, register,
    save_settings, saved_settings_updated, set_config_path,
};
