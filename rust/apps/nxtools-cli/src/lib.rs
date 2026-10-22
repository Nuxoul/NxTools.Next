use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use nxtools_download::{
    DownloadBatchAction, DownloadCommand, DownloadCommandOutcome, DownloadDailySchedule,
    DownloadEngineTask, DownloadEvent, DownloadFault, DownloadFaultCode, DownloadQuery,
    DownloadQueueAction, DownloadRequest, DownloadRequestHeader, DownloadSchedule, DownloadStatus,
    DownloadSubmitMode, DownloadTaskId, DownloadTaskSummary, DownloadWeekday,
};
use nxtools_download_ipc::{
    DOWNLOAD_PIPE_NAME, DownloadIpcClient, DownloadIpcProtocolIncompatible, command_envelope,
};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_SYNTAX: u8 = 2;
pub const EXIT_HOST_UNAVAILABLE: u8 = 3;
pub const EXIT_TASK_NOT_FOUND: u8 = 4;
pub const EXIT_INVALID_STATE: u8 = 5;
pub const EXIT_REQUEST_REJECTED: u8 = 6;
pub const EXIT_OPERATION_FAILED: u8 = 7;
pub const EXIT_PROTOCOL_INCOMPATIBLE: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CliHostPolicy {
    #[default]
    ConnectOrStart,
    ConnectOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInvocation {
    pub command: CliCommand,
    pub host_policy: CliHostPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Add {
        url: String,
        directory: PathBuf,
        directory_explicit: bool,
        name: Option<String>,
        headers: Vec<DownloadRequestHeader>,
        idempotency_key: Option<Uuid>,
        json: bool,
    },
    List {
        filter: ListFilter,
        json: bool,
    },
    Show {
        task_id: DownloadTaskId,
        watch: bool,
        json: bool,
    },
    Action {
        action: CliAction,
        task_ids: Vec<DownloadTaskId>,
        delete_file: bool,
        json: bool,
    },
    Schedule {
        queue_id: String,
        schedule: Option<DownloadSchedule>,
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListFilter {
    #[default]
    All,
    Active,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliAction {
    Pause,
    Resume,
    Cancel,
    Retry,
    Redownload,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    pub exit_code: u8,
    pub message: String,
}

impl CliError {
    fn syntax(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_SYNTAX,
            message: message.into(),
        }
    }

    fn host(error: anyhow::Error) -> Self {
        let message = format!("{error:#}");
        let exit_code = if error
            .downcast_ref::<DownloadIpcProtocolIncompatible>()
            .is_some()
        {
            EXIT_PROTOCOL_INCOMPATIBLE
        } else {
            EXIT_HOST_UNAVAILABLE
        };
        Self { exit_code, message }
    }
}

pub fn parse_invocation<I, S>(
    arguments: I,
    default_directory: PathBuf,
) -> Result<CliInvocation, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut arguments = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
    let host_policy = if arguments
        .first()
        .is_some_and(|value| value == "--connect-only")
    {
        arguments.remove(0);
        CliHostPolicy::ConnectOnly
    } else {
        CliHostPolicy::ConnectOrStart
    };
    Ok(CliInvocation {
        command: parse_args(arguments, default_directory)?,
        host_policy,
    })
}

pub fn parse_args<I, S>(arguments: I, default_directory: PathBuf) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
    if args.first().is_some_and(|value| value == "download") {
        args.remove(0);
    }
    let Some(command) = args.first().cloned() else {
        return Err(CliError::syntax(usage()));
    };
    args.remove(0);
    match command.as_str() {
        "add" => parse_add(args, default_directory),
        "list" => parse_list(args),
        "show" => parse_show(args),
        "pause" => parse_action(CliAction::Pause, args),
        "resume" => parse_action(CliAction::Resume, args),
        "retry" => parse_action(CliAction::Retry, args),
        "redownload" => parse_action(CliAction::Redownload, args),
        "cancel" => parse_action(CliAction::Cancel, args),
        "remove" => parse_action(CliAction::Remove, args),
        "schedule" => parse_schedule(args),
        _ => Err(CliError::syntax(format!(
            "unknown download command '{command}'\n{}",
            usage()
        ))),
    }
}

fn parse_schedule(args: Vec<String>) -> Result<CliCommand, CliError> {
    let mut queue_id = "default".to_owned();
    let mut start = None;
    let mut stop = None;
    let mut days = None;
    let mut disable = false;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--queue" => {
                index += 1;
                queue_id = required_value(&args, index, "--queue")?.to_owned();
            }
            "--start" => {
                index += 1;
                start = Some(parse_clock_minute(required_value(
                    &args, index, "--start",
                )?)?);
            }
            "--stop" => {
                index += 1;
                stop = Some(parse_clock_minute(required_value(&args, index, "--stop")?)?);
            }
            "--days" => {
                index += 1;
                days = Some(parse_weekdays(required_value(&args, index, "--days")?)?);
            }
            "--disable" => disable = true,
            "--json" => json = true,
            option => {
                return Err(CliError::syntax(format!(
                    "unknown schedule option '{option}'"
                )));
            }
        }
        index += 1;
    }
    if disable {
        if start.is_some() || stop.is_some() || days.is_some() {
            return Err(CliError::syntax(
                "--disable cannot be combined with --start, --stop or --days",
            ));
        }
        return Ok(CliCommand::Schedule {
            queue_id,
            schedule: None,
            json,
        });
    }
    if start.is_none() && stop.is_none() {
        return Err(CliError::syntax(
            "schedule requires at least one of --start HH:MM or --stop HH:MM",
        ));
    }
    let days = days.unwrap_or_else(all_weekdays);
    Ok(CliCommand::Schedule {
        queue_id,
        schedule: Some(DownloadSchedule {
            daily: DownloadDailySchedule {
                days,
                start_minute: start.unwrap_or_default(),
                stop_minute: stop.unwrap_or_default(),
                enabled_start: start.is_some(),
                enabled_stop: stop.is_some(),
            },
        }),
        json,
    })
}

