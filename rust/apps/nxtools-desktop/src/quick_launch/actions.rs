use super::search::is_navigable_directory_target;
use super::*;

#[derive(Clone)]
pub(crate) struct PendingQuickLaunchAction {
    pub(crate) correlation_id: uuid::Uuid,
    pub(crate) popup_session_id: uuid::Uuid,
    pub(crate) entry_id: String,
    pub(crate) display_name: String,
    pub(crate) target: String,
    pub(crate) query: String,
    pub(crate) action: QuickLaunchAction,
    pub(crate) directory_visit: Option<String>,
    pub(crate) trigger: QuickLaunchActionTrigger,
    pub(crate) source: ApplicationSource,
    pub(crate) started_at: Instant,
}

pub(crate) fn pending_action_targets_session(
    pending_action: &PendingQuickLaunchAction,
    popup_session_id: uuid::Uuid,
) -> bool {
    pending_action.popup_session_id == popup_session_id
}

pub(crate) fn partition_actions_for_disconnect(
    actions: std::collections::HashMap<uuid::Uuid, PendingQuickLaunchAction>,
    current_session_id: uuid::Uuid,
) -> (Vec<PendingQuickLaunchAction>, usize) {
    let mut current = Vec::new();
    let mut stale_count = 0_usize;
    for (_, pending) in actions {
        if pending_action_targets_session(&pending, current_session_id) {
            current.push(pending);
        } else {
            stale_count = stale_count.saturating_add(1);
        }
    }
    (current, stale_count)
}

pub(crate) fn pending_action_owns_foreground(
    pending_action: &PendingQuickLaunchAction,
    popup_session_id: uuid::Uuid,
    foreground_window_handle: i64,
) -> bool {
    pending_action_targets_session(pending_action, popup_session_id)
        && matches!(
            pending_action.action,
            QuickLaunchAction::SwitchDirectory { window_handle, .. }
                if window_handle == foreground_window_handle
        )
}

#[derive(Clone, Copy)]
pub(crate) struct NavigationForegroundGuard {
    pub(crate) popup_session_id: uuid::Uuid,
    pub(crate) window_handle: i64,
    pub(crate) expires_at: Instant,
}

pub(crate) fn navigation_guard_owns_foreground(
    guard: &mut Option<NavigationForegroundGuard>,
    popup_session_id: uuid::Uuid,
    foreground_window_handle: i64,
    now: Instant,
) -> bool {
    let Some(current) = *guard else {
        return false;
    };
    if current.popup_session_id != popup_session_id || now >= current.expires_at {
        *guard = None;
        return false;
    }
    current.window_handle == foreground_window_handle
}

