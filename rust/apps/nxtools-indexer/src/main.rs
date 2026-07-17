use anyhow::Result;
use nxtools_core::{AppPaths, SingleInstanceGuard, init_file_logging};
use tracing::info;

fn main() -> Result<()> {
    let paths = AppPaths::discover()?;
    let _instance = SingleInstanceGuard::acquire(&paths, "indexer")?;
    let _log_guard = init_file_logging(&paths, "indexer")?;

    info!("indexer process boundary is ready; USN/MFT work begins in the finder phase");
    println!("NxTools Indexer phase-1 boundary ready");
    Ok(())
}