fn parse_clock_minute(value: &str) -> Result<u16, CliError> {
    let Some((hours, minutes)) = value.split_once(':') else {
        return Err(CliError::syntax("时间必须为 HH:MM，例如 09:30"));
    };
    let hours = hours
        .parse::<u16>()
        .map_err(|_| CliError::syntax("时间必须为 HH:MM，例如 09:30"))?;
    let minutes = minutes
        .parse::<u16>()
        .map_err(|_| CliError::syntax("时间必须为 HH:MM，例如 09:30"))?;
    if hours >= 24 || minutes >= 60 {
        return Err(CliError::syntax("时间必须在 00:00 到 23:59 之间"));
    }
    Ok(hours * 60 + minutes)
}

fn parse_weekdays(value: &str) -> Result<Vec<DownloadWeekday>, CliError> {
    let mut days = Vec::new();
    for value in value.split(',').map(str::trim) {
        let day = match value.to_ascii_lowercase().as_str() {
            "mon" | "monday" => DownloadWeekday::Monday,
            "tue" | "tuesday" => DownloadWeekday::Tuesday,
            "wed" | "wednesday" => DownloadWeekday::Wednesday,
            "thu" | "thursday" => DownloadWeekday::Thursday,
            "fri" | "friday" => DownloadWeekday::Friday,
            "sat" | "saturday" => DownloadWeekday::Saturday,
            "sun" | "sunday" => DownloadWeekday::Sunday,
            _ => {
                return Err(CliError::syntax(
                    "--days 使用逗号分隔的 mon,tue,wed,thu,fri,sat,sun",
                ));
            }
        };
        if !days.contains(&day) {
            days.push(day);
        }
    }
    if days.is_empty() {
        return Err(CliError::syntax("--days 至少包含一个星期"));
    }
    Ok(days)
}

fn all_weekdays() -> Vec<DownloadWeekday> {
    DownloadWeekday::ALL.to_vec()
}

fn parse_add(args: Vec<String>, default_directory: PathBuf) -> Result<CliCommand, CliError> {
    let mut url = None;
    let mut directory = default_directory;
    let mut directory_explicit = false;
    let mut name = None;
    let mut headers = Vec::new();
    let mut idempotency_key = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--dir" => {
                index += 1;
                directory = PathBuf::from(required_value(&args, index, "--dir")?);
                directory_explicit = true;
            }
            "--name" => {
                index += 1;
                name = Some(required_value(&args, index, "--name")?.to_owned());
            }
            "--header" => {
                index += 1;
                let value = required_value(&args, index, "--header")?;
                let Some((header_name, header_value)) = value.split_once(':') else {
                    return Err(CliError::syntax("--header must use the form name:value"));
                };
                if header_name.trim().is_empty() {
                    return Err(CliError::syntax("header name cannot be empty"));
                }
                headers.push(DownloadRequestHeader {
                    name: header_name.trim().into(),
                    value: header_value.trim().into(),
                });
            }
            "--idempotency-key" => {
                index += 1;
                let value = required_value(&args, index, "--idempotency-key")?;
                idempotency_key = Some(
                    Uuid::parse_str(value)
                        .map_err(|_| CliError::syntax("--idempotency-key must be a valid UUID"))?,
                );
            }
            "--json" => json = true,
            option if option.starts_with('-') => {
                return Err(CliError::syntax(format!("unknown add option '{option}'")));
            }
            value if url.is_none() => url = Some(value.to_owned()),
            value => {
                return Err(CliError::syntax(format!(
                    "unexpected add argument '{value}'"
                )));
            }
        }
        index += 1;
    }
    let url = url.ok_or_else(|| CliError::syntax("download add requires a URL"))?;
    DownloadRequest {
        source_url: url.clone(),
        save_directory: directory.clone(),
        save_directory_explicit: directory_explicit,
        file_name: name.clone(),
        headers: headers.clone(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: None,
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    }
    .validate()
    .map_err(|error| CliError::syntax(error.to_string()))?;
    Ok(CliCommand::Add {
        url,
        directory,
        directory_explicit,
        name,
        headers,
        idempotency_key,
        json,
    })
}

