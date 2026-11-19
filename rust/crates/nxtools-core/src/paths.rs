//! Shared application directory discovery and bootstrap data-location ownership.
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use directories::ProjectDirs;
use fs2::FileExt;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        Self::resolve(Self::default_locations()?)
    }

    pub fn default_locations() -> Result<Self> {
        let dirs = ProjectDirs::from("dev", "NxTools", "NxTools.Next")
            .context("Windows did not provide application data directories")?;
        Ok(Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
            log_dir: dirs.data_local_dir().join("logs"),
        })
    }

    fn resolve(defaults: Self) -> Result<Self> {
        fs::create_dir_all(&defaults.cache_dir)?;
        let lock = defaults.location_lock()?;
        FileExt::try_lock_shared(&lock).context("software data migration is in progress")?;
        let locator = defaults.location_file();
        let paths = if locator.exists() {
            let root: PathBuf = serde_json::from_slice(&fs::read(locator)?)
                .context("invalid software data location")?;
            ensure!(
                root.is_absolute(),
                "software data location must be absolute"
            );
            let paths = defaults.at_root(&root);
            ensure!(
                paths.config_file().is_file(),
                "software data directory is unavailable"
            );
            paths
        } else {
            defaults
        };
        paths.ensure_created()?;
        Ok(paths)
    }

    pub fn at_root(&self, root: &Path) -> Self {
        Self {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: self.cache_dir.clone(),
            log_dir: self.log_dir.clone(),
        }
    }

    pub fn software_root(&self) -> &Path {
        self.data_dir.parent().unwrap_or(&self.data_dir)
    }

    pub fn validate_data_destination(&self, target: &Path, installation: &Path) -> Result<PathBuf> {
        ensure!(target.is_absolute(), "请选择绝对路径的数据目录。");
        #[cfg(windows)]
        ensure!(
            matches!(target.components().next(), Some(std::path::Component::Prefix(prefix))
            if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_))),
            "请选择本地磁盘上的目录，不支持网络目录。"
        );
        let target = target.canonicalize().context("无法访问所选目录。")?;
        ensure!(
            target.is_dir() && target.parent().is_some(),
            "不能使用磁盘根目录。"
        );
        for protected in [
            &self.config_dir,
            &self.data_dir,
            &self.cache_dir,
            &self.log_dir,
            installation,
        ] {
            let protected = protected
                .canonicalize()
                .unwrap_or_else(|_| protected.to_path_buf());
            ensure!(
                !path_contains(&target, &protected) && !path_contains(&protected, &target),
                "数据目录不能与当前数据、缓存或程序目录重叠。"
            );
        }
        for variable in ["WINDIR", "ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(directory) = std::env::var_os(variable) {
                ensure!(
                    !path_contains(Path::new(&directory), &target),
                    "不能使用 Windows 或程序文件目录。"
                );
            }
        }
        ensure!(
            fs::read_dir(&target)?.next().is_none(),
            "请选择空目录，避免覆盖已有数据。"
        );
        let probe = target.join(format!(".nxtools-write-{}", temporary_id()));
        let writable = (|| -> Result<()> {
            let file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&probe)?;
            file.sync_all()?;
            drop(file);
            fs::remove_file(&probe)?;
            Ok(())
        })();
        if writable.is_err() {
            let _ = fs::remove_file(&probe);
        }
        writable.context("所选目录不可写，请检查权限和可用空间。")?;
        Ok(target)
    }

    /// Held across resource drain, copying and locator publication.
    pub fn begin_data_migration(&self) -> Result<File> {
        let file = self.location_lock()?;
        file.try_lock_exclusive()
            .context("another software data migration is in progress")?;
        Ok(file)
    }

    /// Caller holds begin_data_migration and has completed all destination writes.
    pub fn commit_data_location(&self, root: &Path) -> Result<()> {
        ensure!(
            self.at_root(root).config_file().is_file(),
            "destination configuration is missing"
        );
        let temporary = self
            .cache_dir
            .join(format!(".data-location-{}.tmp", temporary_id()));
        let result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(&serde_json::to_vec(root)?)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, self.location_file())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }

    fn location_file(&self) -> PathBuf {
        self.cache_dir.join("data-location.json")
    }

    fn location_lock(&self) -> Result<File> {
        fs::create_dir_all(&self.cache_dir)?;
        Ok(OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.cache_dir.join("data-migration.lock"))?)
    }

    fn ensure_created(&self) -> Result<()> {
        for path in [
            &self.config_dir,
            &self.data_dir,
            &self.cache_dir,
            &self.log_dir,
        ] {
            fs::create_dir_all(path)
                .with_context(|| format!("failed to create {}", path.display()))?;
        }
        Ok(())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }
}

