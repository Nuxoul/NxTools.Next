pub mod archive_browser;
pub mod archive_clipboard;
pub mod archive_create_destination;
pub mod archive_job;
pub mod archive_password;
pub mod archive_picker;
pub mod archive_read;
pub mod archive_recovery;
pub mod archive_shell_integration;
pub mod archive_space;
pub mod archive_virtual_drag;
pub mod desktop_archive_intent;
pub mod desktop_markdown_intent;
pub mod floating_assistant;
pub mod installed_applications;
pub mod main_window;
pub mod process_window_pick;
pub mod raw_numbered_join;
pub mod system_proxy;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const DEFAULT_PLATFORM_PIPE_NAME: &str = "nxtools-next-platform-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub protocol_version: u16,
    pub id: Uuid,
    pub correlation_id: Option<Uuid>,
    #[serde(flatten)]
    pub message: Message,
}

impl Envelope {
    pub fn new(message: Message) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id: Uuid::new_v4(),
            correlation_id: None,
            message,
        }
    }

    pub fn reply_to(request: &Self, message: Message) -> Self {
        Self {
            correlation_id: Some(request.id),
            ..Self::new(message)
        }
    }

    pub fn encode_line(&self) -> Result<String, ProtocolError> {
        let mut encoded = serde_json::to_string(self)?;
        if encoded.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(encoded.len()));
        }
        encoded.push('\n');
        Ok(encoded)
    }

    pub fn decode_line(line: &str) -> Result<Self, ProtocolError> {
        if line.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(line.len()));
        }
        let json = line.trim_end_matches(['\r', '\n']);
        let header: VersionHeader = serde_json::from_str(json)?;
        if header.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion {
                expected: PROTOCOL_VERSION,
                actual: header.protocol_version,
            });
        }
        Ok(serde_json::from_str(json)?)
    }
}

