//! Mandatory installer operation, separate from starting the Desktop UI.
use anyhow::{Context as _, Result};
use nxtools_core::{AppConfig, AppPaths, init_file_logging};
use nxtools_protocol::PlatformCommandResult;
use nxtools_protocol::archive_shell_integration::{
    ArchiveShellIntegrationAction, ArchiveShellIntegrationResult, ArchiveShellIntegrationState,
};
use std::ffi::OsString;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::Instant;
use uuid::Uuid;

pub(super) const ARGUMENT: &str = "--install-archive-integration";
const APPLIED: i32 = 0; // Also a native NotApplied acknowledgement of an explicit opt-out.
const ASSOCIATION_WARNING: i32 = 10;
const FAILED: i32 = 11;
const WRONG_USER: i32 = 12;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub(super) fn run(arguments: &[OsString]) -> i32 {
    let [_, correlation, mode] = arguments else {
        return 2;
    };
    let Some(correlation_id) = correlation
        .to_str()
        .and_then(|id| Uuid::parse_str(id).ok())
        .filter(|id| !id.is_nil())
    else {
        return 2;
    };
    let fresh = match mode.to_str() {
        Some("fresh") => true,
        Some("upgrade") => false,
        _ => return 2,
    };
    match apply(arguments, correlation_id, fresh) {
        Ok(code) => code,
        Err(_) => FAILED,
    }
}

fn apply(arguments: &[OsString], correlation_id: Uuid, fresh: bool) -> Result<i32> {
    let current = std::env::current_exe()?;
    if let Some(protected) = super::protected_desktop_path()
        && !super::paths_equal(&current, &protected)
        && super::protected_package_matches_release(&current, &protected)?
    {
        let status = Command::new(protected).args(arguments).status()?;
        // Windows returns this actual child's terminal status, not launch or queue admission.
        return Ok(match status.code() {
            Some(code @ (APPLIED | ASSOCIATION_WARNING | FAILED | WRONG_USER)) => code,
            _ => FAILED,
        });
    }
    let platform_host = current
        .parent()
        .context("installed Desktop has no parent")?
        .join("NxTools.PlatformHost.exe");
    // Inno can only use its original-user token if it had one before elevation. Do not even
    // discover/create Rust profile paths until the native boundary proves the user.
    let status = Command::new(&platform_host)
        .args(["--check-archive-install-user", &correlation_id.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if !status.success() {
        return Ok(if status.code() == Some(WRONG_USER) {
            WRONG_USER
        } else {
            FAILED
        });
    }
    let paths = AppPaths::discover()?;
    let _log = init_file_logging(&paths, "archive-install").ok();
    let started = Instant::now();
    tracing::info!(event = "archive.install.accepted", %correlation_id, fresh);
    let mut native_error_code = None;
    let result = (|| {
        let config = AppConfig::archive_integration_for_install(&paths.config_file(), fresh)
            .context("archive_install_config_failed")?;
        let desired = config.archive.shell_integration;
        let revision = desired.desired_revision;
        let enabled = desired.open_with
            || desired.set_default
            || (desired.classic_cascade && !desired.menu_commands.is_empty());
        let command = crate::archive::shell_integration::integration_command(
            ArchiveShellIntegrationAction::Apply,
            desired,
        );
        tracing::info!(event = "archive.install.started", %correlation_id, revision, enabled);
        let reply = crate::runtime::apply_installed_archive_integration(
            &paths,
            &config.platform_pipe_name,
            &platform_host,
            correlation_id,
            command,
        )
        .context("archive_install_transport_failed")?;
        native_error_code = reply.native_error_code;
        Ok::<_, anyhow::Error>(terminal_exit(reply, revision, enabled))
    })();
    let (code, reason) = match result {
        Ok(outcome) => outcome,
        Err(_) => (FAILED, "install_apply_unavailable"),
    };
    tracing::info!(event = "archive.install.terminal", %correlation_id, code, reason,
        ?native_error_code, elapsed_ms = started.elapsed().as_millis() as u64);
    Ok(code)
}

fn terminal_exit(
    reply: PlatformCommandResult,
    revision: i64,
    enabled: bool,
) -> (i32, &'static str) {
    if !reply.success {
        return (FAILED, "native_operation_unconfirmed");
    }
    let Some(observed) = reply
        .data
        .as_deref()
        .and_then(|data| serde_json::from_str::<ArchiveShellIntegrationResult>(data).ok())
    else {
        return (FAILED, "invalid_response");
    };
    if observed.validate().is_err() || observed.desired_revision != Some(revision) {
        return (FAILED, "invalid_response");
    }
    match (observed.state, observed.failure.as_deref()) {
        (ArchiveShellIntegrationState::Applied, failure)
            if enabled
                && observed.applied_revision == Some(revision)
                && observed.applied_values > 0 =>
        {
            match failure {
                None => (APPLIED, "applied"),
                Some("user_choice_locked") => (ASSOCIATION_WARNING, "user_choice_locked"),
                _ => (FAILED, "invalid_response"),
            }
        }
        (ArchiveShellIntegrationState::NotApplied, None)
            if !enabled && observed.applied_values == 0 =>
        {
            (APPLIED, "opted_out")
        }
        (ArchiveShellIntegrationState::Failed, _) => (FAILED, "native_apply_failed"),
        (ArchiveShellIntegrationState::Disabled, _) => (FAILED, "native_apply_disabled"),
        _ => (FAILED, "invalid_response"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_archive_apply_distinguishes_terminal_result_warning_and_failure() {
        fn reply(data: Option<ArchiveShellIntegrationResult>) -> PlatformCommandResult {
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: data
                    .map(|value| serde_json::to_string(&value).expect("integration result wire")),
            }
        }
        let mut observation = ArchiveShellIntegrationResult {
            version: 1,
            state: ArchiveShellIntegrationState::Applied,
            desired_revision: Some(9),
            applied_revision: Some(9),
            applied_values: 1,
            removed: 0,
            restored: 0,
            preserved: 0,
            failure: None,
        };
        assert_eq!(terminal_exit(reply(None), 9, true).0, FAILED);
        assert_eq!(
            terminal_exit(reply(Some(observation.clone())), 10, true).0,
            FAILED
        );
        assert_eq!(
            terminal_exit(reply(Some(observation.clone())), 9, true).0,
            APPLIED
        );
        observation.failure = Some("user_choice_locked".into());
        assert_eq!(
            terminal_exit(reply(Some(observation.clone())), 9, true).0,
            ASSOCIATION_WARNING
        );
        observation.state = ArchiveShellIntegrationState::Failed;
        assert_eq!(
            terminal_exit(reply(Some(observation.clone())), 9, true).0,
            FAILED
        );
        observation.state = ArchiveShellIntegrationState::NotApplied;
        observation.failure = None;
        observation.applied_revision = None;
        observation.applied_values = 0;
        assert_eq!(
            terminal_exit(reply(Some(observation.clone())), 9, true).0,
            FAILED
        );
        assert_eq!(terminal_exit(reply(Some(observation)), 9, false).0, APPLIED);
    }
}
