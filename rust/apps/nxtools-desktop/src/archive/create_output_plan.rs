//! Complete each-input output naming before the original batch/FIFO admission.
use super::*;
use crate::archive::forms::FormCompletion;
use nxtools_protocol::{
    MAX_FRAME_BYTES, PlatformCommand, PlatformCommandResult,
    archive_create_destination::{ArchiveCreateDestinationCommand, ArchiveCreateDestinationResult},
    archive_job::{ArchiveJobFormat, ArchiveJobKind, ArchiveJobRequest, valid_job_name},
};

pub(super) struct Pending {
    command: ArchiveCreateDestinationCommand,
    connection: u64,
    request: ArchiveJobRequest,
    completion: FormCompletion,
    started: Instant,
    _deadline: Task<()>,
}

#[derive(Clone)]
struct OutputName {
    name: String,
    format: ArchiveJobFormat,
    split: bool,
}

impl OutputName {
    fn from_request(request: &ArchiveJobRequest) -> Result<Self, String> {
        let create = request
            .create
            .as_ref()
            .filter(|_| request.kind == ArchiveJobKind::Create)
            .ok_or_else(|| "分别压缩输出计划只接受创建请求。".to_owned())?;
        Ok(Self {
            name: create.name.clone(),
            format: create.format,
            split: create.volume_bytes.is_some(),
        })
    }

    fn owns(&self, candidate: &str) -> bool {
        let candidate = candidate.to_lowercase();
        let name = self.name.to_lowercase();
        if name == candidate {
            return true;
        }
        if !self.split {
            return false;
        }
        let prefix = match self.format {
            ArchiveJobFormat::Zip => {
                let stem = name
                    .rsplit_once('.')
                    .map_or(name.as_str(), |(stem, _)| stem);
                format!("{stem}.z")
            }
            ArchiveJobFormat::SevenZ => format!("{name}."),
            _ => return false,
        };
        candidate.strip_prefix(&prefix).is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
    }
    fn overlaps(&self, other: &Self) -> bool {
        self.owns(&other.name) || other.owns(&self.name)
    }
}

fn expected_extension(format: ArchiveJobFormat) -> &'static str {
    match format {
        ArchiveJobFormat::Zip => ".zip",
        ArchiveJobFormat::SevenZ => ".7z",
        ArchiveJobFormat::Tar => ".tar",
        ArchiveJobFormat::Tgz => ".tar.gz",
        ArchiveJobFormat::Txz => ".tar.xz",
        ArchiveJobFormat::Gz => ".gz",
        ArchiveJobFormat::Xz => ".xz",
    }
}

fn numbered_name(output: &OutputName, number: usize) -> Option<String> {
    let extension = expected_extension(output.format);
    let lower = output.name.to_lowercase();
    let (stem, extension) = if lower.ends_with(extension) {
        output.name.split_at(output.name.len() - extension.len())
    } else if let Some((stem, _)) = output.name.rsplit_once('.') {
        (stem, &output.name[stem.len()..])
    } else {
        (output.name.as_str(), "")
    };
    let suffix = format!(" ({number})");
    let maximum = 255usize.checked_sub(suffix.len() + extension.len())?;
    let mut boundary = stem.len().min(maximum);
    while boundary > 0 && !stem.is_char_boundary(boundary) {
        boundary -= 1;
    }
    let stem = stem[..boundary].trim_end_matches(['.', ' ']);
    if stem.is_empty() {
        return None;
    }
    let candidate = format!("{stem}{suffix}{extension}");
    valid_job_name(&candidate).then_some(candidate)
}

struct DiskNames {
    exact: std::collections::HashSet<String>,
    zip_parts: std::collections::HashSet<String>,
    sevenz_parts: std::collections::HashSet<String>,
}

impl DiskNames {
    fn new(existing: &[String]) -> Self {
        let mut names = Self {
            exact: std::collections::HashSet::with_capacity(existing.len()),
            zip_parts: std::collections::HashSet::new(),
            sevenz_parts: std::collections::HashSet::new(),
        };
        for name in existing {
            let name = name.to_lowercase();
            if let Some((stem, suffix)) = name.rsplit_once('.') {
                let numbered =
                    |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
                if suffix.strip_prefix('z').is_some_and(numbered) {
                    names.zip_parts.insert(stem.to_owned());
                }
                if numbered(suffix) {
                    names.sevenz_parts.insert(stem.to_owned());
                }
            }
            names.exact.insert(name);
        }
        names
    }

