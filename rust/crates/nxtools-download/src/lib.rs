use std::path::PathBuf;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use uuid::Uuid;

mod artifact;
mod capacity;
mod dash;
mod engine;
mod ffmpeg_install;
mod hls;
mod http;
mod media;
mod message;
mod organization;
mod rate_limit;
mod request_policy;
mod storage;

pub use artifact::{ArtifactAllocation, SegmentWriter, SegmentedArtifact, SegmentedArtifactError};
pub use capacity::{DownloadStorageCapacityError, DownloadStorageRequirement};
pub use dash::{
    MAX_DASH_FIELD_BYTES, MAX_DASH_MANIFEST_BYTES, MAX_DASH_PROBE_SERIALIZED_BYTES,
    MAX_DASH_REPRESENTATIONS, MAX_DASH_SEGMENTS, MAX_DASH_URL_BYTES,
};
pub use engine::{
    BrowserDownloadDialogRequest, BrowserDownloadDialogSource, DownloadEngine, DownloadEngineError,
    DownloadEngineTask, DownloadScheduleMoment, daily_schedule_window_contains,
};
pub use ffmpeg_install::{
    FfmpegInstallControl, FfmpegInstallFailure, FfmpegInstallOperation, FfmpegInstallPhase,
    FfmpegInstallProgress, FfmpegInstallSource, FfmpegInstallation,
};
pub use hls::{
    MAX_HLS_ATTRIBUTE_BYTES, MAX_HLS_PROBE_SERIALIZED_BYTES, MAX_HLS_VARIANT_URL_BYTES,
    MAX_HLS_VARIANTS,
};
pub use http::{
    DownloadSegmentTransferControl, DownloadSegmentTransferOutcome,
    DownloadSegmentTransferProgress, DownloadSegmentTransferRequest, DownloadTransferContext,
    DownloadTransferControl, DownloadTransferOutcome, DownloadTransferProgress, HttpDownloadClient,
    HttpDownloadError,
};
pub use media::{
    FFMPEG_BTBN_DOWNLOAD_URL, FFMPEG_GYAN_DOWNLOAD_URL, FfmpegResolution, FfmpegResolutionSource,
    resolve_ffmpeg_executable,
};
pub use message::{
    BrowserBatchSubmissionResult, BrowserBlob, BrowserDownloadSubmission,
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DashProbe, DashTrackProbe, DownloadAggregateMetrics,
    DownloadBatchAction, DownloadBatchItemResult, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadCommandResult, DownloadEvent, DownloadEventEnvelope,
    DownloadEventSequenceError, DownloadEventSequencer, DownloadFault, DownloadFaultCode,
    DownloadMediaProbe, DownloadProbe, DownloadProgress, DownloadQuery, DownloadQueueAction,
    DownloadQueuePlacement, DownloadSegmentProgress, DownloadSnapshot, DownloadSubmitMode,
    DownloadTaskAction, DownloadTaskDetail, DownloadTaskSummary, DownloadTaskTimelineEntry,
    DownloadTaskTimelineEvent, HlsProbe, HlsVariantProbe, MAX_BROWSER_BATCH_ITEMS,
};
pub use nxtools_network::{NetworkOperationSnapshot, NetworkPolicy, NetworkPolicyError};
pub use organization::{DownloadOrganizationPolicy, DownloadOrganizationPolicyError};
pub use rate_limit::{DownloadRateLimitError, DownloadRateLimiter};
pub use request_policy::{
    DownloadHttpPolicy, DownloadHttpPolicyError, MAX_DEFAULT_COOKIE_BYTES,
    MAX_DEFAULT_HEADER_COUNT, MAX_DEFAULT_HEADER_TEXT_BYTES, is_download_transport_owned_header,
};
pub use storage::{DOWNLOAD_SCHEMA_VERSION, DownloadStorageError, SqliteDownloadStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DownloadTaskId(Uuid);

impl DownloadTaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for DownloadTaskId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestOrigin {
    Ui,
    Cli,
    Browser,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadRequestHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadCertificatePolicy {
    #[default]
    Strict,
    AcceptInvalid,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadDuplicateStrategy {
    #[default]
    UseDefault,
    AddNumbered,
    Reject,
}

impl DownloadCertificatePolicy {
    const fn is_strict(&self) -> bool {
        matches!(self, Self::Strict)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadAdaptiveSegmentPolicy {
    pub enabled: bool,
    pub max_segments: u16,
    pub min_segment_mib: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum DownloadRequestMedia {
    Hls(HlsDownloadRequest),
    Dash(DashDownloadRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HlsDownloadRequest {
    pub playlist_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashDownloadRequest {
    pub manifest_url: String,
    pub video_representation_id: Option<String>,
    pub audio_representation_id: Option<String>,
}

impl Default for DownloadAdaptiveSegmentPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_segments: 32,
            min_segment_mib: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub source_url: String,
    pub save_directory: PathBuf,
    #[serde(default)]
    pub save_directory_explicit: bool,
    pub file_name: Option<String>,
    #[serde(default)]
    pub headers: Vec<DownloadRequestHeader>,
    pub cookie: Option<String>,
    pub referrer: Option<String>,
    #[serde(default, skip_serializing_if = "DownloadCertificatePolicy::is_strict")]
    pub certificate_policy: DownloadCertificatePolicy,
    pub connection_limit: Option<u16>,
    #[serde(default)]
    pub speed_limit_bytes_per_second: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_total_bytes: Option<u64>,
    pub scheduled_at_unix_ms: Option<i64>,
    #[serde(default)]
    pub adaptive_segment_policy: Option<DownloadAdaptiveSegmentPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<DownloadRequestMedia>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
    #[serde(default)]
    pub duplicate_strategy: DownloadDuplicateStrategy,
}

impl DownloadRequest {
    pub fn validate(&self) -> Result<(), DownloadRequestError> {
        let parsed =
            url::Url::parse(&self.source_url).map_err(|_| DownloadRequestError::InvalidUrl)?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(DownloadRequestError::UnsupportedScheme(
                parsed.scheme().to_owned(),
            ));
        }
        if self.save_directory.as_os_str().is_empty() {
            return Err(DownloadRequestError::MissingSaveDirectory);
        }
        if let Some(file_name) = &self.file_name
            && !is_safe_windows_file_name(file_name)
        {
            return Err(DownloadRequestError::InvalidFileName);
        }
        if let Some(limit) = self.connection_limit
            && !(1..=64).contains(&limit)
        {
            return Err(DownloadRequestError::InvalidConnectionLimit(limit));
        }
        if let Some(policy) = self.adaptive_segment_policy {
            let initial_segments = self.connection_limit.unwrap_or(1).clamp(1, 64);
            if !(initial_segments..=256).contains(&policy.max_segments) {
                return Err(DownloadRequestError::InvalidAdaptiveMaxSegments(
                    policy.max_segments,
                ));
            }
            if !(1..=100).contains(&policy.min_segment_mib) {
                return Err(DownloadRequestError::InvalidAdaptiveMinSegmentMib(
                    policy.min_segment_mib,
                ));
            }
        }
        if self.speed_limit_bytes_per_second == Some(0) {
            return Err(DownloadRequestError::InvalidSpeedLimit);
        }
        if let Some(DownloadRequestMedia::Hls(hls)) = self.media.as_ref() {
            let playlist_url = url::Url::parse(&hls.playlist_url)
                .map_err(|_| DownloadRequestError::InvalidMediaUrl)?;
            if !matches!(playlist_url.scheme(), "http" | "https") {
                return Err(DownloadRequestError::UnsupportedMediaScheme(
                    playlist_url.scheme().to_owned(),
                ));
            }
            if hls.playlist_url.len() > MAX_HLS_VARIANT_URL_BYTES {
                return Err(DownloadRequestError::MediaUrlTooLong);
            }
        }
        if let Some(DownloadRequestMedia::Dash(dash)) = self.media.as_ref() {
            let manifest_url = url::Url::parse(&dash.manifest_url)
                .map_err(|_| DownloadRequestError::InvalidMediaUrl)?;
            if !matches!(manifest_url.scheme(), "http" | "https") {
                return Err(DownloadRequestError::UnsupportedMediaScheme(
                    manifest_url.scheme().to_owned(),
                ));
            }
            if dash.manifest_url.len() > MAX_DASH_URL_BYTES {
                return Err(DownloadRequestError::MediaUrlTooLong);
            }
            if dash.video_representation_id.is_none() && dash.audio_representation_id.is_none() {
                return Err(DownloadRequestError::MissingMediaSelection);
            }
            if dash
                .video_representation_id
                .iter()
                .chain(dash.audio_representation_id.iter())
                .any(|identity| identity.is_empty() || identity.len() > MAX_DASH_FIELD_BYTES)
            {
                return Err(DownloadRequestError::InvalidMediaSelection);
            }
        }
        if self.headers.len() > MAX_DEFAULT_HEADER_COUNT
            || self
                .headers
                .iter()
                .map(|header| header.name.len() + header.value.len())
                .sum::<usize>()
                > MAX_DEFAULT_HEADER_TEXT_BYTES
            || self
                .headers
                .iter()
                .any(|header| request_policy::validate_header(&header.name, &header.value).is_err())
            || self.cookie.as_deref().is_some_and(|value| {
                value.len() > MAX_DEFAULT_COOKIE_BYTES
                    || value.contains(['\r', '\n'])
                    || !request_policy::is_valid_header_value(value)
            })
            || self.referrer.as_deref().is_some_and(|value| {
                value.contains(['\r', '\n']) || !request_policy::is_valid_header_value(value)
            })
        {
            return Err(DownloadRequestError::InvalidHeader);
        }
        if (self.category_id.is_none() && self.category_name.is_some())
            || self.category_id.as_deref().is_some_and(|value| {
                value.is_empty()
                    || value.len() > 64
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            })
            || self
                .category_name
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 64)
        {
            return Err(DownloadRequestError::InvalidCategory);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadRequestError {
    #[error("download URL is invalid")]
    InvalidUrl,
    #[error("download URL scheme is not supported: {0}")]
    UnsupportedScheme(String),
    #[error("download save directory is required")]
    MissingSaveDirectory,
    #[error("download file name must be a single safe path component")]
    InvalidFileName,
    #[error("download connection limit must be between 1 and 64, got {0}")]
    InvalidConnectionLimit(u16),
    #[error(
        "adaptive segment maximum must be between the initial connection count and 256, got {0}"
    )]
    InvalidAdaptiveMaxSegments(u16),
    #[error("adaptive minimum segment size must be between 1 and 100 MiB, got {0}")]
    InvalidAdaptiveMinSegmentMib(u16),
    #[error("download speed limit must be greater than zero; omit it for unlimited")]
    InvalidSpeedLimit,
    #[error("media playlist URL is invalid")]
    InvalidMediaUrl,
    #[error("media playlist URL scheme is not supported: {0}")]
    UnsupportedMediaScheme(String),
    #[error("media playlist URL exceeds the bounded field limit")]
    MediaUrlTooLong,
    #[error("media request has no selected track")]
    MissingMediaSelection,
    #[error("media track selection is invalid or exceeds the bounded field limit")]
    InvalidMediaSelection,
    #[error("download request contains an invalid header")]
    InvalidHeader,
    #[error("download category identity is invalid")]
    InvalidCategory,
}

pub(crate) fn is_safe_windows_file_name(file_name: &str) -> bool {
    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.ends_with(['.', ' '])
        || file_name.encode_utf16().count() > 255
        || file_name
            .chars()
            .any(|character| character < ' ' || r#"<>:"/\|?*"#.contains(character))
    {
        return false;
    }

    let stem = file_name
        .split_once('.')
        .map_or(file_name, |(stem, _)| stem)
        .to_ascii_uppercase();
    !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferProtocol {
    Http,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResourceValidator {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransferPlan {
    protocol: TransferProtocol,
    final_url: String,
    total_bytes: Option<u64>,
    supports_ranges: bool,
    validator: ResourceValidator,
    segments: Vec<DownloadSegment>,
}

impl TransferPlan {
    pub fn new(
        protocol: TransferProtocol,
        final_url: String,
        total_bytes: Option<u64>,
        supports_ranges: bool,
        validator: ResourceValidator,
        mut segments: Vec<DownloadSegment>,
    ) -> Result<Self, TransferPlanError> {
        segments.sort_by_key(|segment| (segment.start_byte(), segment.index()));

        for (position, segment) in segments.iter().enumerate() {
            if segments[..position]
                .iter()
                .any(|candidate| candidate.index() == segment.index())
            {
                return Err(TransferPlanError::DuplicateSegmentIndex {
                    segment_index: segment.index(),
                });
            }

            if let Some(total_bytes) = total_bytes
                && segment.end_byte() >= total_bytes
            {
                return Err(TransferPlanError::SegmentOutOfBounds {
                    segment_index: segment.index(),
                    end_byte: segment.end_byte(),
                    total_bytes,
                });
            }
        }

        for pair in segments.windows(2) {
            let previous = &pair[0];
            let next = &pair[1];
            if next.start_byte() <= previous.end_byte() {
                return Err(TransferPlanError::OverlappingSegments {
                    previous_index: previous.index(),
                    next_index: next.index(),
                });
            }
        }

        Ok(Self {
            protocol,
            final_url,
            total_bytes,
            supports_ranges,
            validator,
            segments,
        })
    }

    pub const fn protocol(&self) -> TransferProtocol {
        self.protocol
    }

    pub fn final_url(&self) -> &str {
        &self.final_url
    }

    pub const fn total_bytes(&self) -> Option<u64> {
        self.total_bytes
    }

    pub const fn supports_ranges(&self) -> bool {
        self.supports_ranges
    }

    pub const fn validator(&self) -> &ResourceValidator {
        &self.validator
    }

    pub fn segments(&self) -> &[DownloadSegment] {
        &self.segments
    }

    pub fn split_segment_after_checkpoint(
        &mut self,
        segment_index: u32,
        new_segment_index: u32,
        minimum_segment_bytes: u64,
    ) -> Result<(), TransferPlanError> {
        if minimum_segment_bytes == 0 {
            return Err(TransferPlanError::InvalidMinimumSegmentBytes);
        }
        let position = self
            .segments
            .iter()
            .position(|segment| segment.index() == segment_index)
            .ok_or(TransferPlanError::SegmentNotFound { segment_index })?;
        let segment = &self.segments[position];
        if segment.state != DownloadSegmentState::Waiting {
            return Err(TransferPlanError::SegmentNotWaiting { segment_index });
        }
        let remaining_bytes = segment.len() - segment.downloaded_bytes();
        let required_bytes = minimum_segment_bytes
            .checked_mul(2)
            .ok_or(TransferPlanError::SplitSizeOverflow)?;
        if remaining_bytes < required_bytes {
            return Err(TransferPlanError::InsufficientSplitTail {
                segment_index,
                remaining_bytes,
                required_bytes,
            });
        }

        let remaining_start = segment
            .start_byte()
            .checked_add(segment.downloaded_bytes())
            .ok_or(TransferPlanError::SplitSizeOverflow)?;
        let left_remaining_bytes = remaining_bytes / 2;
        let reduced_end = remaining_start
            .checked_add(left_remaining_bytes - 1)
            .ok_or(TransferPlanError::SplitSizeOverflow)?;
        let new_start = reduced_end
            .checked_add(1)
            .ok_or(TransferPlanError::SplitSizeOverflow)?;
        let reduced = DownloadSegment::new(
            segment.index(),
            segment.start_byte(),
            reduced_end,
            segment.downloaded_bytes(),
            DownloadSegmentState::Waiting,
        )?;
        let added = DownloadSegment::new(
            new_segment_index,
            new_start,
            segment.end_byte(),
            0,
            DownloadSegmentState::Waiting,
        )?;
        let mut segments = self.segments.clone();
        segments[position] = reduced;
        segments.push(added);
        *self = Self::new(
            self.protocol,
            self.final_url.clone(),
            self.total_bytes,
            self.supports_ranges,
            self.validator.clone(),
            segments,
        )?;
        Ok(())
    }

    /// Merges the remaining range of `cancelled_segment_index` into
    /// `absorbing_segment_index` and removes the cancelled segment from the
    /// plan. Both segments must already be Waiting (the cancelled worker must
    /// have checkpointed before exiting); the absorbing segment must sit
    /// directly before or after the cancelled one so their ranges are
    /// contiguous.
    pub fn merge_waiting_segment(
        &mut self,
        cancelled_segment_index: u32,
        absorbing_segment_index: u32,
    ) -> Result<(), TransferPlanError> {
        let cancelled_position = self
            .segments
            .iter()
            .position(|segment| segment.index() == cancelled_segment_index)
            .ok_or(TransferPlanError::SegmentNotFound {
                segment_index: cancelled_segment_index,
            })?;
        let absorbing_position = self
            .segments
            .iter()
            .position(|segment| segment.index() == absorbing_segment_index)
            .ok_or(TransferPlanError::SegmentNotFound {
                segment_index: absorbing_segment_index,
            })?;
        let cancelled = &self.segments[cancelled_position];
        if cancelled.state != DownloadSegmentState::Waiting {
            return Err(TransferPlanError::SegmentNotWaiting {
                segment_index: cancelled_segment_index,
            });
        }
        let absorbing = &self.segments[absorbing_position];
        if absorbing.state != DownloadSegmentState::Waiting {
            return Err(TransferPlanError::SegmentNotWaiting {
                segment_index: absorbing_segment_index,
            });
        }
        // Contiguity: cancelled must be the neighbour of the absorber.
        let cancelled_start = cancelled.start_byte();
        let cancelled_end = cancelled.end_byte();
        let absorbing_start = absorbing.start_byte();
        let absorbing_end = absorbing.end_byte();
        let adjacent = (cancelled_end.saturating_add(1) == absorbing_start)
            || (absorbing_end.saturating_add(1) == cancelled_start);
        if !adjacent {
            return Err(TransferPlanError::SegmentNotFound {
                segment_index: cancelled_segment_index,
            });
        }
        // The cancelled side must have checkpointed nothing back (its worker
        // exited before consuming its range), otherwise progress would be
        // lost by the merge; the absorbing segment keeps its own checkpoint.
        if cancelled.downloaded_bytes() != 0 {
            return Err(TransferPlanError::SegmentNotWaiting {
                segment_index: cancelled_segment_index,
            });
        }
        // Progress on the right starts at its old offset; prepending an
        // unwritten left range would falsely turn it into a downloaded prefix.
        if cancelled_end.saturating_add(1) == absorbing_start && absorbing.downloaded_bytes() != 0 {
            return Err(TransferPlanError::SegmentNotWaiting {
                segment_index: absorbing_segment_index,
            });
        }
        let merged_start = cancelled_start.min(absorbing_start);
        let merged_end = cancelled_end.max(absorbing_end);
        let merged = DownloadSegment::new(
            absorbing_segment_index,
            merged_start,
            merged_end,
            absorbing.downloaded_bytes(),
            DownloadSegmentState::Waiting,
        )?;
        let mut segments = self.segments.clone();
        // Retain the absorber's identity even when it follows the shed range.
        segments[absorbing_position] = merged;
        segments.remove(cancelled_position);
        // Reindex remaining segments is unnecessary: indices stay unique.
        *self = Self::new(
            self.protocol,
            self.final_url.clone(),
            self.total_bytes,
            self.supports_ranges,
            self.validator.clone(),
            segments,
        )?;
        Ok(())
    }
}

impl<'de> Deserialize<'de> for TransferPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TransferPlanWire::deserialize(deserializer)?;
        Self::new(
            wire.protocol,
            wire.final_url,
            wire.total_bytes,
            wire.supports_ranges,
            wire.validator,
            wire.segments,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
struct TransferPlanWire {
    protocol: TransferProtocol,
    final_url: String,
    total_bytes: Option<u64>,
    supports_ranges: bool,
    validator: ResourceValidator,
    segments: Vec<DownloadSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransferPlanError {
    #[error(transparent)]
    Segment(#[from] DownloadSegmentError),
    #[error("download segment {segment_index} does not exist")]
    SegmentNotFound { segment_index: u32 },
    #[error("download segment {segment_index} must be waiting before it can be split")]
    SegmentNotWaiting { segment_index: u32 },
    #[error("minimum split segment size must be greater than zero")]
    InvalidMinimumSegmentBytes,
    #[error("download segment split size overflowed")]
    SplitSizeOverflow,
    #[error(
        "download segment {segment_index} has {remaining_bytes} bytes remaining; splitting requires at least {required_bytes}"
    )]
    InsufficientSplitTail {
        segment_index: u32,
        remaining_bytes: u64,
        required_bytes: u64,
    },
    #[error("download segment index {segment_index} is duplicated")]
    DuplicateSegmentIndex { segment_index: u32 },
    #[error("download segments overlap: segment {previous_index} overlaps segment {next_index}")]
    OverlappingSegments {
        previous_index: u32,
        next_index: u32,
    },
    #[error(
        "download segment {segment_index} ends at {end_byte}, outside resource length {total_bytes}"
    )]
    SegmentOutOfBounds {
        segment_index: u32,
        end_byte: u64,
        total_bytes: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadSegmentState {
    Waiting,
    Active,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DownloadSegment {
    index: u32,
    start_byte: u64,
    end_byte: u64,
    downloaded_bytes: u64,
    pub state: DownloadSegmentState,
}

impl DownloadSegment {
    pub fn new(
        index: u32,
        start_byte: u64,
        end_byte: u64,
        downloaded_bytes: u64,
        state: DownloadSegmentState,
    ) -> Result<Self, DownloadSegmentError> {
        if end_byte < start_byte {
            return Err(DownloadSegmentError::InvalidRange {
                start_byte,
                end_byte,
            });
        }
        let Some(length) = end_byte
            .checked_sub(start_byte)
            .and_then(|difference| difference.checked_add(1))
        else {
            return Err(DownloadSegmentError::RangeTooLarge);
        };
        if downloaded_bytes > length {
            return Err(DownloadSegmentError::ProgressExceedsRange {
                downloaded_bytes,
                length,
            });
        }
        Ok(Self {
            index,
            start_byte,
            end_byte,
            downloaded_bytes,
            state,
        })
    }

    pub fn len(&self) -> u64 {
        self.end_byte - self.start_byte + 1
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn start_byte(&self) -> u64 {
        self.start_byte
    }

    pub fn end_byte(&self) -> u64 {
        self.end_byte
    }
}

impl<'de> Deserialize<'de> for DownloadSegment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DownloadSegmentWire::deserialize(deserializer)?;
        Self::new(
            wire.index,
            wire.start_byte,
            wire.end_byte,
            wire.downloaded_bytes,
            wire.state,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
struct DownloadSegmentWire {
    index: u32,
    start_byte: u64,
    end_byte: u64,
    downloaded_bytes: u64,
    state: DownloadSegmentState,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadSegmentError {
    #[error("download segment range is invalid: {start_byte}..={end_byte}")]
    InvalidRange { start_byte: u64, end_byte: u64 },
    #[error("download segment progress {downloaded_bytes} exceeds range length {length}")]
    ProgressExceedsRange { downloaded_bytes: u64, length: u64 },
    #[error("download segment range is too large")]
    RangeTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveConnection {
    pub id: u32,
    pub segment_index: u32,
    pub started_at_unix_ms: i64,
    pub downloaded_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadAttempt {
    pub number: u32,
    pub started_at_unix_ms: i64,
    pub finished_at_unix_ms: Option<i64>,
    pub error: Option<DownloadAttemptError>,
    pub retry_at_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadAttemptError {
    pub category: DownloadAttemptErrorCategory,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadAttemptErrorCategory {
    Network,
    Protocol,
    Storage,
    Permission,
    DiskFull,
    FileInUse,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadQueue {
    pub id: String,
    pub name: String,
    pub position: i32,
    pub max_concurrent_tasks: Option<u16>,
    pub connection_limit: Option<u16>,
    pub speed_limit_bytes_per_second: Option<u64>,
    pub default_save_directory: Option<PathBuf>,
    pub schedule: Option<DownloadSchedule>,
    pub is_running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadSchedule {
    pub daily: DownloadDailySchedule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadDailySchedule {
    pub days: Vec<DownloadWeekday>,
    pub start_minute: u16,
    pub stop_minute: u16,
    pub enabled_start: bool,
    pub enabled_stop: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadWeekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl DownloadWeekday {
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Monday => "周一",
            Self::Tuesday => "周二",
            Self::Wednesday => "周三",
            Self::Thursday => "周四",
            Self::Friday => "周五",
            Self::Saturday => "周六",
            Self::Sunday => "周日",
        }
    }
}

impl DownloadDailySchedule {
    pub fn validate(&self) -> Result<(), DownloadScheduleError> {
        if self.days.is_empty() {
            return Err(DownloadScheduleError::NoDays);
        }
        if self.start_minute >= 24 * 60 {
            return Err(DownloadScheduleError::InvalidMinute {
                field: "start_minute",
                value: self.start_minute,
            });
        }
        if self.stop_minute >= 24 * 60 {
            return Err(DownloadScheduleError::InvalidMinute {
                field: "stop_minute",
                value: self.stop_minute,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadScheduleError {
    #[error("daily schedule must include at least one weekday")]
    NoDays,
    #[error("daily schedule {field} must be a minute within the local day, got {value}")]
    InvalidMinute { field: &'static str, value: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadArtifactKind {
    TemporaryFile,
    FinalFile,
    MediaTrack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadArtifactState {
    Pending,
    Committed,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadArtifact {
    pub id: String,
    pub kind: DownloadArtifactKind,
    pub state: DownloadArtifactState,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Created,
    Probing,
    Waiting,
    Downloading,
    Paused,
    Finalizing,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }

    fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Created => matches!(next, Self::Probing | Self::Paused | Self::Cancelled),
            Self::Probing => {
                matches!(
                    next,
                    Self::Waiting | Self::Paused | Self::Failed | Self::Cancelled
                )
            }
            Self::Waiting => {
                matches!(
                    next,
                    Self::Downloading | Self::Paused | Self::Failed | Self::Cancelled
                )
            }
            Self::Downloading => {
                matches!(
                    next,
                    Self::Paused | Self::Finalizing | Self::Failed | Self::Cancelled
                )
            }
            Self::Paused => matches!(next, Self::Waiting | Self::Cancelled),
            Self::Finalizing => {
                matches!(
                    next,
                    Self::Paused
                        | Self::Verifying
                        | Self::Completed
                        | Self::Failed
                        | Self::Cancelled
                )
            }
            Self::Verifying => {
                matches!(next, Self::Completed | Self::Failed | Self::Cancelled)
            }
            Self::Failed => matches!(next, Self::Waiting | Self::Cancelled),
            Self::Completed | Self::Cancelled => false,
        }
    }

    fn was_interrupted(self) -> bool {
        INTERRUPTED_DOWNLOAD_STATUSES.contains(&self)
    }

    pub(crate) const fn storage_name(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Probing => "probing",
            Self::Waiting => "waiting",
            Self::Downloading => "downloading",
            Self::Paused => "paused",
            Self::Finalizing => "finalizing",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

pub(crate) const INTERRUPTED_DOWNLOAD_STATUSES: [DownloadStatus; 4] = [
    DownloadStatus::Probing,
    DownloadStatus::Downloading,
    DownloadStatus::Finalizing,
    DownloadStatus::Verifying,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DownloadTask {
    id: DownloadTaskId,
    request: DownloadRequest,
    origin: RequestOrigin,
    status: DownloadStatus,
    created_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    completed_at_unix_ms: Option<i64>,
}

impl DownloadTask {
    pub fn new(request: DownloadRequest, origin: RequestOrigin, now_unix_ms: i64) -> Self {
        Self {
            id: DownloadTaskId::new(),
            request,
            origin,
            status: DownloadStatus::Created,
            created_at_unix_ms: now_unix_ms,
            updated_at_unix_ms: now_unix_ms,
            completed_at_unix_ms: None,
        }
    }

    pub fn transition_to(
        &mut self,
        next: DownloadStatus,
        now_unix_ms: i64,
    ) -> Result<(), DownloadTaskError> {
        if !self.status.can_transition_to(next) {
            return Err(DownloadTaskError::InvalidTransition {
                from: self.status,
                to: next,
            });
        }

        self.status = next;
        self.updated_at_unix_ms = now_unix_ms;
        if next == DownloadStatus::Completed {
            self.completed_at_unix_ms = Some(now_unix_ms);
        }
        Ok(())
    }

    pub const fn id(&self) -> DownloadTaskId {
        self.id
    }

    pub fn request(&self) -> &DownloadRequest {
        &self.request
    }

    pub(crate) fn set_speed_limit(&mut self, bytes_per_second: Option<u64>, now_unix_ms: i64) {
        self.request.speed_limit_bytes_per_second = bytes_per_second;
        self.updated_at_unix_ms = now_unix_ms;
    }

    pub(crate) fn set_media(&mut self, media: DownloadRequestMedia, now_unix_ms: i64) {
        self.request.media = Some(media);
        self.updated_at_unix_ms = now_unix_ms;
    }

    pub(crate) fn set_file_name(&mut self, file_name: String, now_unix_ms: i64) {
        self.request.file_name = Some(file_name);
        self.updated_at_unix_ms = now_unix_ms;
    }

    pub(crate) fn clear_scheduled_start(&mut self, now_unix_ms: i64) {
        self.request.scheduled_at_unix_ms = None;
        self.updated_at_unix_ms = now_unix_ms;
    }

    pub const fn origin(&self) -> RequestOrigin {
        self.origin
    }

    pub const fn status(&self) -> DownloadStatus {
        self.status
    }

    pub const fn created_at_unix_ms(&self) -> i64 {
        self.created_at_unix_ms
    }

    pub const fn updated_at_unix_ms(&self) -> i64 {
        self.updated_at_unix_ms
    }

    pub const fn completed_at_unix_ms(&self) -> Option<i64> {
        self.completed_at_unix_ms
    }

    pub fn recover_interrupted(&mut self, now_unix_ms: i64) -> bool {
        if !self.status.was_interrupted() {
            return false;
        }

        self.status = DownloadStatus::Paused;
        self.updated_at_unix_ms = now_unix_ms;
        true
    }

    pub(crate) fn restore(
        id: DownloadTaskId,
        request: DownloadRequest,
        origin: RequestOrigin,
        status: DownloadStatus,
        created_at_unix_ms: i64,
        updated_at_unix_ms: i64,
        completed_at_unix_ms: Option<i64>,
    ) -> Result<Self, DownloadTaskInvariantError> {
        let task = Self {
            id,
            request,
            origin,
            status,
            created_at_unix_ms,
            updated_at_unix_ms,
            completed_at_unix_ms,
        };
        task.validate_invariants()?;
        Ok(task)
    }

    fn validate_invariants(&self) -> Result<(), DownloadTaskInvariantError> {
        self.request
            .validate()
            .map_err(DownloadTaskInvariantError::InvalidRequest)?;
        if self.updated_at_unix_ms < self.created_at_unix_ms {
            return Err(DownloadTaskInvariantError::TimestampOrder);
        }
        match (self.status, self.completed_at_unix_ms) {
            (DownloadStatus::Completed, Some(completed_at))
                if completed_at >= self.created_at_unix_ms =>
            {
                Ok(())
            }
            (DownloadStatus::Completed, _) => {
                Err(DownloadTaskInvariantError::MissingCompletionTimestamp)
            }
            (_, Some(_)) => Err(DownloadTaskInvariantError::UnexpectedCompletionTimestamp),
            (_, None) => Ok(()),
        }
    }
}

impl<'de> Deserialize<'de> for DownloadTask {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DownloadTaskWire::deserialize(deserializer)?;
        Self::restore(
            wire.id,
            wire.request,
            wire.origin,
            wire.status,
            wire.created_at_unix_ms,
            wire.updated_at_unix_ms,
            wire.completed_at_unix_ms,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Deserialize)]
struct DownloadTaskWire {
    id: DownloadTaskId,
    request: DownloadRequest,
    origin: RequestOrigin,
    status: DownloadStatus,
    created_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    completed_at_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadTaskInvariantError {
    #[error("stored download request is invalid: {0}")]
    InvalidRequest(DownloadRequestError),
    #[error("download task timestamps are out of order")]
    TimestampOrder,
    #[error("completed download task has no valid completion timestamp")]
    MissingCompletionTimestamp,
    #[error("non-completed download task has a completion timestamp")]
    UnexpectedCompletionTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DownloadTaskError {
    #[error("invalid download task transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: DownloadStatus,
        to: DownloadStatus,
    },
}
