use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    CaptureOutcome, CaptureSource, ClipboardHistory, ClipboardHistoryError, ClipboardNativeFormat,
    FilesCapture, ImageCapture, TextCapture,
};

const ARCHIVE_VERSION: u32 = 1;
const MAX_ARCHIVE_ITEMS: usize = 100_000;
const MAX_MANIFEST_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOTAL_ASSET_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardArchiveExportReport {
    pub item_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClipboardArchiveImportReport {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub assets_written: usize,
}

#[derive(Debug, Error)]
pub enum ClipboardArchiveError {
    #[error("clipboard history operation failed: {0}")]
    History(#[from] ClipboardHistoryError),
    #[error("clipboard archive I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid clipboard ZIP archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid clipboard archive manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported clipboard archive: {0}")]
    Invalid(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveManifest {
    version: u32,
    items: Vec<ArchiveItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveItem {
    kind: String,
    text: String,
    image_entry: Option<String>,
    image_width: u32,
    image_height: u32,
    file_paths: Vec<String>,
    source: ArchiveSource,
    captured_at_unix_ms: i64,
    is_favorite: bool,
    is_pinned: bool,
    formats: Vec<ArchiveFormat>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveSource {
    window_handle: i64,
    process_id: u32,
    process_name: String,
    icon_entry: Option<String>,
    window_title: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveFormat {
    format_id: u32,
    format_name: String,
    data_entry: String,
    size_bytes: u64,
    sort_order: u32,
}

#[derive(Debug, Clone, Copy)]
enum AssetKind {
    Image,
    Icon,
    NativeFormat,
}

impl AssetKind {
    fn archive_directory(self) -> &'static str {
        match self {
            Self::Image => "images",
            Self::Icon => "icons",
            Self::NativeFormat => "formats",
        }
    }

    fn destination(self, root: &Path, digest: &str) -> PathBuf {
        match self {
            Self::Image => root.join(format!("{digest}.png")),
            Self::Icon => root.join(format!("source-{digest}.png")),
            Self::NativeFormat => root.join("native-formats").join(format!("{digest}.bin")),
        }
    }
}

pub(crate) fn export_archive(
    history: &ClipboardHistory,
    destination: &Path,
) -> Result<ClipboardArchiveExportReport, ClipboardArchiveError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension(format!("nxtclip-{}.tmp", uuid::Uuid::new_v4()));
    let result = export_archive_to(history, &temporary);
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(&temporary, destination)?;
    result
}

fn export_archive_to(
    history: &ClipboardHistory,
    destination: &Path,
) -> Result<ClipboardArchiveExportReport, ClipboardArchiveError> {
    let output = File::create(destination)?;
    let mut archive = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut written_entries = HashSet::new();
    let mut manifest_items = Vec::new();

    for item in history.recent(MAX_ARCHIVE_ITEMS)? {
        let image_entry = add_asset(
            &mut archive,
            &mut written_entries,
            &item.image_path,
            AssetKind::Image,
            options,
        )?;
        let icon_entry = add_asset(
            &mut archive,
            &mut written_entries,
            &item.source.process_icon_path,
            AssetKind::Icon,
            options,
        )?;
        let mut formats = Vec::new();
        for format in history.native_formats(item.id)? {
            let Some(data_entry) = add_asset(
                &mut archive,
                &mut written_entries,
                &format.data_path,
                AssetKind::NativeFormat,
                options,
            )?
            else {
                continue;
            };
            formats.push(ArchiveFormat {
                format_id: format.format_id,
                format_name: format.format_name,
                data_entry,
                size_bytes: format.size_bytes,
                sort_order: format.sort_order,
            });
        }
        manifest_items.push(ArchiveItem {
            kind: item.kind.as_str().into(),
            text: item.text,
            image_entry,
            image_width: item.image_width,
            image_height: item.image_height,
            file_paths: item.file_paths,
            source: ArchiveSource {
                window_handle: item.source.window_handle,
                process_id: item.source.process_id,
                process_name: item.source.process_name,
                icon_entry,
                window_title: item.source.window_title,
            },
            captured_at_unix_ms: item.captured_at_unix_ms,
            is_favorite: item.is_favorite,
            is_pinned: item.is_pinned,
            formats,
        });
    }

    archive.start_file("manifest.json", options)?;
    serde_json::to_writer(
        &mut archive,
        &ArchiveManifest {
            version: ARCHIVE_VERSION,
            items: manifest_items,
        },
    )?;
    let output = archive.finish()?;
    output.sync_all()?;
    Ok(ClipboardArchiveExportReport {
        item_count: history.counts()?.total.min(MAX_ARCHIVE_ITEMS),
        asset_count: written_entries.len(),
    })
}

fn add_asset(
    archive: &mut ZipWriter<File>,
    written_entries: &mut HashSet<String>,
    source: &str,
    kind: AssetKind,
    options: SimpleFileOptions,
) -> Result<Option<String>, ClipboardArchiveError> {
    if source.is_empty() || !Path::new(source).is_file() {
        return Ok(None);
    }
    let digest = file_digest(Path::new(source))?;
    let entry_name = format!(
        "{}/{}",
        kind.archive_directory(),
        archive_file_name(kind, &digest)
    );
    if written_entries.insert(entry_name.clone()) {
        archive.start_file(&entry_name, options)?;
        let mut input = File::open(source)?;
        std::io::copy(&mut input, archive)?;
    }
    Ok(Some(entry_name))
}

fn archive_file_name(kind: AssetKind, digest: &str) -> String {
    match kind {
        AssetKind::Image | AssetKind::Icon => format!("{digest}.png"),
        AssetKind::NativeFormat => format!("{digest}.bin"),
    }
}

pub(crate) fn import_archive(
    history: &mut ClipboardHistory,
    archive_path: &Path,
    asset_root: &Path,
) -> Result<ClipboardArchiveImportReport, ClipboardArchiveError> {
    fs::create_dir_all(asset_root.join("native-formats"))?;
    let input = File::open(archive_path)?;
    let mut archive = ZipArchive::new(input)?;
    let manifest = read_manifest(&mut archive)?;
    if manifest.version != ARCHIVE_VERSION {
        return Err(ClipboardArchiveError::Invalid(format!(
            "manifest version {} is not supported",
            manifest.version
        )));
    }
    if manifest.items.len() > MAX_ARCHIVE_ITEMS {
        return Err(ClipboardArchiveError::Invalid(format!(
            "manifest contains too many items: {}",
            manifest.items.len()
        )));
    }

    let mut report = ClipboardArchiveImportReport::default();
    let mut total_asset_bytes = 0_u64;
    for item in manifest.items {
        let image_path = match item.image_entry.as_deref() {
            Some(entry) => extract_asset(
                &mut archive,
                entry,
                AssetKind::Image,
                asset_root,
                &mut total_asset_bytes,
                &mut report.assets_written,
            )?,
            None => String::new(),
        };
        let icon_path = match item.source.icon_entry.as_deref() {
            Some(entry) => extract_asset(
                &mut archive,
                entry,
                AssetKind::Icon,
                asset_root,
                &mut total_asset_bytes,
                &mut report.assets_written,
            )?,
            None => String::new(),
        };
        let mut formats = Vec::with_capacity(item.formats.len());
        for format in item.formats {
            let data_path = extract_asset(
                &mut archive,
                &format.data_entry,
                AssetKind::NativeFormat,
                asset_root,
                &mut total_asset_bytes,
                &mut report.assets_written,
            )?;
            formats.push(ClipboardNativeFormat {
                format_id: format.format_id,
                format_name: format.format_name,
                data_path,
                size_bytes: format.size_bytes,
                sort_order: format.sort_order,
            });
        }
        let source = CaptureSource {
            window_handle: item.source.window_handle,
            process_id: item.source.process_id,
            process_name: item.source.process_name,
            process_icon_path: icon_path,
            window_title: item.source.window_title,
        };
        let outcome = match item.kind.as_str() {
            "text" if !item.text.is_empty() => history.capture(TextCapture {
                text: item.text,
                native_formats: formats,
                source,
                captured_at_unix_ms: item.captured_at_unix_ms,
            })?,
            "image" if !image_path.is_empty() => history.capture_image(ImageCapture {
                image_path,
                width: item.image_width,
                height: item.image_height,
                native_formats: formats,
                source,
                captured_at_unix_ms: item.captured_at_unix_ms,
            })?,
            "files" if !item.file_paths.is_empty() => history.capture_files(FilesCapture {
                paths: item.file_paths,
                native_formats: formats,
                source,
                captured_at_unix_ms: item.captured_at_unix_ms,
            })?,
            _ => {
                report.skipped += 1;
                continue;
            }
        };
        let imported = match &outcome {
            CaptureOutcome::Inserted(item) => {
                report.inserted += 1;
                item
            }
            CaptureOutcome::Updated(item) => {
                report.updated += 1;
                item
            }
        };
        if item.is_favorite {
            history.set_favorite(imported.id, true)?;
        }
        if item.is_pinned {
            history.set_pinned(imported.id, true)?;
        }
    }
    Ok(report)
}

fn read_manifest(archive: &mut ZipArchive<File>) -> Result<ArchiveManifest, ClipboardArchiveError> {
    let mut entry = archive.by_name("manifest.json")?;
    if entry.size() > MAX_MANIFEST_BYTES {
        return Err(ClipboardArchiveError::Invalid(
            "manifest.json exceeds the size limit".into(),
        ));
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn extract_asset(
    archive: &mut ZipArchive<File>,
    entry_name: &str,
    kind: AssetKind,
    asset_root: &Path,
    total_asset_bytes: &mut u64,
    assets_written: &mut usize,
) -> Result<String, ClipboardArchiveError> {
    validate_entry_name(entry_name, kind.archive_directory())?;
    let mut entry = archive.by_name(entry_name)?;
    if !entry.is_file() || entry.size() > MAX_ASSET_BYTES {
        return Err(ClipboardArchiveError::Invalid(format!(
            "asset entry is not a bounded file: {entry_name}"
        )));
    }
    *total_asset_bytes = total_asset_bytes.saturating_add(entry.size());
    if *total_asset_bytes > MAX_TOTAL_ASSET_BYTES {
        return Err(ClipboardArchiveError::Invalid(
            "archive assets exceed the total size limit".into(),
        ));
    }

    let temporary = asset_root.join(format!(".import-{}.tmp", uuid::Uuid::new_v4()));
    let mut output = File::create(&temporary)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = entry.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
    }
    output.sync_all()?;
    drop(output);
    let digest = format!("{:x}", hasher.finalize());
    let destination = kind.destination(asset_root, &digest);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if destination.exists() {
        fs::remove_file(&temporary)?;
    } else {
        fs::rename(&temporary, &destination)?;
        *assets_written += 1;
    }
    Ok(destination.to_string_lossy().into_owned())
}

fn validate_entry_name(
    entry_name: &str,
    expected_directory: &str,
) -> Result<(), ClipboardArchiveError> {
    if entry_name.contains('\\') || !entry_name.starts_with(&format!("{expected_directory}/")) {
        return Err(ClipboardArchiveError::Invalid(format!(
            "unsafe asset entry: {entry_name}"
        )));
    }
    let path = Path::new(entry_name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ClipboardArchiveError::Invalid(format!(
            "unsafe asset entry: {entry_name}"
        )));
    }
    Ok(())
}

fn file_digest(path: &Path) -> Result<String, std::io::Error> {
    let mut input = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
