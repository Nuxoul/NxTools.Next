//! P206 explicit Explorer integration request/observed result. Rust owns the saved desired
//! intent/revision; C# owns the current-user Registry transaction and applied facts.
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const MAX_EXTENSIONS: usize = 32;

pub const MENU_COMMANDS: [&str; 9] = [
    "open",
    "extract_smart",
    "extract_here",
    "extract_name",
    "extract_dialog",
    "test",
    "create_zip",
    "create_7z",
    "create",
];
fn default_double_click() -> String {
    "open".into()
}
fn default_menu_layout() -> String {
    "cascade".into()
}
fn default_menu_commands() -> Vec<String> {
    [
        "open",
        "extract_smart",
        "extract_here",
        "extract_name",
        "test",
        "create_zip",
        "create_7z",
        "create",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveShellIntegrationAction {
    Status,
    Apply,
    Revert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveShellIntegrationCommand {
    pub version: u32,
    pub action: ArchiveShellIntegrationAction,
    pub desired_revision: i64,
    pub classic_cascade: bool,
    pub open_with: bool,
    pub extensions: Vec<String>,
    #[serde(default = "default_double_click")]
    pub double_click_action: String,
    #[serde(default = "default_menu_layout")]
    pub menu_layout: String,
    #[serde(default = "default_menu_commands")]
    pub menu_commands: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveShellIntegrationState {
    NotApplied,
    Applied,
    Reverted,
    RevertRequired,
    /// Real application is not authorized in this build; nothing was captured or written.
    Disabled,
    Failed,
}

/// Observed native outcome. `applied_revision` is only present for observed applied facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveShellIntegrationResult {
    pub version: u32,
    pub state: ArchiveShellIntegrationState,
    pub desired_revision: Option<i64>,
    pub applied_revision: Option<i64>,
    pub applied_values: u32,
    pub removed: u32,
    pub restored: u32,
    pub preserved: u32,
    pub failure: Option<String>,
}

impl ArchiveShellIntegrationCommand {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != VERSION {
            return Err("unsupported_version");
        }
        if self.desired_revision < 0 || self.extensions.len() > MAX_EXTENSIONS {
            return Err("invalid_request");
        }
        if !matches!(self.double_click_action.as_str(), "open" | "smart" | "name")
            || !matches!(self.menu_layout.as_str(), "cascade" | "flat")
            || self.menu_commands.len() > MENU_COMMANDS.len()
            || self
                .menu_commands
                .iter()
                .enumerate()
                .any(|(index, command)| {
                    !MENU_COMMANDS.contains(&command.as_str())
                        || self.menu_commands[..index].contains(command)
                })
        {
            return Err("invalid_request");
        }
        if self.action == ArchiveShellIntegrationAction::Apply
            && (self.open_with
                || (self.classic_cascade
                    && self.menu_commands.iter().any(|command| {
                        !matches!(command.as_str(), "create" | "create_zip" | "create_7z")
                    })))
            && self.extensions.is_empty()
        {
            return Err("invalid_request");
        }
        if self.extensions.iter().any(|extension| {
            let value = extension.strip_prefix('.').unwrap_or(extension);
            value.is_empty()
                || value.len() > 31
                || !value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        }) {
            return Err("invalid_request");
        }
        Ok(())
    }
}

impl ArchiveShellIntegrationResult {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != VERSION {
            return Err("unsupported_version");
        }
        // Failed/Disabled/RevertRequired may report facts that are still applied.
        let revision_required = self.state == ArchiveShellIntegrationState::Applied;
        let revision_forbidden = matches!(
            self.state,
            ArchiveShellIntegrationState::NotApplied | ArchiveShellIntegrationState::Reverted
        );
        if (revision_required && self.applied_revision.is_none())
            || (revision_forbidden && self.applied_revision.is_some())
        {
            return Err("invalid_response");
        }
        if (self.state == ArchiveShellIntegrationState::Failed
            || self.state == ArchiveShellIntegrationState::Disabled)
            && self.failure.is_none()
        {
            return Err("invalid_response");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(action: ArchiveShellIntegrationAction) -> ArchiveShellIntegrationCommand {
        ArchiveShellIntegrationCommand {
            version: VERSION,
            action,
            desired_revision: 3,
            classic_cascade: true,
            open_with: false,
            extensions: vec![".zip".into()],
            double_click_action: default_double_click(),
            menu_layout: default_menu_layout(),
            menu_commands: default_menu_commands(),
        }
    }

    #[test]
    fn strict_wire_and_bounds() {
        let value = command(ArchiveShellIntegrationAction::Apply);
        let json = serde_json::to_string(&crate::PlatformCommand::ArchiveShellIntegration(
            value.clone(),
        ))
        .expect("encode");
        assert!(json.contains("\"command\":\"archive_shell_integration\""));
        assert!(json.contains("\"action\":\"apply\""));
        assert!(json.contains("\"desired_revision\":3"));
        assert_eq!(value.validate(), Ok(()));
        let mut invalid = value.clone();
        invalid.extensions = vec!["z ip".into()];
        assert_eq!(invalid.validate(), Err("invalid_request"));
        invalid.extensions = vec![];
        assert_eq!(invalid.validate(), Err("invalid_request"));
        invalid.extensions = vec![".zip".into(); MAX_EXTENSIONS + 1];
        assert_eq!(invalid.validate(), Err("invalid_request"));
        let mut revert = command(ArchiveShellIntegrationAction::Revert);
        revert.extensions.clear();
        assert_eq!(revert.validate(), Ok(()));
        assert!(
            serde_json::from_str::<ArchiveShellIntegrationCommand>(
                &serde_json::to_string(&value)
                    .expect("plain")
                    .replace("\"version\":1", "\"version\":1,\"path\":\"C:\\\\x\"")
            )
            .is_err(),
            "command carries no executable/registry path authority"
        );
    }

    #[test]
    fn observed_result_never_claims_applied_without_revision() {
        let result: ArchiveShellIntegrationResult = serde_json::from_str(
            r#"{"version":1,"state":"disabled","desired_revision":3,"applied_revision":null,"applied_values":0,"removed":0,"restored":0,"preserved":0,"failure":"apply_not_authorized"}"#,
        )
        .expect("C# disabled wire");
        assert_eq!(result.validate(), Ok(()));
        let mut forged = result.clone();
        forged.state = ArchiveShellIntegrationState::Applied;
        assert_eq!(forged.validate(), Err("invalid_response"));
        forged.applied_revision = Some(3);
        assert_eq!(forged.validate(), Ok(()));
        let mut silent = result;
        silent.failure = None;
        assert_eq!(silent.validate(), Err("invalid_response"));
    }
}

#[cfg(test)]
mod settings_completion_tests {
    use super::*;
    #[test]
    fn shell_settings_wire_legacy_and_invalid_values() {
        let legacy = serde_json::json!({"version":1,"action":"apply","desired_revision":1,"classic_cascade":true,"open_with":true,"extensions":[".zip"]});
        let command: ArchiveShellIntegrationCommand =
            serde_json::from_value(legacy.clone()).expect("old wire");
        assert_eq!(command.double_click_action, "open");
        assert_eq!(command.menu_layout, "cascade");
        assert_eq!(command.menu_commands, default_menu_commands());
        for commands in [
            serde_json::json!([]),
            serde_json::json!(["create_zip", "create_7z", "create"]),
        ] {
            let mut value = legacy.clone();
            value["open_with"] = serde_json::json!(false);
            value["extensions"] = serde_json::json!([]);
            value["menu_commands"] = commands;
            let command: ArchiveShellIntegrationCommand =
                serde_json::from_value(value).expect("creation-only wire");
            assert_eq!(command.validate(), Ok(()));
        }
        for (field, value) in [
            ("double_click_action", serde_json::json!("admin")),
            ("menu_layout", serde_json::json!("windows11")),
            ("menu_commands", serde_json::json!(["open", "open"])),
            ("menu_commands", serde_json::json!(["delete_source"])),
            ("menu_commands", serde_json::Value::Null),
        ] {
            let mut value_json = legacy.clone();
            value_json[field] = value;
            if let Ok(value) = serde_json::from_value::<ArchiveShellIntegrationCommand>(value_json)
            {
                assert!(value.validate().is_err());
            }
        }
    }
}
