//! Host-neutral archive operations. Windows handles, Shell and GPUI stay outside this crate.
//! Only verified operations are advertised; Windows extraction awaits a native output capability.
mod control;
pub mod creation;
mod encoding;
mod engine;
mod error;
mod formats;
pub mod interaction;
pub mod metadata;
mod model;
pub mod output;
pub mod output_readmission;
mod path_policy;
pub mod planning;
pub mod product;
pub mod product_job;
pub mod source_deletion;
pub mod worker;
pub mod worker_browser;
#[cfg(feature = "isolated-codecs")]
mod worker_codecs;
mod worker_conflict;
mod worker_create;
mod worker_job;
mod worker_metadata;
mod worker_output;
mod worker_password;
#[cfg(feature = "isolated-codecs")]
mod worker_sevenz_create;
mod worker_sources;
mod worker_tar_create;
mod worker_tree_io;
mod worker_volume_output;
mod worker_zip_create;
mod worker_zip_edit;
mod zip_container;
pub mod zip_edit;
mod zip_risk;

pub use control::{ArchiveProgress, OperationControl};
pub use engine::ArchiveEngine;
pub use error::{ArchiveError, Result};
pub use model::{
    ArchiveCapabilities, ArchiveEntry, ArchiveFormat, ArchiveOutcome, ArchiveRequest,
    ArchiveSnapshot, EntryId, EntryKind, EntryWarning, NameEncoding, NameProvenance, OpenRequest,
    ResourceLimits,
};
