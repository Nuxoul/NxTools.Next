pub(crate) mod action;
pub(crate) mod custom_command_editor;
pub(crate) mod dialog;
pub(crate) mod field;
pub(crate) mod hotkey;
pub(crate) mod list;
pub(crate) mod menu;
pub(crate) mod native_context_menu;
pub(crate) mod path_list;
pub(crate) mod popover;
pub(crate) mod project_menu;
pub(crate) mod quick_launch_diagnostics;
pub(crate) mod settings;
pub(crate) mod shortcut;
pub(crate) mod state;
pub(crate) mod styled_menu_surface;
pub(crate) mod titlebar;
pub(crate) mod tooltip;
pub(crate) mod volume_scope;

#[cfg(test)]
mod kit_adoption_tests;

#[cfg(test)]
mod support_migration_tests;

#[cfg(test)]
mod ordinary_rem_tests;
