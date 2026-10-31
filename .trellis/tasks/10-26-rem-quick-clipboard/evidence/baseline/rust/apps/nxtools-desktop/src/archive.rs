//! Keyed browsers and bounded jobs over the original archive transport/worker.
mod application;
mod browser;
mod draft;
mod forms;
mod model;
mod progress;
mod queue;
#[cfg(test)]
mod virtual_drag_spike;
#[cfg(test)]
pub(crate) use application::settings_updated;
pub(crate) use application::shell_integration;
pub(crate) use application::{
    ArchiveApplication, clear_recent, extract_from_window, handle, new_from_window, open_internal,
    open_recent, pick_from_window, pick_temp_directory, recent_owner, recent_snapshot, register,
    save_settings, saved_settings_updated, set_config_path,
};
