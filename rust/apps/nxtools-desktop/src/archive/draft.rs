//! Operation drafts: no filesystem, settings writes, codec or native authority.
use nxtools_core::archive::{
    ArchiveCompressionLevel, ArchiveConflictPolicy, ArchiveCreateFormat, ArchiveDestination,
    ArchiveFilenameEncoding, ArchiveSettings, ArchiveVolumeSize, ArchiveVolumeUnit,
};
use nxtools_protocol::{
    archive_browser::ArchiveBrowserContext, archive_job::*, archive_password::PrivatePassword,
};
use std::path::PathBuf;
use uuid::Uuid;

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
    pub self_extracting: bool,
    pub level: ArchiveCompressionLevel,
    pub password: String,
    pub confirm: String,
    pub solid: bool,
    pub encrypt_header: bool,
    pub strip_single_root: bool,
    pub exclusions: Vec<String>,
    pub test_after_create: bool,
    pub separately: bool,
    /// Integer split value; the original MiB entrance retains its explicit default unit.
    pub volume_mib: String,
    pub volume_unit: ArchiveVolumeUnit,
}
pub(super) struct CreateOptions {
    pub level: Option<ArchiveCompressionLevel>,
    pub method: Option<&'static str>,
    pub volume_bytes: Option<u64>,
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
            self_extracting: false,
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
            volume_mib: settings
                .create_volume_size
                .map(|size| size.value.to_string())
                .unwrap_or_default(),
            volume_unit: settings
                .create_volume_size
                .map_or(ArchiveVolumeUnit::MiB, |size| size.unit),
        }
    }
    pub fn set_format(&mut self, format: ArchiveCreateFormat) {
        let stem = self.output_stem();
        self.format = format;
        self.self_extracting &= format == ArchiveCreateFormat::Zip;
        self.name = format!("{stem}.{}", self.output_extension());
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
    fn output_extension(&self) -> &'static str {
        if self.self_extracting {
            "exe"
        } else {
            extension(self.format)
        }
    }
    fn output_stem(&self) -> String {
        let suffix = format!(".{}", self.output_extension());
        match self.name.len().checked_sub(suffix.len()) {
            Some(end)
                if self
                    .name
                    .get(end..)
                    .is_some_and(|value| value.eq_ignore_ascii_case(&suffix)) =>
            {
                self.name[..end].to_owned()
            }
            _ => self.name.clone(),
        }
    }
    pub fn set_self_extracting(&mut self, value: bool) {
        let stem = self.output_stem();
        self.self_extracting = value && self.format == ArchiveCreateFormat::Zip;
        self.name = format!("{stem}.{}", self.output_extension());
    }
    pub fn options(&self) -> Result<CreateOptions, String> {
        if self.sources.is_empty() || self.sources.len() > 32 {
            return Err("请选择 1–32 个来源；不会丢弃超出的输入。".into());
        }
        if self.self_extracting && self.format != ArchiveCreateFormat::Zip {
            return Err("只有 ZIP 支持创建自解压压缩包。".into());
        }
        if self.self_extracting && (!self.password.is_empty() || !self.confirm.is_empty()) {
            return Err("自解压 ZIP 不支持密码；请清除密码或取消自解压。".into());
        }
        if !self.separately && !valid_job_name(&self.name) {
            return Err("压缩包名称必须是一个有效文件名。".into());
        }
        if !self.separately
            && !self
                .name
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", self.output_extension()))
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
        let volume_bytes = self.volume_bytes()?;
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
            volume_bytes,
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
    pub fn volume_size(&self) -> Result<Option<ArchiveVolumeSize>, String> {
        let digits = self.volume_mib.trim();
        if digits.is_empty() {
            return Ok(None);
        }
        if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("分卷大小必须是正整数；请选择 KiB、MiB 或 GiB。".into());
        }
        let value = digits.parse().map_err(|_| "分卷大小超出可表示范围。")?;
        Ok(Some(ArchiveVolumeSize {
            value,
            unit: self.volume_unit,
        }))
    }
    pub fn volume_bytes(&self) -> Result<Option<u64>, String> {
        let Some(size) = self.volume_size()? else {
            return Ok(None);
        };
        if !matches!(
            self.format,
            ArchiveCreateFormat::Zip | ArchiveCreateFormat::SevenZ
        ) {
            return Err("仅 ZIP / 7Z 可创建分卷；请选择“不分卷”或切换回 ZIP / 7Z。".into());
        }
        let bytes = size.bytes()?;
        if bytes > ArchiveJobLimits::default().max_output_bytes {
            return Err("分卷大小不能超过本次输出预算。".into());
        }
        Ok(Some(bytes))
    }
    pub fn requests(&self, settings: &ArchiveSettings) -> Result<Vec<ArchiveJobRequest>, String> {
        let options = self.options()?;
        if !valid_job_path(&self.destination) {
            return Err("请选择本地绝对目标文件夹（例如 C:\\Archive）。".into());
        }
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
                format!("{stem}.{}", self.output_extension())
            } else {
                self.name.clone()
            };
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
                    self_extracting: self.self_extracting,
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
                    volume_bytes: options.volume_bytes,
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
    pub remember_destination: bool,
    pub delete_source: bool,
    pub reclamation: bool,
    pub continue_reclamation: bool,
    pub recovery_id: String,
}
impl ExtractDraft {
    pub fn new(
        settings: &ArchiveSettings,
        source: PathBuf,
        selection: Option<DraftSelection>,
    ) -> Self {
        let destination = if settings.default_destination != ArchiveDestination::Here {
            settings.extract_custom_destination.clone()
        } else {
            None
        }
        .unwrap_or_else(|| {
            source
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
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
            remember_destination: settings.extract_custom_destination.is_some(),
            delete_source: false,
            reclamation: false,
            continue_reclamation: false,
            recovery_id: Uuid::new_v4().to_string(),
        }
    }
    pub fn continue_reclamation(settings: &ArchiveSettings, source: PathBuf) -> Self {
        let mut draft = Self::new(settings, source, None);
        draft.strategy = ArchiveDestination::Here;
        draft.conflict = ArchiveConflictPolicy::Ask;
        draft.selected_only = false;
        draft.reclamation = true;
        draft.continue_reclamation = true;
        draft.recovery_id.clear();
        draft
    }
    pub fn reclamation_refusal(&self) -> Option<String> {
        if self.source.as_os_str().is_empty() {
            return Some("请先选择普通 .zip 原压缩包。".into());
        }
        if self
            .source
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("zip"))
        {
            return Some("初始空间回收只支持普通单卷 .zip；其他格式仍可使用普通解压。".into());
        }
        if self.strategy != ArchiveDestination::Here {
            return Some("空间回收必须使用“当前文件夹”目标策略。".into());
        }
        if self.conflict != ArchiveConflictPolicy::Ask {
            return Some("空间回收必须使用“询问”冲突处理，不能跳过、覆盖或自动重命名。".into());
        }
        if self.selected_only {
            return Some("空间回收只支持完整解压全部文件，不能使用选定文件。".into());
        }
        if self.delete_source {
            return Some("空间回收会保留已消耗的原压缩包外壳，不能同时删除来源。".into());
        }
        None
    }
    fn reclamation_intent(&self) -> Result<Option<ArchiveReclamationIntent>, String> {
        if !self.reclamation {
            if self.continue_reclamation {
                return Err("继续空间回收解压必须保持空间回收模式启用。".into());
            }
            return Ok(None);
        }
        if let Some(reason) = self.reclamation_refusal() {
            return Err(reason);
        }
        let recovery_id = Uuid::parse_str(self.recovery_id.trim())
            .map_err(|_| "请输入恢复记录或源标记中的有效恢复 UUID。".to_owned())?;
        if recovery_id.is_nil() {
            return Err("恢复 UUID 不能为全零值。".into());
        }
        Ok(Some(ArchiveReclamationIntent {
            mode: if self.continue_reclamation {
                ArchiveReclamationMode::Continue
            } else {
                ArchiveReclamationMode::Start
            },
            recovery_id,
        }))
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
                strip_prefix: None,
            })
        } else {
            None
        };
        let reclamation = self.reclamation_intent()?;
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
                reclamation,
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
    pub source_prefix: Option<String>,
    pub added: Vec<String>,
    pub codepage: u32,
}
impl ModifyDraft {
    pub fn request(&self) -> Result<ArchiveJobRequest, String> {
        if self.browser.format == "SevenZ" {
            let facts = self
                .browser
                .sevenz_edit
                .as_ref()
                .ok_or("尚未确认源结构或元数据可安全保留，不能修改。")?;
            if let Some(refusal) = facts.refusal {
                return Err(refusal.message().into());
            }
        }
        if matches!(self.browser.format.as_str(), "Tar" | "TarGz" | "TarXz") {
            let facts = self
                .browser
                .tar_edit
                .as_ref()
                .ok_or("尚未确认源结构或元数据可安全保留，不能修改。")?;
            if let Some(refusal) = facts.refusal {
                return Err(refusal.message().into());
            }
        }
        if self.action == ArchiveJobModifyAction::ReplaceContent
            && (!matches!(
                self.browser.format.as_str(),
                "SevenZ" | "Tar" | "TarGz" | "TarXz"
            ) || self.added.len() != 1
                || self.ordinals.len() != 1
                || self.value.is_some())
        {
            return Err("替换内容只接受一个原始文件条目和一个普通替换文件。".into());
        }
        let source = self.source.to_str().ok_or("压缩包路径不是有效 Unicode。")?;
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
                source_prefix: self.source_prefix.clone(),
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
