use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use nxtools_core::{AppConfig, AppPaths};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone, Default)]
pub(crate) struct MigrationSlot(pub Arc<Mutex<Option<DataMigration>>>);
impl gpui_kit::Global for MigrationSlot {}

pub(crate) struct DataMigration {
    pub source: AppPaths,
    pub destination: PathBuf,
    pub correlation_id: Uuid,
    pub started_at: Instant,
    pub _lock: File,
}

impl DataMigration {
    pub fn prepare(
        source: AppPaths,
        destination: &Path,
        installation: &Path,
        correlation_id: Uuid,
    ) -> Result<Self> {
        let destination = source.validate_data_destination(destination, installation)?;
        let lock = source.begin_data_migration()?;
        Ok(Self {
            source,
            destination,
            correlation_id,
            started_at: Instant::now(),
            _lock: lock,
        })
    }

    /// Feature workers are drained by main::run before this boundary is entered.
    pub fn execute(&self) -> Result<()> {
        info!(event = "data_directory.copy.started", correlation_id = %self.correlation_id,
            "software data copy started after resource drain");
        let destination = self.source.at_root(&self.destination);
        ensure!(
            fs::read_dir(&self.destination)?.next().is_none(),
            "目标目录已非空，迁移未开始。"
        );
        let result = (|| -> Result<()> {
            let mut config = AppConfig::load_from(&self.source.config_file())?;
            let migrate_clipboard = config.clipboard.storage_directory.is_empty()
                && self.source.data_dir.join("clipboard.db").is_file();
            if migrate_clipboard {
                let history = nxtools_clipboard::ClipboardHistory::open(
                    self.source.data_dir.join("clipboard.db"),
                )?;
                history.migrate_storage(
                    &self.source.data_dir.join("clipboard-images"),
                    &destination.data_dir,
                )?;
            }
            copy_tree(
                &self.source.data_dir,
                &destination.data_dir,
                migrate_clipboard,
                true,
            )?;
            copy_tree(
                &self.source.config_dir,
                &destination.config_dir,
                false,
                false,
            )?;
            for (source_root, destination_root) in [
                (&self.source.config_dir, &destination.config_dir),
                (&self.source.data_dir, &destination.data_dir),
            ] {
                for package in &mut config.extensions.packages {
                    if let Some(path) = &mut package.code_directory {
                        rebase(path, source_root, destination_root);
                    }
                    if let Some(approval) = &mut package.trust {
                        rebase(
                            &mut approval.runtime.executable,
                            source_root,
                            destination_root,
                        );
                    }
                    if let Some(path) = &mut package.runtime_override {
                        rebase(path, source_root, destination_root);
                    }
                }
                for path in config.extensions.runtime_defaults.values_mut() {
                    rebase(path, source_root, destination_root);
                }
            }
            let mut ffmpeg = PathBuf::from(&config.download.ffmpeg_executable_path);
            rebase(&mut ffmpeg, &self.source.data_dir, &destination.data_dir);
            config.download.ffmpeg_executable_path = ffmpeg.to_string_lossy().into_owned();
            config.save_to(&destination.config_file())?;
            self.source.commit_data_location(&self.destination)?;
            Ok(())
        })();
        if result.is_err() {
            // Only directories created by this operation are removed; source remains authoritative.
            let _ = fs::remove_dir_all(&destination.config_dir);
            let _ = fs::remove_dir_all(&destination.data_dir);
            warn!(event = "data_directory.copy.failed", correlation_id = %self.correlation_id,
                elapsed_ms = self.started_at.elapsed().as_millis(), "software data migration failed; original directory retained");
        } else {
            info!(event = "data_directory.copy.completed", correlation_id = %self.correlation_id,
                elapsed_ms = self.started_at.elapsed().as_millis(), "software data location committed");
        }
        result
    }
}

fn rebase(path: &mut PathBuf, source: &Path, destination: &Path) {
    let canonical_source = source
        .canonicalize()
        .unwrap_or_else(|_| source.to_path_buf());
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.clone());
    if let Ok(relative) = canonical_path.strip_prefix(&canonical_source) {
        *path = destination.join(relative);
    }
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    clipboard_already_migrated: bool,
    data_root: bool,
) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if (data_root && name_text == "browser-blob-staging")
            || (clipboard_already_migrated
                && matches!(
                    name_text.as_ref(),
                    "clipboard.db" | "clipboard.db-wal" | "clipboard.db-shm" | "clipboard-images"
                ))
        {
            continue;
        }
        let target = destination.join(name);
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, false, false)?;
        } else {
            let mut input = File::open(entry.path())?;
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(target)?;
            io::copy(&mut input, &mut output)?;
            output.sync_all()?;
        }
    }
    Ok(())
}

