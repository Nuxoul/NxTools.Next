use std::io;

/// Fixed, payload-free categories. Display/Debug never include archive names or backend text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ArchiveError {
    #[error("unsupported archive format")]
    UnsupportedFormat,
    #[error("unsupported archive method")]
    UnsupportedMethod,
    #[error("unsupported archive encryption")]
    UnsupportedEncryption,
    #[error("unsupported archive operation")]
    UnsupportedOperation,
    #[error("a password is required")]
    PasswordRequired,
    #[error("the supplied password is incorrect")]
    WrongPassword,
    #[error("authentication or archive integrity failed")]
    AuthenticationOrDataError,
    #[error("archive is corrupt or truncated")]
    CorruptArchive,
    #[error("archive metadata is invalid or not representable")]
    InvalidMetadata,
    #[error("a required volume is missing")]
    MissingVolume,
    #[error("unsafe archive path")]
    UnsafePath,
    #[error("archive output paths collide")]
    OutputCollision,
    #[error("finite resource budget exceeded")]
    ResourceLimit,
    #[error("source changed since listing")]
    SourceChanged,
    #[error("archive operation is busy")]
    Busy,
    #[error("operation cancelled")]
    Cancelled,
    #[error("archive I/O failed")]
    IoFailure,
}

impl From<io::Error> for ArchiveError {
    fn from(_: io::Error) -> Self {
        Self::IoFailure
    }
}

impl From<zip::result::ZipError> for ArchiveError {
    fn from(value: zip::result::ZipError) -> Self {
        use zip::result::ZipError;
        match value {
            ZipError::InvalidArchive(_) | ZipError::FileNotFound => Self::CorruptArchive,
            ZipError::InvalidPassword => Self::AuthenticationOrDataError,
            ZipError::CompressionMethodNotSupported(_) | ZipError::UnsupportedArchive(_) => {
                Self::UnsupportedMethod
            }
            ZipError::Io(_) => Self::IoFailure,
            _ => Self::CorruptArchive,
        }
    }
}

pub type Result<T> = std::result::Result<T, ArchiveError>;