#[derive(Clone)]
pub(crate) struct PendingShellQuery {
    pub(crate) mode_id: uuid::Uuid,
    pub(crate) level_id: uuid::Uuid,
    pub(crate) entry_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum QuickLaunchAction {
    Open,
    SwitchDirectory {
        host: DirectoryNavigationHost,
        window_handle: i64,
        target_directory: String,
        host_context_id: Option<String>,
    },
    Reveal,
    CopyPath,
    CopyResult,
    CopyCommand,
    RunElevated,
    Shell {
        session_id: String,
        command_id: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QuickLaunchActionTrigger {
    Enter,
    Shortcut,
    Pointer,
    ActionMode,
    RecentDirectory,
}

impl QuickLaunchActionTrigger {
    pub(crate) const fn diagnostic_label(self) -> &'static str {
        match self {
            Self::Enter => "enter",
            Self::Shortcut => "shortcut",
            Self::Pointer => "pointer",
            Self::ActionMode => "action_mode",
            Self::RecentDirectory => "recent_directory",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActionSuccessPolicy {
    RecordAndHide,
    HideWithoutRecording,
    KeepOpen,
}

pub(crate) fn action_success_policy(action: &QuickLaunchAction) -> ActionSuccessPolicy {
    match action {
        QuickLaunchAction::CopyPath | QuickLaunchAction::CopyCommand => {
            ActionSuccessPolicy::KeepOpen
        }
        QuickLaunchAction::CopyResult => ActionSuccessPolicy::HideWithoutRecording,
        QuickLaunchAction::Open
        | QuickLaunchAction::SwitchDirectory { .. }
        | QuickLaunchAction::Reveal
        | QuickLaunchAction::RunElevated
        | QuickLaunchAction::Shell { .. } => ActionSuccessPolicy::RecordAndHide,
    }
}

pub(crate) fn quick_launch_action_label(action: &QuickLaunchAction) -> &str {
    if matches!(action, QuickLaunchAction::SwitchDirectory { .. }) {
        return "切换到此目录";
    }
    if matches!(action, QuickLaunchAction::Shell { .. }) {
        return "Windows 操作";
    }
    if matches!(action, QuickLaunchAction::CopyResult) {
        return "复制结果";
    }
    if matches!(action, QuickLaunchAction::CopyCommand) {
        return "复制命令";
    }
    QUICK_LAUNCH_ACTIONS
        .iter()
        .find(|item| &item.action == action)
        .map(|item| item.label)
        .unwrap_or("执行操作")
}

pub(crate) fn quick_launch_action_diagnostic_label(action: &QuickLaunchAction) -> &'static str {
    match action {
        QuickLaunchAction::Open => "open",
        QuickLaunchAction::SwitchDirectory { .. } => "switch_directory",
        QuickLaunchAction::Reveal => "reveal",
        QuickLaunchAction::CopyPath => "copy_path",
        QuickLaunchAction::CopyResult => "copy_result",
        QuickLaunchAction::CopyCommand => "copy_command",
        QuickLaunchAction::RunElevated => "run_elevated",
        QuickLaunchAction::Shell { .. } => "shell_action",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QuickLaunchBuiltinActionItem {
    pub(crate) action: QuickLaunchAction,
    pub(crate) label: &'static str,
    pub(crate) icon: MaterialIcon,
}

pub(crate) const QUICK_LAUNCH_ACTIONS: [QuickLaunchBuiltinActionItem; 4] = [
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::Open,
        label: "打开",
        icon: MaterialIcon::OpenInNew,
    },
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::Reveal,
        label: "打开文件所在位置",
        icon: MaterialIcon::Folder,
    },
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::CopyPath,
        label: "复制路径",
        icon: MaterialIcon::ContentCopy,
    },
    QuickLaunchBuiltinActionItem {
        action: QuickLaunchAction::RunElevated,
        label: "以管理员身份运行",
        icon: MaterialIcon::ProgressActivity,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QuickLaunchActionItem {
    pub(crate) action: Option<QuickLaunchAction>,
    pub(crate) label: gpui_kit::SharedString,
    pub(crate) icon: Option<MaterialIcon>,
    pub(crate) icon_path: Option<PathBuf>,
    pub(crate) enabled: bool,
    pub(crate) submenu: Option<QuickLaunchShellSubmenu>,
    pub(crate) section_label: Option<gpui_kit::SharedString>,
    pub(crate) divider_before: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QuickLaunchShellSubmenu {
    pub(crate) session_id: String,
    pub(crate) submenu_id: u32,
}

impl QuickLaunchActionItem {
    pub(crate) fn builtin(item: QuickLaunchBuiltinActionItem) -> Self {
        Self {
            action: Some(item.action),
            label: item.label.into(),
            icon: Some(item.icon),
            icon_path: None,
            enabled: true,
            submenu: None,
            section_label: None,
            divider_before: false,
        }
    }

    pub(crate) fn is_activatable(&self) -> bool {
        self.enabled && (self.action.is_some() || self.submenu.is_some())
    }

    pub(crate) fn switch_directory(
        host: DirectoryNavigationHost,
        window_handle: i64,
        target_directory: String,
        host_context_id: Option<String>,
    ) -> Self {
        Self::builtin(QuickLaunchBuiltinActionItem {
            action: QuickLaunchAction::SwitchDirectory {
                host,
                window_handle,
                target_directory,
                host_context_id,
            },
            label: "切换到此目录",
            icon: MaterialIcon::Folder,
        })
    }
}

pub(crate) fn shell_action_items(
    session_id: &str,
    items: Vec<ShellActionItem>,
) -> Vec<QuickLaunchActionItem> {
    let mut converted = Vec::new();
    let mut divider_before = false;
    for item in items {
        if item.separator {
            divider_before = !converted.is_empty();
            continue;
        }
        let submenu = item.submenu_id.map(|submenu_id| QuickLaunchShellSubmenu {
            session_id: session_id.into(),
            submenu_id,
        });
        let action = if submenu.is_none() {
            item.command_id.map(|command_id| QuickLaunchAction::Shell {
                session_id: session_id.into(),
                command_id,
            })
        } else {
            None
        };
        converted.push(QuickLaunchActionItem {
            action,
            label: item.title.into(),
            icon: None,
            icon_path: item
                .icon_path
                .map(PathBuf::from)
                .filter(|path| path.is_file()),
            enabled: item.enabled,
            submenu,
            section_label: None,
            divider_before,
        });
        divider_before = false;
    }
    converted
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QuickLaunchResultsContext {
    pub(crate) query: String,
    pub(crate) selected: usize,
    pub(crate) selected_entry_id: String,
}

pub(crate) fn selection_for_results_context(
    results: &[SearchResult],
    context: &QuickLaunchResultsContext,
) -> usize {
    results
        .iter()
        .position(|result| result.entry.id == context.selected_entry_id)
        .unwrap_or_else(|| context.selected.min(results.len().saturating_sub(1)))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QuickLaunchEscapeAction {
    ExitActions,
    ClearQuery,
    Close,
}

pub(crate) fn quick_launch_escape_action(
    action_mode_active: bool,
    query_is_empty: bool,
) -> QuickLaunchEscapeAction {
    if action_mode_active {
        QuickLaunchEscapeAction::ExitActions
    } else if query_is_empty {
        QuickLaunchEscapeAction::Close
    } else {
        QuickLaunchEscapeAction::ClearQuery
    }
}

/// One attempt to hide the quick-launch panel, tracked until Windows
/// confirms it. The user report of a panel that never disappears maps to a
/// lost or rejected visibility command, which this guard makes visible.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PendingPanelHide {
    pub(crate) popup_window_handle: i64,
    pub(crate) attempt: u8,
    pub(crate) session_id: uuid::Uuid,
    pub(crate) requested_at: std::time::Instant,
}

impl PendingPanelHide {
    pub(crate) fn correlation_id(&self) -> uuid::Uuid {
        uuid::Uuid::new_v4()
    }
}

/// Result-wait before the orphan recheck re-sends the hide command.
pub(crate) const PANEL_HIDE_RECHECK_DELAY_MS: u64 = 400;
/// Total hide attempts: the original command plus one self-healing retry.
pub(crate) const PANEL_HIDE_RETRY_LIMIT: u8 = 1;

pub(crate) fn quick_launch_dismiss_commands(
    popup_window_handle: i64,
    restore_window_handle: Option<i64>,
) -> Vec<PlatformCommand> {
    let mut commands = Vec::with_capacity(3);
    if popup_window_handle != 0 {
        commands.push(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand {
                window_handle: popup_window_handle,
                visible: false,
            },
        ));
        commands.push(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
            window_handle: popup_window_handle,
            enabled: false,
        }));
    }
    if let Some(window_handle) = restore_window_handle
        && window_handle != 0
        && window_handle != popup_window_handle
    {
        commands.push(PlatformCommand::ActivateWindow(ActivateWindowCommand {
            window_handle,
        }));
    }
    commands
}

#[derive(Clone)]
pub(crate) struct QuickLaunchActionLevel {
    pub(crate) id: uuid::Uuid,
    pub(crate) title: gpui_kit::SharedString,
    pub(crate) items: Vec<QuickLaunchActionItem>,
    pub(crate) filter: String,
    pub(crate) selected: usize,
    pub(crate) shell_pending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingShellSubmenuLevel {
    pub(crate) level_id: uuid::Uuid,
    pub(crate) session_id: String,
    pub(crate) submenu_id: u32,
}

#[derive(Clone)]
pub(crate) struct QuickLaunchActionMode {
    pub(crate) id: uuid::Uuid,
    pub(crate) target: SearchResult,
    pub(crate) target_title: gpui_kit::SharedString,
    pub(crate) parent: QuickLaunchResultsContext,
    pub(crate) navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
    pub(crate) levels: Vec<QuickLaunchActionLevel>,
}

impl QuickLaunchActionMode {
    #[cfg(test)]
    pub(crate) fn enter(target: SearchResult, query: String, selected: usize) -> Self {
        Self::enter_in_context(target, query, selected, None)
    }

    pub(crate) fn enter_in_context(
        target: SearchResult,
        query: String,
        selected: usize,
        navigation_context: Option<&nxtools_protocol::DirectoryNavigationContext>,
    ) -> Self {
        let selected_entry_id = target.entry.id.clone();
        let copy_text = target.copy_text().is_some();
        let items = if target.entry.source == ApplicationSource::CustomCommand {
            vec![
                QuickLaunchActionItem::builtin(QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::Open,
                    label: "运行命令",
                    icon: MaterialIcon::OpenInNew,
                }),
                QuickLaunchActionItem::builtin(QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::CopyCommand,
                    label: "复制命令",
                    icon: MaterialIcon::ContentCopy,
                }),
            ]
        } else if target.entry.source == ApplicationSource::EnvironmentVariable && copy_text {
            vec![QuickLaunchActionItem::builtin(
                QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::CopyResult,
                    label: "复制变量值",
                    icon: MaterialIcon::ContentCopy,
                },
            )]
        } else if copy_text {
            vec![QuickLaunchActionItem::builtin(
                QuickLaunchBuiltinActionItem {
                    action: QuickLaunchAction::CopyResult,
                    label: "复制结果",
                    icon: MaterialIcon::ContentCopy,
                },
            )]
        } else {
            QUICK_LAUNCH_ACTIONS
                .iter()
                .filter(|item| quick_launch_action_available(&target, &item.action))
                .cloned()
                .map(QuickLaunchActionItem::builtin)
                .collect::<Vec<_>>()
        };
        let shell_pending = quick_launch_result_capabilities(&target).shell_actions;
        Self {
            id: uuid::Uuid::new_v4(),
            target_title: target.entry.name.clone().into(),
            target,
            parent: QuickLaunchResultsContext {
                query,
                selected,
                selected_entry_id,
            },
            navigation_context: navigation_context
                .filter(|context| context.window_handle != 0)
                .cloned(),
            levels: vec![QuickLaunchActionLevel {
                id: uuid::Uuid::new_v4(),
                title: "操作".into(),
                items,
                filter: String::new(),
                selected: 0,
                shell_pending,
            }],
        }
    }

    pub(crate) fn target(&self) -> &SearchResult {
        &self.target
    }

    pub(crate) fn id(&self) -> uuid::Uuid {
        self.id
    }

    pub(crate) fn matches_shell_query(
        &self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: &str,
    ) -> bool {
        self.id == mode_id
            && self.current_level().id == level_id
            && self.target.entry.id == entry_id
    }

    pub(crate) fn exit(self) -> QuickLaunchResultsContext {
        self.parent
    }

    pub(crate) fn current_level(&self) -> &QuickLaunchActionLevel {
        self.levels
            .last()
            .expect("action mode always has a root level")
    }

    pub(crate) fn current_level_mut(&mut self) -> &mut QuickLaunchActionLevel {
        self.levels
            .last_mut()
            .expect("action mode always has a root level")
    }

    pub(crate) fn depth(&self) -> usize {
        self.levels.len()
    }

    pub(crate) fn title(&self) -> gpui_kit::SharedString {
        self.current_level().title.clone()
    }

    pub(crate) fn current_level_id(&self) -> uuid::Uuid {
        self.current_level().id
    }

    pub(crate) fn shell_pending(&self) -> bool {
        self.current_level().shell_pending
    }

    pub(crate) fn set_filter(&mut self, filter: impl Into<String>) {
        let level = self.current_level_mut();
        level.filter = filter.into();
        level.selected = first_activatable_index(&visible_action_items(level)).unwrap_or(0);
    }

    pub(crate) fn visible_actions(&self) -> Vec<QuickLaunchActionItem> {
        visible_action_items(self.current_level())
    }

    pub(crate) fn selected(&self) -> usize {
        self.current_level().selected
    }

    pub(crate) fn set_selected(&mut self, selected: usize) {
        let visible = self.visible_actions();
        if visible
            .get(selected)
            .is_some_and(QuickLaunchActionItem::is_activatable)
        {
            self.current_level_mut().selected = selected;
        }
    }

    pub(crate) fn apply_shell_menu(&mut self, level_id: uuid::Uuid, menu: Option<ShellActionMenu>) {
        if self.current_level().id != level_id {
            return;
        }
        self.current_level_mut().shell_pending = false;
        let navigation_directory = menu
            .as_ref()
            .and_then(|menu| menu.navigation_directory.clone())
            .filter(|directory| is_navigable_directory_target(directory));
        let navigation_action = self
            .navigation_context
            .as_ref()
            .map(|context| {
                (
                    context.host,
                    context.window_handle,
                    context.host_context_id.clone(),
                )
            })
            .zip(navigation_directory);
        if self.depth() == 1
            && let Some(((host, window_handle, host_context_id), target_directory)) =
                navigation_action
        {
            self.current_level_mut().items.insert(
                0,
                QuickLaunchActionItem::switch_directory(
                    host,
                    window_handle,
                    target_directory,
                    host_context_id,
                ),
            );
        }
        let Some(menu) = menu else {
            return;
        };
        let mut shell_items = shell_action_items(&menu.session_id, menu.items);
        if self.depth() == 1 {
            if let Some(first) = shell_items.first_mut() {
                first.section_label = Some("Windows 操作".into());
                first.divider_before = true;
            }
            self.current_level_mut().items.extend(shell_items);
        } else {
            self.current_level_mut().items = shell_items;
        }
        let level = self.current_level_mut();
        level.selected = first_activatable_index(&visible_action_items(level)).unwrap_or(0);
    }

    pub(crate) fn push_selected_submenu(&mut self) -> Option<PendingShellSubmenuLevel> {
        let selected = self
            .visible_actions()
            .get(self.selected())
            .cloned()
            .filter(QuickLaunchActionItem::is_activatable)?;
        let submenu = selected.submenu?;
        let level_id = uuid::Uuid::new_v4();
        self.levels.push(QuickLaunchActionLevel {
            id: level_id,
            title: selected.label,
            items: Vec::new(),
            filter: String::new(),
            selected: 0,
            shell_pending: true,
        });
        Some(PendingShellSubmenuLevel {
            level_id,
            session_id: submenu.session_id,
            submenu_id: submenu.submenu_id,
        })
    }

    pub(crate) fn pop_level(&mut self) -> Option<String> {
        if self.levels.len() <= 1 {
            return None;
        }
        self.levels.pop();
        Some(self.current_level().filter.clone())
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_actions();
        let selectable = visible
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.is_activatable().then_some(index))
            .collect::<Vec<_>>();
        if selectable.is_empty() {
            self.current_level_mut().selected = 0;
            return;
        }
        let current = selectable
            .iter()
            .position(|index| *index == self.selected())
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(selectable.len() as isize) as usize;
        self.current_level_mut().selected = selectable[next];
    }

    pub(crate) fn selected_action(&self) -> Option<QuickLaunchAction> {
        self.visible_actions()
            .get(self.selected())
            .filter(|item| item.enabled && item.submenu.is_none())
            .and_then(|item| item.action.clone())
    }
}

pub(crate) fn visible_action_items(level: &QuickLaunchActionLevel) -> Vec<QuickLaunchActionItem> {
    if level.shell_pending {
        return Vec::new();
    }
    let filter = level.filter.trim().to_lowercase();
    level
        .items
        .iter()
        .filter(|item| filter.is_empty() || item.label.to_lowercase().contains(&filter))
        .cloned()
        .collect()
}

pub(crate) fn first_activatable_index(items: &[QuickLaunchActionItem]) -> Option<usize> {
    items.iter().position(QuickLaunchActionItem::is_activatable)
}

pub(crate) fn quick_launch_action_available(
    result: &SearchResult,
    action: &QuickLaunchAction,
) -> bool {
    if is_packaged_application(result) && !matches!(action, QuickLaunchAction::Open) {
        return false;
    }
    if (result.entry.id.starts_with("extension:")
        || result.entry.id.starts_with("extension-script:"))
        && matches!(
            action,
            QuickLaunchAction::RunElevated | QuickLaunchAction::Shell { .. }
        )
    {
        return false;
    }
    match action {
        QuickLaunchAction::Open => result.opens_target(),
        QuickLaunchAction::SwitchDirectory {
            host,
            window_handle,
            target_directory,
            host_context_id,
        } => {
            *window_handle != 0
                && is_navigable_directory_target(target_directory)
                && (*host != DirectoryNavigationHost::ThirdPartyFileManager
                    || host_context_id.is_some())
        }
        QuickLaunchAction::Reveal => {
            result.opens_target() && !matches!(result.entry.source, ApplicationSource::System)
        }
        QuickLaunchAction::CopyPath => {
            result.opens_target() && !result.entry.target.trim().is_empty()
        }
        QuickLaunchAction::CopyResult => result.copy_text().is_some_and(|text| !text.is_empty()),
        QuickLaunchAction::CopyCommand => {
            result.entry.source == ApplicationSource::CustomCommand
                && result
                    .subtitle
                    .as_ref()
                    .is_some_and(|text| !text.is_empty())
        }
        QuickLaunchAction::RunElevated => {
            result.opens_target()
                && !is_settings_target(result)
                && (result.entry.source == ApplicationSource::System
                    || PathBuf::from(&result.entry.target)
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| {
                            extension.eq_ignore_ascii_case("exe")
                                || extension.eq_ignore_ascii_case("lnk")
                        }))
        }
        QuickLaunchAction::Shell { .. } => true,
    }
}