pub(crate) fn save_result(paths: &AppPaths, success: bool) -> Result<()> {
    fs::write(
        paths.cache_dir.join("data-migration-result.json"),
        serde_json::to_vec(&success)?,
    )?;
    Ok(())
}

pub(crate) fn take_result(paths: &AppPaths) -> Option<bool> {
    let file = paths.cache_dir.join("data-migration-result.json");
    let bytes = fs::read(&file).ok()?;
    let result = serde_json::from_slice(&bytes).ok();
    let _ = fs::remove_file(file);
    result
}

pub(crate) fn restart() -> Result<()> {
    std::process::Command::new(std::env::current_exe()?)
        .args(["--wait-for-instance-release-ms", "10000"])
        .spawn()
        .context("无法重新启动 NxTools，请从原程序入口启动。")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (PathBuf, AppPaths, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("nxtools-data-migration-{}", Uuid::new_v4()));
        let source = AppPaths {
            config_dir: root.join("source/config"),
            data_dir: root.join("source/data"),
            cache_dir: root.join("runtime/cache"),
            log_dir: root.join("runtime/logs"),
        };
        for directory in [
            &source.config_dir,
            &source.data_dir,
            &source.cache_dir,
            &source.log_dir,
        ] {
            fs::create_dir_all(directory).expect("source directory");
        }
        let target = root.join("target");
        let install = root.join("install");
        fs::create_dir_all(&target).expect("target");
        fs::create_dir_all(&install).expect("install");
        AppConfig::default()
            .save_to(&source.config_file())
            .expect("source config");
        (root, source, target, install)
    }

    #[test]
    fn data_migration_copies_config_databases_and_rebases_clipboard_assets() {
        let (root, source, target, install) = fixture();
        let assets = source.data_dir.join("clipboard-images");
        fs::create_dir_all(&assets).expect("assets");
        let image = assets.join("image.png");
        fs::write(&image, b"synthetic image").expect("image");
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(source.data_dir.join("clipboard.db"))
                .expect("history");
        history
            .capture_image(nxtools_clipboard::ImageCapture {
                image_path: image.to_string_lossy().into_owned(),
                width: 1,
                height: 1,
                native_formats: Vec::new(),
                captured_at_unix_ms: 10,
                source: nxtools_clipboard::CaptureSource {
                    window_handle: 0,
                    process_id: 0,
                    process_name: "fixture".into(),
                    process_icon_path: String::new(),
                    window_title: String::new(),
                },
            })
            .expect("capture image");
        drop(history);
        fs::write(
            source.data_dir.join("downloads.sqlite3"),
            b"database fixture",
        )
        .expect("other database");
        let migration = DataMigration::prepare(source.clone(), &target, &install, Uuid::new_v4())
            .expect("prepare");
        migration.execute().expect("migration");
        let destination = source.at_root(&migration.destination);
        let moved =
            nxtools_clipboard::ClipboardHistory::open(destination.data_dir.join("clipboard.db"))
                .expect("moved history");
        let items = moved.recent(10).expect("items");
        assert_eq!(items.len(), 1);
        assert!(
            Path::new(&items[0].image_path)
                .starts_with(destination.data_dir.join("clipboard-images"))
        );
        assert!(Path::new(&items[0].image_path).is_file());
        assert_eq!(
            fs::read(destination.data_dir.join("downloads.sqlite3")).expect("copied database"),
            b"database fixture"
        );
        assert!(image.is_file(), "last-good source remains available");
        assert!(source.config_file().is_file());
        assert!(source.cache_dir.join("data-location.json").is_file());
        drop(moved);
        drop(migration);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn data_migration_failed_copy_does_not_publish_or_remove_source() {
        let (root, source, target, install) = fixture();
        let migration = DataMigration::prepare(source.clone(), &target, &install, Uuid::new_v4())
            .expect("prepare");
        fs::remove_file(source.config_file()).expect("inject missing config");
        fs::write(source.data_dir.join("keep"), b"keep").expect("source");
        assert!(migration.execute().is_err());
        assert!(!source.cache_dir.join("data-location.json").exists());
        assert_eq!(
            fs::read(source.data_dir.join("keep")).expect("source retained"),
            b"keep"
        );
        assert_eq!(fs::read_dir(&target).expect("target").count(), 0);
        drop(migration);
        fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn data_migration_can_change_the_location_again_without_changing_runtime_paths() {
        let (root, source, target, install) = fixture();
        fs::write(source.data_dir.join("keep"), b"persistent").expect("source");
        let first = DataMigration::prepare(source.clone(), &target, &install, Uuid::new_v4())
            .expect("first prepare");
        first.execute().expect("first migration");
        let middle = source.at_root(&first.destination);
        drop(first);
        let target2 = root.join("target2");
        fs::create_dir(&target2).expect("second target");
        let second = DataMigration::prepare(middle, &target2, &install, Uuid::new_v4())
            .expect("second prepare");
        second.execute().expect("second migration");
        let final_paths = source.at_root(&second.destination);
        assert_eq!(
            fs::read(final_paths.data_dir.join("keep")).expect("final data"),
            b"persistent"
        );
        let locator: PathBuf = serde_json::from_slice(
            &fs::read(source.cache_dir.join("data-location.json")).expect("locator"),
        )
        .expect("path");
        assert_eq!(locator, second.destination);
        assert_eq!(final_paths.cache_dir, source.cache_dir);
        drop(second);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn data_migration_locator_failure_retains_the_previous_location() {
        let (root, source, target, install) = fixture();
        source
            .commit_data_location(source.software_root())
            .expect("original locator");
        let locator = source.cache_dir.join("data-location.json");
        let before = fs::read(&locator).expect("previous bytes");
        let migration = DataMigration::prepare(source.clone(), &target, &install, Uuid::new_v4())
            .expect("prepare");
        let original_permissions = fs::metadata(&locator).expect("metadata").permissions();
        let mut permissions = original_permissions.clone();
        permissions.set_readonly(true);
        fs::set_permissions(&locator, permissions).expect("deny replacement");
        assert!(migration.execute().is_err());
        assert_eq!(fs::read(&locator).expect("unchanged locator"), before);
        assert!(source.config_file().is_file());
        assert_eq!(fs::read_dir(&target).expect("target").count(), 0);
        fs::set_permissions(&locator, original_permissions).expect("restore fixture");
        drop(migration);
        fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn staged_exe_approval_remains_valid_across_two_data_migrations() {
        let (root, source, target, install) = fixture();
        let input = root.join("package-input");
        fs::create_dir(&input).expect("package input");
        fs::write(
            input.join("fixture.exe"),
            b"owned synthetic executable identity",
        )
        .expect("fixture bytes");
        fs::write(
            input.join("extension.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema_version":1,"id":"fixture.migrate","name":"Migration fixture","version":"1",
                "runtime":{"program":"exe","timeout_ms":30000},"files":["fixture.exe"],
                "handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action",
                    "action":{"type":"script","entry_file":"fixture.exe"}}]
            }))
            .expect("manifest bytes"),
        )
        .expect("manifest");
        let staged = nxtools_extensions::ImportPreview::from_directory(&input)
            .expect("preview")
            .stage_into(&source.config_dir.join("extensions/packages"))
            .expect("stage");
        let hash = staged.content_hash.clone();
        let executable = staged
            .code_directory
            .as_ref()
            .expect("canonical package")
            .join("fixture.exe");
        let mut config = AppConfig::load_from(&source.config_file()).expect("config");
        config
            .extensions
            .install(staged, "synthetic".into(), false)
            .expect("install");
        config
            .extensions
            .approve(
                "fixture.migrate",
                &hash,
                nxtools_extensions::RuntimeIdentity::inspect(&executable)
                    .expect("runtime identity"),
            )
            .expect("approval");
        config
            .save_to(&source.config_file())
            .expect("approved config");
        let mut current = source.clone();
        for target in [target, root.join("target2")] {
            fs::create_dir_all(&target).expect("target");
            let migration = DataMigration::prepare(current, &target, &install, Uuid::new_v4())
                .expect("prepare");
            migration.execute().expect("migrate");
            current = source.at_root(&migration.destination);
            let moved = AppConfig::load_from(&current.config_file()).expect("moved config");
            let package = &moved.extensions.packages[0];
            assert!(
                package
                    .code_directory
                    .as_ref()
                    .expect("code")
                    .starts_with(&current.config_dir)
            );
            assert!(
                package
                    .trust
                    .as_ref()
                    .expect("trust")
                    .runtime
                    .executable
                    .starts_with(&current.config_dir)
            );
            package
                .verify_approval()
                .expect("same approved package at new location");
            drop(migration);
        }
        fs::remove_dir_all(root).expect("cleanup");
    }
}
