//! Passive owned payloads: bounded file reads/decoders, never path-backed rendering or Shell.
use super::model::RowId;
use crate::ui::markdown_document::DocumentMode;
use gpui_kit::RenderImage;
use image::{ImageDecoder as _, ImageFormat, ImageReader};
use nxtools_protocol::archive_browser::ArchiveBrowserContext;
use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use uuid::Uuid;

pub(super) const IMAGE_BYTES: u64 = 8 << 20;
pub(super) const TEXT_BYTES: u64 = 64 << 10;
const MAX_AXIS: u32 = 8192;
const MAX_PIXELS: u64 = 8 * 1024 * 1024;
// Decoder scratch/output allowance 32 MiB + separate final RGBA allowance 32 MiB.
// Encoded input is separately <=8 MiB. Reject >8-bit channels before allocating pixels.
const DECODE_BYTES: u64 = 32 << 20;

#[derive(Clone)]
pub(crate) enum PreviewPayload {
    Image {
        image: Arc<RenderImage>,
        width: u32,
        height: u32,
    },
    Text {
        text: String,
        mode: DocumentMode,
    },
}
#[derive(Clone)]
pub(crate) enum PreviewResult {
    Idle,
    Loading,
    Ready(PreviewPayload),
    Unsupported,
    TooLarge,
    ReadLimit,
    TextLines,
    ImageDimensions,
    ImageMemory,
    DocumentLimit,
    Directory,
    MultipleSelection,
    Cancelled,
    PasswordCancelled,
    SourceChanged,
    Disconnected,
    Corrupt,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreviewKind {
    Image,
    Text(DocumentMode),
}
impl PreviewKind {
    pub(super) fn limit(self) -> u64 {
        match self {
            Self::Image => IMAGE_BYTES,
            Self::Text(_) => TEXT_BYTES,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreviewFailure {
    TooLarge,
    TextLines,
    ImageDimensions,
    ImageMemory,
    DocumentLimit,
    Unsupported,
    Corrupt,
    Cancelled,
    Io,
}
impl From<PreviewFailure> for PreviewResult {
    fn from(value: PreviewFailure) -> Self {
        match value {
            PreviewFailure::TooLarge => Self::TooLarge,
            PreviewFailure::TextLines => Self::TextLines,
            PreviewFailure::ImageDimensions => Self::ImageDimensions,
            PreviewFailure::ImageMemory => Self::ImageMemory,
            PreviewFailure::DocumentLimit => Self::DocumentLimit,
            PreviewFailure::Unsupported => Self::Unsupported,
            PreviewFailure::Corrupt => Self::Corrupt,
            PreviewFailure::Cancelled => Self::Cancelled,
            PreviewFailure::Io => Self::Failed,
        }
    }
}
#[derive(Clone)]
pub(super) struct PreviewRequest {
    pub window_id: Uuid,
    pub generation: u64,
    pub browser: ArchiveBrowserContext,
    pub row: RowId,
    pub selection_generation: u64,
    pub kind: PreviewKind,
    pub started: Instant,
}
#[derive(Default)]
pub(super) struct PreviewState {
    pub selection_generation: u64,
    pub row: Option<RowId>,
    pub latest: Option<PreviewRequest>,
    /// Occupied until native actualcleanup AND any actual read/decode complete.
    pub running: Option<Uuid>,
    pub cancel: Arc<AtomicBool>,
    pub retired: bool,
    pub blocked: bool,
}

pub(super) fn kind(name: &str) -> Option<PreviewKind> {
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => PreviewKind::Image,
        "md" | "markdown" => PreviewKind::Text(DocumentMode::Markdown),
        "rs" => PreviewKind::Text(DocumentMode::Literal(Some("rust"))),
        "js" | "mjs" | "cjs" => PreviewKind::Text(DocumentMode::Literal(Some("javascript"))),
        "ts" => PreviewKind::Text(DocumentMode::Literal(Some("typescript"))),
        "tsx" | "jsx" => PreviewKind::Text(DocumentMode::Literal(Some("tsx"))),
        "py" => PreviewKind::Text(DocumentMode::Literal(Some("python"))),
        "json" => PreviewKind::Text(DocumentMode::Literal(Some("json"))),
        "html" | "htm" | "xml" => PreviewKind::Text(DocumentMode::Literal(Some("html"))),
        "css" => PreviewKind::Text(DocumentMode::Literal(Some("css"))),
        "c" | "h" => PreviewKind::Text(DocumentMode::Literal(Some("c"))),
        "cpp" | "hpp" => PreviewKind::Text(DocumentMode::Literal(Some("cpp"))),
        "cs" => PreviewKind::Text(DocumentMode::Literal(Some("csharp"))),
        "java" => PreviewKind::Text(DocumentMode::Literal(Some("java"))),
        "go" => PreviewKind::Text(DocumentMode::Literal(Some("go"))),
        "sh" => PreviewKind::Text(DocumentMode::Literal(Some("bash"))),
        "sql" => PreviewKind::Text(DocumentMode::Literal(Some("sql"))),
        "toml" | "yaml" | "yml" | "kt" | "ps1" | "bat" | "cmd" | "txt" | "log" | "csv" | "ini"
        | "cfg" | "conf" | "gitignore" => PreviewKind::Text(DocumentMode::Literal(None)),
        _ => return None,
    })
}

fn permits() -> &'static (async_channel::Sender<()>, async_channel::Receiver<()>) {
    static PERMITS: OnceLock<(async_channel::Sender<()>, async_channel::Receiver<()>)> =
        OnceLock::new();
    PERMITS.get_or_init(|| {
        let (send, receive) = async_channel::bounded(2);
        let _ = send.try_send(());
        let _ = send.try_send(());
        (send, receive)
    })
}
pub(super) struct DecodePermit;
impl Drop for DecodePermit {
    fn drop(&mut self) {
        let _ = permits().0.try_send(());
    }
}
pub(super) async fn acquire() -> DecodePermit {
    let _ = permits().1.recv().await;
    DecodePermit
}
fn cancelled(stop: &AtomicBool) -> Result<(), PreviewFailure> {
    if stop.load(Ordering::Acquire) {
        Err(PreviewFailure::Cancelled)
    } else {
        Ok(())
    }
}
pub(super) fn read_bounded(
    mut reader: impl Read,
    limit: u64,
    stop: &AtomicBool,
) -> Result<Vec<u8>, PreviewFailure> {
    // Exact allocation avoids geometric Vec growth beyond the encoded-input allowance.
    let mut bytes = Vec::with_capacity(limit as usize + 1);
    let mut chunk = [0; 8192];
    loop {
        cancelled(stop)?;
        let remaining = (limit + 1)
            .saturating_sub(bytes.len() as u64)
            .min(chunk.len() as u64) as usize;
        if remaining == 0 {
            return Err(PreviewFailure::TooLarge);
        }
        let read = reader
            .read(&mut chunk[..remaining])
            .map_err(|_| PreviewFailure::Io)?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() as u64 > limit {
            return Err(PreviewFailure::TooLarge);
        }
    }
}
pub(super) fn prepare(
    path: &Path,
    kind: PreviewKind,
    stop: &AtomicBool,
) -> Result<PreviewPayload, PreviewFailure> {
    cancelled(stop)?;
    let shape = std::fs::symlink_metadata(path).map_err(|_| PreviewFailure::Io)?;
    if shape.file_type().is_symlink() || !shape.is_file() {
        return Err(PreviewFailure::Unsupported);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        if shape.file_attributes() & 0x400 != 0 {
            return Err(PreviewFailure::Unsupported);
        }
    }
    let file = File::open(path).map_err(|_| PreviewFailure::Io)?;
    let metadata = file.metadata().map_err(|_| PreviewFailure::Io)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(PreviewFailure::Unsupported);
        }
    }
    if !metadata.is_file() {
        return Err(PreviewFailure::Unsupported);
    }
    if metadata.len() > kind.limit() {
        return Err(PreviewFailure::TooLarge);
    }
    let bytes = read_bounded(&file, kind.limit(), stop)?;
    if file.metadata().map_err(|_| PreviewFailure::Io)?.len() != bytes.len() as u64 {
        return Err(PreviewFailure::Io);
    }
    decode_bytes(bytes, kind, stop)
}
pub(super) fn check_dimensions(width: u32, height: u32) -> Result<(), PreviewFailure> {
    if width == 0 || height == 0 {
        Err(PreviewFailure::Corrupt)
    } else if width > MAX_AXIS
        || height > MAX_AXIS
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        Err(PreviewFailure::ImageDimensions)
    } else {
        Ok(())
    }
}
pub(super) fn decode_bytes(
    bytes: Vec<u8>,
    kind: PreviewKind,
    stop: &AtomicBool,
) -> Result<PreviewPayload, PreviewFailure> {
    cancelled(stop)?;
    if bytes.len() as u64 > kind.limit() {
        return Err(PreviewFailure::TooLarge);
    }
    if let PreviewKind::Text(mut mode) = kind {
        let source = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        let text = std::str::from_utf8(source).map_err(|_| PreviewFailure::Unsupported)?;
        if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
        {
            return Err(PreviewFailure::Unsupported);
        }
        if text.lines().take(1201).count() > 1200 {
            return Err(PreviewFailure::TextLines);
        }
        if text.chars().take(65537).count() > 65536 {
            return Err(PreviewFailure::DocumentLimit);
        }
        // Same safe projection as Clipboard/Quick Launch; unsafe/complex Markdown becomes literal.
        if crate::ui::markdown_document::prepared_source(text, mode).is_none() {
            mode = DocumentMode::Literal(None);
        }
        if crate::ui::markdown_document::prepared_source(text, mode).is_none() {
            return Err(PreviewFailure::DocumentLimit);
        }
        cancelled(stop)?;
        return Ok(PreviewPayload::Text {
            text: text.to_owned(),
            mode,
        });
    }
    if image::guess_format(&bytes).ok() == Some(ImageFormat::WebP) {
        webp_admission(&bytes)?;
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| PreviewFailure::Corrupt)?;
    if !matches!(
        reader.format(),
        Some(
            ImageFormat::Png
                | ImageFormat::Jpeg
                | ImageFormat::Gif
                | ImageFormat::WebP
                | ImageFormat::Bmp
        )
    ) {
        return Err(PreviewFailure::Corrupt);
    }
    let format = reader.format().ok_or(PreviewFailure::Corrupt)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_AXIS);
    limits.max_image_height = Some(MAX_AXIS);
    limits.max_alloc = Some(DECODE_BYTES);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().map_err(image_failure)?;
    let (width, height) = decoder.dimensions();
    check_dimensions(width, height)?;
    let encoded_bytes = reader_input_estimate(format);
    let pixels = u64::from(width) * u64::from(height);
    // Locked JPEG/WebP ignore max_alloc for internal tables. Conservatively reserve
    // 32 bytes/pixel plus input copy/table slack before invoking their pixel decoder.
    if matches!(format, ImageFormat::Jpeg | ImageFormat::WebP)
        && pixels.saturating_mul(32).saturating_add(encoded_bytes) > DECODE_BYTES
    {
        return Err(PreviewFailure::ImageMemory);
    }
    if !matches!(
        decoder.color_type(),
        image::ColorType::L8
            | image::ColorType::La8
            | image::ColorType::Rgb8
            | image::ColorType::Rgba8
    ) {
        return Err(PreviewFailure::Unsupported);
    }
    if decoder.total_bytes() > DECODE_BYTES {
        return Err(PreviewFailure::ImageMemory);
    }
    // Mirror ImageReader::decode: reserve decoded bytes BEFORE allowing decoder scratch.
    // Calling DynamicImage::from_decoder alone does not perform this reservation.
    limits
        .reserve(decoder.total_bytes())
        .map_err(image_failure)?;
    decoder.set_limits(limits).map_err(image_failure)?;
    cancelled(stop)?;
    // DynamicImage reads only the first frame; decoder is consumed/dropped before conversion.
    // Original <=32 MiB + final <=32 MiB may coexist. RGBA is moved without cloning.
    let mut pixels = image::DynamicImage::from_decoder(decoder)
        .map_err(image_failure)?
        .into_rgba8();
    cancelled(stop)?;
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let image = Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]));
    Ok(PreviewPayload::Image {
        image,
        width,
        height,
    })
}
fn image_failure(failure: image::ImageError) -> PreviewFailure {
    match failure {
        image::ImageError::Limits(limit) => match limit.kind() {
            image::error::LimitErrorKind::DimensionError => PreviewFailure::ImageDimensions,
            _ => PreviewFailure::ImageMemory,
        },
        _ => PreviewFailure::Corrupt,
    }
}