    fn conflicts(&self, output: &OutputName) -> bool {
        let name = output.name.to_lowercase();
        self.exact.contains(&name)
            || output.split
                && match output.format {
                    ArchiveJobFormat::Zip => {
                        let stem = name
                            .rsplit_once('.')
                            .map_or(name.as_str(), |(stem, _)| stem);
                        self.zip_parts.contains(stem)
                    }
                    ArchiveJobFormat::SevenZ => self.sevenz_parts.contains(&name),
                    _ => false,
                }
    }
}

pub(super) fn plan_output_names(
    requests: &mut [ArchiveJobRequest],
    existing: &[String],
) -> Result<(), String> {
    if requests.len() < 2 {
        return Ok(());
    }
    let destination = requests
        .first()
        .and_then(|request| request.destination.as_deref())
        .ok_or_else(|| "分别压缩缺少目标文件夹。".to_owned())?;
    if requests.iter().any(|request| {
        request.destination.as_deref() != Some(destination) || request.validate().is_err()
    }) {
        return Err("分别压缩批次的目标或格式选项不一致。".into());
    }
    let requested = requests
        .iter()
        .map(OutputName::from_request)
        .collect::<Result<Vec<_>, _>>()?;
    let mut assigned = Vec::<OutputName>::with_capacity(requested.len());
    let limit = existing
        .len()
        .checked_add(requested.len())
        .and_then(|value| value.checked_add(2))
        .ok_or_else(|| "目标名称数量超过规划上限。".to_owned())?;
    let disk = DiskNames::new(existing);
    for (index, original) in requested.iter().enumerate() {
        let direct = !disk.conflicts(original)
            && assigned.iter().all(|accepted| !original.overlaps(accepted));
        let selected = if direct {
            original.clone()
        } else {
            let mut selected = None;
            for number in 2..=limit {
                let Some(name) = numbered_name(original, number) else {
                    break;
                };
                let candidate = OutputName {
                    name,
                    format: original.format,
                    split: original.split,
                };
                if !disk.conflicts(&candidate)
                    && assigned
                        .iter()
                        .all(|accepted| !candidate.overlaps(accepted))
                    && requested
                        .iter()
                        .all(|reserved| !candidate.overlaps(reserved))
                {
                    selected = Some(candidate);
                    break;
                }
            }
            selected.ok_or_else(|| "无法为分别压缩分配不冲突的目标名称。".to_owned())?
        };
        requests[index]
            .create
            .as_mut()
            .expect("validated create request")
            .name = selected.name.clone();
        assigned.push(selected);
    }
    Ok(())
}

impl ArchiveApplication {
    fn create_output_form_current(&self, session: Uuid, generation: u64, cx: &App) -> bool {
        self.jobs
            .forms
            .get(&session)
            .filter(|slot| slot.generation == generation)
            .and_then(|slot| slot.view.upgrade())
            .is_some_and(|view| {
                view.read_with(cx, |form, _| form.output_plan_current(session, generation))
            })
    }

