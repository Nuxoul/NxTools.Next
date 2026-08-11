use std::collections::HashSet;

use roxmltree::Node;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::{DashProbe, DashTrackProbe, DownloadProbe};

pub const MAX_DASH_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_DASH_REPRESENTATIONS: usize = 32;
pub const MAX_DASH_SEGMENTS: usize = 10_000;
pub const MAX_DASH_URL_BYTES: usize = 8 * 1024;
pub const MAX_DASH_FIELD_BYTES: usize = 512;
pub const MAX_DASH_PROBE_SERIALIZED_BYTES: usize = 512 * 1024;
const MAX_DASH_EXPANDED_URL_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DashTrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DashSegment {
    pub index: u32,
    pub number: u64,
    pub time: Option<u64>,
    pub url: String,
    pub duration_millis: u64,
    pub duration_units: u64,
    pub timescale: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DashTrack {
    pub kind: DashTrackKind,
    pub representation_id: String,
    pub bandwidth: u64,
    pub resolution: Option<(u32, u32)>,
    pub codecs: String,
    pub mime_type: String,
    pub language: Option<String>,
    pub initialization_url: Option<String>,
    pub segments: Vec<DashSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DashManifest {
    pub final_url: String,
    pub tracks: Vec<DashTrack>,
}

impl DashManifest {
    pub(crate) fn probe(&self) -> DashProbe {
        let mut video_tracks = self
            .tracks
            .iter()
            .filter(|track| track.kind == DashTrackKind::Video)
            .map(track_probe)
            .collect::<Vec<_>>();
        let mut audio_tracks = self
            .tracks
            .iter()
            .filter(|track| track.kind == DashTrackKind::Audio)
            .map(track_probe)
            .collect::<Vec<_>>();
        video_tracks.sort_by(|left, right| {
            right
                .bandwidth
                .cmp(&left.bandwidth)
                .then_with(|| pixel_count(right.resolution).cmp(&pixel_count(left.resolution)))
                .then_with(|| left.representation_id.cmp(&right.representation_id))
        });
        audio_tracks.sort_by(|left, right| {
            right
                .bandwidth
                .cmp(&left.bandwidth)
                .then_with(|| left.representation_id.cmp(&right.representation_id))
        });
        DashProbe {
            video_tracks,
            audio_tracks,
        }
    }
}

fn track_probe(track: &DashTrack) -> DashTrackProbe {
    DashTrackProbe {
        representation_id: track.representation_id.clone(),
        bandwidth: track.bandwidth,
        resolution: track.resolution,
        codecs: track.codecs.clone(),
        mime_type: track.mime_type.clone(),
        language: track.language.clone(),
        segment_count: u32::try_from(track.segments.len()).unwrap_or(u32::MAX),
        has_initialization: track.initialization_url.is_some(),
    }
}

fn pixel_count(resolution: Option<(u32, u32)>) -> u64 {
    resolution.map_or(0, |(width, height)| u64::from(width) * u64::from(height))
}

pub(crate) fn validate_dash_probe_size(probe: &DownloadProbe) -> Result<(), DashError> {
    let encoded = serde_json::to_vec(probe).map_err(|_| DashError::MetadataTooLarge)?;
    if encoded.len() > MAX_DASH_PROBE_SERIALIZED_BYTES {
        return Err(DashError::MetadataTooLarge);
    }
    Ok(())
}

pub(crate) fn parse_dash_manifest(
    manifest_url: &Url,
    bytes: &[u8],
) -> Result<DashManifest, DashError> {
    if bytes.len() > MAX_DASH_MANIFEST_BYTES {
        return Err(DashError::MetadataTooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| DashError::Invalid("MPD is not UTF-8"))?;
    let document =
        roxmltree::Document::parse(text).map_err(|_| DashError::Invalid("MPD XML is invalid"))?;
    let mpd = document.root_element();
    if !mpd.has_tag_name("MPD") {
        return Err(DashError::Invalid("MPD root element is missing"));
    }
    if mpd
        .attribute("type")
        .is_some_and(|value| !value.eq_ignore_ascii_case("static"))
    {
        return Err(DashError::Unsupported("dynamic_mpd"));
    }
    if mpd
        .descendants()
        .any(|node| node.is_element() && node.tag_name().name() == "ContentProtection")
    {
        return Err(DashError::Unsupported("content_protection"));
    }
    if mpd.descendants().any(|node| {
        node.is_element() && matches!(node.tag_name().name(), "SegmentList" | "SegmentBase")
    }) {
        return Err(DashError::Unsupported("segment_addressing_mode"));
    }

    let periods = element_children(mpd, "Period").collect::<Vec<_>>();
    if periods.len() != 1 {
        return Err(if periods.is_empty() {
            DashError::Invalid("MPD must contain one Period")
        } else {
            DashError::Unsupported("multiple_periods")
        });
    }
    let period = periods[0];
    let presentation_duration_nanos = period
        .attribute("duration")
        .or_else(|| mpd.attribute("mediaPresentationDuration"))
        .map(parse_iso8601_duration_nanos)
        .transpose()?;
    if presentation_duration_nanos == Some(0) {
        return Err(DashError::Invalid("static MPD duration is zero"));
    }

    let mpd_base = resolve_node_base(manifest_url, mpd)?;
    let period_base = resolve_node_base(&mpd_base, period)?;
    let period_template = template_at(period, None)?;
    let mut tracks = Vec::new();
    let mut representation_count = 0_usize;
    let mut expanded_url_bytes = 0_usize;
    let mut identities = HashSet::new();

    for adaptation in element_children(period, "AdaptationSet") {
        let adaptation_base = resolve_node_base(&period_base, adaptation)?;
        let adaptation_template = template_at(adaptation, period_template.as_ref())?;
        let adaptation_mime = bounded_text(adaptation.attribute("mimeType").unwrap_or(""))?;
        let adaptation_codecs = bounded_text(adaptation.attribute("codecs").unwrap_or(""))?;
        let language = optional_bounded_text(adaptation.attribute("lang"))?;
        let adaptation_kind = track_kind(
            adaptation.attribute("contentType"),
            adaptation.attribute("mimeType"),
        );

        for representation in element_children(adaptation, "Representation") {
            representation_count = representation_count.saturating_add(1);
            if representation_count > MAX_DASH_REPRESENTATIONS {
                return Err(DashError::TooManyRepresentations);
            }
            let mime_type = bounded_text(
                representation
                    .attribute("mimeType")
                    .unwrap_or(&adaptation_mime),
            )?;
            let kind = track_kind(representation.attribute("contentType"), Some(&mime_type))
                .or(adaptation_kind);
            let Some(kind) = kind else {
                continue;
            };
            let supported_mime = match kind {
                DashTrackKind::Video => "video/mp4",
                DashTrackKind::Audio => "audio/mp4",
            };
            if !mime_type.eq_ignore_ascii_case(supported_mime) {
                return Err(DashError::Unsupported("media_container"));
            }
            let representation_id = bounded_text(
                representation
                    .attribute("id")
                    .ok_or(DashError::Invalid("DASH Representation id is missing"))?,
            )?;
            if representation_id.is_empty() || !identities.insert((kind, representation_id.clone()))
            {
                return Err(DashError::Invalid(
                    "DASH Representation id is empty or duplicated",
                ));
            }
            let bandwidth = parse_u64_attribute(representation, "bandwidth")?.unwrap_or(0);
            let width = parse_u32_attribute(representation, "width")?;
            let height = parse_u32_attribute(representation, "height")?;
            let resolution = match (width, height) {
                (Some(width), Some(height)) if width > 0 && height > 0 => Some((width, height)),
                (None, None) => None,
                _ => return Err(DashError::Invalid("DASH resolution is incomplete")),
            };
            let codecs = bounded_text(
                representation
                    .attribute("codecs")
                    .unwrap_or(&adaptation_codecs),
            )?;
            let representation_base = resolve_node_base(&adaptation_base, representation)?;
            let template = template_at(representation, adaptation_template.as_ref())?
                .ok_or(DashError::Unsupported("segment_template_missing"))?;
            let (initialization_url, segments) = expand_template(
                &representation_base,
                &template,
                &representation_id,
                bandwidth,
                presentation_duration_nanos,
            )?;
            if segments.is_empty() {
                return Err(DashError::Invalid(
                    "DASH Representation has no media segments",
                ));
            }
            expanded_url_bytes = expanded_url_bytes
                .saturating_add(initialization_url.as_ref().map_or(0, String::len))
                .saturating_add(
                    segments
                        .iter()
                        .map(|segment| segment.url.len())
                        .sum::<usize>(),
                );
            if expanded_url_bytes > MAX_DASH_EXPANDED_URL_BYTES {
                return Err(DashError::MetadataTooLarge);
            }
            tracks.push(DashTrack {
                kind,
                representation_id,
                bandwidth,
                resolution,
                codecs,
                mime_type,
                language: language.clone(),
                initialization_url,
                segments,
            });
        }
    }

    if !tracks
        .iter()
        .any(|track| track.kind == DashTrackKind::Video)
        && !tracks
            .iter()
            .any(|track| track.kind == DashTrackKind::Audio)
    {
        return Err(DashError::Invalid(
            "MPD has no playable audio or video Representation",
        ));
    }
    Ok(DashManifest {
        final_url: manifest_url.to_string(),
        tracks,
    })
}

fn track_kind(content_type: Option<&str>, mime_type: Option<&str>) -> Option<DashTrackKind> {
    match content_type.unwrap_or("").to_ascii_lowercase().as_str() {
        "video" => Some(DashTrackKind::Video),
        "audio" => Some(DashTrackKind::Audio),
        _ => match mime_type.unwrap_or("").to_ascii_lowercase() {
            value if value.starts_with("video/") => Some(DashTrackKind::Video),
            value if value.starts_with("audio/") => Some(DashTrackKind::Audio),
            _ => None,
        },
    }
}

#[derive(Debug, Clone, Default)]
struct DashTemplate {
    initialization: Option<String>,
    media: Option<String>,
    timescale: Option<u64>,
    duration: Option<u64>,
    start_number: Option<u64>,
    timeline: Option<Vec<TimelineEntry>>,
}

#[derive(Debug, Clone)]
struct TimelineEntry {
    start: Option<u64>,
    duration: u64,
    repeat: u32,
}

fn template_at(
    parent: Node<'_, '_>,
    inherited: Option<&DashTemplate>,
) -> Result<Option<DashTemplate>, DashError> {
    let templates = element_children(parent, "SegmentTemplate").collect::<Vec<_>>();
    if templates.len() > 1 {
        return Err(DashError::Invalid(
            "multiple SegmentTemplate elements are ambiguous",
        ));
    }
    let Some(node) = templates.first().copied() else {
        return Ok(inherited.cloned());
    };
    if node.attribute("presentationTimeOffset").is_some() || node.attribute("eptDelta").is_some() {
        return Err(DashError::Unsupported("segment_timeline_offset"));
    }
    let mut template = inherited.cloned().unwrap_or_default();
    if let Some(value) = node.attribute("initialization") {
        template.initialization = Some(bounded_text(value)?);
    }
    if let Some(value) = node.attribute("media") {
        template.media = Some(bounded_text(value)?);
    }
    if let Some(value) = parse_u64_attribute(node, "timescale")? {
        template.timescale = Some(value);
    }
    if let Some(value) = parse_u64_attribute(node, "duration")? {
        template.duration = Some(value);
    }
    if let Some(value) = parse_u64_attribute(node, "startNumber")? {
        template.start_number = Some(value);
    }
    let timelines = element_children(node, "SegmentTimeline").collect::<Vec<_>>();
    if timelines.len() > 1 {
        return Err(DashError::Invalid(
            "multiple SegmentTimeline elements are ambiguous",
        ));
    }
    if let Some(timeline) = timelines.first().copied() {
        let mut entries = Vec::new();
        for segment in element_children(timeline, "S") {
            let duration = parse_u64_attribute(segment, "d")?
                .filter(|duration| *duration > 0)
                .ok_or(DashError::Invalid(
                    "SegmentTimeline duration is missing or zero",
                ))?;
            let repeat = match segment.attribute("r") {
                Some(value) => {
                    let parsed = value
                        .parse::<i64>()
                        .map_err(|_| DashError::Invalid("SegmentTimeline repeat is invalid"))?;
                    if parsed < 0 {
                        return Err(DashError::Unsupported("negative_timeline_repeat"));
                    }
                    u32::try_from(parsed).map_err(|_| DashError::TooManySegments)?
                }
                None => 0,
            };
            entries.push(TimelineEntry {
                start: parse_u64_attribute(segment, "t")?,
                duration,
                repeat,
            });
        }
        if entries.is_empty() {
            return Err(DashError::Invalid("SegmentTimeline is empty"));
        }
        template.timeline = Some(entries);
    }
    Ok(Some(template))
}

fn expand_template(
    base_url: &Url,
    template: &DashTemplate,
    representation_id: &str,
    bandwidth: u64,
    presentation_duration_nanos: Option<u128>,
) -> Result<(Option<String>, Vec<DashSegment>), DashError> {
    let timescale = template.timescale.unwrap_or(1);
    if timescale == 0 {
        return Err(DashError::Invalid("SegmentTemplate timescale is zero"));
    }
    let media = template
        .media
        .as_deref()
        .ok_or(DashError::Invalid("SegmentTemplate media is missing"))?;
    let start_number = template.start_number.unwrap_or(1);
    let initialization_url = template
        .initialization
        .as_deref()
        .map(|value| substitute_template(value, representation_id, bandwidth, None, None))
        .transpose()?
        .map(|value| resolve_resource_url(base_url, &value))
        .transpose()?;
    let mut descriptors = Vec::new();
    if let Some(timeline) = template.timeline.as_ref() {
        let mut current_time = 0_u64;
        let mut number = start_number;
        for entry in timeline {
            if let Some(start) = entry.start {
                current_time = start;
            }
            for _ in 0..=entry.repeat {
                if descriptors.len() >= MAX_DASH_SEGMENTS {
                    return Err(DashError::TooManySegments);
                }
                descriptors.push((number, Some(current_time), entry.duration));
                number = number.checked_add(1).ok_or(DashError::TooManySegments)?;
                current_time = current_time
                    .checked_add(entry.duration)
                    .ok_or(DashError::Invalid("SegmentTimeline time overflowed"))?;
            }
        }
    } else {
        let duration = template
            .duration
            .filter(|duration| *duration > 0)
            .ok_or(DashError::Unsupported("segment_duration_missing"))?;
        let presentation_duration_nanos = presentation_duration_nanos
            .ok_or(DashError::Invalid("static MPD duration is missing"))?;
        let numerator = presentation_duration_nanos
            .checked_mul(u128::from(timescale))
            .ok_or(DashError::TooManySegments)?;
        let denominator = u128::from(duration) * 1_000_000_000;
        let count = numerator.div_ceil(denominator);
        let count = usize::try_from(count).map_err(|_| DashError::TooManySegments)?;
        if count == 0 || count > MAX_DASH_SEGMENTS {
            return Err(DashError::TooManySegments);
        }
        for offset in 0..count {
            let offset = u64::try_from(offset).map_err(|_| DashError::TooManySegments)?;
            descriptors.push((
                start_number
                    .checked_add(offset)
                    .ok_or(DashError::TooManySegments)?,
                Some(
                    offset
                        .checked_mul(duration)
                        .ok_or(DashError::TooManySegments)?,
                ),
                duration,
            ));
        }
    }
    let mut segments = Vec::with_capacity(descriptors.len());
    for (index, (number, time, duration)) in descriptors.into_iter().enumerate() {
        let reference =
            substitute_template(media, representation_id, bandwidth, Some(number), time)?;
        let url = resolve_resource_url(base_url, &reference)?;
        let duration_millis =
            u64::try_from((u128::from(duration) * 1000).div_ceil(u128::from(timescale)))
                .map_err(|_| DashError::Invalid("Segment duration overflowed"))?;
        segments.push(DashSegment {
            index: u32::try_from(index).map_err(|_| DashError::TooManySegments)?,
            number,
            time,
            url,
            duration_millis,
            duration_units: duration,
            timescale,
        });
    }
    Ok((initialization_url, segments))
}

fn substitute_template(
    template: &str,
    representation_id: &str,
    bandwidth: u64,
    number: Option<u64>,
    time: Option<u64>,
) -> Result<String, DashError> {
    let mut output = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut cursor = 0_usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'$' {
            let character = template[cursor..]
                .chars()
                .next()
                .ok_or(DashError::Invalid("DASH template is invalid UTF-8"))?;
            output.push(character);
            cursor += character.len_utf8();
            continue;
        }
        if bytes.get(cursor + 1) == Some(&b'$') {
            output.push('$');
            cursor += 2;
            continue;
        }
        let relative_end = template[cursor + 1..]
            .find('$')
            .ok_or(DashError::Invalid("DASH template token is unterminated"))?;
        let end = cursor + 1 + relative_end;
        let token = &template[cursor + 1..end];
        let (name, width) = parse_template_token(token)?;
        let value = match name {
            "RepresentationID" => representation_id.to_owned(),
            "Bandwidth" => format_number(bandwidth, width),
            "Number" => format_number(
                number.ok_or(DashError::Invalid("DASH Number token has no value"))?,
                width,
            ),
            "Time" => format_number(
                time.ok_or(DashError::Invalid("DASH Time token has no value"))?,
                width,
            ),
            _ => return Err(DashError::Unsupported("template_token")),
        };
        output.push_str(&value);
        if output.len() > MAX_DASH_URL_BYTES {
            return Err(DashError::MetadataTooLarge);
        }
        cursor = end + 1;
    }
    Ok(output)
}

fn parse_template_token(token: &str) -> Result<(&str, Option<usize>), DashError> {
    let Some((name, format)) = token.split_once('%') else {
        return Ok((token, None));
    };
    let width = format
        .strip_prefix('0')
        .and_then(|value| value.strip_suffix('d'))
        .ok_or(DashError::Unsupported("template_format"))?
        .parse::<usize>()
        .map_err(|_| DashError::Unsupported("template_format"))?;
    if !(1..=20).contains(&width) {
        return Err(DashError::Unsupported("template_format"));
    }
    Ok((name, Some(width)))
}

fn format_number(value: u64, width: Option<usize>) -> String {
    width.map_or_else(|| value.to_string(), |width| format!("{value:0width$}"))
}

fn resolve_node_base(parent: &Url, node: Node<'_, '_>) -> Result<Url, DashError> {
    let base_urls = element_children(node, "BaseURL").collect::<Vec<_>>();
    if base_urls.len() > 1 {
        return Err(DashError::Unsupported("multiple_base_urls"));
    }
    let Some(base_url) = base_urls.first() else {
        return Ok(parent.clone());
    };
    let value = base_url
        .text()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(DashError::Invalid("DASH BaseURL is empty"))?;
    let resolved = parent
        .join(value)
        .map_err(|_| DashError::Invalid("DASH BaseURL is invalid"))?;
    validate_url(resolved)
}

fn resolve_resource_url(base_url: &Url, value: &str) -> Result<String, DashError> {
    let resolved = base_url
        .join(value)
        .map_err(|_| DashError::Invalid("DASH resource URL is invalid"))?;
    let resolved = validate_url(resolved)?;
    if resolved.as_str().len() > MAX_DASH_URL_BYTES {
        return Err(DashError::MetadataTooLarge);
    }
    Ok(resolved.to_string())
}

fn validate_url(url: Url) -> Result<Url, DashError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(DashError::Unsupported("resource_scheme"));
    }
    if url.as_str().len() > MAX_DASH_URL_BYTES {
        return Err(DashError::MetadataTooLarge);
    }
    Ok(url)
}

