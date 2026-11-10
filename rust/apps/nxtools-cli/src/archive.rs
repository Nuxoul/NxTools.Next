//! Noninteractive projection of the original archive product protocols; no codec/native IO.
mod execution;

use crate::{CliError, EXIT_OPERATION_FAILED, EXIT_REQUEST_REJECTED, EXIT_SYNTAX};
use nxtools_protocol::archive_job::ArchiveJobConflict;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveAction {
    List,
    Test,
    Extract,
}
impl ArchiveAction {
    fn name(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Test => "test",
            Self::Extract => "extract",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveCommand {
    pub action: ArchiveAction,
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    pub conflict: ArchiveJobConflict,
    pub create_destination: bool,
    pub codepage: u32,
    pub json: bool,
}

pub(crate) fn parse(args: Vec<String>) -> Result<ArchiveCommand, CliError> {
    let syntax = || {
        CliError::syntax(
            "压缩包用法：archive list|test <source> [--json] [--codepage N]；archive extract <source> --dest <directory> [--conflict fail|skip|overwrite|rename] [--create-destination] [--json]",
        )
    };
    let action = match args.first().map(String::as_str) {
        Some("list") => ArchiveAction::List,
        Some("test") => ArchiveAction::Test,
        Some("extract") => ArchiveAction::Extract,
        _ => return Err(syntax()),
    };
    let source = args
        .get(1)
        .filter(|p| !p.starts_with('-'))
        .ok_or_else(syntax)?;
    validate_path(source)?;
    let mut result = ArchiveCommand {
        action,
        source: PathBuf::from(source),
        destination: None,
        conflict: ArchiveJobConflict::Ask,
        create_destination: false,
        codepage: 0,
        json: false,
    };
    let mut seen = std::collections::HashSet::new();
    let mut index = 2;
    while index < args.len() {
        let option = args[index].as_str();
        if !seen.insert(option) {
            return Err(CliError::syntax("压缩包选项不能重复"));
        }
        match option {
            "--json" => result.json = true,
            "--create-destination" if action == ArchiveAction::Extract => {
                result.create_destination = true
            }
            "--codepage" => {
                index += 1;
                result.codepage = args
                    .get(index)
                    .and_then(|s| s.parse().ok())
                    .filter(|cp| nxtools_protocol::archive_read::valid_codepage(*cp))
                    .ok_or_else(syntax)?;
            }
            "--dest" if action == ArchiveAction::Extract => {
                index += 1;
                let value = args
                    .get(index)
                    .filter(|p| !p.starts_with('-'))
                    .ok_or_else(syntax)?;
                validate_path(value)?;
                result.destination = Some(PathBuf::from(value));
            }
            "--conflict" if action == ArchiveAction::Extract => {
                index += 1;
                result.conflict = match args.get(index).map(String::as_str) {
                    Some("fail") => ArchiveJobConflict::Ask,
                    Some("skip") => ArchiveJobConflict::Skip,
                    Some("overwrite") => ArchiveJobConflict::Overwrite,
                    Some("rename") => ArchiveJobConflict::AutoRename,
                    _ => return Err(syntax()),
                };
            }
            _ => return Err(syntax()),
        }
        index += 1;
    }
    if action == ArchiveAction::Extract && result.destination.is_none() {
        return Err(CliError::syntax(
            "解压必须显式指定 --dest；不会使用当前目录",
        ));
    }
    Ok(result)
}
fn validate_path(value: &str) -> Result<(), CliError> {
    if value.is_empty() || value.len() > 32767 || value.chars().any(char::is_control) {
        return Err(CliError::syntax("路径为空、过长或含控制字符"));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct ArchiveReport {
    schema_version: u32,
    request_id: Uuid,
    command: &'static str,
    success: bool,
    result: Value,
    error: Option<ArchiveFailure>,
    #[serde(skip)]
    exit_code: u8,
}
#[derive(Debug, Serialize)]
pub(crate) struct ArchiveFailure {
    code: String,
    native_error_code: Option<i32>,
}
impl ArchiveReport {
    fn failed(id: Uuid, action: ArchiveAction, code: &str, exit_code: u8) -> Self {
        Self {
            schema_version: 1,
            request_id: id,
            command: action.name(),
            success: false,
            result: Value::Null,
            error: Some(ArchiveFailure {
                code: code.into(),
                native_error_code: None,
            }),
            exit_code,
        }
    }
    fn with_terminal(
        id: Uuid,
        action: ArchiveAction,
        result: Value,
        success: bool,
        code: Option<String>,
        native: Option<i32>,
    ) -> Self {
        let mut report = Self::failed(
            id,
            action,
            code.as_deref().unwrap_or("operation_failed"),
            EXIT_OPERATION_FAILED,
        );
        report.result = result;
        report.success = success;
        if success {
            report.error = None;
            report.exit_code = 0;
        } else if let Some(error) = &mut report.error {
            error.native_error_code = native;
            report.exit_code = match error.code.as_str() {
                "incompatible_version" | "missing_capability" => crate::EXIT_PROTOCOL_INCOMPATIBLE,
                "invalid_request"
                | "interaction_required"
                | "password_required"
                | "output_collision"
                | "destination_required" => EXIT_REQUEST_REJECTED,
                _ => EXIT_OPERATION_FAILED,
            };
        }
        report
    }
}

pub(crate) fn run(command: ArchiveCommand, executable: &Path) -> Result<(), CliError> {
    let json_output = command.json;
    let report = execution::execute(command, executable);
    emit(&report, json_output)?;
    if report.success {
        Ok(())
    } else {
        Err(CliError {
            exit_code: report.exit_code,
            message: String::new(),
        })
    }
}
fn emit(report: &ArchiveReport, json_output: bool) -> Result<(), CliError> {
    use std::io::Write;
    let output_error = || CliError {
        exit_code: EXIT_OPERATION_FAILED,
        message: "无法写出压缩包命令结果".into(),
    };
    if json_output {
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        serde_json::to_writer(&mut output, report).map_err(|_| output_error())?;
        output.write_all(b"\n").map_err(|_| output_error())?;
    } else if report.success {
        if let Some(entries) = report.result.get("entries").and_then(Value::as_array) {
            for entry in entries {
                let name = entry.get("name").cloned().unwrap_or(Value::Null);
                let kind = entry.get("kind").and_then(Value::as_str).unwrap_or("?");
                let size = entry
                    .get("size")
                    .filter(|s| !s.is_null())
                    .map(Value::to_string)
                    .unwrap_or_else(|| "?".into());
                println!("{kind}\t{size}\t{name}");
            }
        }
        println!(
            "压缩包 {} 完成；request_id={}",
            report.command, report.request_id
        );
        println!("{}", report.result);
    } else {
        let code = report
            .error
            .as_ref()
            .map(|e| e.code.as_str())
            .unwrap_or("operation_failed");
        eprintln!(
            "压缩包 {} 失败：{code}；request_id={}；result={}",
            report.command, report.request_id, report.result
        );
    }
    Ok(())
}

// Kept separate from download syntax errors; archive JSON must not echo rejected secret argv.
pub fn is_archive_invocation(args: &[String]) -> bool {
    args.first().is_some_and(|a| a == "archive")
        || args.first().is_some_and(|a| a == "--connect-only")
            && args.get(1).is_some_and(|a| a == "archive")
}
pub fn emit_syntax_failure(args: &[String]) -> Result<(), CliError> {
    let action = if args.iter().any(|a| a == "extract") {
        ArchiveAction::Extract
    } else if args.iter().any(|a| a == "test") {
        ArchiveAction::Test
    } else {
        ArchiveAction::List
    };
    emit(
        &ArchiveReport::failed(Uuid::new_v4(), action, "invalid_request", EXIT_SYNTAX),
        true,
    )
}