    fn create_output_form_error(
        &self,
        session: Uuid,
        generation: u64,
        message: String,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self
            .jobs
            .forms
            .get(&session)
            .filter(|slot| slot.generation == generation)
        else {
            return;
        };
        let view = slot.view.clone();
        let window = slot.window;
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                let _ = view.update(cx, |form, cx| {
                    if form.output_plan_current(session, generation) {
                        form.fail_submit(message, window, cx);
                    }
                });
            });
        });
    }

    pub(in crate::archive) fn plan_create_outputs(
        &mut self,
        form: Uuid,
        generation: u64,
        request: ArchiveJobRequest,
        completion: FormCompletion,
        cx: &mut Context<Self>,
    ) -> Option<(ArchiveJobRequest, FormCompletion)> {
        if request.kind != ArchiveJobKind::Create || completion.additional_requests.is_empty() {
            return Some((request, completion));
        }
        let batch_valid = request.validate().is_ok()
            && completion.additional_requests.iter().all(|additional| {
                additional.kind == ArchiveJobKind::Create
                    && additional.destination == request.destination
                    && additional.validate().is_ok()
            });
        if !batch_valid {
            self.create_output_form_error(
                form,
                generation,
                "分别压缩批次的目标或格式选项不一致。".into(),
                cx,
            );
            return None;
        }
        let destination = request.destination.clone();
        let operation = Uuid::new_v4();
        let command = destination.map(|destination| ArchiveCreateDestinationCommand {
            version: 1,
            operation_id: operation,
            session_id: form,
            generation,
            destination,
        });
        let connection = self.platform.connection_generation();
        let current = self.create_output_form_current(form, generation, cx);
        let duplicate = self
            .jobs
            .create_plans
            .values()
            .any(|pending| pending.command.session_id == form);
        let Some(command) = command.filter(|command| command.validate().is_ok()) else {
            self.create_output_form_error(
                form,
                generation,
                "分别压缩缺少有效目标文件夹。".into(),
                cx,
            );
            return None;
        };
        if !current || connection.is_none() || duplicate || self.jobs.create_plans.len() >= 8 {
            self.create_output_form_error(
                form,
                generation,
                "目标文件夹观察不可用或已有请求正在进行，请重试。".into(),
                cx,
            );
            return None;
        }
        let deadline = cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(7))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.create_output_result(
                    operation,
                    &PlatformCommandResult {
                        success: false,
                        error: Some("destination_timeout".into()),
                        native_error_code: None,
                        data: None,
                    },
                    cx,
                );
            });
        });
        self.jobs.create_plans.insert(
            operation,
            Pending {
                command: command.clone(),
                connection: connection.expect("connected"),
                request,
                completion,
                started: Instant::now(),
                _deadline: deadline,
            },
        );
        tracing::info!(event="archive.create_output_plan.accepted",correlation_id=%operation,session_id=%form,generation,"create output observation accepted");
        if self
            .platform
            .send_command_with_id_correlated(
                PlatformCommand::ArchiveCreateDestination(command),
                operation,
                operation,
            )
            .is_err()
        {
            self.create_output_result(
                operation,
                &PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
        None
    }

    pub(super) fn create_output_result(
        &mut self,
        operation: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.jobs.create_plans.remove(&operation) else {
            return false;
        };
        let response = result
            .data
            .as_deref()
            .filter(|data| data.len() <= MAX_FRAME_BYTES)
            .and_then(|data| serde_json::from_str::<ArchiveCreateDestinationResult>(data).ok())
            .filter(|response| response.validate(&pending.command).is_ok());
        let current = self.platform.connection_generation() == Some(pending.connection)
            && self.create_output_form_current(
                pending.command.session_id,
                pending.command.generation,
                cx,
            );
        let outcome = if !current {
            Err("连接或原创建表单已变化；未提交任何子任务。".into())
        } else if result.success {
            response
                .filter(|response| response.code.is_none())
                .map(|response| response.existing_names)
                .ok_or_else(|| "目标文件夹观察结果无效或超过预算。".into())
        } else {
            Err(match result.error.as_deref() {
                Some("destination_timeout" | "platform command timed out") => {
                    "目标文件夹观察超时，请重试。"
                }
                Some("destination_limit") => "目标文件夹项目过多，无法安全规划本批名称。",
                Some("cancelled") => "目标文件夹观察已取消。",
                _ => "无法观察目标文件夹，请检查访问权限后重试。",
            }
            .into())
        };
        tracing::info!(event="archive.create_output_plan.finished",correlation_id=%operation,session_id=%pending.command.session_id,generation=pending.command.generation,success=outcome.is_ok(),native_error_code=?result.native_error_code,elapsed_ms=pending.started.elapsed().as_millis() as u64,"create output observation finished");
        let owner = cx.entity().downgrade();
        let form = pending.command.session_id;
        let generation = pending.command.generation;
        let connection = pending.connection;
        let request = pending.request;
        let mut completion = pending.completion;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                if app.platform.connection_generation() != Some(connection)
                    || !app.create_output_form_current(form, generation, cx)
                {
                    app.create_output_form_error(
                        form,
                        generation,
                        "连接或原创建表单已变化；未提交任何子任务。".into(),
                        cx,
                    );
                    return;
                }
                match outcome.and_then(|existing| {
                    let mut requests = vec![request];
                    requests.append(&mut completion.additional_requests);
                    plan_output_names(&mut requests, &existing)?;
                    Ok(requests)
                }) {
                    Ok(mut requests) => {
                        let request = requests.remove(0);
                        completion.additional_requests = requests;
                        app.submit_form_admitted(form, generation, request, completion, cx);
                    }
                    Err(message) => {
                        app.create_output_form_error(form, generation, message, cx);
                    }
                }
            });
        });
        true
    }

    pub(super) fn create_output_disconnected(&mut self, generation: u64, cx: &mut Context<Self>) {
        let operations = self
            .jobs
            .create_plans
            .iter()
            .filter(|(_, pending)| pending.connection == generation)
            .map(|(operation, _)| *operation)
            .collect::<Vec<_>>();
        for operation in operations {
            self.create_output_result(
                operation,
                &PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }

    pub(super) fn retire_create_output_plan(&mut self, form: Uuid) {
        self.jobs.create_plans.retain(|operation, pending| {
            if pending.command.session_id != form {
                return true;
            }
            tracing::info!(event="archive.create_output_plan.finished",correlation_id=%operation,session_id=%form,generation=pending.command.generation,success=false,reason="form_retired",elapsed_ms=pending.started.elapsed().as_millis() as u64,"create output observation retired");
            false
        });
    }
}
