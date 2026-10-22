//! One-time first-run/legacy registration through the original config lock and package store.
use crate::{AppConfig, AppPaths};
use anyhow::{Context, Result};
use fs2::FileExt;
use std::{
    fs,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(super) fn load_or_create(paths: &AppPaths) -> Result<AppConfig> {
    let path = paths.config_file();
    // Same sibling inter-process lock as AppConfig::try_update_section. Recheck
    // existence under it, so two first-run clients cannot replace one another.
    let lock_path = path.with_extension("json.lock");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .context("failed to open configuration initialization lock")?;
    lock.lock_exclusive()
        .context("failed to lock configuration initialization")?;
    let result = (|| {
        let mut config = if path.exists() {
            AppConfig::load_from(&path)?
        } else {
            AppConfig::default()
        };
        if config.extensions.bundled_defaults_initialized {
            return Ok(config);
        }
        let started = Instant::now();
        let correlation_id = format!(
            "bundled-{}-{:x}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |value| value.as_nanos())
        );
        tracing::info!(event="extension.bundled.accepted",correlation_id=%correlation_id,"bundled extension initialization accepted");
        let seeded = (|| -> Result<usize> {
            let previews = nxtools_extensions::bundled_previews()?;
            let packages_root = paths.config_dir.join("extensions");
            let mut added = 0;
            for preview in previews {
                if config
                    .extensions
                    .packages
                    .iter()
                    .any(|package| package.manifest.id == preview.manifest.id)
                {
                    continue;
                }
                let staged = preview.stage_into(&packages_root)?;
                config
                    .extensions
                    .install(staged, "内置扩展".into(), false)?;
                added += 1;
            }
            tracing::info!(event="extension.bundled.staged",correlation_id=%correlation_id,added,"bundled snapshots staged");
            config.extensions.bundled_defaults_initialized = true;
            config.normalize();
            config.save_to(&path)?;
            Ok(added)
        })();
        match seeded {
            Ok(added) => {
                tracing::info!(event="extension.bundled.terminal",correlation_id=%correlation_id,success=true,added,elapsed_ms=started.elapsed().as_millis(),"bundled initialization committed");
                Ok(config)
            }
            Err(failure) => {
                tracing::warn!(event="extension.bundled.terminal",correlation_id=%correlation_id,success=false,error_code="initialization_failed",elapsed_ms=started.elapsed().as_millis(),"bundled initialization failed before config publication");
                Err(failure).context("failed to initialize bundled extensions")
            }
        }
    })();
    let _ = FileExt::unlock(&lock);
    result
}
