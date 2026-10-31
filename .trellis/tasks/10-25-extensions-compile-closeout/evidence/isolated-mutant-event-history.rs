//! Bounded metadata only: never retains an event payload or plugin-provided output.
use nxtools_extensions::{InvokeOutcome, RuntimeError};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EventOutcome {
    Succeeded,
    NotHandled,
    HandlerFailed,
    Timeout,
    Untrusted,
    Busy,
    Unavailable,
    Protocol,
    Cancelled,
}
impl EventOutcome {
    pub(crate) fn from_result(result: &Result<InvokeOutcome, RuntimeError>) -> Self {
        match result {
            Ok(outcome) => match outcome.status.as_str() {
                "success" | "handled" => Self::Succeeded,
                "not_handled" => Self::NotHandled,
                _ => Self::HandlerFailed,
            },
            Err(RuntimeError::Timeout) => Self::Timeout,
            Err(RuntimeError::Untrusted | RuntimeError::Stale) => Self::Untrusted,
            Err(RuntimeError::QueueFull) => Self::Busy,
            Err(RuntimeError::UnsupportedTransport) => Self::Succeeded,
            Err(
                RuntimeError::NativeUnavailable
                | RuntimeError::HandlerUnavailable
                | RuntimeError::InvalidSettings,
            ) => Self::Unavailable,
            Err(RuntimeError::Cancelled) => Self::Cancelled,
            Err(RuntimeError::Protocol) => Self::Protocol,
            Err(RuntimeError::HandlerFailed) => Self::HandlerFailed,
        }
    }
    pub(crate) fn success(self) -> bool {
        matches!(self, Self::Succeeded | Self::NotHandled)
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Succeeded => "处理完成",
            Self::NotHandled => "未处理 / 无需修改",
            Self::HandlerFailed => "插件报告失败：请检查插件设置和输入条件",
            Self::Timeout => "处理超时：请检查运行环境或外部服务",
            Self::Untrusted => "信任或内容已变化：请重新检查并信任",
            Self::Busy => "运行队列繁忙：等待空闲后恢复",
            Self::Unavailable => "运行通道或动作不可用：请检查运行环境",
            Self::Protocol => "插件协议无效：请检查插件版本或联系作者",
            Self::Cancelled => "调用已取消，外部效果不能撤回",
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct EventTerminal {
    pub(crate) event_id: Uuid,
    pub(crate) correlation_id: Uuid,
    pub(crate) package_id: String,
    pub(crate) handler_id: String,
    pub(crate) at_unix_ms: i64,
    pub(crate) elapsed_ms: u128,
    pub(crate) outcome: EventOutcome,
}
impl EventTerminal {
    pub(crate) fn summary(&self) -> String {
        let time = chrono::DateTime::from_timestamp_millis(self.at_unix_ms)
            .map(|time| {
                time.with_timezone(&chrono::Local)
                    .format("%H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "时间不可用".into());
        format!(
            "{time} · {} · {} 毫秒 · 关联 ID {}",
            self.outcome.label(),
            self.elapsed_ms,
            self.correlation_id
        )
    }
}