fn parse_iso8601_duration_nanos(value: &str) -> Result<u128, DashError> {
    let rest = value
        .strip_prefix("PT")
        .ok_or(DashError::Unsupported("calendar_duration"))?;
    if rest.is_empty() {
        return Err(DashError::Invalid("DASH duration is empty"));
    }
    let mut remaining = rest;
    let mut total_nanos = 0_u128;
    for (suffix, seconds) in [('H', 3_600_u128), ('M', 60_u128)] {
        if let Some(index) = remaining.find(suffix) {
            let component = remaining[..index]
                .parse::<u128>()
                .map_err(|_| DashError::Invalid("DASH duration is invalid"))?;
            total_nanos = total_nanos
                .checked_add(
                    component
                        .checked_mul(seconds * 1_000_000_000)
                        .ok_or(DashError::Invalid("DASH duration overflowed"))?,
                )
                .ok_or(DashError::Invalid("DASH duration overflowed"))?;
            remaining = &remaining[index + 1..];
        }
    }
    if let Some(seconds) = remaining.strip_suffix('S') {
        let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
        if whole.is_empty()
            || fraction.len() > 9
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(DashError::Invalid("DASH duration is invalid"));
        }
        let whole = whole
            .parse::<u128>()
            .map_err(|_| DashError::Invalid("DASH duration is invalid"))?;
        let fraction = if fraction.is_empty() {
            0
        } else {
            fraction
                .parse::<u128>()
                .map_err(|_| DashError::Invalid("DASH duration is invalid"))?
                * 10_u128.pow(u32::try_from(9 - fraction.len()).unwrap_or_default())
        };
        total_nanos = total_nanos
            .checked_add(
                whole
                    .checked_mul(1_000_000_000)
                    .and_then(|value| value.checked_add(fraction))
                    .ok_or(DashError::Invalid("DASH duration overflowed"))?,
            )
            .ok_or(DashError::Invalid("DASH duration overflowed"))?;
        remaining = "";
    }
    if !remaining.is_empty() {
        return Err(DashError::Invalid("DASH duration is invalid"));
    }
    Ok(total_nanos)
}