fn parse_list(args: Vec<String>) -> Result<CliCommand, CliError> {
    let mut filter = ListFilter::All;
    let mut filter_set = false;
    let mut json = false;
    for argument in args {
        match argument.as_str() {
            "--json" => json = true,
            "--active" | "--completed" | "--failed" => {
                if filter_set {
                    return Err(CliError::syntax(
                        "list accepts only one of --active, --completed or --failed",
                    ));
                }
                filter_set = true;
                filter = match argument.as_str() {
                    "--active" => ListFilter::Active,
                    "--completed" => ListFilter::Completed,
                    _ => ListFilter::Failed,
                };
            }
            _ => {
                return Err(CliError::syntax(format!(
                    "unknown list option '{argument}'"
                )));
            }
        }
    }
    Ok(CliCommand::List { filter, json })
}

fn parse_show(args: Vec<String>) -> Result<CliCommand, CliError> {
    let mut task_id = None;
    let mut watch = false;
    let mut json = false;
    for argument in args {
        match argument.as_str() {
            "--watch" => watch = true,
            "--json" => json = true,
            option if option.starts_with('-') => {
                return Err(CliError::syntax(format!("unknown show option '{option}'")));
            }
            value if task_id.is_none() => task_id = Some(parse_task_id(value)?),
            value => {
                return Err(CliError::syntax(format!(
                    "unexpected show argument '{value}'"
                )));
            }
        }
    }
    Ok(CliCommand::Show {
        task_id: task_id.ok_or_else(|| CliError::syntax("download show requires a task ID"))?,
        watch,
        json,
    })
}

fn parse_action(action: CliAction, args: Vec<String>) -> Result<CliCommand, CliError> {
    let mut task_ids = Vec::new();
    let mut delete_file = false;
    let mut json = false;
    for argument in args {
        match argument.as_str() {
            "--json" => json = true,
            "--delete-file" if action == CliAction::Remove => delete_file = true,
            option if option.starts_with('-') => {
                return Err(CliError::syntax(format!(
                    "unknown action option '{option}'"
                )));
            }
            value => task_ids.push(parse_task_id(value)?),
        }
    }
    if task_ids.is_empty() {
        return Err(CliError::syntax(
            "download action requires at least one task ID",
        ));
    }
    Ok(CliCommand::Action {
        action,
        task_ids,
        delete_file,
        json,
    })
}

fn required_value<'a>(
    arguments: &'a [String],
    index: usize,
    option: &str,
) -> Result<&'a str, CliError> {
    arguments
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| CliError::syntax(format!("{option} requires a value")))
}

fn parse_task_id(value: &str) -> Result<DownloadTaskId, CliError> {
    Uuid::parse_str(value)
        .map(DownloadTaskId::from_uuid)
        .map_err(|_| CliError::syntax(format!("'{value}' is not a valid task ID")))
}

pub fn run(command: CliCommand, executable: &Path) -> Result<(), CliError> {
    run_with_host_policy(command, executable, CliHostPolicy::ConnectOrStart)
}

pub fn run_with_host_policy(
    command: CliCommand,
    executable: &Path,
    host_policy: CliHostPolicy,
) -> Result<(), CliError> {
    let json_output = command.json_output();
    let mut client = match connect_with_policy(executable, host_policy) {
        Ok(client) => client,
        Err(mut error) => {
            if json_output {
                print_json(
                    Uuid::new_v4(),
                    false,
                    None,
                    Some(json!({
                        "code": "host_unavailable",
                        "message": error.message
                    })),
                );
                error.message.clear();
            }
            return Err(error);
        }
    };
    run_connected_command(command, &mut client)
}

fn run_connected_command(
    command: CliCommand,
    client: &mut DownloadIpcClient,
) -> Result<(), CliError> {
    let json_output = command.json_output();
    let result = match command {
        CliCommand::Add {
            url,
            directory,
            directory_explicit,
            name,
            headers,
            idempotency_key,
            json,
        } => run_add(
            client,
            AddOptions {
                url,
                directory,
                directory_explicit,
                name,
                headers,
                idempotency_key,
                json_output: json,
            },
        ),
        CliCommand::List { filter, json } => run_list(client, filter, json),
        CliCommand::Show {
            task_id,
            watch,
            json,
        } => run_show(client, task_id, watch, json),
        CliCommand::Action {
            action,
            task_ids,
            delete_file,
            json,
        } => run_action(client, action, task_ids, delete_file, json),
        CliCommand::Schedule {
            queue_id,
            schedule,
            json,
        } => run_schedule(client, queue_id, schedule, json),
    };
    match result {
        Err(mut error) if json_output && !error.message.is_empty() => {
            print_json(
                Uuid::new_v4(),
                false,
                None,
                Some(json!({
                    "code": cli_error_code(error.exit_code),
                    "message": error.message
                })),
            );
            error.message.clear();
            Err(error)
        }
        result => result,
    }
}

impl CliCommand {
    fn json_output(&self) -> bool {
        match self {
            Self::Add { json, .. }
            | Self::List { json, .. }
            | Self::Show { json, .. }
            | Self::Action { json, .. }
            | Self::Schedule { json, .. } => *json,
        }
    }
}