pub(crate) fn quick_launch_action_for_enter(secondary: bool, shift: bool) -> QuickLaunchAction {
    match (secondary, shift) {
        (true, true) => QuickLaunchAction::RunElevated,
        (true, false) => QuickLaunchAction::Reveal,
        _ => QuickLaunchAction::Open,
    }
}

pub(crate) fn quick_launch_primary_action(
    result: &SearchResult,
    secondary: bool,
    shift: bool,
) -> QuickLaunchAction {
    if result.prepared_command().is_some() {
        QuickLaunchAction::Open
    } else if result.copy_text().is_some() {
        QuickLaunchAction::CopyResult
    } else {
        quick_launch_action_for_enter(secondary, shift)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QuickLaunchResultCapabilities {
    pub(crate) preview: bool,
    pub(crate) shell_actions: bool,
    pub(crate) shell_icon: bool,
}

pub(crate) fn quick_launch_result_capabilities(
    result: &SearchResult,
) -> QuickLaunchResultCapabilities {
    let uses_native_file_integration =
        result.opens_target() && !is_settings_target(result) && !is_packaged_application(result);
    QuickLaunchResultCapabilities {
        preview: uses_native_file_integration,
        shell_actions: uses_native_file_integration
            && !result.entry.id.starts_with("extension:")
            && !result.entry.id.starts_with("extension-script:"),
        shell_icon: uses_native_file_integration
            || (result.opens_target() && is_packaged_application(result))
            || result.entry.source == ApplicationSource::CustomCommand,
    }
}

fn is_packaged_application(result: &SearchResult) -> bool {
    nxtools_finder::is_packaged_application_target(&result.entry.target)
}

fn is_settings_target(result: &SearchResult) -> bool {
    result.entry.source == ApplicationSource::System && result.entry.target == "ms-settings:"
}

pub(crate) fn shell_action_for(
    source: ApplicationSource,
    action: &QuickLaunchAction,
) -> Option<ShellOpenAction> {
    if source == ApplicationSource::Calculator {
        return None;
    }
    if source == ApplicationSource::CustomCommand {
        return None;
    }
    match action {
        QuickLaunchAction::Open if source == ApplicationSource::System => {
            Some(ShellOpenAction::SystemTool)
        }
        QuickLaunchAction::Open => Some(ShellOpenAction::Open),
        QuickLaunchAction::SwitchDirectory { .. } => None,
        QuickLaunchAction::Reveal if source == ApplicationSource::System => None,
        QuickLaunchAction::Reveal => Some(ShellOpenAction::SelectInExplorer),
        QuickLaunchAction::CopyPath
        | QuickLaunchAction::CopyResult
        | QuickLaunchAction::CopyCommand => None,
        QuickLaunchAction::RunElevated => Some(ShellOpenAction::RunElevated),
        QuickLaunchAction::Shell { .. } => None,
    }
}

pub(crate) fn platform_command_for_action(
    result: &SearchResult,
    action: &QuickLaunchAction,
    owner_window_handle: i64,
) -> Option<PlatformCommand> {
    if is_packaged_application(result) {
        return (matches!(action, QuickLaunchAction::Open) && result.opens_target()).then(|| {
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: result.entry.target.clone(),
                action: ShellOpenAction::Open,
            })
        });
    }
    if result.entry.id.starts_with("extension:") || result.entry.id.starts_with("extension-script:")
    {
        if matches!(
            action,
            QuickLaunchAction::RunElevated | QuickLaunchAction::Shell { .. }
        ) {
            return None;
        }
        if matches!(action, QuickLaunchAction::Open) && result.opens_target() {
            return Some(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: result.entry.target.clone(),
                action: ShellOpenAction::Open,
            }));
        }
    }
    match action {
        QuickLaunchAction::Open => result
            .prepared_command()
            .map(|prepared| {
                PlatformCommand::LaunchProcess(LaunchProcessCommand {
                    executable: prepared.executable.clone(),
                    arguments: prepared.arguments.clone(),
                    working_directory: prepared.working_directory.clone(),
                    run_as_admin: prepared.run_as_admin,
                    run_silently: prepared.run_silently,
                })
            })
            .or_else(|| {
                shell_action_for(result.entry.source, action).map(|shell_action| {
                    PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                        target: result.entry.target.clone(),
                        action: shell_action,
                    })
                })
            }),
        QuickLaunchAction::CopyPath => Some(PlatformCommand::PasteText(PasteTextCommand {
            text: result.entry.target.clone(),
            paste_after_write: false,
            target_window_handle: 0,
            target_focus_window_handle: 0,
        })),
        QuickLaunchAction::CopyResult => Some(PlatformCommand::PasteText(PasteTextCommand {
            text: result.copy_text()?.to_owned(),
            paste_after_write: false,
            target_window_handle: 0,
            target_focus_window_handle: 0,
        })),
        QuickLaunchAction::CopyCommand => Some(PlatformCommand::PasteText(PasteTextCommand {
            text: result.subtitle.clone()?,
            paste_after_write: false,
            target_window_handle: 0,
            target_focus_window_handle: 0,
        })),
        QuickLaunchAction::SwitchDirectory {
            host,
            window_handle,
            target_directory,
            host_context_id,
        } => Some(PlatformCommand::NavigateDirectory(
            NavigateDirectoryCommand {
                host: *host,
                window_handle: *window_handle,
                target_directory: target_directory.clone(),
                host_context_id: host_context_id.clone(),
            },
        )),
        QuickLaunchAction::Shell {
            session_id,
            command_id,
        } => Some(PlatformCommand::InvokeShellAction(
            InvokeShellActionCommand {
                session_id: session_id.clone(),
                command_id: *command_id,
                owner_window_handle,
            },
        )),
        _ => shell_action_for(result.entry.source, action).map(|shell_action| {
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: result.entry.target.clone(),
                action: shell_action,
            })
        }),
    }
}

