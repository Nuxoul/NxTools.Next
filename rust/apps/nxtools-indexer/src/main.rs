use std::io::BufReader;

use anyhow::Result;
use nxtools_core::{AppPaths, SingleInstanceGuard, init_file_logging};
use nxtools_indexer::{IndexerService, serve};
use tracing::info;

#[cfg(windows)]
mod desktop_lifetime;
#[cfg(windows)]
mod elevated_helper_broker;
#[cfg(windows)]
mod windows_service_host;

fn main() -> Result<()> {
    let paths = AppPaths::discover()?;
    let _instance = SingleInstanceGuard::acquire(&paths, "indexer")?;
    let _log_guard = init_file_logging(&paths, "indexer")?;

    #[cfg(windows)]
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--service")) {
        return windows_service_host::run_dispatcher().map_err(anyhow::Error::from);
    }

    let service = IndexerService::discover();
    info!(
        ntfs_volumes = service.status().drives.len(),
        indexed_items = service.status().indexed_items,
        error = service.status().last_error.as_deref().unwrap_or(""),
        "indexer process boundary ready"
    );
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve(BufReader::new(stdin), stdout, service)
}