fn run_schedule(
    client: &mut DownloadIpcClient,
    queue_id: String,
    schedule: Option<DownloadSchedule>,
    json_output: bool,
) -> Result<(), CliError> {
    let result = client
        .command(command_envelope(DownloadCommand::Queue {
            action: DownloadQueueAction::SetSchedule { queue_id, schedule },
        }))
        .map_err(CliError::host)?;
    let request_id = result.request_id;
    match result.outcome {
        DownloadCommandOutcome::QueueChanged { queue } => {
            if json_output {
                print_json(request_id, true, Some(json!({ "queue": queue })), None);
            } else if queue.schedule.is_some() {
                println!("已更新队列“{}”的每日计划", queue.name);
            } else {
                println!("已关闭队列“{}”的每日计划", queue.name);
            }
            Ok(())
        }
        DownloadCommandOutcome::Rejected { fault } => {
            Err(reported_fault_error(request_id, fault, json_output, false))
        }
        _ => Err(unexpected_result("schedule")),
    }
}

struct AddOptions {
    url: String,
    directory: PathBuf,
    directory_explicit: bool,
    name: Option<String>,
    headers: Vec<DownloadRequestHeader>,
    idempotency_key: Option<Uuid>,
    json_output: bool,
}

fn run_add(client: &mut DownloadIpcClient, options: AddOptions) -> Result<(), CliError> {
    let AddOptions {
        url,
        directory,
        directory_explicit,
        name,
        headers,
        idempotency_key,
        json_output,
    } = options;
    let request = DownloadRequest {
        source_url: url,
        save_directory: directory,
        save_directory_explicit: directory_explicit,
        file_name: name,
        headers,
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: None,
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    request
        .validate()
        .map_err(|error| CliError::syntax(error.to_string()))?;
    let result = client
        .command(command_envelope(DownloadCommand::Submit {
            request,
            mode: DownloadSubmitMode::StartNow,
            idempotency_key,
        }))
        .map_err(CliError::host)?;
    let request_id = result.request_id;
    match result.outcome {
        DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        }
        | DownloadCommandOutcome::IdempotentAccepted { task_id, .. } => {
            if json_output {
                print_json(request_id, true, Some(json!({ "task_id": task_id })), None);
            } else {
                println!("已创建下载任务 {}", task_id.as_uuid());
            }
            Ok(())
        }
        DownloadCommandOutcome::Rejected { fault } => {
            Err(reported_fault_error(request_id, fault, json_output, true))
        }
        _ => Err(CliError {
            exit_code: EXIT_OPERATION_FAILED,
            message: "download host returned an unexpected add result".into(),
        }),
    }
}

fn run_list(
    client: &mut DownloadIpcClient,
    filter: ListFilter,
    json_output: bool,
) -> Result<(), CliError> {
    let result = client
        .command(command_envelope(DownloadCommand::Snapshot {
            query: DownloadQuery {
                statuses: statuses_for_filter(filter),
                search: None,
                limit: None,
                compact: true,
            },
        }))
        .map_err(CliError::host)?;
    let request_id = result.request_id;
    match result.outcome {
        DownloadCommandOutcome::Snapshot(snapshot) => {
            let tasks = snapshot
                .task_summaries
                .iter()
                .map(safe_task_summary_projection)
                .collect::<Vec<_>>();
            if json_output {
                print_json(
                    request_id,
                    true,
                    Some(json!({
                        "generation": snapshot.generation,
                        "sequence": snapshot.sequence,
                        "tasks": tasks
                    })),
                    None,
                );
            } else {
                print_task_summary_table(&snapshot.task_summaries);
            }
            Ok(())
        }
        DownloadCommandOutcome::Rejected { fault } => {
            Err(reported_fault_error(request_id, fault, json_output, false))
        }
        _ => Err(unexpected_result("list")),
    }
}