pub(crate) fn selection_after_results_refresh(
    current_results: &[SearchResult],
    current_index: usize,
    refreshed_results: &[SearchResult],
) -> usize {
    current_results
        .get(current_index)
        .map(|result| result.entry.id.as_str())
        .and_then(|selected_id| {
            refreshed_results
                .iter()
                .position(|result| result.entry.id == selected_id)
        })
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QuickLaunchWindowActivationStep {
    PreTopmost(u8),
    Activate(u8),
    ConfirmTopmost(u8),
}

#[derive(Clone, Copy)]
pub(crate) struct PendingWindowActivation {
    pub(crate) window_handle: i64,
    pub(crate) step: QuickLaunchWindowActivationStep,
}

pub(crate) fn next_window_activation_step(
    step: QuickLaunchWindowActivationStep,
    success: bool,
) -> Option<QuickLaunchWindowActivationStep> {
    use QuickLaunchWindowActivationStep::*;

    match step {
        PreTopmost(attempt) if !success && attempt < 1 => Some(PreTopmost(attempt + 1)),
        PreTopmost(_) => Some(Activate(0)),
        Activate(attempt) if !success && attempt < 1 => Some(Activate(attempt + 1)),
        Activate(_) if success => Some(ConfirmTopmost(0)),
        Activate(_) => None,
        ConfirmTopmost(_) if success => None,
        ConfirmTopmost(attempt) if attempt < 2 => Some(ConfirmTopmost(attempt + 1)),
        ConfirmTopmost(_) => None,
    }
}

pub(crate) fn should_focus_search_after_activation(
    step: QuickLaunchWindowActivationStep,
    success: bool,
) -> bool {
    matches!(step, QuickLaunchWindowActivationStep::Activate(_)) && success
}

pub(crate) fn schedule_application_update<F>(
    application: WeakEntity<QuickLaunchApplication>,
    update: F,
    cx: &mut App,
) where
    F: FnOnce(&mut QuickLaunchApplication, &mut Context<QuickLaunchApplication>) + 'static,
{
    cx.defer(move |cx| {
        let _ = application.update(cx, update);
    });
}