fn path_contains(parent: &Path, child: &Path) -> bool {
    let parent = parent
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .to_lowercase();
    let child = child
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .to_lowercase();
    child == parent || child.starts_with(&(parent.trim_end_matches('\\').to_owned() + "\\"))
}

fn temporary_id() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(root: &Path) -> AppPaths {
        AppPaths {
            config_dir: root.join("old/config"),
            data_dir: root.join("old/data"),
            cache_dir: root.join("runtime/cache"),
            log_dir: root.join("runtime/logs"),
        }
    }

    #[test]
    fn data_location_changes_config_and_data_but_not_runtime_identity() {
        let root = std::env::temp_dir().join(format!("nxtools-location-{}", temporary_id()));
        let defaults = fixture(&root);
        let original = AppPaths::resolve(defaults.clone()).expect("default discovery");
        assert_eq!(original.data_dir, defaults.data_dir);
        let target = root.join("custom");
        fs::create_dir_all(target.join("config")).expect("config directory");
        fs::write(target.join("config/config.json"), b"{}").expect("config");
        let guard = original
            .begin_data_migration()
            .expect("migration admission");
        assert!(
            AppPaths::resolve(defaults.clone()).is_err(),
            "startup must not enter during migration"
        );
        original.commit_data_location(&target).expect("commit");
        drop(guard);
        let moved = AppPaths::resolve(defaults.clone()).expect("custom discovery");
        assert_eq!(moved.data_dir, target.join("data"));
        assert_eq!(moved.config_file(), target.join("config/config.json"));
        assert_eq!(moved.cache_dir, defaults.cache_dir);
        assert_eq!(moved.log_dir, defaults.log_dir);
        fs::remove_file(moved.config_file()).expect("remove destination config");
        assert!(
            AppPaths::resolve(defaults).is_err(),
            "missing destination must not reset to defaults"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn data_destination_rejects_overlap_nonempty_and_relative_paths() {
        let root = std::env::temp_dir().join(format!("nxtools-location-{}", temporary_id()));
        let paths = AppPaths::resolve(fixture(&root)).expect("discovery");
        let install = root.join("install");
        let target = root.join("custom");
        fs::create_dir_all(&install).expect("install");
        fs::create_dir_all(&target).expect("target");
        assert!(
            paths
                .validate_data_destination(Path::new("relative"), &install)
                .is_err()
        );
        assert!(
            paths
                .validate_data_destination(&paths.data_dir, &install)
                .is_err()
        );
        assert!(paths.validate_data_destination(&root, &install).is_err());
        assert!(paths.validate_data_destination(&install, &install).is_err());
        assert!(paths.validate_data_destination(&target, &install).is_ok());
        assert_eq!(fs::read_dir(&target).expect("target").count(), 0);
        fs::write(target.join("keep.txt"), b"keep").expect("existing file");
        assert!(paths.validate_data_destination(&target, &install).is_err());
        assert_eq!(
            fs::read(target.join("keep.txt")).expect("existing file"),
            b"keep"
        );
        assert!(!paths.location_file().exists());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