fn run_show(
    client: &mut DownloadIpcClient,
    task_id: DownloadTaskId,
    watch: bool,
    json_output: bool,
) -> Result<(), CliError> {
    let (request_id, task, sequence) = match fetch_task(client, task_id) {
        Ok(task) => task,
        Err(mut error) => {
            if json_output {
                print_json(
                    Uuid::new_v4(),
                    false,
                    None,
                    Some(json!({
                        "code": if error.exit_code == EXIT_TASK_NOT_FOUND {
                            "not_found"
                        } else {
                            "show_failed"
                        },
                        "message": error.message
                    })),
                );
                error.message.clear();
            }
            return Err(error);
        }
    };
    if json_output {
        print_json(request_id, true, Some(safe_task_value(&task)), None);
    } else {
        print_task_detail(&task);
    }
    if !watch || is_watch_terminal(task.task.status()) {
        return Ok(());
    }
    let expected_generation = client
        .host_snapshot()
        .map(|snapshot| snapshot.engine_generation);
    let subscription = client
        .command(command_envelope(DownloadCommand::Subscribe {
            after_sequence: Some(sequence),
            generation: expected_generation,
        }))
        .map_err(CliError::host)?;
    match subscription.outcome {
        DownloadCommandOutcome::Accepted { .. } | DownloadCommandOutcome::Subscribed { .. } => {}
        DownloadCommandOutcome::Rejected { fault } => {
            return Err(error_from_fault(fault, false));
        }
        _ => return Err(unexpected_result("subscribe")),
    }
    while let Some(event) = client.next_event().map_err(CliError::host)? {
        if expected_generation
            .is_some_and(|generation| event.generation != 0 && event.generation != generation)
        {
            continue;
        }
        match event.event {
            DownloadEvent::TaskChanged { task } if task.id() == task_id => {
                if json_output {
                    println!(
                        "{}",
                        json!({
                            "schema_version": 1,
                            "event": "task_changed",
                            "sequence": event.sequence,
                            "task": safe_task_summary(&task)
                        })
                    );
                } else {
                    println!("{}  {}", task.id().as_uuid(), status_name(task.status()));
                }
                if is_watch_terminal(task.status()) {
                    return Ok(());
                }
            }
            DownloadEvent::TaskProgress { progress } if progress.task_id == task_id => {
                if json_output {
                    println!(
                        "{}",
                        json!({
                            "schema_version": 1,
                            "event": "task_progress",
                            "sequence": event.sequence,
                            "progress": progress
                        })
                    );
                } else {
                    println!(
                        "{} / {}  {}/s",
                        progress.downloaded_bytes,
                        progress
                            .total_bytes
                            .map_or_else(|| "?".into(), |bytes| bytes.to_string()),
                        progress.bytes_per_second
                    );
                }
            }
            DownloadEvent::TaskRemoved { task_id: removed } if removed == task_id => {
                return Err(CliError {
                    exit_code: EXIT_TASK_NOT_FOUND,
                    message: "task was removed while watching".into(),
                });
            }
            DownloadEvent::SnapshotRequired { .. } => {
                let mut refresh =
                    DownloadIpcClient::connect(DOWNLOAD_PIPE_NAME, Duration::from_secs(2))
                        .map_err(CliError::host)?;
                let (_, refreshed, _) = fetch_task(&mut refresh, task_id)?;
                if json_output {
                    println!(
                        "{}",
                        json!({
                            "schema_version": 1,
                            "event": "snapshot",
                            "task": safe_task_value(&refreshed)
                        })
                    );
                } else {
                    print_task_detail(&refreshed);
                }
                if is_watch_terminal(refreshed.task.status()) {
                    return Ok(());
                }
            }
            _ => {}
        }
    }
    Err(CliError {
        exit_code: EXIT_HOST_UNAVAILABLE,
        message: "download host disconnected while watching".into(),
    })
}

fn run_action(
    client: &mut DownloadIpcClient,
    action: CliAction,
    task_ids: Vec<DownloadTaskId>,
    delete_file: bool,
    json_output: bool,
) -> Result<(), CliError> {
    let engine_action = match action {
        CliAction::Pause => DownloadBatchAction::Pause,
        CliAction::Resume => DownloadBatchAction::Resume,
        CliAction::Retry => DownloadBatchAction::Retry,
        CliAction::Redownload => DownloadBatchAction::Redownload,
        CliAction::Cancel => DownloadBatchAction::Cancel,
        CliAction::Remove if delete_file => DownloadBatchAction::RemoveWithArtifact,
        CliAction::Remove => DownloadBatchAction::Remove,
    };
    let response = client
        .command(command_envelope(DownloadCommand::Batch {
            task_ids,
            action: engine_action,
        }))
        .map_err(CliError::host)?;
    let DownloadCommandOutcome::Batch {
        results: batch_results,
    } = response.outcome
    else {
        return Err(unexpected_result("action"));
    };
    let mut results = Vec::with_capacity(batch_results.len());
    let mut first_error = None;
    for result in batch_results {
        if let Some(fault) = result.fault {
            let error = error_from_fault(fault.clone(), false);
            first_error.get_or_insert(error);
            results.push(json!({
                "task_id": result.task_id,
                "ok": false,
                "error": safe_fault_value(&fault)
            }));
            if !json_output {
                eprintln!("{}：{}", result.task_id.as_uuid(), fault.message);
            }
        } else {
            results.push(json!({ "task_id": result.task_id, "ok": true }));
            if !json_output {
                println!("{}：已接受", result.task_id.as_uuid());
            }
        }
    }
    if json_output {
        let ok = first_error.is_none();
        print_json(
            Uuid::new_v4(),
            ok,
            Some(json!({ "tasks": results })),
            first_error
                .as_ref()
                .map(|error| json!({ "code": "batch_failed", "message": error.message })),
        );
    }
    if let Some(mut error) = first_error {
        error.message.clear();
        Err(error)
    } else {
        Ok(())
    }
}

fn fetch_task(
    client: &mut DownloadIpcClient,
    task_id: DownloadTaskId,
) -> Result<(Uuid, DownloadEngineTask, u64), CliError> {
    let snapshot_result = client
        .command(command_envelope(DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        }))
        .map_err(CliError::host)?;
    let DownloadCommandOutcome::Snapshot(snapshot) = snapshot_result.outcome else {
        return Err(unexpected_result("show"));
    };
    if !snapshot
        .task_summaries
        .iter()
        .any(|summary| summary.task_id == task_id)
    {
        return Err(CliError {
            exit_code: EXIT_TASK_NOT_FOUND,
            message: format!("download task {} was not found", task_id.as_uuid()),
        });
    }
    let detail_result = client
        .command(command_envelope(DownloadCommand::TaskDetail { task_id }))
        .map_err(CliError::host)?;
    let request_id = detail_result.request_id;
    let DownloadCommandOutcome::TaskDetail(detail) = detail_result.outcome else {
        return Err(unexpected_result("show detail"));
    };
    Ok((request_id, detail.task, snapshot.sequence))
}

