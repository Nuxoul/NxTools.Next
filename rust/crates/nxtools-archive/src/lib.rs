//! Host-neutral archive operations. Windows handles, Shell and GPUI stay outside this crate.
//! Only verified operations are advertised; Windows extraction awaits a native output capability.
mod control;
pub mod creation;
mod encoding;
mod engine;
mod error;
mod formats;
pub mod interaction;
pub mod member_query;
pub mod metadata;
mod model;
#[cfg(test)]
mod opt05_edit_selection_fixture;
pub mod output;
pub mod output_readmission;
mod path_policy;
pub mod planning;
pub mod product;
pub mod product_job;
mod reclamation;
#[cfg(feature = "isolated-codecs")]
mod sevenz_edit_header;
#[cfg(feature = "isolated-codecs")]
mod sevenz_edit_plan;
pub mod source_deletion;
mod tar_edit;
mod tar_edit_outer;
pub mod worker;
pub mod worker_browser;
mod worker_bulk;
#[cfg(feature = "isolated-codecs")]
mod worker_codecs;
mod worker_conflict;
mod worker_create;
mod worker_drag;
#[cfg(feature = "isolated-codecs")]
mod worker_formats;
mod worker_job;
mod worker_metadata;
mod worker_output;
mod worker_password;
#[cfg(feature = "isolated-codecs")]
mod worker_sevenz_create;
#[cfg(feature = "isolated-codecs")]
mod worker_sevenz_edit;
mod worker_sources;
mod worker_tar_create;
mod worker_tar_edit;
mod worker_tree_io;
mod worker_volume_output;
mod worker_zip_create;
mod worker_zip_edit;
mod zip_container;
pub mod zip_edit;
mod zip_risk;

pub use control::{ArchiveProgress, ArchiveWorkObservation, OperationControl};
pub use engine::ArchiveEngine;
pub use error::{ArchiveError, Result};
pub use model::{
    ArchiveCapabilities, ArchiveEntry, ArchiveFormat, ArchiveModificationRefusal, ArchiveOperation,
    ArchiveOutcome, ArchiveRequest, ArchiveSnapshot, EntryId, EntryKind, EntryWarning,
    NameEncoding, NameProvenance, OpenRequest, ResourceLimits,
};