fn bounded_text(value: &str) -> Result<String, DashError> {
    if value.len() > MAX_DASH_FIELD_BYTES {
        return Err(DashError::MetadataTooLarge);
    }
    Ok(value.to_owned())
}

fn optional_bounded_text(value: Option<&str>) -> Result<Option<String>, DashError> {
    value.map(bounded_text).transpose()
}

fn parse_u64_attribute(node: Node<'_, '_>, name: &str) -> Result<Option<u64>, DashError> {
    node.attribute(name)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| DashError::Invalid("DASH numeric attribute is invalid"))
        })
        .transpose()
}

fn parse_u32_attribute(node: Node<'_, '_>, name: &str) -> Result<Option<u32>, DashError> {
    parse_u64_attribute(node, name)?
        .map(|value| {
            u32::try_from(value)
                .map_err(|_| DashError::Invalid("DASH numeric attribute is too large"))
        })
        .transpose()
}

fn element_children<'a, 'input>(
    parent: Node<'a, 'input>,
    local_name: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    parent
        .children()
        .filter(move |child| child.is_element() && child.tag_name().name() == local_name)
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum DashError {
    #[error("invalid DASH MPD: {0}")]
    Invalid(&'static str),
    #[error("DASH feature is unsupported: {0}")]
    Unsupported(&'static str),
    #[error("DASH MPD contains too many Representations")]
    TooManyRepresentations,
    #[error("DASH MPD expands to too many Segments")]
    TooManySegments,
    #[error("DASH metadata exceeds bounded limits")]
    MetadataTooLarge,
}

#[cfg(test)]
mod tests {
    use super::{
        DashError, MAX_DASH_FIELD_BYTES, MAX_DASH_MANIFEST_BYTES, MAX_DASH_REPRESENTATIONS,
        parse_dash_manifest,
    };

    #[test]
    fn dynamic_and_protected_presentations_are_rejected_explicitly() {
        let base = url::Url::parse("https://media.example.test/manifest.mpd").expect("base URL");
        let dynamic = br#"<MPD type="dynamic"><Period duration="PT1S"/></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, dynamic),
            Err(DashError::Unsupported("dynamic_mpd"))
        );
        let protected = br#"<MPD type="static" mediaPresentationDuration="PT1S"><Period><AdaptationSet><ContentProtection/></AdaptationSet></Period></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, protected),
            Err(DashError::Unsupported("content_protection"))
        );
    }

    #[test]
    fn finite_timeline_does_not_require_a_presentation_duration() {
        let base = url::Url::parse("https://media.example.test/manifest.mpd").expect("base URL");
        let manifest = br#"<MPD type="static"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate media="$Time$.m4s"><SegmentTimeline><S t="0" d="2" r="1"/></SegmentTimeline></SegmentTemplate><Representation id="v" bandwidth="1"/></AdaptationSet></Period></MPD>"#;
        let parsed = parse_dash_manifest(&base, manifest).expect("parse finite timeline");
        assert_eq!(parsed.tracks[0].segments.len(), 2);
    }

    #[test]
    fn segment_list_is_rejected_even_when_a_template_is_inherited() {
        let base = url::Url::parse("https://media.example.test/manifest.mpd").expect("base URL");
        let manifest = br#"<MPD type="static" mediaPresentationDuration="PT1S"><Period><SegmentTemplate media="$Number$.m4s" duration="1"/><AdaptationSet contentType="video"><Representation id="v"><SegmentList/></Representation></AdaptationSet></Period></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, manifest),
            Err(DashError::Unsupported("segment_addressing_mode"))
        );
    }

    #[test]
    fn duration_template_expands_time_tokens_without_silently_applying_offsets() {
        let base = url::Url::parse("https://media.example.test/manifest.mpd").expect("base URL");
        let manifest = br#"<MPD type="static" mediaPresentationDuration="PT4S"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate timescale="1" media="$Time$.m4s" duration="2"/><Representation id="v"/></AdaptationSet></Period></MPD>"#;
        let parsed = parse_dash_manifest(&base, manifest).expect("parse duration Time template");
        assert!(parsed.tracks[0].segments[0].url.ends_with("/0.m4s"));
        assert!(parsed.tracks[0].segments[1].url.ends_with("/2.m4s"));

        let offset = br#"<MPD type="static" mediaPresentationDuration="PT4S"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate timescale="1" media="$Time$.m4s" duration="2" presentationTimeOffset="1"/><Representation id="v"/></AdaptationSet></Period></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, offset),
            Err(DashError::Unsupported("segment_timeline_offset"))
        );
    }

    #[test]
    fn fractional_presentation_duration_does_not_round_into_an_extra_segment() {
        let base = url::Url::parse("https://media.example.test/manifest.mpd").expect("base URL");
        let manifest = br#"<MPD type="static" mediaPresentationDuration="PT1.0001S"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate timescale="10000" media="$Time$.m4s" duration="10001"/><Representation id="v"/></AdaptationSet></Period></MPD>"#;
        let parsed = parse_dash_manifest(&base, manifest).expect("parse fractional duration");
        assert_eq!(parsed.tracks[0].segments.len(), 1);
        assert!(parsed.tracks[0].segments[0].url.ends_with("/0.m4s"));
    }

    #[test]
    fn unsupported_addressing_tokens_periods_repeats_and_containers_are_typed_failures() {
        let base = url::Url::parse("https://media.example.test/manifest.mpd").expect("base URL");
        let multiple = br#"<MPD type="static"><Period/><Period/></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, multiple),
            Err(DashError::Unsupported("multiple_periods"))
        );
        let negative = br#"<MPD type="static"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate media="$Time$.m4s"><SegmentTimeline><S d="1" r="-1"/></SegmentTimeline></SegmentTemplate><Representation id="v"/></AdaptationSet></Period></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, negative),
            Err(DashError::Unsupported("negative_timeline_repeat"))
        );
        let unknown = br#"<MPD type="static" mediaPresentationDuration="PT1S"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate media="$Unknown$.m4s" duration="1"/><Representation id="v"/></AdaptationSet></Period></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, unknown),
            Err(DashError::Unsupported("template_token"))
        );
        let webm = br#"<MPD type="static" mediaPresentationDuration="PT1S"><Period><AdaptationSet contentType="video" mimeType="video/webm"><SegmentTemplate media="$Number$.webm" duration="1"/><Representation id="v"/></AdaptationSet></Period></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, webm),
            Err(DashError::Unsupported("media_container"))
        );
    }

    #[test]
    fn manifest_representation_segment_and_field_budgets_are_hard_limits() {
        let base = url::Url::parse("https://media.example.test/manifest.mpd").expect("base URL");
        assert_eq!(
            parse_dash_manifest(&base, &vec![b' '; MAX_DASH_MANIFEST_BYTES + 1]),
            Err(DashError::MetadataTooLarge)
        );

        let representations = (0..=MAX_DASH_REPRESENTATIONS)
            .map(|index| format!(r#"<Representation id="v{index}"/>"#))
            .collect::<String>();
        let manifest = format!(
            r#"<MPD type="static" mediaPresentationDuration="PT1S"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate media="$Number$.m4s" duration="1"/>{representations}</AdaptationSet></Period></MPD>"#
        );
        assert_eq!(
            parse_dash_manifest(&base, manifest.as_bytes()),
            Err(DashError::TooManyRepresentations)
        );

        let too_many_segments = br#"<MPD type="static"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate media="$Time$.m4s"><SegmentTimeline><S d="1" r="10000"/></SegmentTimeline></SegmentTemplate><Representation id="v"/></AdaptationSet></Period></MPD>"#;
        assert_eq!(
            parse_dash_manifest(&base, too_many_segments),
            Err(DashError::TooManySegments)
        );

        let long_id = "x".repeat(MAX_DASH_FIELD_BYTES + 1);
        let manifest = format!(
            r#"<MPD type="static" mediaPresentationDuration="PT1S"><Period><AdaptationSet contentType="video" mimeType="video/mp4"><SegmentTemplate media="$Number$.m4s" duration="1"/><Representation id="{long_id}"/></AdaptationSet></Period></MPD>"#
        );
        assert_eq!(
            parse_dash_manifest(&base, manifest.as_bytes()),
            Err(DashError::MetadataTooLarge)
        );
    }
}