fn connect_with_policy(
    executable: &Path,
    host_policy: CliHostPolicy,
) -> Result<DownloadIpcClient, CliError> {
    connect_with_policy_named(executable, host_policy, DOWNLOAD_PIPE_NAME)
}

fn connect_with_policy_named(
    executable: &Path,
    host_policy: CliHostPolicy,
    pipe_name: &str,
) -> Result<DownloadIpcClient, CliError> {
    let initial_error = match DownloadIpcClient::connect(pipe_name, Duration::from_millis(150)) {
        Ok(client) => return Ok(client),
        Err(error) => error,
    };
    if host_policy == CliHostPolicy::ConnectOnly {
        return Err(CliError::host(initial_error));
    }
    let host = host_executable_for(executable);
    start_background_host(&host).map_err(CliError::host)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_error = None;
    while Instant::now() < deadline {
        match DownloadIpcClient::connect(pipe_name, Duration::from_millis(250)) {
            Ok(client) => return Ok(client),
            Err(error) => last_error = Some(error),
        }
        std::thread::sleep(Duration::from_millis(75));
    }
    Err(CliError::host(last_error.unwrap_or_else(|| {
        anyhow::anyhow!("download host did not become available within five seconds")
    })))
}

pub fn host_executable_for(cli_executable: &Path) -> PathBuf {
    cli_executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("NxTools.exe")
}

fn start_background_host(host: &Path) -> Result<()> {
    if !host.is_file() {
        anyhow::bail!("packaged download host was not found at {}", host.display());
    }
    let mut command = Command::new(host);
    command
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .with_context(|| format!("failed to start download host {}", host.display()))?;
    Ok(())
}

fn statuses_for_filter(filter: ListFilter) -> Vec<DownloadStatus> {
    match filter {
        ListFilter::All => Vec::new(),
        ListFilter::Active => vec![
            DownloadStatus::Created,
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
            DownloadStatus::Verifying,
        ],
        ListFilter::Completed => vec![DownloadStatus::Completed],
        ListFilter::Failed => vec![DownloadStatus::Failed],
    }
}

fn safe_task_value(task: &DownloadEngineTask) -> Value {
    json!({
        "task": safe_task_summary(&task.task),
        "downloaded_bytes": task.downloaded_bytes,
        "total_bytes": task.total_bytes,
        "final_path": task.final_path,
        "temporary_path": task.temporary_path,
        "error": task.error.as_ref().map(safe_fault_value)
    })
}

fn safe_task_summary_projection(summary: &DownloadTaskSummary) -> Value {
    json!({
        "id": summary.task_id,
        "source_url": summary.source_url,
        "file_name": summary.file_name,
        "status": summary.status,
        "origin": summary.origin,
        "created_at_unix_ms": summary.created_at_unix_ms,
        "updated_at_unix_ms": summary.updated_at_unix_ms,
        "completed_at_unix_ms": summary.completed_at_unix_ms,
        "downloaded_bytes": summary.downloaded_bytes,
        "total_bytes": summary.total_bytes,
        "error": summary.error.as_ref().map(safe_fault_value),
        "queue_eligible": summary.queue_eligible,
        "has_final_artifact": summary.has_final_artifact
    })
}

fn safe_task_summary(task: &nxtools_download::DownloadTask) -> Value {
    json!({
        "id": task.id(),
        "source_url": task.request().source_url,
        "save_directory": task.request().save_directory,
        "file_name": task.request().file_name,
        "status": task.status(),
        "origin": task.origin(),
        "created_at_unix_ms": task.created_at_unix_ms(),
        "updated_at_unix_ms": task.updated_at_unix_ms(),
        "completed_at_unix_ms": task.completed_at_unix_ms()
    })
}

fn safe_fault_value(fault: &DownloadFault) -> Value {
    json!({
        "code": fault.code,
        "message": fault.message,
        "retryable": fault.retryable
    })
}

fn print_json(request_id: Uuid, ok: bool, result: Option<Value>, error: Option<Value>) {
    #[derive(Serialize)]
    struct Envelope {
        schema_version: u16,
        request_id: Uuid,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<Value>,
    }
    println!(
        "{}",
        serde_json::to_string(&Envelope {
            schema_version: 1,
            request_id,
            ok,
            result,
            error,
        })
        .unwrap_or_else(|_| "{\"schema_version\":1,\"ok\":false}".into())
    );
}

// This fault has already reached stdout/stderr; callers must not emit it again.
fn reported_fault_error(
    request_id: Uuid,
    fault: DownloadFault,
    json_output: bool,
    request_rejected: bool,
) -> CliError {
    print_fault(request_id, &fault, json_output);
    let mut error = error_from_fault(fault, request_rejected);
    error.message.clear();
    error
}

fn print_fault(request_id: Uuid, fault: &DownloadFault, json_output: bool) {
    if json_output {
        print_json(request_id, false, None, Some(safe_fault_value(fault)));
    } else {
        eprintln!("{}", fault.message);
    }
}

fn summary_task_name(task: &DownloadTaskSummary) -> String {
    task.file_name.clone().unwrap_or_else(|| {
        url::Url::parse(&task.source_url)
            .ok()
            .and_then(|url| url.path_segments()?.next_back().map(str::to_owned))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| task.source_url.clone())
    })
}

