//! Operation drafts: no filesystem, settings writes, codec or native authority.
use nxtools_core::archive::{
    ArchiveCompressionLevel, ArchiveConflictPolicy, ArchiveCreateFormat, ArchiveDestination,
    ArchiveFilenameEncoding, ArchiveSettings,
};
use nxtools_protocol::{
    archive_browser::ArchiveBrowserContext, archive_job::*, archive_password::PrivatePassword,
};
use std::path::PathBuf;

#[derive(Clone)]
pub(super) struct DraftSource {
    pub path: PathBuf,
    pub directory: bool,
    pub bytes: Option<u64>,
}
pub(super) struct CreateDraft {
    pub sources: Vec<DraftSource>,
    pub destination: String,
    pub name: String,
    pub format: ArchiveCreateFormat,
    pub level: ArchiveCompressionLevel,
    pub password: String,
    pub confirm: String,
    pub solid: bool,
    pub encrypt_header: bool,
    pub strip_single_root: bool,
    pub exclusions: Vec<String>,
    pub test_after_create: bool,
    pub separately: bool,
    /// Optional genuine ZIP/7z volume size, in MiB; no partial/overwrite family mode.
    pub volume_mib: String,
}
pub(super) struct CreateOptions {
    pub level: Option<ArchiveCompressionLevel>,
    pub method: Option<&'static str>,
}
impl CreateDraft {
    pub fn new(settings: &ArchiveSettings, sources: Vec<DraftSource>) -> Self {
        let first = sources.first().map(|s| s.path.as_path());
        let destination = first
            .and_then(std::path::Path::parent)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = sources
            .first()
            .and_then(|source| {
                if source.directory {
                    source.path.file_name()
                } else {
                    source.path.file_stem()
                }
            })
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "新建压缩包".into());
        Self {
            sources,
            destination,
            name: format!("{stem}.{}", extension(settings.create_format)),
            format: settings.create_format,
            level: if settings.create_format != ArchiveCreateFormat::Zip
                && settings.create_format != ArchiveCreateFormat::Tar
                && settings.compression_level == ArchiveCompressionLevel::Store
            {
                ArchiveCompressionLevel::Normal
            } else {
                settings.compression_level
            },
            password: String::new(),
            confirm: String::new(),
            solid: false,
            encrypt_header: false,
            strip_single_root: settings.strip_single_root,
            exclusions: settings.exclude_patterns.clone(),
            test_after_create: settings.test_after_create,
            separately: false,
            volume_mib: String::new(),
        }
    }
    pub fn set_format(&mut self, format: ArchiveCreateFormat) {
        let old_extension = format!(".{}", extension(self.format));
        let stem = self
            .name
            .strip_suffix(&old_extension)
            .unwrap_or(&self.name)
            .to_owned();
        self.format = format;
        self.name = format!("{stem}.{}", extension(format));
        if !encrypted(format) {
            self.clear_secrets();
        }
        if format != ArchiveCreateFormat::SevenZ {
            self.solid = false;
            self.encrypt_header = false;
        }
        if format != ArchiveCreateFormat::Zip && self.level == ArchiveCompressionLevel::Store {
            self.level = ArchiveCompressionLevel::Normal;
        }
    }
    pub fn options(&self) -> Result<CreateOptions, String> {
        if self.sources.is_empty() || self.sources.len() > 32 {
            return Err("请选择 1–32 个来源；不会丢弃超出的输入。".into());
        }
        if !self.separately && !valid_job_name(&self.name) {
            return Err("压缩包名称必须是一个有效文件名。".into());
        }
        if !self.separately
            && !self
                .name
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", extension(self.format)))
        {
            return Err("文件名扩展名与所选格式不一致。".into());
        }
        if self.password != self.confirm {
            return Err("两次输入的密码不一致。".into());
        }
        if self.password.len() > 4096 || self.password.contains('\0') {
            return Err("密码不能超过 4096 UTF-8 字节或包含空字符。".into());
        }
        if !encrypted(self.format) && !self.password.is_empty() {
            return Err("当前格式不支持密码，请切换格式或清除密码。".into());
        }
        if self.format != ArchiveCreateFormat::SevenZ && (self.solid || self.encrypt_header) {
            return Err("只有 7Z 支持固实与文件名加密。".into());
        }
        if self.encrypt_header && self.password.is_empty() {
            return Err("文件名加密需要设置密码。".into());
        }
        if matches!(
            self.format,
            ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
        ) && ((!self.separately && self.sources.len() != 1)
            || self.sources.iter().any(|source| source.directory))
        {
            return Err("GZ / XZ 每个压缩包只能包含一个普通文件；请使用分别压缩，目录请选择 ZIP / 7Z / TAR。".into());
        }
        if self.format != ArchiveCreateFormat::Zip
            && self.format != ArchiveCreateFormat::Tar
            && self.level == ArchiveCompressionLevel::Store
        {
            return Err("当前格式不支持仅存储等级。".into());
        }
        self.volume_bytes()?;
        let settings = ArchiveSettings {
            create_format: self.format,
            compression_level: self.level,
            exclude_patterns: if matches!(
                self.format,
                ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
            ) {
                Vec::new()
            } else {
                self.exclusions.clone()
            },
            ..Default::default()
        };
        settings.validate()?;
        Ok(CreateOptions {
            level: (self.format != ArchiveCreateFormat::Tar).then_some(self.level),
            method: (self.format == ArchiveCreateFormat::Zip).then_some(
                if self.level == ArchiveCompressionLevel::Store {
                    "store"
                } else {
                    "deflate"
                },
            ),
        })
    }
    pub fn volume_bytes(&self) -> Result<Option<u64>, String> {
        if self.volume_mib.trim().is_empty() {
            return Ok(None);
        }
        if !matches!(
            self.format,
            ArchiveCreateFormat::Zip | ArchiveCreateFormat::SevenZ
        ) {
            return Err("仅 ZIP / 7Z 可创建分卷；请选择新目标文件名。".into());
        }
        let digits = self.volume_mib.trim();
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err("分卷大小必须是正整数 MiB。".into());
        }
        let mib: u64 = digits.parse().map_err(|_| "分卷大小超出可表示范围。")?;
        let bytes = mib.checked_mul(1 << 20).ok_or("分卷大小超出可表示范围。")?;
        if bytes < 65536 || bytes > ArchiveJobLimits::default().max_output_bytes {
            return Err("分卷大小必须在 64 KiB 到本次输出预算之间。".into());
        }
        Ok(Some(bytes))
    }
    pub fn requests(&self, settings: &ArchiveSettings) -> Result<Vec<ArchiveJobRequest>, String> {
        let options = self.options()?;
        if !valid_job_path(&self.destination) {
            return Err("请选择本地绝对目标文件夹（例如 C:\\Archive）。".into());
        }
        let mut names = std::collections::HashSet::new();
        let groups: Vec<Vec<&DraftSource>> = if self.separately {
            self.sources.iter().map(|source| vec![source]).collect()
        } else {
            vec![self.sources.iter().collect()]
        };
        let mut requests = Vec::with_capacity(groups.len());
        for sources in groups {
            let name = if self.separately {
                let source = sources[0];
                let stem = if source.directory {
                    source.path.file_name()
                } else {
                    source.path.file_stem()
                };
                let stem = stem
                    .and_then(|s| s.to_str())
                    .ok_or("来源名称不是有效 Unicode，无法生成压缩包名。")?;
                format!("{stem}.{}", extension(self.format))
            } else {
                self.name.clone()
            };
            if !names.insert(name.to_lowercase()) {
                return Err("分别压缩会产生同名目标，请分批选择或更改来源名称。".into());
            }
            let mut roots = std::collections::HashSet::new();
            let paths = sources
                .iter()
                .map(|source| {
                    let path = source.path.to_str().ok_or("来源路径不是有效 Unicode。")?;
                    let root = source
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .ok_or("请选择有名称的文件或文件夹。")?;
                    if !roots.insert(root.to_lowercase()) {
                        return Err("来源具有重复的根名称，请分别压缩或分批选择。");
                    }
                    Ok(path.to_owned())
                })
                .collect::<Result<Vec<_>, &str>>()?;
            let stream = matches!(
                self.format,
                ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
            );
            let request = ArchiveJobRequest {
                kind: ArchiveJobKind::Create,
                paths,
                destination: Some(self.destination.clone()),
                codepage: codepage(settings.filename_encoding),
                limits: ArchiveJobLimits::default(),
                extract: None,
                readmission: None,
                modify: None,
                create: Some(ArchiveJobCreate {
                    format: match self.format {
                        ArchiveCreateFormat::Zip => ArchiveJobFormat::Zip,
                        ArchiveCreateFormat::SevenZ => ArchiveJobFormat::SevenZ,
                        ArchiveCreateFormat::Tar => ArchiveJobFormat::Tar,
                        ArchiveCreateFormat::Tgz => ArchiveJobFormat::Tgz,
                        ArchiveCreateFormat::Txz => ArchiveJobFormat::Txz,
                        ArchiveCreateFormat::Gz => ArchiveJobFormat::Gz,
                        ArchiveCreateFormat::Xz => ArchiveJobFormat::Xz,
                    },
                    level: options.level.map(|level| match level {
                        ArchiveCompressionLevel::Store => ArchiveJobLevel::Store,
                        ArchiveCompressionLevel::Fast => ArchiveJobLevel::Fast,
                        ArchiveCompressionLevel::Normal => ArchiveJobLevel::Normal,
                        ArchiveCompressionLevel::Maximum => ArchiveJobLevel::Maximum,
                    }),
                    method: options.method.map(|method| {
                        if method == "store" {
                            ArchiveJobMethod::Store
                        } else {
                            ArchiveJobMethod::Deflate
                        }
                    }),
                    password: (!self.password.is_empty())
                        .then(|| PrivatePassword::new(&self.password)),
                    solid: (self.format == ArchiveCreateFormat::SevenZ).then_some(self.solid),
                    encrypt_header: (self.format == ArchiveCreateFormat::SevenZ)
                        .then_some(self.encrypt_header),
                    strip_single_root: !stream
                        && self.strip_single_root
                        && sources.len() == 1
                        && sources[0].directory,
                    exclude_patterns: if stream {
                        Vec::new()
                    } else {
                        self.exclusions.clone()
                    },
                    test_after_create: self.test_after_create,
                    name,
                    volume_bytes: self.volume_bytes()?,
                }),
            };
            request
                .validate()
                .map_err(|_| "表单参数超出操作协议限制，请检查来源、目标和名称。")?;
            requests.push(request);
        }
        Ok(requests)
    }
    pub fn clear_secrets(&mut self) {
        clear_secret(&mut self.password);
        clear_secret(&mut self.confirm);
    }
}
pub(super) fn encrypted(format: ArchiveCreateFormat) -> bool {
    matches!(
        format,
        ArchiveCreateFormat::Zip | ArchiveCreateFormat::SevenZ
    )
}
pub(super) fn extension(format: ArchiveCreateFormat) -> &'static str {
    match format {
        ArchiveCreateFormat::Zip => "zip",
        ArchiveCreateFormat::SevenZ => "7z",
        ArchiveCreateFormat::Tar => "tar",
        ArchiveCreateFormat::Tgz => "tar.gz",
        ArchiveCreateFormat::Txz => "tar.xz",
        ArchiveCreateFormat::Gz => "gz",
        ArchiveCreateFormat::Xz => "xz",
    }
}
pub(super) fn archive_stem(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for suffix in [
        ".tar.gz",
        ".tar.bz2",
        ".tar.xz",
        ".tar.zstd",
        ".tar.zst",
        ".zip",
        ".7z",
        ".rar",
        ".tar",
        ".tgz",
        ".tbz2",
        ".txz",
        ".tzst",
        ".gz",
        ".bz2",
        ".xz",
        ".zst",
    ] {
        if lower.ends_with(suffix) {
            return name[..name.len() - suffix.len()].to_owned();
        }
    }
    name.to_owned()
}
#[derive(Clone)]
pub(super) struct DraftSelection {
    pub browser: ArchiveBrowserContext,
    pub ordinals: Vec<u64>,
}
pub(super) struct ExtractDraft {
    pub source: PathBuf,
    pub destination: String,
    pub strategy: ArchiveDestination,
    pub conflict: ArchiveConflictPolicy,
    pub copy_motw: bool,
    pub single_file_folder: bool,
    pub selection: Option<DraftSelection>,
    pub selected_only: bool,
    pub open_after: bool,
    pub keep_open: bool,
    pub remember: bool,
    pub delete_source: bool,
}
impl ExtractDraft {
    pub fn new(
        settings: &ArchiveSettings,
        source: PathBuf,
        selection: Option<DraftSelection>,
    ) -> Self {
        let destination = source
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            source,
            destination,
            strategy: settings.default_destination,
            conflict: settings.conflict_policy,
            copy_motw: settings.copy_motw,
            single_file_folder: settings.smart_single_file_folder,
            selected_only: selection.as_ref().is_some_and(|s| !s.ordinals.is_empty()),
            selection,
            open_after: settings.open_after_extract,
            keep_open: settings.keep_extract_progress_open,
            remember: false,
            delete_source: false,
        }
    }
    pub fn request(&self, settings: &ArchiveSettings) -> Result<ArchiveJobRequest, String> {
        if self.delete_source {
            return Err("原始对象回收尚未验收；当前操作不能删除来源。".into());
        }
        if !valid_job_path(&self.destination) {
            return Err("请选择本地绝对目标文件夹（例如 C:\\Archive）。".into());
        }
        let selection = if self.selected_only {
            let selected = self
                .selection
                .as_ref()
                .filter(|s| !s.ordinals.is_empty())
                .ok_or("没有可用的已选条目，请选择全部文件。")?;
            Some(ArchiveJobSelection {
                browser: selected.browser.clone(),
                ordinals: selected.ordinals.clone(),
            })
        } else {
            None
        };
        let request = ArchiveJobRequest {
            kind: ArchiveJobKind::Extract,
            paths: vec![
                self.source
                    .to_str()
                    .ok_or("来源路径不是有效 Unicode。")?
                    .to_owned(),
            ],
            destination: Some(self.destination.clone()),
            codepage: codepage(settings.filename_encoding),
            limits: ArchiveJobLimits::default(),
            create: None,
            readmission: None,
            modify: None,
            extract: Some(ArchiveJobExtract {
                strategy: match self.strategy {
                    ArchiveDestination::Ask | ArchiveDestination::Here => ArchiveJobStrategy::Here,
                    ArchiveDestination::Name => ArchiveJobStrategy::Name,
                    ArchiveDestination::Smart => ArchiveJobStrategy::Smart,
                },
                single_file_folder: self.strategy == ArchiveDestination::Smart
                    && self.single_file_folder,
                conflict: match self.conflict {
                    ArchiveConflictPolicy::Ask => ArchiveJobConflict::Ask,
                    ArchiveConflictPolicy::Overwrite => ArchiveJobConflict::Overwrite,
                    ArchiveConflictPolicy::Skip => ArchiveJobConflict::Skip,
                    ArchiveConflictPolicy::AutoRename => ArchiveJobConflict::AutoRename,
                },
                copy_motw: self.copy_motw,
                selection,
            }),
        };
        request
            .validate()
            .map_err(|_| "表单参数或选定条目的会话无效，请重新选择。")?;
        Ok(request)
    }
}
pub(super) fn codepage(encoding: ArchiveFilenameEncoding) -> u32 {
    match encoding {
        ArchiveFilenameEncoding::Auto => 0,
        ArchiveFilenameEncoding::Utf8 => 65001,
        ArchiveFilenameEncoding::Gbk => 936,
        ArchiveFilenameEncoding::Big5 => 950,
        ArchiveFilenameEncoding::ShiftJis => 932,
        ArchiveFilenameEncoding::Korean => 949,
        ArchiveFilenameEncoding::Cp437 => 437,
        ArchiveFilenameEncoding::Windows1250 => 1250,
        ArchiveFilenameEncoding::Windows1251 => 1251,
        ArchiveFilenameEncoding::Windows1252 => 1252,
    }
}
fn clear_secret(secret: &mut String) {
    let mut bytes = std::mem::take(secret).into_bytes();
    bytes.fill(0);
}
pub(super) struct ModifyDraft {
    pub source: PathBuf,
    pub browser: ArchiveBrowserContext,
    pub action: ArchiveJobModifyAction,
    pub ordinals: Vec<u64>,
    pub value: Option<String>,
    pub added: Vec<String>,
    pub codepage: u32,
}
impl ModifyDraft {
    pub fn request(&self) -> Result<ArchiveJobRequest, String> {
        let source = self.source.to_str().ok_or("归档路径不是有效 Unicode。")?;
        let request = ArchiveJobRequest {
            kind: ArchiveJobKind::Modify,
            paths: std::iter::once(source.to_owned())
                .chain(self.added.iter().cloned())
                .collect(),
            destination: None,
            codepage: self.codepage,
            limits: ArchiveJobLimits::default(),
            extract: None,
            create: None,
            modify: Some(ArchiveJobModify {
                browser: self.browser.clone(),
                action: self.action,
                ordinals: self.ordinals.clone(),
                value: self.value.clone(),
            }),
            readmission: None,
        };
        request
            .validate()
            .map_err(|_| "编辑参数无效或源快照已失效；请检查名称、选中条目与来源。".to_owned())?;
        Ok(request)
    }
}
#[cfg(test)]
#[path = "draft_tests.rs"]
mod tests;
