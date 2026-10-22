use std::{path::PathBuf, sync::Arc, time::SystemTime};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveFormat {
    Zip,
    SevenZ,
    Rar4,
    Rar5,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    TarZstd,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    File,
    Directory,
    Link,
    Special,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameEncoding {
    Auto,
    Utf8,
    Gbk,
    Big5,
    ShiftJis,
    Korean,
    Cp437,
    Windows1250,
    Windows1251,
    Windows1252,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameProvenance {
    Utf8Flag,
    UnicodeExtra,
    FormatUnicode,
    Explicit(NameEncoding),
    /// Unique bounded script candidate, not authoritative or guaranteed language detection.
    Heuristic(NameEncoding),
    UncertainCp437,
}

/// Identity is session-scoped, never a display path or sorted row index.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntryId {
    pub session: Uuid,
    pub ordinal: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryWarning {
    UnsafePath,
    OutputCollision,
    LinkNotExtracted,
    UncertainEncoding,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct ZipAes {
    pub version: u16,
    pub strength: u8,
    pub method: u16,
}

#[derive(Clone)]
pub(crate) struct ZipLocation {
    pub header_offset: u64,
    pub directory_offset: u64,
    pub flags: u16,
    pub zip64_sizes: bool,
    pub aes: Option<ZipAes>,
}

#[derive(Clone)]
pub struct ArchiveEntry {
    pub(crate) id: EntryId,
    pub(crate) raw_name: Vec<u8>,
    pub(crate) display_path: String,
    pub(crate) provenance: NameProvenance,
    pub(crate) kind: EntryKind,
    pub(crate) size: Option<u64>,
    pub(crate) compressed_size: Option<u64>,
    pub(crate) method: u16,
    pub(crate) encrypted: bool,
    pub(crate) crc32: Option<u32>,
    /// Original DOS local wall time, not falsely labeled UTC.
    pub(crate) dos_modified: Option<(u16, u16)>,
    pub(crate) metadata: crate::metadata::OutputMetadata,
    pub(crate) warnings: Vec<EntryWarning>,
    pub(crate) zip: Option<ZipLocation>,
}

impl ArchiveEntry {
    pub fn id(&self) -> EntryId {
        self.id
    }
    pub fn raw_name(&self) -> &[u8] {
        &self.raw_name
    }
    pub fn display_path(&self) -> &str {
        &self.display_path
    }
    pub fn name_provenance(&self) -> NameProvenance {
        self.provenance
    }
    pub fn kind(&self) -> EntryKind {
        self.kind
    }
    pub fn size(&self) -> Option<u64> {
        self.size
    }
    pub fn compressed_size(&self) -> Option<u64> {
        self.compressed_size
    }
    pub fn method(&self) -> u16 {
        self.method
    }
    pub fn encrypted(&self) -> bool {
        self.encrypted
    }
    pub fn crc32(&self) -> Option<u32> {
        self.crc32
    }
    pub fn dos_modified(&self) -> Option<(u16, u16)> {
        self.dos_modified
    }
    pub fn metadata(&self) -> crate::metadata::OutputMetadata {
        self.metadata
    }
    pub fn warnings(&self) -> &[EntryWarning] {
        &self.warnings
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ArchiveCapabilities {
    pub test: bool,
    pub extract: bool,
    pub create: bool,
    pub modify: bool,
    pub split_read: bool,
    pub split_write: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct SourceRevision {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub digest: [u8; 32],
    pub volumes: Vec<SourceRevision>,
}

/// Immutable bounded metadata; does not retain a native handle or GPUI entity.
#[derive(Clone)]
pub struct ArchiveSnapshot {
    pub(crate) revision: SourceRevision,
    pub(crate) session: Uuid,
    pub(crate) format: ArchiveFormat,
    pub(crate) capabilities: ArchiveCapabilities,
    pub(crate) entries: Arc<[ArchiveEntry]>,
    pub(crate) comment: String,
    pub(crate) backend_safe: bool,
}

impl ArchiveSnapshot {
    pub fn format(&self) -> ArchiveFormat {
        self.format
    }
    pub fn session_id(&self) -> Uuid {
        self.session
    }
    pub fn capabilities(&self) -> ArchiveCapabilities {
        self.capabilities
    }
    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    pub fn comment(&self) -> &str {
        &self.comment
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ResourceLimits {
    pub max_entries: u64,
    pub max_metadata_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_output_bytes: u64,
    pub max_working_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_metadata_bytes: 256 << 20,
            max_entry_bytes: 32 << 30,
            max_output_bytes: 100 << 30,
            max_working_bytes: 512 << 20,
        }
    }
}

pub struct OpenRequest {
    pub path: PathBuf,
    pub encoding: NameEncoding,
}

impl OpenRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            encoding: NameEncoding::Auto,
        }
    }
}

/// Only implemented operations are exposed; extraction is gated on the native output capability.
pub struct ArchiveRequest {
    pub(crate) snapshot: ArchiveSnapshot,
}
impl ArchiveRequest {
    pub fn test(snapshot: ArchiveSnapshot) -> Self {
        Self { snapshot }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveOutcome {
    pub(crate) files_verified: u64,
    pub(crate) bytes_verified: u64,
}
impl ArchiveOutcome {
    pub fn files_verified(&self) -> u64 {
        self.files_verified
    }
    pub fn bytes_verified(&self) -> u64 {
        self.bytes_verified
    }
}