fn print_task_summary_table(tasks: &[DownloadTaskSummary]) {
    if tasks.is_empty() {
        println!("没有下载任务");
        return;
    }
    println!("{:<36}  {:<10}  文件", "ID", "状态");
    for task in tasks {
        println!(
            "{:<36}  {:<10}  {}",
            task.task_id.as_uuid(),
            status_name(task.status),
            summary_task_name(task)
        );
    }
}

fn print_task_detail(task: &DownloadEngineTask) {
    println!("ID: {}", task.task.id().as_uuid());
    println!("状态: {}", status_name(task.task.status()));

    println!("文件: {}", task_name(task));
    println!("来源: {}", task.task.request().source_url);
    println!(
        "进度: {} / {}",
        task.downloaded_bytes,
        task.total_bytes
            .map_or_else(|| "未知".into(), |bytes| bytes.to_string())
    );
    if let Some(path) = &task.final_path {
        println!("目标: {}", path.display());
    }
    if let Some(error) = &task.error {
        println!("错误: {}", error.message);
    }
}

fn task_name(task: &DownloadEngineTask) -> String {
    task.task
        .request()
        .file_name
        .clone()
        .or_else(|| {
            url::Url::parse(&task.task.request().source_url)
                .ok()
                .and_then(|url| url.path_segments()?.next_back().map(str::to_owned))
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| task.task.request().source_url.clone())
}

fn status_name(status: DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Created => "已创建",
        DownloadStatus::Probing => "探测中",
        DownloadStatus::Waiting => "等待",
        DownloadStatus::Downloading => "下载中",
        DownloadStatus::Paused => "已暂停",
        DownloadStatus::Finalizing => "落盘中",
        DownloadStatus::Verifying => "校验中",
        DownloadStatus::Completed => "已完成",
        DownloadStatus::Failed => "失败",
        DownloadStatus::Cancelled => "已取消",
    }
}

fn is_watch_terminal(status: DownloadStatus) -> bool {
    matches!(
        status,
        DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
    )
}

fn error_from_fault(fault: DownloadFault, request_rejected: bool) -> CliError {
    let exit_code = match fault.code {
        DownloadFaultCode::NotFound => EXIT_TASK_NOT_FOUND,
        DownloadFaultCode::InvalidState => EXIT_INVALID_STATE,
        DownloadFaultCode::Protocol => EXIT_PROTOCOL_INCOMPATIBLE,
        DownloadFaultCode::Unavailable => EXIT_HOST_UNAVAILABLE,
        DownloadFaultCode::InvalidRequest if request_rejected => EXIT_REQUEST_REJECTED,
        _ if request_rejected => EXIT_REQUEST_REJECTED,
        _ => EXIT_OPERATION_FAILED,
    };
    CliError {
        exit_code,
        message: fault.message,
    }
}

fn cli_error_code(exit_code: u8) -> &'static str {
    match exit_code {
        EXIT_SYNTAX => "invalid_syntax",
        EXIT_HOST_UNAVAILABLE => "host_unavailable",
        EXIT_TASK_NOT_FOUND => "not_found",
        EXIT_INVALID_STATE => "invalid_state",
        EXIT_REQUEST_REJECTED => "request_rejected",
        EXIT_OPERATION_FAILED => "operation_failed",
        EXIT_PROTOCOL_INCOMPATIBLE => "protocol_incompatible",
        _ => "unknown",
    }
}

fn unexpected_result(command: &str) -> CliError {
    CliError {
        exit_code: EXIT_OPERATION_FAILED,
        message: format!("download host returned an unexpected {command} result"),
    }
}

pub fn default_download_directory() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|home| home.join("Downloads"))
        .unwrap_or_else(|| PathBuf::from("Downloads"))
}

pub fn usage() -> &'static str {
    "用法：
  NxTools.Cli.exe [--connect-only] download add <url> [--dir <path>] [--name <name>] [--header <k:v>] [--idempotency-key <uuid>] [--json]
  NxTools.Cli.exe download list [--active|--completed|--failed] [--json]
  NxTools.Cli.exe download show <id> [--watch] [--json]
  NxTools.Cli.exe download pause|resume|retry|redownload|cancel <id...> [--json]
  NxTools.Cli.exe download remove <id...> [--delete-file] [--json]
  NxTools.Cli.exe download schedule [--queue <id>] [--start HH:MM] [--stop HH:MM] [--days mon,tue,...] [--json]
  NxTools.Cli.exe download schedule [--queue <id>] --disable [--json]