#[derive(Deserialize)]
struct VersionHeader {
    protocol_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum Message {
    Hello(Hello),
    HelloAck(HelloAck),
    Ping(Ping),
    Pong(Pong),
    PlatformEvent(PlatformEvent),
    PlatformCommand(PlatformCommand),
    PlatformCommandResult(PlatformCommandResult),
    IndexerCommand(IndexerCommand),
    IndexerEvent(IndexerEvent),
    DesktopArchiveIntent(desktop_archive_intent::DesktopArchiveIntent),
    DesktopArchiveReceipt(desktop_archive_intent::DesktopArchiveReceipt),
    DesktopMarkdownIntent(desktop_markdown_intent::DesktopMarkdownIntent),
    DesktopMarkdownReceipt(desktop_markdown_intent::DesktopMarkdownReceipt),
    Error(ProtocolFault),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub client_name: String,
    pub client_version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAck {
    pub server_name: String,
    pub server_version: String,
    pub accepted_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    pub sent_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub sent_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum PlatformEvent {
    ArchiveRead(archive_read::ArchiveReadEvent),
    ArchivePassword(archive_password::ArchivePasswordPrompt),
    ArchiveJob(Box<archive_job::ArchiveJobEvent>),
    ArchiveVirtualDragProgress(Box<archive_virtual_drag::ArchiveVirtualDragProgress>),
    ArchivePicker(archive_picker::ArchivePickerResult),
    FloatingAssistant(floating_assistant::FloatingAssistantEvent),
    ProcessWindowPickChanged(process_window_pick::ProcessWindowPickChanged),
    TrayActionRequested(TrayActionRequest),
    SystemMotionPreferenceChanged(SystemMotionPreferenceChanged),
    SystemThemePreferenceChanged(SystemThemePreferenceChanged),
    ForegroundChanged(ForegroundContext),
    GlobalPointerPressed(PointerPressContext),
    ClipboardCaptured(ClipboardCapture),
    ClipboardPanelRequested(ClipboardPanelRequest),
    QuickLaunchPanelRequested(QuickLaunchPanelRequest),
    QuickLaunchKeyboardHealthChanged(QuickLaunchKeyboardHealthChanged),
    QuickLaunchDirectoryContextResolved(QuickLaunchDirectoryContextResolution),
    QuickLaunchAutoSwitchRequested(QuickLaunchAutoSwitchRequest),
    QuickLaunchFileDialogSearchHostChanged(QuickLaunchFileDialogSearchHost),
    TaskbarOverlayEditGestureFinished(TaskbarOverlayEditGesture),
    ElevatedDirectoryHelperRequested(ElevatedDirectoryHelperRequest),
}

/// Native Windows client-area animation preference; nonzero native error means fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemMotionPreferenceChanged {
    pub correlation_id: Uuid,
    pub reduced_motion: bool,
    pub native_error_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemThemePreferenceChanged {
    pub correlation_id: Uuid,
    pub dark: bool,
    pub native_error_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayActionRequest {
    pub correlation_id: uuid::Uuid,
    pub action: TrayAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayAction {
    Open,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskbarOverlayEditGesture {
    pub window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundContext {
    pub window_handle: i64,
    pub focus_window_handle: i64,
    pub process_id: u32,
    pub process_name: String,
    pub window_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerPressContext {
    pub window_handle: i64,
    pub root_window_handle: i64,
    pub process_id: u32,
    #[serde(default)]
    pub sampled_at_unix_ms: i64,
    #[serde(default)]
    pub event_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardCapture {
    pub text: Option<String>,
    pub image: Option<ClipboardImageData>,
    pub paths: Vec<String>,
    #[serde(default)]
    pub native_formats: Vec<ClipboardNativeFormat>,
    pub source_icon_staging_path: Option<String>,
    pub source: ForegroundContext,
    pub captured_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardNativeFormat {
    pub format_id: u32,
    pub format_name: String,
    pub data_path: String,
    pub size_bytes: u64,
    pub sort_order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardImageData {
    pub staging_path: String,
    pub width: u32,
    pub height: u32,
}

/// One cursor-monitor observation; work-area coordinates are absolute device pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorMonitorSnapshot {
    pub display_id: u64,
    pub work_left: i32,
    pub work_top: i32,
    pub work_width: u32,
    pub work_height: u32,
    pub dpi: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardPanelRequest {
    pub target: ForegroundContext,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub cursor_display_index: u32,
    pub cursor_monitor_dpi: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_monitor: Option<CursorMonitorSnapshot>,
    pub requested_at_unix_ms: i64,
    #[serde(default)]
    pub event_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickLaunchPanelRequest {
    pub target: ForegroundContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_navigation_context: Option<DirectoryNavigationContext>,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub cursor_display_index: u32,
    pub cursor_monitor_dpi: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_monitor: Option<CursorMonitorSnapshot>,
    pub requested_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<uuid::Uuid>,
}

/// Observable hook installation and owned-thread lifecycle; not silent hook removal detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickLaunchKeyboardHealthChanged {
    pub correlation_id: Uuid,
    pub state: QuickLaunchKeyboardHealthState,
    pub native_error_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuickLaunchKeyboardHealthState {
    InstallFailed,
    ThreadExited,
    Reinstalled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickLaunchDirectoryContextResolution {
    pub target_window_handle: i64,
    pub requested_at_unix_ms: i64,
    pub directory_navigation_context: DirectoryNavigationContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickLaunchAutoSwitchRequest {
    pub target: DirectoryNavigationContext,
    pub source_directory: String,
    pub requested_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickLaunchFileDialogSearchHost {
    pub window_handle: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_directory: Option<String>,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub monitor_dpi: u32,
    pub visible: bool,
    pub changed_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quick_switch_source: Option<QuickLaunchDirectorySource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickLaunchDirectorySource {
    pub context: DirectoryNavigationContext,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevatedDirectoryHelperRequest {
    pub pipe_name: String,
    pub authentication_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryNavigationContext {
    pub host: DirectoryNavigationHost,
    pub window_handle: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_context_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryNavigationHost {
    Explorer,
    Desktop,
    FileDialog,
    ThirdPartyFileManager,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum IndexerCommand {
    Search(IndexSearchRequest),
    CancelSearch(IndexSearchCancel),
    UpdateSettings(IndexSettings),
    RefreshVolumes,
    RebuildIndex,
    EnsureElevatedDirectoryHelper(ElevatedDirectoryHelperRequest),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSettings {
    pub indexed_volumes: Vec<String>,
    pub excluded_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum IndexerEvent {
    SearchBatch(IndexSearchBatch),
    SearchFailed(IndexSearchFailure),
    Status(IndexStatus),
    ElevatedDirectoryHelper(ElevatedDirectoryHelperStatus),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSearchFailure {
    pub generation: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevatedDirectoryHelperStatus {
    pub pipe_name: String,
    pub launched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexPhase {
    DiscoveringVolumes,
    VolumesReady,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexStatus {
    pub phase: IndexPhase,
    pub indexed_items: u64,
    pub drives: Vec<IndexDriveStatus>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDriveStatus {
    pub mount_point: String,
    pub file_system: String,
    pub indexed_items: u64,
    pub journal_id: Option<u64>,
    pub next_usn: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<IndexDriveMaintenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum IndexDriveMaintenance {
    InitialBuilding,
    Rebuilding { reason: IndexRebuildReason },
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexRebuildReason {
    JournalWrapped,
    BulkChanges,
    CursorInvalid,
    Requested,
}

impl IndexRebuildReason {
    pub fn code(self) -> &'static str {
        match self {
            Self::JournalWrapped => "journal_wrapped",
            Self::BulkChanges => "bulk_changes",
            Self::CursorInvalid => "cursor_invalid",
            Self::Requested => "requested",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSearchCancel {
    pub request_id: Uuid,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSearchRequest {
    pub generation: u64,
    pub query: String,
    pub limit: usize,
    pub context: IndexSearchContext,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSearchContext {
    pub directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_directory: Option<String>,
    pub include_files: bool,
    pub include_directories: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub retain_kinds_apart: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_homophone_fallback: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recent_modified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSearchBatch {
    pub generation: u64,
    pub items: Vec<IndexSearchItem>,
    pub complete: bool,
    pub elapsed_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSearchItem {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at_unix_ms: Option<i64>,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum PlatformCommand {
    ArchiveRead(archive_read::ArchiveReadCommand),
    ArchiveEntry(archive_browser::ArchiveEntryCommand),
    ArchiveVirtualDrag(Box<archive_virtual_drag::ArchiveVirtualDragCommand>),
    ArchiveVirtualDragSpike(Box<archive_virtual_drag::ArchiveVirtualDragCommand>),
    ArchiveBrowserRead(archive_read::ArchiveReadCommand),
    ArchivePasswordReply(archive_password::ArchivePasswordReply),
    ArchiveJob(Box<archive_job::ArchiveJobCommand>),
    RawNumberedJoin(raw_numbered_join::RawNumberedJoinCommand),
    ArchivePicker(archive_picker::ArchivePickerCommand),
    ReadClipboardFileDrop(archive_clipboard::ArchiveClipboardCommand),
    ArchiveCreateDestination(archive_create_destination::ArchiveCreateDestinationCommand),
    ArchiveSpace(archive_space::ArchiveSpaceCommand),
    ArchiveRecoveryInspect(archive_recovery::ArchiveRecoveryInspectCommand),
    PrepareMainWindow(main_window::PrepareMainWindowCommand),
    ConfigureDesktopShell(ConfigureDesktopShellCommand),
    ReadClipboardText,
    QueryCursorMonitor,
    QueryInstalledApplications,
    ProcessWindowPick(process_window_pick::ProcessWindowPickCommand),
    PasteText(PasteTextCommand),
    PasteFiles(PasteFilesCommand),
    PasteImage(PasteImageCommand),
    PasteNativeFormats(PasteNativeFormatsCommand),
    ConfigureClipboard(ConfigureClipboardCommand),
    SetClipboardCapturePaused(SetClipboardCapturePausedCommand),
    ConfigureQuickLaunch(ConfigureQuickLaunchCommand),
    SetClipboardHotKeyRecording(SetClipboardHotKeyRecordingCommand),
    SetQuickLaunchHotKeyRecording(SetQuickLaunchHotKeyRecordingCommand),
    SetWindowTopmost(SetWindowTopmostCommand),
    SetWindowShadow(SetWindowShadowCommand),
    SetWindowNoActivate(SetWindowNoActivateCommand),
    SetWindowBounds(SetWindowBoundsCommand),
    SetWindowBoundsCentered(SetWindowBoundsCenteredCommand),
    PresentFixedWindowOverAnchor(PresentFixedWindowOverAnchorCommand),
    PrepareNotificationWindow(PrepareNotificationWindowCommand),
    FloatingAssistant(floating_assistant::FloatingAssistantCommand),
    StashWindowBounds(StashWindowBoundsCommand),
    MoveWindowCenteredOverKey(MoveWindowCenteredOverKeyCommand),
    SetTaskbarProgress(SetTaskbarProgressCommand),
    PlaceTaskbarProgressOverlay(PlaceTaskbarProgressOverlayCommand),
    SetTaskbarProgressOverlayVisibility(SetTaskbarProgressOverlayVisibilityCommand),
    PlaceFileDialogSearchWindow(PlaceFileDialogSearchWindowCommand),
    SetWindowOwner(SetWindowOwnerCommand),
    SetWindowMinimized(SetWindowMinimizedCommand),
    ToggleWindowMaximized(ToggleWindowMaximizedCommand),
    SetWindowVisibility(SetWindowVisibilityCommand),
    SetWindowPosition(SetWindowPositionCommand),
    QueryWindowGeometry(QueryWindowGeometryCommand),
    ActivateWindow(ActivateWindowCommand),
    BeginWindowResize(BeginWindowResizeCommand),
    BeginWindowMove(BeginWindowMoveCommand),
    BeginFileDrag(BeginFileDragCommand),
    ShowContextMenu(ShowContextMenuCommand),
    CancelContextMenu(CancelContextMenuCommand),
    OpenShellTarget(OpenShellTargetCommand),
    PlaySystemNotificationSound,
    ReadSystemProxy,
    OpenDefaultAppsSettings(OpenDefaultAppsSettingsCommand),
    ShutdownSystem(ShutdownSystemCommand),
    SuspendSystem(SuspendSystemCommand),
    ExportBrowserExtension(ExportBrowserExtensionCommand),
    PickBrowserExtensionExportPath(PickBrowserExtensionExportPathCommand),
    LaunchProcess(LaunchProcessCommand),
    NavigateDirectory(NavigateDirectoryCommand),
    OpenFileInDialog(OpenFileInDialogCommand),
    QueryShellActions(QueryShellActionsCommand),
    QueryShellSubmenu(QueryShellSubmenuCommand),
    InvokeShellAction(InvokeShellActionCommand),
    ExtractShellIcon(ExtractShellIconCommand),
    PickClipboardArchivePath(PickClipboardArchivePathCommand),
    PickArchivePath(PickArchivePathCommand),
    PickClipboardStorageDirectory(PickClipboardStorageDirectoryCommand),
    PickDirectory(PickDirectoryCommand),
    PickProgram(PickProgramCommand),
    PickMarkdownDocument(PickMarkdownDocumentCommand),
    PickMarkdownExport(PickMarkdownExportCommand),
    PrintMarkdownPdf(PrintMarkdownPdfCommand),
    MarkdownAssociation(MarkdownAssociationCommand),
    PickExtensionManifest(PickExtensionManifestCommand),
    PickExtensionRuntime(PickExtensionRuntimeCommand),
    PickExtensionSettingPath(PickExtensionSettingPathCommand),
    InspectExtensionRuntime(InspectExtensionRuntimeCommand),
    EnsureIndexerService,
    EnsureBrowserNativeMessaging(EnsureBrowserNativeMessagingCommand),
    ArchiveShellIntegration(archive_shell_integration::ArchiveShellIntegrationCommand),
}

/// Fixed Windows default-apps page. There is deliberately no target/URI/path authority.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OpenDefaultAppsSettingsCommand {}
impl<'de> Deserialize<'de> for OpenDefaultAppsSettingsCommand {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EmptyObject;
        impl<'de> serde::de::Visitor<'de> for EmptyObject {
            type Value = OpenDefaultAppsSettingsCommand;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an empty object")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                if map.next_key::<String>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "default apps request accepts no fields",
                    ));
                }
                Ok(OpenDefaultAppsSettingsCommand {})
            }
        }
        deserializer.deserialize_map(EmptyObject)
    }
}

/// Source-client device pixels; Windows adapter owns screen conversion and popup placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowContextMenuCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub x: i32,
    pub y: i32,
    /// Nonzero: prepare an independent GPUI surface only; never create an HMENU.
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub popup_window_handle: i64,
    pub items: Vec<ContextMenuItem>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextMenuItem {
    pub id: u32,
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
    pub separator: bool,
    pub children: Vec<ContextMenuItem>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelContextMenuCommand {
    pub session_id: Uuid,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextMenuResult {
    pub selected_id: Option<u32>,
    pub dismissed_outside: bool,
}

fn is_zero_i64(value: &i64) -> bool {
    *value == 0
}

/// Actual native work-area client viewport and anchor, all in device pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyledMenuPlacement {
    pub width: i32,
    pub height: i32,
    pub anchor_x: i32,
    pub anchor_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigureDesktopShellCommand {
    pub start_on_login_silently: bool,
    pub tray_icon_visible: bool,
}

/// Runtime capture gate, independent from hotkey registration and persistent enabled state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetClipboardCapturePausedCommand {
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigureClipboardCommand {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub hotkey_modifiers: Vec<String>,
    pub hotkey_key: String,
    pub hotkey_display: String,
    pub excluded_processes: Vec<String>,
    #[serde(default = "default_true")]
    pub suppress_in_fullscreen: bool,
    #[serde(default = "default_clipboard_max_text_bytes")]
    pub max_text_bytes: u64,
    #[serde(default = "default_clipboard_max_image_bytes")]
    pub max_image_bytes: u64,
    #[serde(default = "default_clipboard_max_image_pixels")]
    pub max_image_pixels: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigureQuickLaunchCommand {
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub auto_quick_switch_enabled: bool,
    #[serde(default = "default_true")]
    pub file_dialog_search_enabled: bool,
    #[serde(default = "default_true")]
    pub suppress_in_exclusive_fullscreen: bool,
    pub hotkey_modifiers: Vec<String>,
    pub hotkey_key: String,
    pub hotkey_display: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnsureBrowserNativeMessagingCommand {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub chromium_extension_id: String,
    pub firefox_extension_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFileInDialogCommand {
    pub window_handle: i64,
    pub file_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickClipboardStorageDirectoryCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub current_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickDirectoryCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub current_directory: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickProgramCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub current_path: String,
}

/// Opens an owned Windows document chooser; selecting paths does not open or read files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickMarkdownDocumentCommand {
    pub source_window_handle: u64,
    pub source_process_id: u32,
}

/// JSON object encoded in a successful [`PlatformCommandResult::data`] string.
/// An empty `paths` vector means the user dismissed the chooser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickMarkdownDocumentResult {
    pub paths: Vec<String>,
}

impl PickMarkdownDocumentResult {
    pub const MAX_PATHS: usize = 32;
    pub const MAX_PATH_BYTES: usize = 128 * 1024;

    /// Validate transport budgets before applying the result to a reader.
    pub fn selection_valid(&self) -> bool {
        self.paths.len() <= Self::MAX_PATHS
            && self
                .paths
                .iter()
                .all(|path| !path.is_empty() && !path.contains('\0'))
            && self.paths.iter().map(String::len).sum::<usize>() <= Self::MAX_PATH_BYTES
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownExportFormat {
    Html,
    Pdf,
}

/// Owned native Save selection. Successful result data is a plain full path, or absent on Cancel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickMarkdownExportCommand {
    pub source_window_handle: u64,
    pub source_process_id: u32,
    pub format: MarkdownExportFormat,
    pub suggested_name: String,
}

/// Render local standalone HTML to a new Desktop staging path, not the selected destination.
/// Success has no data; Desktop owns atomic publication to the selected destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintMarkdownPdfCommand {
    pub html_path: String,
    pub output_path: String,
}

/// Query or modify only the current-user Markdown reader registration, never its default status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MarkdownAssociationCommand {
    pub action: MarkdownAssociationAction,
}

impl<'de> Deserialize<'de> for MarkdownAssociationCommand {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ActionObject;
        impl<'de> serde::de::Visitor<'de> for ActionObject {
            type Value = MarkdownAssociationCommand;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a Markdown association action object")
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                match map.next_key::<String>()?.as_deref() {
                    Some("action") => {}
                    Some(other) => return Err(serde::de::Error::unknown_field(other, &["action"])),
                    None => return Err(serde::de::Error::missing_field("action")),
                }
                let action = map.next_value()?;
                if map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::custom("only action is accepted"));
                }
                Ok(MarkdownAssociationCommand { action })
            }
        }
        deserializer.deserialize_map(ActionObject)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownAssociationAction {
    Query,
    Register,
    Unregister,
    /// Opens Windows Settings; successful result data is `opened`, not association state.
    OpenDefaults,
}

/// Observed Windows state encoded as JSON for successful query/register/unregister results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownAssociationState {
    pub md: MarkdownAssociationExtensionState,
    pub markdown: MarkdownAssociationExtensionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownAssociationExtensionState {
    pub registered: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickExtensionManifestCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub current_path: String,
}

/// Opens an EXE-only picker; selecting a path does not execute or trust it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickExtensionRuntimeCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub program: String,
    pub current_path: String,
}

/// Human-driven settings path selection; native owner validates authenticated source HWND.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickExtensionSettingPathCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub kind: String,
    pub current_path: String,
}

/// Explicit bounded version probe for python/node/powershell/pwsh (not package EXEs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectExtensionRuntimeCommand {
    pub program: String,
    pub executable: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetClipboardHotKeyRecordingCommand {
    pub recording: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetQuickLaunchHotKeyRecordingCommand {
    pub recording: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteNativeFormatsCommand {
    pub formats: Vec<ClipboardNativeFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_path_fallback: Option<String>,
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteTextCommand {
    pub text: String,
    #[serde(default = "default_true")]
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteFilesCommand {
    pub paths: Vec<String>,
    #[serde(default = "default_true")]
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteImageCommand {
    pub image_path: String,
    #[serde(default = "default_true")]
    pub include_file_drop_fallback: bool,
    #[serde(default = "default_true")]
    pub paste_after_write: bool,
    pub target_window_handle: i64,
    pub target_focus_window_handle: i64,
}

const fn default_clipboard_max_text_bytes() -> u64 {
    8 * 1024 * 1024
}

const fn default_clipboard_max_image_bytes() -> u64 {
    64 * 1024 * 1024
}

const fn default_clipboard_max_image_pixels() -> u64 {
    100_000_000
}

const fn default_true() -> bool {
    true
}
/// Prepare a hidden passive notification popup at one source-monitor work-area anchor.
/// Width/height are logical pixels; C# owns DPI conversion and final device-pixel readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepareNotificationWindowCommand {
    pub window_handle: i64,
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub position: String,
    pub width: i32,
    pub height: i32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowTopmostCommand {
    pub window_handle: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowNoActivateCommand {
    pub window_handle: i64,
    pub enabled: bool,
}

/// Toggles the DWM drop shadow on one window. The 链接识别 panel turns it
/// off: a floating hint card reads cleaner without the system shadow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowShadowCommand {
    pub window_handle: i64,
    pub enabled: bool,
}
/// Applies outer screen x/y with an explicit client/content width/height in
/// device pixels. C# compensates the live native frame and verifies final client
/// dimensions; window placement acceptance is not mouse-activation policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowBoundsCommand {
    pub window_handle: i64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryWindowGeometryCommand {
    pub window_handle: i64,
}

/// Absolute device-pixel rectangle. The outer rectangle includes native frame insets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeometryRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Readback from the requested HWND (including when hidden), carried in CommandResult.data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub outer: WindowGeometryRect,
    pub client: WindowGeometryRect,
    /// HMONITOR for the largest intersection; zero when entirely offscreen.
    pub display_id: u64,
    pub dpi: u32,
    pub visible: bool,
}

/// Resizes the window while keeping its real current visual center. GPUI's
/// cached window origin never follows a user drag, so the centered geometry
/// must be computed from the native rectangle on the C# side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowBoundsCenteredCommand {
    pub window_handle: i64,
    pub width: i32,
    pub height: i32,
}

/// Captures a window's real native rectangle under a key before it is
/// destroyed, so a replacement window can inherit that position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashWindowBoundsCommand {
    pub window_handle: i64,
    pub key: String,
}

/// Places the window centered over the stashed anchor rectangle (consumed on
/// use). A missing stash entry resizes in place instead of teleporting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveWindowCenteredOverKeyCommand {
    pub window_handle: i64,
    pub key: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTaskbarProgressCommand {
    pub window_handle: i64,
    pub state: TaskbarProgressState,
    pub completed: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceTaskbarProgressOverlayCommand {
    pub window_handle: i64,
    pub enabled: bool,
    pub visible: bool,
    pub anchor: TaskbarOverlayAnchor,
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub interactive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTaskbarProgressOverlayVisibilityCommand {
    pub window_handle: i64,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarOverlayAnchor {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskbarProgressState {
    None,
    Indeterminate,
    Normal,
    Error,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceFileDialogSearchWindowCommand {
    pub window_handle: i64,
    pub owner_window_handle: i64,
    pub visible_width: i32,
    pub visible_height: i32,
    pub bottom_overlap: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowOwnerCommand {
    pub window_handle: i64,
    pub owner_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowMinimizedCommand {
    pub window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleWindowMaximizedCommand {
    pub window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowVisibilityCommand {
    pub window_handle: i64,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWindowPositionCommand {
    pub window_handle: i64,
    pub x: i32,
    pub y: i32,
}

/// One-shot hidden-window presentation. Dimensions are final device-pixel client sizes.
/// Apply bounds centered on the anchor before nonactivating show and native readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentFixedWindowOverAnchorCommand {
    pub window_handle: i64,
    pub anchor_window_handle: i64,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivateWindowCommand {
    pub window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginWindowResizeCommand {
    pub window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginWindowMoveCommand {
    pub window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginFileDragCommand {
    pub path: String,
    pub source_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenShellTargetCommand {
    pub target: String,
    pub action: ShellOpenAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownSystemCommand {
    pub delay_seconds: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspendSystemCommand {
    pub hibernate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportBrowserExtensionCommand {
    /// "chrome" packages a signed CRX3; "firefox" packages an XPI.
    pub format: String,
    /// Absolute output file path, selected by a separate human-modal command.
    /// Machine-only export rejects empty destinations.
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickBrowserExtensionExportPathCommand {
    pub format: String,
    pub source_window_handle: i64,
    pub source_process_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchProcessCommand {
    pub executable: String,
    pub arguments: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub run_as_admin: bool,
    #[serde(default)]
    pub run_silently: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigateDirectoryCommand {
    pub host: DirectoryNavigationHost,
    pub window_handle: i64,
    pub target_directory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_context_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryShellActionsCommand {
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryShellSubmenuCommand {
    pub session_id: String,
    pub submenu_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokeShellActionCommand {
    pub session_id: String,
    pub command_id: u32,
    pub owner_window_handle: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellActionMenu {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation_directory: Option<String>,
    pub items: Vec<ShellActionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellActionItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<u32>,
    pub title: String,
    pub enabled: bool,
    pub separator: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submenu_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractShellIconCommand {
    pub target: String,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickClipboardArchivePathCommand {
    pub source_window_handle: i64,
    pub source_process_id: u32,
    pub mode: ClipboardArchivePickerMode,
    pub suggested_file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickArchivePathCommand {
    pub source_hwnd: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArchivePickerResult {
    Selected { path: String },
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardArchivePickerMode {
    Export,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellOpenAction {
    Open,
    OpenInExplorer,
    SelectInExplorer,
    OpenWith,
    RunElevated,
    RegistryKey,
    SystemTool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformCommandResult {
    pub success: bool,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_error_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolFault {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("frame is {0} bytes, exceeding the protocol limit")]
    FrameTooLarge(usize),
    #[error("unsupported protocol version {actual}; expected {expected}")]
    UnsupportedVersion { expected: u16, actual: u16 },
    #[error("invalid JSON frame: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_text_read_is_an_additive_no_payload_command() {
        let value =
            serde_json::to_value(PlatformCommand::ReadClipboardText).expect("encode text read");
        assert_eq!(value, serde_json::json!({"command": "read_clipboard_text"}));
        assert_eq!(
            serde_json::from_value::<PlatformCommand>(value).expect("decode text read"),
            PlatformCommand::ReadClipboardText
        );
    }

    fn fixture_context() -> ForegroundContext {
        ForegroundContext {
            window_handle: 42,
            focus_window_handle: 43,
            process_id: 7,
            process_name: "fixture".into(),
            window_title: "fixture window".into(),
        }
    }

    #[test]
    fn hello_round_trips_with_correlation() {
        let request = Envelope::new(Message::Hello(Hello {
            client_name: "platform_host".into(),
            client_version: "0.1.0".into(),
            capabilities: vec!["foreground_context".into()],
        }));
        let reply = Envelope::reply_to(
            &request,
            Message::HelloAck(HelloAck {
                server_name: "desktop".into(),
                server_version: "0.1.0".into(),
                accepted_capabilities: vec!["foreground_context".into()],
            }),
        );

        let decoded = Envelope::decode_line(&reply.encode_line().expect("encode")).expect("decode");
        assert_eq!(decoded, reply);
        assert_eq!(decoded.correlation_id, Some(request.id));
    }

    #[test]
    fn indexer_service_install_command_has_no_privileged_payload() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::EnsureIndexerService,
        ));
        let encoded = envelope.encode_line().expect("encode service command");

        assert!(encoded.contains("\"command\":\"ensure_indexer_service\""));
        assert!(!encoded.contains("indexer_path"));
        assert!(!encoded.contains("mft"));
    }

    #[test]
    fn browser_native_messaging_registration_preserves_exact_extension_ids() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::EnsureBrowserNativeMessaging(EnsureBrowserNativeMessagingCommand {
                enabled: true,
                chromium_extension_id: "haohhnpjfggpdadhcbjakjpkmkljkpif".into(),
                firefox_extension_id: "nxtools-download-interceptor@nxtools.local".into(),
            }),
        ));
        let encoded = envelope.encode_line().expect("encode browser command");

        assert!(encoded.contains("\"command\":\"ensure_browser_native_messaging\""));
        assert!(encoded.contains("haohhnpjfggpdadhcbjakjpkmkljkpif"));
        assert!(encoded.contains("nxtools-download-interceptor@nxtools.local"));
    }

    #[test]
    fn index_settings_round_trip_volume_scope_and_exclusions() {
        let envelope = Envelope::new(Message::IndexerCommand(IndexerCommand::UpdateSettings(
            IndexSettings {
                indexed_volumes: vec![r"D:\".into()],
                excluded_paths: vec![r"D:\Build".into()],
            },
        )));

        let restored = Envelope::decode_line(&envelope.encode_line().expect("encode settings"))
            .expect("decode settings");

        assert_eq!(restored, envelope);
    }

    #[test]
    fn incompatible_versions_are_rejected_before_dispatch() {
        let line = format!(
            r#"{{"protocol_version":99,"id":"{}","correlation_id":null,"type":"ping","payload":{{"sent_at_unix_ms":0}}}}"#,
            Uuid::new_v4()
        );

        assert!(matches!(
            Envelope::decode_line(&line),
            Err(ProtocolError::UnsupportedVersion {
                expected: PROTOCOL_VERSION,
                actual: 99
            })
        ));
    }

    #[test]
    fn incompatible_version_wins_over_unknown_message_payload() {
        let line = format!(
            r#"{{"protocol_version":99,"id":"{}","correlation_id":null,"type":"future_message","payload":{{"shape":"unknown"}}}}"#,
            Uuid::new_v4()
        );

        assert!(matches!(
            Envelope::decode_line(&line),
            Err(ProtocolError::UnsupportedVersion { actual: 99, .. })
        ));
    }

    #[test]
    fn oversized_frames_are_rejected() {
        let line = "x".repeat(MAX_FRAME_BYTES + 1);
        assert!(matches!(
            Envelope::decode_line(&line),
            Err(ProtocolError::FrameTooLarge(_))
        ));
    }

    #[test]
    fn clipboard_text_capture_round_trips_as_platform_event() {
        let envelope = Envelope::new(Message::PlatformEvent(PlatformEvent::ClipboardCaptured(
            ClipboardCapture {
                text: Some("真实剪贴板内容".into()),
                image: None,
                paths: Vec::new(),
                native_formats: Vec::new(),
                source_icon_staging_path: None,
                source: ForegroundContext {
                    window_handle: 42,
                    focus_window_handle: 43,
                    process_id: 7,
                    process_name: "notepad".into(),
                    window_title: "notes.txt".into(),
                },
                captured_at_unix_ms: 1_700_000_000_000,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("clipboard event must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn taskbar_overlay_edit_finish_round_trips_as_platform_event() {
        let envelope = Envelope::new(Message::PlatformEvent(
            PlatformEvent::TaskbarOverlayEditGestureFinished(TaskbarOverlayEditGesture {
                window_handle: 42,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("taskbar edit event must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn paste_text_command_round_trips_with_target_window() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::PasteText(
            PasteTextCommand {
                text: "paste me".into(),
                paste_after_write: false,
                target_window_handle: 99,
                target_focus_window_handle: 100,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("paste command must decode");
        assert_eq!(decoded, envelope);
        let Message::PlatformCommand(PlatformCommand::PasteText(command)) = decoded.message else {
            panic!("expected text clipboard command");
        };
        assert!(!command.paste_after_write);
    }

    #[test]
    fn image_and_files_round_trip_as_capture_and_paste_messages() {
        for message in [
            Message::PlatformEvent(PlatformEvent::ClipboardCaptured(ClipboardCapture {
                text: Some("alternate representation".into()),
                image: Some(ClipboardImageData {
                    staging_path: "C:/temp/image.png".into(),
                    width: 640,
                    height: 480,
                }),
                paths: vec!["C:/one.txt".into(), "C:/two.txt".into()],
                native_formats: Vec::new(),
                source_icon_staging_path: Some("C:/temp/source.png".into()),
                source: fixture_context(),
                captured_at_unix_ms: 10,
            })),
            Message::PlatformCommand(PlatformCommand::PasteFiles(PasteFilesCommand {
                paths: vec!["C:/one.txt".into()],
                paste_after_write: true,
                target_window_handle: 5,
                target_focus_window_handle: 6,
            })),
            Message::PlatformCommand(PlatformCommand::PasteImage(PasteImageCommand {
                image_path: "C:/image.png".into(),
                include_file_drop_fallback: true,
                paste_after_write: true,
                target_window_handle: 5,
                target_focus_window_handle: 6,
            })),
        ] {
            let envelope = Envelope::new(message);
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("multi-format message must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn native_clipboard_formats_round_trip_by_staged_file() {
        let format = ClipboardNativeFormat {
            format_id: 49_222,
            format_name: "HTML Format".into(),
            data_path: "C:/temp/html-format.bin".into(),
            size_bytes: 128,
            sort_order: 2,
        };
        let capture = Envelope::new(Message::PlatformEvent(PlatformEvent::ClipboardCaptured(
            ClipboardCapture {
                text: Some("formatted".into()),
                image: None,
                paths: Vec::new(),
                native_formats: vec![format.clone()],
                source_icon_staging_path: None,
                source: fixture_context(),
                captured_at_unix_ms: 12,
            },
        )));
        let command = Envelope::new(Message::PlatformCommand(
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                formats: vec![format],
                image_path_fallback: Some("C:/images/history.png".into()),
                paste_after_write: false,
                target_window_handle: 42,
                target_focus_window_handle: 43,
            }),
        ));

        for envelope in [capture, command] {
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("native clipboard payload must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn topmost_command_round_trips_with_native_window() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::SetWindowTopmost(
            SetWindowTopmostCommand {
                window_handle: 777,
                enabled: true,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("topmost command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn visibility_command_round_trips_with_native_window() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: 778,
                visible: false,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("visibility command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn no_activate_command_round_trips_with_native_window() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                window_handle: 779,
                enabled: true,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("no-activate command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn bounds_command_round_trips_with_native_window() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::SetWindowBounds(
            SetWindowBoundsCommand {
                window_handle: 780,
                x: 120,
                y: 240,
                width: 400,
                height: 410,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("window-bounds command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn taskbar_progress_round_trips_without_ui_state_leaking_into_the_adapter() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::SetTaskbarProgress(SetTaskbarProgressCommand {
                window_handle: 781,
                state: TaskbarProgressState::Normal,
                completed: 42,
                total: 100,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("taskbar progress command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn taskbar_overlay_placement_round_trips_as_a_windows_only_command() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::PlaceTaskbarProgressOverlay(PlaceTaskbarProgressOverlayCommand {
                window_handle: 782,
                enabled: true,
                visible: false,
                anchor: TaskbarOverlayAnchor::Right,
                offset_x: 12,
                offset_y: 0,
                width: 280,
                height: 3,
                interactive: true,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("taskbar overlay placement command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn taskbar_overlay_visibility_round_trips_as_a_state_transition() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::SetTaskbarProgressOverlayVisibility(
                SetTaskbarProgressOverlayVisibilityCommand {
                    window_handle: 782,
                    visible: true,
                },
            ),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("taskbar overlay visibility command must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn file_dialog_search_placement_round_trips_with_live_owner() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::PlaceFileDialogSearchWindow(PlaceFileDialogSearchWindowCommand {
                window_handle: 501,
                owner_window_handle: 77,
                visible_width: 500,
                visible_height: 72,
                bottom_overlap: 0,
            }),
        ));
        let json = serde_json::to_string(&envelope).expect("serialize placement");
        let decoded: Envelope = serde_json::from_str(&json).expect("deserialize placement");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn fixed_window_presentation_preserves_final_client_dimensions_and_anchor() {
        let mut envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::PresentFixedWindowOverAnchor(PresentFixedWindowOverAnchorCommand {
                window_handle: 702,
                anchor_window_handle: 701,
                width: 560,
                height: 278,
            }),
        ));
        envelope.correlation_id = Some(envelope.id);
        let line = envelope.encode_line().expect("encode fixed presentation");
        assert!(line.contains("present_fixed_window_over_anchor"));
        assert_eq!(
            Envelope::decode_line(&line).expect("decode fixed presentation"),
            envelope
        );
    }

    #[test]
    fn window_placement_resize_and_shell_commands_round_trip() {
        let commands = [
            PlatformCommand::SetWindowPosition(SetWindowPositionCommand {
                window_handle: 778,
                x: 120,
                y: 240,
            }),
            PlatformCommand::SetWindowOwner(SetWindowOwnerCommand {
                window_handle: 778,
                owner_window_handle: 77,
            }),
            PlatformCommand::SetWindowMinimized(SetWindowMinimizedCommand { window_handle: 778 }),
            PlatformCommand::ToggleWindowMaximized(ToggleWindowMaximizedCommand {
                window_handle: 778,
            }),
            PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle: 778 }),
            PlatformCommand::PresentFixedWindowOverAnchor(PresentFixedWindowOverAnchorCommand {
                window_handle: 779,
                anchor_window_handle: 778,
                width: 700,
                height: 348,
            }),
            PlatformCommand::BeginWindowResize(BeginWindowResizeCommand { window_handle: 779 }),
            PlatformCommand::BeginWindowMove(BeginWindowMoveCommand { window_handle: 780 }),
            PlatformCommand::BeginFileDrag(BeginFileDragCommand {
                path: "C:/data/one.txt".into(),
                source_window_handle: 781,
            }),
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: "C:/data/one.txt".into(),
                action: ShellOpenAction::SelectInExplorer,
            }),
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: "C:/Apps/Editor.lnk".into(),
                action: ShellOpenAction::RunElevated,
            }),
            PlatformCommand::PlaySystemNotificationSound,
            PlatformCommand::ShutdownSystem(ShutdownSystemCommand { delay_seconds: 1 }),
            PlatformCommand::ExportBrowserExtension(ExportBrowserExtensionCommand {
                format: "chrome".into(),
                destination: r"C:\Users\u\Desktop\ext.crx".into(),
            }),
            PlatformCommand::NavigateDirectory(NavigateDirectoryCommand {
                host: DirectoryNavigationHost::Explorer,
                window_handle: 88,
                target_directory: r"C:\Work\Reports".into(),
                host_context_id: None,
            }),
            PlatformCommand::OpenFileInDialog(OpenFileInDialogCommand {
                window_handle: 77,
                file_path: r"C:\Work\Report.txt".into(),
            }),
            PlatformCommand::NavigateDirectory(NavigateDirectoryCommand {
                host: DirectoryNavigationHost::FileDialog,
                window_handle: 77,
                target_directory: r"C:\Work\Reports".into(),
                host_context_id: None,
            }),
            PlatformCommand::ExtractShellIcon(ExtractShellIconCommand {
                target: "C:/Apps/Editor.lnk".into(),
                size: 32,
            }),
        ];
        for command in commands {
            let envelope = Envelope::new(Message::PlatformCommand(command));
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("platform command must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn desktop_shell_command_and_tray_events_round_trip() {
        let command = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ConfigureDesktopShell(ConfigureDesktopShellCommand {
                start_on_login_silently: true,
                tray_icon_visible: true,
            }),
        ));
        assert_eq!(
            Envelope::decode_line(&command.encode_line().expect("encode shell command"))
                .expect("decode shell command"),
            command
        );

        for action in [TrayAction::Open, TrayAction::Exit] {
            let event = Envelope::new(Message::PlatformEvent(PlatformEvent::TrayActionRequested(
                TrayActionRequest {
                    correlation_id: uuid::Uuid::new_v4(),
                    action,
                },
            )));
            assert_eq!(
                Envelope::decode_line(&event.encode_line().expect("encode tray event"))
                    .expect("decode tray event"),
                event
            );
        }
    }

    #[test]
    fn system_motion_preference_event_round_trips_with_correlation_and_failure() {
        for (reduced_motion, native_error_code) in [(false, 0), (true, 0), (true, 5)] {
            let correlation_id = Uuid::new_v4();
            let mut envelope = Envelope::new(Message::PlatformEvent(
                PlatformEvent::SystemMotionPreferenceChanged(SystemMotionPreferenceChanged {
                    correlation_id,
                    reduced_motion,
                    native_error_code,
                }),
            ));
            envelope.correlation_id = Some(correlation_id);
            let encoded = envelope.encode_line().expect("encode motion event");
            assert!(encoded.contains("system_motion_preference_changed"));
            assert_eq!(
                Envelope::decode_line(&encoded).expect("decode motion event"),
                envelope
            );
        }
    }

    #[test]
    fn shell_action_session_commands_and_nested_menu_round_trip() {
        let commands = [
            PlatformCommand::QueryShellActions(QueryShellActionsCommand {
                target: "C:/data/one.txt".into(),
            }),
            PlatformCommand::QueryShellSubmenu(QueryShellSubmenuCommand {
                session_id: "session-1".into(),
                submenu_id: 7,
            }),
            PlatformCommand::InvokeShellAction(InvokeShellActionCommand {
                session_id: "session-1".into(),
                command_id: 42,
                owner_window_handle: 778,
            }),
        ];
        for command in commands {
            let envelope = Envelope::new(Message::PlatformCommand(command));
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("shell action command must decode");
            assert_eq!(decoded, envelope);
        }

        let menu = ShellActionMenu {
            session_id: "session-1".into(),
            navigation_directory: Some("C:/Apps".into()),
            items: vec![ShellActionItem {
                command_id: None,
                title: "发送到".into(),
                enabled: true,
                separator: false,
                submenu_id: Some(7),
                icon_path: Some("C:/icons/send-to.png".into()),
            }],
        };
        let json = serde_json::to_string(&menu).expect("serialize shell action menu");
        assert_eq!(
            serde_json::from_str::<ShellActionMenu>(&json).expect("deserialize shell action menu"),
            menu
        );
    }

    #[test]
    fn clipboard_platform_configuration_round_trips() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ConfigureClipboard(ConfigureClipboardCommand {
                enabled: false,
                hotkey_modifiers: vec!["control".into(), "shift".into()],
                hotkey_key: "V".into(),
                hotkey_display: "Ctrl+Shift+V".into(),
                excluded_processes: vec!["KeePass.exe".into(), "Bitwarden.exe".into()],
                suppress_in_fullscreen: false,
                max_text_bytes: 12 * 1024 * 1024,
                max_image_bytes: 96 * 1024 * 1024,
                max_image_pixels: 120_000_000,
            }),
        ));
        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("clipboard configuration must decode");
        assert_eq!(decoded, envelope);
        let Message::PlatformCommand(PlatformCommand::ConfigureClipboard(command)) =
            decoded.message
        else {
            panic!("expected clipboard configuration");
        };
        let mut legacy = serde_json::to_value(&command).expect("legacy clipboard configuration");
        assert_eq!(
            legacy
                .as_object_mut()
                .expect("configuration object")
                .remove("enabled"),
            Some(false.into())
        );
        let mut expected =
            serde_json::to_value(&command).expect("expected clipboard configuration");
        expected["enabled"] = true.into();
        assert_eq!(
            serde_json::to_value(
                serde_json::from_value::<ConfigureClipboardCommand>(legacy)
                    .expect("legacy enabled default")
            )
            .expect("encode legacy enabled default"),
            expected,
        );

        let recording = Envelope::new(Message::PlatformCommand(
            PlatformCommand::SetClipboardHotKeyRecording(SetClipboardHotKeyRecordingCommand {
                recording: true,
            }),
        ));
        let decoded = Envelope::decode_line(&recording.encode_line().expect("encode"))
            .expect("clipboard recording command must decode");
        assert_eq!(decoded, recording);
    }

    #[test]
    fn quick_launch_keyboard_health_round_trip_and_unknown_event_rejection() {
        for (state, code, name) in [
            (
                QuickLaunchKeyboardHealthState::InstallFailed,
                5,
                "install_failed",
            ),
            (
                QuickLaunchKeyboardHealthState::ThreadExited,
                0,
                "thread_exited",
            ),
            (
                QuickLaunchKeyboardHealthState::Reinstalled,
                0,
                "reinstalled",
            ),
        ] {
            let correlation_id = Uuid::new_v4();
            let mut event = Envelope::new(Message::PlatformEvent(
                PlatformEvent::QuickLaunchKeyboardHealthChanged(QuickLaunchKeyboardHealthChanged {
                    correlation_id,
                    state,
                    native_error_code: code,
                }),
            ));
            event.correlation_id = Some(correlation_id);
            let encoded = event.encode_line().expect("health event encodes");
            assert!(encoded.contains("quick_launch_keyboard_health_changed"));
            assert!(encoded.contains(&format!("\"state\":\"{name}\"")));
            assert_eq!(
                Envelope::decode_line(&encoded).expect("health event decodes"),
                event
            );
        }
        let unknown = Envelope::new(Message::PlatformEvent(PlatformEvent::TrayActionRequested(
            TrayActionRequest {
                correlation_id: Uuid::new_v4(),
                action: TrayAction::Open,
            },
        )))
        .encode_line()
        .expect("encode existing event")
        .replace("tray_action_requested", "newer_unknown_event");
        assert!(matches!(
            Envelope::decode_line(&unknown),
            Err(ProtocolError::InvalidJson(_))
        ));
    }

    #[test]
    fn quick_launch_hotkey_and_panel_request_round_trip() {
        let configuration = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ConfigureQuickLaunch(ConfigureQuickLaunchCommand {
                enabled: true,
                auto_quick_switch_enabled: true,
                file_dialog_search_enabled: true,
                suppress_in_exclusive_fullscreen: true,
                hotkey_modifiers: Vec::new(),
                hotkey_key: "Control".into(),
                hotkey_display: "双击 Ctrl".into(),
            }),
        ));
        let request_id = Uuid::new_v4();
        let mut request = Envelope::new(Message::PlatformEvent(
            PlatformEvent::QuickLaunchPanelRequested(QuickLaunchPanelRequest {
                target: fixture_context(),
                directory_navigation_context: Some(DirectoryNavigationContext {
                    host: DirectoryNavigationHost::Explorer,
                    window_handle: 42,
                    current_directory: Some(r"C:\Work".into()),
                    host_context_id: None,
                }),
                cursor_x: 100,
                cursor_y: 200,
                cursor_display_index: 1,
                cursor_monitor_dpi: 144,
                cursor_monitor: None,
                requested_at_unix_ms: 1_700_000_000_000,
                correlation_id: Some(request_id),
            }),
        ));
        request.correlation_id = Some(request_id);
        assert_eq!(request.correlation_id, Some(request_id));
        let Message::PlatformEvent(PlatformEvent::QuickLaunchPanelRequested(request_payload)) =
            &request.message
        else {
            panic!("expected Quick Launch panel event");
        };
        assert_eq!(request_payload.correlation_id, Some(request_id));
        let legacy_config: ConfigureQuickLaunchCommand = serde_json::from_str(
            r#"{"enabled":true,"hotkey_modifiers":[],"hotkey_key":"Control","hotkey_display":"双击 Ctrl"}"#,
        )
        .expect("legacy Quick Launch config must remain readable");
        assert!(legacy_config.suppress_in_exclusive_fullscreen);
        assert!(legacy_config.auto_quick_switch_enabled);
        assert!(legacy_config.file_dialog_search_enabled);
        let dialog_request = Envelope::new(Message::PlatformEvent(
            PlatformEvent::QuickLaunchPanelRequested(QuickLaunchPanelRequest {
                target: fixture_context(),
                directory_navigation_context: Some(DirectoryNavigationContext {
                    host: DirectoryNavigationHost::FileDialog,
                    window_handle: 43,
                    current_directory: Some(r"C:\Incoming".into()),
                    host_context_id: None,
                }),
                cursor_x: 100,
                cursor_y: 200,
                cursor_display_index: 1,
                cursor_monitor_dpi: 144,
                cursor_monitor: None,
                correlation_id: None,
                requested_at_unix_ms: 1_700_000_000_100,
            }),
        ));
        let recording = Envelope::new(Message::PlatformCommand(
            PlatformCommand::SetQuickLaunchHotKeyRecording(SetQuickLaunchHotKeyRecordingCommand {
                recording: true,
            }),
        ));
        let resolved = Envelope::new(Message::PlatformEvent(
            PlatformEvent::QuickLaunchDirectoryContextResolved(
                QuickLaunchDirectoryContextResolution {
                    target_window_handle: 42,
                    requested_at_unix_ms: 1_700_000_000_000,
                    directory_navigation_context: DirectoryNavigationContext {
                        host: DirectoryNavigationHost::ThirdPartyFileManager,
                        window_handle: 42,
                        current_directory: Some(r"D:\Elevated".into()),
                        host_context_id: Some("elevated:request-42".into()),
                    },
                },
            ),
        ));
        let auto_switch = Envelope::new(Message::PlatformEvent(
            PlatformEvent::QuickLaunchAutoSwitchRequested(QuickLaunchAutoSwitchRequest {
                target: DirectoryNavigationContext {
                    host: DirectoryNavigationHost::FileDialog,
                    window_handle: 43,
                    current_directory: Some(r"C:\Incoming".into()),
                    host_context_id: None,
                },
                source_directory: r"D:\Work".into(),
                requested_at_unix_ms: 1_700_000_000_100,
            }),
        ));
        let file_dialog_search_host = Envelope::new(Message::PlatformEvent(
            PlatformEvent::QuickLaunchFileDialogSearchHostChanged(
                QuickLaunchFileDialogSearchHost {
                    window_handle: 43,
                    current_directory: Some(r"C:\Incoming".into()),
                    left: 100,
                    top: 200,
                    width: 900,
                    height: 640,
                    monitor_dpi: 144,
                    visible: true,
                    changed_at_unix_ms: 1_700_000_000_200,
                    quick_switch_source: Some(QuickLaunchDirectorySource {
                        context: DirectoryNavigationContext {
                            host: DirectoryNavigationHost::ThirdPartyFileManager,
                            window_handle: 88,
                            current_directory: Some(r"D:\Workspace".into()),
                            host_context_id: Some("dopus:lister-88".into()),
                        },
                        display_name: "Workspace (Directory Opus)".into(),
                    }),
                },
            ),
        ));

        for envelope in [
            configuration,
            request,
            dialog_request,
            recording,
            resolved,
            auto_switch,
            file_dialog_search_host,
        ] {
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("quick-launch message must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn desktop_directory_navigation_host_round_trips() {
        let context: DirectoryNavigationContext = serde_json::from_str(
            r#"{"host":"desktop","window_handle":66,"current_directory":"C:\\Users\\Test\\Desktop"}"#,
        )
        .expect("desktop navigation context");

        assert_eq!(context.host, DirectoryNavigationHost::Desktop);
        assert_eq!(context.window_handle, 66);
        assert_eq!(
            context.current_directory.as_deref(),
            Some(r"C:\Users\Test\Desktop")
        );
        assert_eq!(
            serde_json::to_value(context)
                .expect("serialize context")
                .get("host")
                .and_then(serde_json::Value::as_str),
            Some("desktop")
        );
    }

    #[test]
    fn third_party_file_manager_directory_navigation_host_round_trips() {
        let context = DirectoryNavigationContext {
            host: DirectoryNavigationHost::ThirdPartyFileManager,
            window_handle: 44,
            current_directory: Some(r"D:\Workspace".into()),
            host_context_id: Some("pane-context-44".into()),
        };

        let json = serde_json::to_string(&context).expect("serialize host");
        assert!(json.contains(r#""host":"third_party_file_manager""#));
        assert!(json.contains(r#""host_context_id":"pane-context-44""#));
        let decoded: DirectoryNavigationContext =
            serde_json::from_str(&json).expect("deserialize host");
        assert_eq!(decoded, context);
    }

    #[test]
    fn elevated_directory_helper_request_round_trips_across_both_process_seams() {
        let request = ElevatedDirectoryHelperRequest {
            pipe_name: "nxtools-next-elevated-nav-0123456789abcdef".into(),
            authentication_token:
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
        };
        for envelope in [
            Envelope::new(Message::PlatformEvent(
                PlatformEvent::ElevatedDirectoryHelperRequested(request.clone()),
            )),
            Envelope::new(Message::IndexerCommand(
                IndexerCommand::EnsureElevatedDirectoryHelper(request.clone()),
            )),
            Envelope::new(Message::IndexerEvent(
                IndexerEvent::ElevatedDirectoryHelper(ElevatedDirectoryHelperStatus {
                    pipe_name: request.pipe_name.clone(),
                    launched: true,
                    error: None,
                }),
            )),
        ] {
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode request"))
                .expect("decode request");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn indexer_search_batches_preserve_generation_and_completion() {
        let request = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
            IndexSearchRequest {
                generation: 41,
                query: "d: ^report !tmp .pdf$".into(),
                limit: 50,
                context: IndexSearchContext {
                    directory: Some(r"D:\Work".into()),
                    preferred_directory: None,
                    include_files: true,
                    include_directories: true,
                    retain_kinds_apart: false,
                    ..Default::default()
                },
            },
        )));
        let batch = Envelope::reply_to(
            &request,
            Message::IndexerEvent(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: 41,
                items: vec![IndexSearchItem {
                    path: r"D:\Work\report.pdf".into(),
                    name: "report.pdf".into(),
                    is_directory: false,
                    size_bytes: Some(128),
                    modified_at_unix_ms: Some(1_700_000_000_000),
                    score: 900,
                }],
                complete: true,
                elapsed_us: 700,
            })),
        );

        for envelope in [request, batch] {
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("indexer frame must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn index_search_items_do_not_invent_unavailable_file_metadata() {
        let item = IndexSearchItem {
            path: r"C:\Work".into(),
            name: "Work".into(),
            is_directory: true,
            size_bytes: None,
            modified_at_unix_ms: None,
            score: 700,
        };

        let json = serde_json::to_value(&item).expect("serialize search item");

        assert!(json.get("size_bytes").is_none());
        assert!(json.get("modified_at_unix_ms").is_none());
        assert_eq!(
            serde_json::from_value::<IndexSearchItem>(json)
                .expect("deserialize search item")
                .size_bytes,
            None
        );
    }

    #[test]
    fn indexer_status_reports_real_volume_identity_and_phase() {
        let status = Envelope::new(Message::IndexerEvent(IndexerEvent::Status(IndexStatus {
            phase: IndexPhase::DiscoveringVolumes,
            indexed_items: 0,
            drives: vec![IndexDriveStatus {
                maintenance: None,
                mount_point: r"C:\".into(),
                file_system: "NTFS".into(),
                indexed_items: 0,
                journal_id: None,
                next_usn: None,
            }],
            last_error: None,
        })));
        let rebuild = Envelope::new(Message::IndexerCommand(IndexerCommand::RebuildIndex));

        for envelope in [status, rebuild] {
            let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
                .expect("indexer control frame must decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn clipboard_panel_request_preserves_target_and_cursor_position() {
        let envelope = Envelope::new(Message::PlatformEvent(
            PlatformEvent::ClipboardPanelRequested(ClipboardPanelRequest {
                target: ForegroundContext {
                    window_handle: 88,
                    focus_window_handle: 89,
                    process_id: 9,
                    process_name: "editor".into(),
                    window_title: "draft.md".into(),
                },
                cursor_x: 1200,
                cursor_y: 700,
                cursor_display_index: 1,
                cursor_monitor_dpi: 144,
                cursor_monitor: None,
                requested_at_unix_ms: 1_700_000_000_500,
                event_sequence: 41,
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("panel request must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn global_pointer_press_preserves_root_window_and_process() {
        let envelope = Envelope::new(Message::PlatformEvent(PlatformEvent::GlobalPointerPressed(
            PointerPressContext {
                window_handle: 90,
                root_window_handle: 80,
                process_id: 7,
                sampled_at_unix_ms: 1_700_000_000_400,
                event_sequence: 42,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode"))
            .expect("global pointer press must decode");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn command_result_is_correlated_to_the_original_command() {
        let command = Envelope::new(Message::PlatformCommand(PlatformCommand::PasteText(
            PasteTextCommand {
                text: "paste".into(),
                paste_after_write: true,
                target_window_handle: 9,
                target_focus_window_handle: 10,
            },
        )));
        let result = Envelope::reply_to(
            &command,
            Message::PlatformCommandResult(PlatformCommandResult {
                success: false,
                error: Some("target closed".into()),
                native_error_code: Some(5),
                data: Some("C:/backup/history.nxtclip".into()),
            }),
        );

        let decoded = Envelope::decode_line(&result.encode_line().expect("encode"))
            .expect("command result must decode");
        assert_eq!(decoded.correlation_id, Some(command.id));
        assert_eq!(decoded, result);
    }

    #[test]
    fn export_destination_picker_requires_source_identity_and_keeps_export_machine_only() {
        let chooser = PlatformCommand::PickBrowserExtensionExportPath(
            PickBrowserExtensionExportPathCommand {
                format: "chrome_store".into(),
                source_window_handle: 123,
                source_process_id: 42,
            },
        );
        let envelope = Envelope::new(Message::PlatformCommand(chooser));
        let encoded = envelope.encode_line().expect("encode chooser");
        let data: serde_json::Value = serde_json::from_str(&encoded).expect("JSON chooser");
        assert_eq!(
            data["payload"]["command"],
            "pick_browser_extension_export_path"
        );
        assert_eq!(data["payload"]["data"]["source_process_id"], 42);
        assert_eq!(
            Envelope::decode_line(&encoded).expect("decode chooser"),
            envelope
        );
        let mut missing = data;
        missing["payload"]["data"]
            .as_object_mut()
            .expect("picker data")
            .remove("source_process_id");
        assert!(
            Envelope::decode_line(&serde_json::to_string(&missing).expect("missing field JSON"))
                .is_err()
        );
    }

    #[test]
    fn clipboard_archive_picker_round_trips_its_mode_and_suggested_name() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::PickClipboardArchivePath(PickClipboardArchivePathCommand {
                source_window_handle: 42,
                source_process_id: 7,
                mode: ClipboardArchivePickerMode::Export,
                suggested_file_name: "NxTools-clipboard.nxtclip".into(),
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode picker"))
            .expect("decode picker");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn clipboard_storage_directory_picker_round_trips_the_current_directory() {
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::PickClipboardStorageDirectory(PickClipboardStorageDirectoryCommand {
                source_window_handle: 42,
                source_process_id: 7,
                current_directory: r"D:\NxToolsData\Clipboard".into(),
            }),
        ));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode picker"))
            .expect("decode picker");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn generic_directory_picker_round_trips_title_and_initial_directory() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::PickDirectory(
            PickDirectoryCommand {
                source_window_handle: 42,
                source_process_id: 7,
                current_directory: r"D:\Cache".into(),
                title: "选择要排除的目录".into(),
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode picker"))
            .expect("decode picker");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn program_picker_round_trips_the_current_path() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::PickProgram(
            PickProgramCommand {
                source_window_handle: 42,
                source_process_id: 7,
                current_path: r"C:\Tools\runner.exe".into(),
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode picker"))
            .expect("decode picker");
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn extension_manifest_picker_round_trips_its_typed_wire_command() {
        for current_path in ["", r"C:\Packages\扩展.json"] {
            let envelope = Envelope::new(Message::PlatformCommand(
                PlatformCommand::PickExtensionManifest(PickExtensionManifestCommand {
                    source_window_handle: 42,
                    source_process_id: 7,
                    current_path: current_path.into(),
                }),
            ));
            let encoded = envelope.encode_line().expect("encode manifest picker");
            let wire: serde_json::Value =
                serde_json::from_str(&encoded).expect("manifest picker JSON");
            assert_eq!(wire["payload"]["command"], "pick_extension_manifest");
            assert_eq!(wire["payload"]["data"]["current_path"], current_path);
            assert_eq!(
                Envelope::decode_line(&encoded).expect("decode manifest picker"),
                envelope
            );
        }
    }

    #[test]
    fn extension_runtime_commands_round_trip_typed_wire_fields() {
        for program in ["python", "node", "powershell", "pwsh"] {
            let commands = [
                (
                    PlatformCommand::PickExtensionRuntime(PickExtensionRuntimeCommand {
                        source_window_handle: 42,
                        source_process_id: 7,
                        program: program.into(),
                        current_path: String::new(),
                    }),
                    "pick_extension_runtime",
                    "current_path",
                    "",
                ),
                (
                    PlatformCommand::InspectExtensionRuntime(InspectExtensionRuntimeCommand {
                        program: program.into(),
                        executable: r"C:\Runtime\运行.exe".into(),
                    }),
                    "inspect_extension_runtime",
                    "executable",
                    r"C:\Runtime\运行.exe",
                ),
            ];
            for (command, name, field, expected) in commands {
                let envelope = Envelope::new(Message::PlatformCommand(command));
                let encoded = envelope.encode_line().expect("encode runtime command");
                let wire: serde_json::Value =
                    serde_json::from_str(&encoded).expect("runtime command JSON");
                assert_eq!(wire["payload"]["command"], name);
                assert_eq!(wire["payload"]["data"]["program"], program);
                assert_eq!(wire["payload"]["data"][field], expected);
                assert_eq!(
                    Envelope::decode_line(&encoded).expect("decode runtime command"),
                    envelope
                );
            }
        }
    }

    #[test]
    fn typed_process_launch_preserves_argument_boundaries_and_flags() {
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::LaunchProcess(
            LaunchProcessCommand {
                executable: r"C:\Program Files\Tool\tool.exe".into(),
                arguments: vec![
                    "--name".into(),
                    "two words".into(),
                    "quoted \"value\"".into(),
                ],
                working_directory: Some(r"D:\Work".into()),
                run_as_admin: true,
                run_silently: true,
            },
        )));

        let decoded = Envelope::decode_line(&envelope.encode_line().expect("encode launch"))
            .expect("decode launch");
        assert_eq!(decoded, envelope);
    }
}

#[cfg(test)]
mod installed_feedback_protocol_tests {
    use super::*;

    #[test]
    fn open_with_and_system_theme_have_typed_wire_roundtrips() {
        assert_eq!(
            serde_json::to_string(&ShellOpenAction::OpenWith).expect("action JSON"),
            "\"open_with\""
        );
        let event = PlatformEvent::SystemThemePreferenceChanged(SystemThemePreferenceChanged {
            correlation_id: Uuid::new_v4(),
            dark: false,
            native_error_code: 0,
        });
        let json = serde_json::to_string(&event).expect("theme JSON");
        assert_eq!(
            serde_json::from_str::<PlatformEvent>(&json).expect("theme roundtrip"),
            event
        );
    }
}

#[cfg(test)]
mod search_retention_tests {
    use super::*;
    #[test]
    fn w3_kind_retention_defaults_off_and_roundtrips_opt_in() {
        let old =
            serde_json::json!({"directory":null,"include_files":true,"include_directories":true});
        let mut context: IndexSearchContext = serde_json::from_value(old).expect("legacy context");
        assert!(!context.retain_kinds_apart);
        assert!(!context.allow_homophone_fallback);
        assert!(!context.recent_modified);
        let legacy = serde_json::to_value(&context).expect("legacy encoding");
        assert!(legacy.get("allow_homophone_fallback").is_none());
        assert!(legacy.get("recent_modified").is_none());
        context.allow_homophone_fallback = true;
        context.recent_modified = true;
        context.retain_kinds_apart = true;
        let json = serde_json::to_value(&context).expect("context");
        assert_eq!(json["retain_kinds_apart"], true);
        assert_eq!(
            serde_json::from_value::<IndexSearchContext>(json).expect("roundtrip"),
            context
        );
    }
}

#[cfg(test)]
mod recent_failure_tests {
    use super::*;
    #[test]
    fn w4_search_failure_has_typed_generation_and_roundtrip() {
        let event = IndexerEvent::SearchFailed(IndexSearchFailure {
            generation: 9,
            reason: "recent_classifier_budget".into(),
        });
        let json = serde_json::to_value(&event).expect("failure");
        assert_eq!(json["event"], "search_failed");
        assert_eq!(
            serde_json::from_value::<IndexerEvent>(json).expect("roundtrip"),
            event
        );
    }
}

#[cfg(test)]
mod context_menu_tests {
    use super::*;
    #[test]
    fn native_context_menu_roundtrips_tree_client_pixels_and_exact_cancel() {
        let command = PlatformCommand::ShowContextMenu(ShowContextMenuCommand {
            source_window_handle: 42,
            source_process_id: 99,
            x: 448,
            y: 30,
            popup_window_handle: 0,
            items: vec![ContextMenuItem {
                id: 0,
                label: "扩展处理".into(),
                enabled: true,
                checked: false,
                separator: false,
                children: vec![ContextMenuItem {
                    id: 7,
                    label: "按行去重".into(),
                    enabled: true,
                    checked: false,
                    separator: false,
                    children: vec![],
                }],
            }],
        });
        let value = serde_json::to_value(&command).expect("wire");
        assert_eq!(value["command"], "show_context_menu");
        assert_eq!(
            serde_json::from_value::<PlatformCommand>(value).expect("roundtrip"),
            command
        );
        let id = Uuid::new_v4();
        let cancel =
            PlatformCommand::CancelContextMenu(CancelContextMenuCommand { session_id: id });
        assert_eq!(
            serde_json::from_slice::<PlatformCommand>(
                &serde_json::to_vec(&cancel).expect("cancel wire")
            )
            .expect("cancel roundtrip"),
            cancel
        );
        assert!(
            serde_json::from_str::<ContextMenuResult>(
                r#"{"selected_id":7,"dismissed_outside":false,"extra":1}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<ContextMenuResult>(
                r#"{"selected_id":7,"selected_id":8,"dismissed_outside":false}"#
            )
            .is_err()
        );
    }

    #[test]
    fn screen_notification_prepare_round_trips_source_and_logical_size() {
        let command =
            PlatformCommand::PrepareNotificationWindow(PrepareNotificationWindowCommand {
                window_handle: 200,
                source_window_handle: 100,
                source_process_id: 123,
                position: "bottom_center".into(),
                width: 360,
                height: 136,
                keep_visible: false,
            });
        let value = serde_json::to_value(&command).expect("notification wire");
        assert_eq!(value["command"], "prepare_notification_window");
        assert_eq!(value["data"]["source_window_handle"], 100);
        assert_eq!(value["data"]["width"], 360);
        assert!(
            value["data"].get("keep_visible").is_none(),
            "legacy hidden preparation omits false"
        );
        let mut visible = value.clone();
        visible["data"]["keep_visible"] = serde_json::Value::Bool(true);
        let adjusted =
            serde_json::from_value::<PlatformCommand>(visible).expect("visible adjustment");
        assert!(
            matches!(&adjusted, PlatformCommand::PrepareNotificationWindow(p) if p.keep_visible)
        );
        assert_eq!(
            serde_json::to_value(adjusted).expect("visible wire")["data"]["keep_visible"],
            true
        );
        assert_eq!(
            serde_json::from_value::<PlatformCommand>(value).expect("notification roundtrip"),
            command
        );
    }
}

#[cfg(test)]
mod capture_pause_tests {
    use super::*;
    #[test]
    fn capture_pause_wire_is_independent_strict_and_boolean() {
        for paused in [true, false] {
            let command =
                PlatformCommand::SetClipboardCapturePaused(SetClipboardCapturePausedCommand {
                    paused,
                });
            let value = serde_json::to_value(&command).expect("pause command");
            assert_eq!(value["command"], "set_clipboard_capture_paused");
            assert_eq!(value["data"]["paused"], paused);
            assert_eq!(
                serde_json::from_value::<PlatformCommand>(value).expect("roundtrip"),
                command
            );
        }
        for data in [
            serde_json::json!({}),
            serde_json::json!({"paused":"true"}),
            serde_json::json!({"paused":true,"enabled":false}),
        ] {
            assert!(
                serde_json::from_value::<PlatformCommand>(
                    serde_json::json!({"command":"set_clipboard_capture_paused","data":data})
                )
                .is_err()
            );
        }
    }
}

#[cfg(test)]
mod default_apps_command_tests {
    use super::*;
    #[test]
    fn default_apps_fixed_empty_payload_rejects_arbitrary_targets() {
        let command = PlatformCommand::OpenDefaultAppsSettings(OpenDefaultAppsSettingsCommand {});
        assert_eq!(
            serde_json::to_value(&command).expect("encode"),
            serde_json::json!({"command":"open_default_apps_settings","data":{}})
        );
        for data in [
            serde_json::json!({"uri":"ms-settings:privacy"}),
            serde_json::json!({"path":"C:\\x"}),
            serde_json::Value::Null,
            serde_json::json!([]),
        ] {
            assert!(
                serde_json::from_value::<PlatformCommand>(
                    serde_json::json!({"command":"open_default_apps_settings","data":data})
                )
                .is_err()
            );
        }
    }
}

#[cfg(test)]
mod markdown_association_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn markdown_association_uses_a_closed_action_and_per_extension_state() {
        for (action, name) in [
            (MarkdownAssociationAction::Query, "query"),
            (MarkdownAssociationAction::Register, "register"),
            (MarkdownAssociationAction::Unregister, "unregister"),
            (MarkdownAssociationAction::OpenDefaults, "open_defaults"),
        ] {
            let command =
                PlatformCommand::MarkdownAssociation(MarkdownAssociationCommand { action });
            let wire = json!({"command":"markdown_association","data":{"action":name}});
            assert_eq!(serde_json::to_value(&command).expect("encode action"), wire);
            assert!(matches!(
                serde_json::from_value::<PlatformCommand>(wire).expect("decode action"),
                PlatformCommand::MarkdownAssociation(decoded) if decoded.action == action
            ));
        }
        for data in [
            json!({}),
            json!({"action":"set_default"}),
            json!({"action":"register","path":"C:\\Other.exe"}),
            json!({"action":"open_defaults","uri":"ms-settings:other"}),
            json!({"action":"open_defaults","registeredAppUser":"Other App"}),
            json!(["query"]),
        ] {
            assert!(
                serde_json::from_value::<PlatformCommand>(
                    json!({"command":"markdown_association","data":data})
                )
                .is_err()
            );
        }
        let state: MarkdownAssociationState = serde_json::from_str(
            r#"{"md":{"registered":true,"is_default":false},"markdown":{"registered":false,"is_default":true}}"#,
        )
        .expect("decode native readback");
        assert!(state.md.registered && !state.md.is_default);
        assert!(!state.markdown.registered && state.markdown.is_default);
    }
}

#[cfg(test)]
mod index_maintenance_tests {
    use super::*;

    #[test]
    fn u4_drive_maintenance_roundtrip_and_legacy_default() {
        let legacy = serde_json::json!({
            "mount_point": "C:\\", "file_system": "NTFS", "indexed_items": 12,
            "journal_id": 17, "next_usn": 100
        });
        let decoded: IndexDriveStatus =
            serde_json::from_value(legacy.clone()).expect("legacy drive");
        assert_eq!(
            serde_json::to_value(decoded).expect("legacy projection"),
            legacy
        );
        for maintenance in [
            serde_json::json!({"state": "initial_building"}),
            serde_json::json!({"state": "rebuilding", "reason": "journal_wrapped"}),
            serde_json::json!({"state": "ready"}),
        ] {
            let mut current = legacy.clone();
            current["maintenance"] = maintenance;
            let decoded: IndexDriveStatus =
                serde_json::from_value(current.clone()).expect("maintenance drive");
            assert_eq!(
                serde_json::to_value(decoded).expect("maintenance projection"),
                current,
                "per-drive maintenance must survive the current wire projection"
            );
            #[derive(Deserialize)]
            struct LegacyDrive {
                mount_point: String,
                file_system: String,
                indexed_items: u64,
                journal_id: Option<u64>,
                next_usn: Option<i64>,
            }
            let old: LegacyDrive =
                serde_json::from_value(current).expect("old DTO ignores new optional field");
            assert_eq!(old.mount_point, "C:\\");
            assert_eq!(old.file_system, "NTFS");
            assert_eq!(
                (old.indexed_items, old.journal_id, old.next_usn),
                (12, Some(17), Some(100))
            );
        }
    }
}

#[cfg(test)]
mod search_cancel_tests {
    use super::*;

    #[test]
    fn search_retirement_cancel_roundtrip_preserves_original_request_and_generation() {
        let request_id = Uuid::new_v4();
        let cancel = IndexSearchCancel {
            request_id,
            generation: 17,
        };
        let mut envelope = Envelope::new(Message::IndexerCommand(IndexerCommand::CancelSearch(
            cancel,
        )));
        envelope.correlation_id = Some(request_id);
        let encoded = envelope.encode_line().expect("cancel frame");
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("cancel JSON");
        assert_eq!(value["payload"]["command"], "cancel_search");
        assert_eq!(
            value["payload"]["data"]["request_id"],
            request_id.to_string()
        );
        assert_eq!(value["payload"]["data"]["generation"], 17);
        assert_eq!(
            Envelope::decode_line(&encoded).expect("cancel decode"),
            envelope
        );
        assert_ne!(envelope.id, request_id);
    }
}

#[cfg(test)]
mod b5_completion_tests {
    use super::*;
    #[test]
    fn b5_suspend_wire_preserves_sleep_hibernate_and_required_state() {
        for hibernate in [false, true] {
            let command = PlatformCommand::SuspendSystem(SuspendSystemCommand { hibernate });
            let wire = serde_json::to_value(&command).expect("typed suspend command");
            assert_eq!(
                wire,
                serde_json::json!({"command":"suspend_system","data":{"hibernate":hibernate}})
            );
            assert_eq!(
                serde_json::from_value::<PlatformCommand>(wire).expect("roundtrip"),
                command
            );
        }
        assert!(
            serde_json::from_value::<PlatformCommand>(
                serde_json::json!({"command":"suspend_system","data":{}})
            )
            .is_err()
        );
    }
}