fn reader_input_estimate(format: ImageFormat) -> u64 {
    if matches!(format, ImageFormat::Jpeg | ImageFormat::WebP) {
        IMAGE_BYTES + (1 << 20)
    } else {
        0
    }
}
/// The released wrapper cannot cap WebP's meta-Huffman allocations. Admit static,
/// directly bounded bitstreams only; do not silently trust max_alloc or animate.
fn webp_admission(bytes: &[u8]) -> Result<(), PreviewFailure> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err(PreviewFailure::Corrupt);
    }
    if u32::from_le_bytes(
        bytes[4..8]
            .try_into()
            .map_err(|_| PreviewFailure::Corrupt)?,
    ) as usize
        + 8
        != bytes.len()
    {
        return Err(PreviewFailure::Corrupt);
    }
    let mut at = 12usize;
    let mut pixels = false;
    while at < bytes.len() {
        let header = bytes.get(at..at + 8).ok_or(PreviewFailure::Corrupt)?;
        let size = u32::from_le_bytes(
            header[4..8]
                .try_into()
                .map_err(|_| PreviewFailure::Corrupt)?,
        ) as usize;
        let start = at.checked_add(8).ok_or(PreviewFailure::Corrupt)?;
        let end = start.checked_add(size).ok_or(PreviewFailure::Corrupt)?;
        let data = bytes.get(start..end).ok_or(PreviewFailure::Corrupt)?;
        match &header[..4] {
            b"VP8X" => {
                if data.len() != 10 {
                    return Err(PreviewFailure::Corrupt);
                }
                if data[0] & 2 != 0 {
                    return Err(PreviewFailure::Unsupported);
                }
                let width = 1 + u32::from_le_bytes([data[4], data[5], data[6], 0]);
                let height = 1 + u32::from_le_bytes([data[7], data[8], data[9], 0]);
                check_dimensions(width, height)?;
                if (u64::from(width) * u64::from(height))
                    .saturating_mul(32)
                    .saturating_add(reader_input_estimate(ImageFormat::WebP))
                    > DECODE_BYTES
                {
                    return Err(PreviewFailure::ImageMemory);
                }
            }
            b"ANIM" | b"ANMF" | b"ALPH" => return Err(PreviewFailure::Unsupported),
            b"VP8L" => {
                if pixels || data.len() < 6 || data[0] != 0x2f {
                    return Err(PreviewFailure::Corrupt);
                }
                let header = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                let width = 1 + (header & 0x3fff);
                let height = 1 + ((header >> 14) & 0x3fff);
                check_dimensions(width, height)?;
                if (u64::from(width) * u64::from(height))
                    .saturating_mul(32)
                    .saturating_add(reader_input_estimate(ImageFormat::WebP))
                    > DECODE_BYTES
                {
                    return Err(PreviewFailure::ImageMemory);
                }
                // VP8L header40bits, then transform-present/cache-present/meta-Huffman.
                // Subtract-green (type2) is allocation-free; other transforms are refused.
                let mut bit = 40usize;
                let read = |bit: &mut usize, count: usize| -> Result<u8, PreviewFailure> {
                    let mut value = 0;
                    for shift in 0..count {
                        let byte = *data.get(*bit / 8).ok_or(PreviewFailure::Corrupt)?;
                        value |= ((byte >> (*bit % 8)) & 1) << shift;
                        *bit += 1;
                    }
                    Ok(value)
                };
                if read(&mut bit, 1)? != 0 && (read(&mut bit, 2)? != 2 || read(&mut bit, 1)? != 0) {
                    return Err(PreviewFailure::Unsupported);
                }
                if read(&mut bit, 1)? != 0 || read(&mut bit, 1)? != 0 {
                    return Err(PreviewFailure::Unsupported);
                }
                pixels = true;
            }
            b"VP8 " => {
                if pixels || data.len() < 10 || data[0] & 1 != 0 || data[3..6] != [0x9d, 1, 0x2a] {
                    return Err(PreviewFailure::Corrupt);
                }
                let width = u16::from_le_bytes([data[6], data[7]]) & 0x3fff;
                let height = u16::from_le_bytes([data[8], data[9]]) & 0x3fff;
                check_dimensions(u32::from(width), u32::from(height))?;
                if (u64::from(width) * u64::from(height))
                    .saturating_mul(32)
                    .saturating_add(reader_input_estimate(ImageFormat::WebP))
                    > DECODE_BYTES
                {
                    return Err(PreviewFailure::ImageMemory);
                }
                pixels = true;
            }
            _ => {}
        }
        at = end.checked_add(size & 1).ok_or(PreviewFailure::Corrupt)?;
    }
    if pixels && at == bytes.len() {
        Ok(())
    } else {
        Err(PreviewFailure::Corrupt)
    }
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