全局选项：--connect-only 只连接已运行的下载宿主，连接失败时不会启动后台桌面。"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_only_is_an_explicit_global_invocation_policy() {
        let invocation = parse_invocation(
            ["--connect-only", "download", "list", "--json"],
            PathBuf::new(),
        )
        .expect("parse connect-only invocation");
        assert_eq!(invocation.host_policy, CliHostPolicy::ConnectOnly);
        assert!(matches!(
            invocation.command,
            CliCommand::List { json: true, .. }
        ));
    }

    #[test]
    fn connect_only_never_crosses_the_background_host_start_boundary() {
        let directory = tempfile::tempdir().expect("create connect-only fixture");
        let cli = directory.path().join("NxTools.Cli.exe");
        let pipe_name = format!("nxtools-connect-only-test-{}", Uuid::new_v4());

        let error = match connect_with_policy_named(&cli, CliHostPolicy::ConnectOnly, &pipe_name) {
            Ok(_) => panic!("unique pipe must be unavailable"),
            Err(error) => error,
        };

        assert_eq!(error.exit_code, EXIT_HOST_UNAVAILABLE);
        assert!(!error.message.contains("packaged download host"));
        assert!(!directory.path().join("NxTools.exe").exists());
    }

    #[test]
    fn add_parses_repeated_headers_without_exposing_them_to_output_types() {
        let parsed = parse_args(
            [
                "download",
                "add",
                "https://example.com/file.bin",
                "--dir",
                r"D:\Downloads",
                "--header",
                "Referer:https://example.com",
                "--header",
                "Authorization:Bearer secret",
                "--idempotency-key",
                "12345678-1234-5678-1234-567812345678",
                "--json",
            ],
            PathBuf::from(r"C:\Downloads"),
        )
        .expect("parse add");
        let CliCommand::Add {
            directory,
            headers,
            idempotency_key,
            json,
            ..
        } = parsed
        else {
            panic!("expected add");
        };
        assert_eq!(
            idempotency_key,
            Some(
                Uuid::parse_str("12345678-1234-5678-1234-567812345678")
                    .expect("fixture idempotency key is valid"),
            )
        );
        assert_eq!(directory, PathBuf::from(r"D:\Downloads"));
        assert_eq!(headers.len(), 2);
        assert!(json);
    }

    #[test]
    fn list_rejects_conflicting_filters() {
        let error = parse_args(["download", "list", "--active", "--failed"], PathBuf::new())
            .expect_err("conflicting filters must fail");
        assert_eq!(error.exit_code, EXIT_SYNTAX);
    }

    #[test]
    fn invalid_add_request_fails_before_any_host_is_needed() {
        let error = parse_args(
            ["download", "add", "ftp://example.com/file.bin"],
            PathBuf::from(r"C:\Downloads"),
        )
        .expect_err("unsupported URL must fail during parsing");
        assert_eq!(error.exit_code, EXIT_SYNTAX);
    }

    #[test]
    fn schedule_parses_cross_midnight_weekdays_and_disable_mode() {
        let parsed = parse_args(
            [
                "download", "schedule", "--start", "22:30", "--stop", "02:15", "--days", "fri,sat",
            ],
            PathBuf::new(),
        )
        .expect("parse daily schedule");
        let CliCommand::Schedule {
            queue_id,
            schedule: Some(schedule),
            ..
        } = parsed
        else {
            panic!("daily schedule expected");
        };
        assert_eq!(queue_id, "default");
        let daily = schedule.daily;
        assert_eq!(daily.start_minute, 22 * 60 + 30);
        assert_eq!(daily.stop_minute, 2 * 60 + 15);
        assert_eq!(
            daily.days,
            vec![DownloadWeekday::Friday, DownloadWeekday::Saturday]
        );

        let parsed =
            parse_args(["schedule", "--disable"], PathBuf::new()).expect("parse schedule disable");
        assert!(matches!(
            parsed,
            CliCommand::Schedule { schedule: None, .. }
        ));
    }

    #[test]
    fn batch_action_requires_valid_task_ids() {
        let error = parse_args(["download", "pause", "not-a-task"], PathBuf::new())
            .expect_err("invalid task ID must fail");
        assert_eq!(error.exit_code, EXIT_SYNTAX);
    }

    #[test]
    fn explicit_retry_and_redownload_actions_are_distinct_from_resume() {
        for (verb, expected) in [
            ("resume", CliAction::Resume),
            ("retry", CliAction::Retry),
            ("redownload", CliAction::Redownload),
        ] {
            let parsed = parse_args(
                ["download", verb, "12345678-1234-5678-1234-567812345678"],
                PathBuf::new(),
            )
            .expect("parse explicit task action");
            let CliCommand::Action { action, .. } = parsed else {
                panic!("task action expected");
            };
            assert_eq!(action, expected);
        }
    }

    #[test]
    fn fault_exit_codes_are_stable() {
        let fault = |code| DownloadFault {
            code,
            message: "fixture".into(),
            retryable: false,
        };
        assert_eq!(
            error_from_fault(fault(DownloadFaultCode::NotFound), false).exit_code,
            EXIT_TASK_NOT_FOUND
        );
        assert_eq!(
            error_from_fault(fault(DownloadFaultCode::InvalidState), false).exit_code,
            EXIT_INVALID_STATE
        );
        assert_eq!(
            error_from_fault(fault(DownloadFaultCode::Network), false).exit_code,
            EXIT_OPERATION_FAILED
        );
        assert_eq!(
            error_from_fault(fault(DownloadFaultCode::InvalidRequest), true).exit_code,
            EXIT_REQUEST_REJECTED
        );
    }

    #[test]
    fn cli_finds_the_host_beside_itself() {
        assert_eq!(
            host_executable_for(Path::new(r"C:\NxTools\NxTools.Cli.exe")),
            PathBuf::from(r"C:\NxTools\NxTools.exe")
        );
    }
}

#[cfg(test)]
mod output_tests;
