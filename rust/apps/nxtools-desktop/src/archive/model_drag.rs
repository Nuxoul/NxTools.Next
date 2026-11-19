//! Bounded metadata-only expansion on the ORIGINAL immutable browser snapshot.
use super::*;
use nxtools_archive::planning::{
    CatalogSelection, SelectionEntry, SelectionWork, bind_ordinals, expand_selection,
};
use nxtools_protocol::archive_virtual_drag::valid_drag_name_component;

pub(crate) struct FrozenDrop {
    pub wrapper: String,
    pub direct: bool,
    pub ordinals: Option<Vec<u64>>,
    pub strip_prefix: Option<String>,
}

impl BrowserModel {
    /// Freeze ordinary extraction intent without virtual-file descriptors or content requests.
    pub fn freeze_drop(
        &self,
        tree_path: Option<&str>,
        display_name: &str,
    ) -> Result<FrozenDrop, String> {
        if tree_path == Some("") {
            if self.entries.is_empty() {
                return Err("空压缩包没有可拖出的项目；未拖出任何项目。".into());
            }
            return Ok(FrozenDrop {
                wrapper: archive_drag_stem(display_name)?.into(),
                direct: false,
                ordinals: None,
                strip_prefix: None,
            });
        }
        let mut ids = Vec::new();
        let mut directories = Vec::new();
        if let Some(path) = tree_path {
            directories.push(path);
        } else {
            for id in &self.selected {
                match id {
                    RowId::Entry(snapshot, ordinal) if *snapshot == self.snapshot => {
                        ids.push(*ordinal)
                    }
                    RowId::Folder(snapshot, path) if *snapshot == self.snapshot => {
                        directories.push(path.as_str())
                    }
                    _ => return Err("所选条目来自已失效的快照，请刷新后重试。".into()),
                }
            }
        }
        if ids.is_empty() && directories.is_empty() {
            return Err("请选择要拖出的项目；未拖出任何项目。".into());
        }
        let directory = (ids.is_empty() && directories.len() == 1).then(|| directories[0]);
        let direct = tree_path.is_none() && directories.is_empty() && !ids.is_empty();
        let wrapper = if let Some(path) = directory {
            let name = path.rsplit('/').next().unwrap_or("");
            if !valid_drag_name_component(name) {
                return Err(rejected(name, "文件夹名称不满足Windows输出限制"));
            }
            name.to_owned()
        } else {
            archive_drag_stem(display_name)?.to_owned()
        };
        let base = directory.unwrap_or(&self.folder);
        let strip_prefix = (!base.is_empty()).then(|| format!("{base}/"));
        let mut work = SelectionWork::default();
        let credit = self
            .selection_metadata_credit(&mut work)
            .map_err(selection_failure)?;
        let positions = bind_ordinals(
            &self.entries,
            &ids,
            credit,
            |entry| entry.ordinal,
            || Ok(()),
            &mut work,
        )
        .map_err(selection_failure)?;
        let mut failure = None;
        let ordinals = expand_selection(
            &self.entries,
            CatalogSelection::Entries {
                positions: &positions,
                directories: &directories,
            },
            credit,
            |_, entry| SelectionEntry {
                ordinal: entry.ordinal,
                path: &entry.name,
                directory: entry.kind == "Directory",
            },
            |_, entry, path, _| {
                let valid = matches!(entry.kind.as_str(), "File" | "Directory")
                    && !entry
                        .warning_codes
                        .iter()
                        .any(|code| code == "UnsafePath" || code == "LinkNotExtracted")
                    && path
                        .trim_end_matches('/')
                        .split('/')
                        .all(valid_drag_name_component)
                    && strip_prefix.as_deref().is_none_or(|prefix| {
                        path.starts_with(prefix)
                            || entry.kind == "Directory"
                                && path.trim_end_matches('/') == prefix.trim_end_matches('/')
                    });
                if !valid {
                    failure = Some(rejected(
                        &entry.name,
                        "条目类型、原始路径或当前文件夹不允许解压",
                    ));
                    return Err(nxtools_archive::ArchiveError::UnsafePath);
                }
                Ok(0)
            },
            || Ok(()),
            &mut work,
        )
        .map_err(|error| failure.unwrap_or_else(|| selection_failure(error)))?;
        if ordinals.is_empty() {
            return Err("所选条目已不在当前快照中，请刷新后重试。".into());
        }
        Ok(FrozenDrop {
            wrapper,
            direct,
            ordinals: Some(ordinals),
            strip_prefix,
        })
    }
}

#[cfg(test)]
pub(crate) struct FrozenDragItem {
    pub name: String,
    pub directory: bool,
    pub ordinal: Option<u64>,
    pub size: Option<u64>,
}

fn rejected(name: &str, reason: &str) -> String {
    format!("无法拖出“{name}”：{reason}；未拖出任何项目。")
}
fn selection_failure(error: nxtools_archive::ArchiveError) -> String {
    match error {
        nxtools_archive::ArchiveError::SourceChanged => {
            "所选条目、目录或其成员已不在当前快照中，请刷新后重试；未拖出任何项目。".into()
        }
        _ => "选择超过当前元数据预算；未拖出任何项目。".into(),
    }
}
fn archive_drag_stem(display_name: &str) -> Result<&str, String> {
    if !valid_drag_name_component(display_name) {
        return Err(rejected(
            display_name,
            "压缩包名称含Windows保留名、非法字符或名称段过长",
        ));
    }
    let folded = display_name.to_ascii_lowercase();
    let compound = [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".tar.zstd"]
        .into_iter()
        .find(|suffix| folded.ends_with(*suffix));
    let stem = if let Some(suffix) = compound {
        &display_name[..display_name.len() - suffix.len()]
    } else {
        display_name
            .rsplit_once('.')
            .map_or(display_name, |(stem, _)| stem)
    };
    valid_drag_name_component(stem)
        .then_some(stem)
        .ok_or_else(|| {
            rejected(
                display_name,
                "去除扩展名后的目录名称不能由Windows文件描述符表示",
            )
        })
}

// Only descriptor presentation changes. Original member ordinal/path still owns content reads.
#[cfg(test)]
fn descriptor_name(
    frozen: &BTreeMap<String, FrozenDragItem>,
    parent: &str,
    component: &str,
    directory: bool,
) -> String {
    let join = |leaf: &str| {
        if parent.is_empty() {
            leaf.to_owned()
        } else {
            format!("{parent}/{leaf}")
        }
    };
    let original = join(component);
    if frozen
        .get(&original.to_lowercase())
        .is_none_or(|item| directory && item.directory)
    {
        return frozen
            .get(&original.to_lowercase())
            .map_or(original, |item| item.name.clone());
    }
    let (stem, extension) = if directory {
        (component, String::new())
    } else {
        component
            .rsplit_once('.')
            .filter(|(stem, _)| !stem.is_empty())
            .map_or((component, String::new()), |(stem, extension)| {
                (stem, format!(".{extension}"))
            })
    };
    for number in 2..=frozen.len() + 2 {
        let suffix = format!(" ({number}){extension}");
        let mut stem = stem.to_owned();
        let name = loop {
            let leaf = format!("{stem}{suffix}");
            let name = join(&leaf);
            if (valid_drag_name_component(&leaf) && name.encode_utf16().count() < 260)
                || stem.is_empty()
            {
                break name;
            }
            stem.pop();
        };
        if !frozen.contains_key(&name.to_lowercase()) {
            return name;
        }
    }
    unreachable!("more candidates than occupied descriptor names")
}
impl BrowserModel {
    /// The original model, visible projection and selection coexist with operation-local scratch.
    pub fn selection_metadata_credit(
        &self,
        work: &mut SelectionWork,
    ) -> nxtools_archive::Result<u64> {
        let mut bytes = 0u64;
        for entry in &self.entries {
            work.metadata_visits += 1;
            let strings = entry.name.len()
                + entry.kind.len()
                + entry.provenance.len()
                + entry
                    .warning_codes
                    .iter()
                    .map(|code| std::mem::size_of::<String>() + code.len())
                    .sum::<usize>();
            bytes = bytes
                .checked_add(2 * std::mem::size_of::<ArchiveReadEntry>() as u64)
                .and_then(|bytes| bytes.checked_add(strings as u64))
                .ok_or(nxtools_archive::ArchiveError::ResourceLimit)?;
        }
        for row in self.visible() {
            work.metadata_visits += 1;
            bytes = bytes
                .checked_add(2 * std::mem::size_of::<Row>() as u64)
                .and_then(|bytes| {
                    bytes.checked_add((row.name.len() + row.path.len() + row.kind.len()) as u64)
                })
                .ok_or(nxtools_archive::ArchiveError::ResourceLimit)?;
        }
        for id in &self.selected {
            work.metadata_visits += 1;
            let path = match id {
                RowId::Folder(_, path) | RowId::Parent(_, path) => path.len(),
                _ => 0,
            };
            bytes = bytes
                .checked_add(96 + path as u64)
                .ok_or(nxtools_archive::ArchiveError::ResourceLimit)?;
        }
        nxtools_archive::ResourceLimits::default()
            .max_metadata_bytes
            .checked_sub(bytes)
            .ok_or(nxtools_archive::ArchiveError::ResourceLimit)
    }
    #[cfg(test)]
    pub fn freeze_drag_selection(&self) -> Result<Vec<FrozenDragItem>, String> {
        self.freeze_drag_selection_authorized(false)
    }
    #[cfg(test)]
    pub fn freeze_drag_selection_authorized(
        &self,
        authorized: bool,
    ) -> Result<Vec<FrozenDragItem>, String> {
        self.freeze_drag_selection_observed(authorized, &mut SelectionWork::default())
    }
    #[cfg(test)]
    pub fn freeze_drag_selection_observed(
        &self,
        authorized: bool,
        work: &mut SelectionWork,
    ) -> Result<Vec<FrozenDragItem>, String> {
        if self.selected.is_empty() {
            return Err("请选择要拖出的项目；未拖出任何项目。".into());
        }
        let mut ordinals = Vec::new();
        let mut roots = Vec::new();
        for id in &self.selected {
            match id {
                RowId::Entry(snapshot, ordinal) if *snapshot == self.snapshot => {
                    ordinals.push(*ordinal)
                }
                RowId::Folder(snapshot, path) if *snapshot == self.snapshot => {
                    roots.push(path.as_str())
                }
                _ => return Err("所选条目来自已失效的快照，请刷新后重试。".into()),
            }
        }
        let credit = self
            .selection_metadata_credit(work)
            .map_err(selection_failure)?;
        let positions = bind_ordinals(
            &self.entries,
            &ordinals,
            credit,
            |entry| entry.ordinal,
            || Ok(()),
            work,
        )
        .map_err(selection_failure)?;
        let prefix = if self.folder.is_empty() {
            String::new()
        } else {
            format!("{}/", self.folder)
        };
        self.freeze_drag_roots(
            CatalogSelection::Entries {
                positions: &positions,
                directories: &roots,
            },
            &prefix,
            None,
            authorized,
            credit,
            work,
        )
    }
    #[cfg(test)]
    pub fn freeze_drag_tree(
        &self,
        path: &str,
        display_name: &str,
        authorized: bool,
    ) -> Result<Vec<FrozenDragItem>, String> {
        let mut work = SelectionWork::default();
        let credit = self
            .selection_metadata_credit(&mut work)
            .map_err(selection_failure)?;
        if path.is_empty() {
            if self.entries.is_empty() {
                return Err("空压缩包没有可拖出的项目；未拖出任何项目。".into());
            }
            let stem = archive_drag_stem(display_name)?;
            self.freeze_drag_roots(
                CatalogSelection::Entries {
                    positions: &[],
                    directories: &[""],
                },
                "",
                Some(stem),
                authorized,
                credit,
                &mut work,
            )
        } else {
            let prefix = path
                .rsplit_once('/')
                .map_or(String::new(), |(parent, _)| format!("{parent}/"));
            self.freeze_drag_roots(
                CatalogSelection::Entries {
                    positions: &[],
                    directories: &[path],
                },
                &prefix,
                None,
                authorized,
                credit,
                &mut work,
            )
        }
    }
    #[cfg(test)]
    fn freeze_drag_roots(
        &self,
        selection: CatalogSelection<'_>,
        prefix: &str,
        wrapper: Option<&str>,
        authorized: bool,
        credit: u64,
        work: &mut SelectionWork,
    ) -> Result<Vec<FrozenDragItem>, String> {
        let mut frozen = BTreeMap::<String, FrozenDragItem>::new();
        let mut directories = BTreeMap::<String, String>::new();
        let mut ordinals = BTreeSet::new();
        let mut presentation_bytes = 0u64;
        let mut failure = None;
        let expansion = expand_selection(
            &self.entries,
            selection,
            credit,
            |_, entry| SelectionEntry {
                ordinal: entry.ordinal,
                path: &entry.name,
                directory: entry.kind == "Directory",
            },
            |_, entry, normalized, reservation| {
                let rendered = (|| -> Result<(), String> {
                    let path = normalized.trim_end_matches('/');
                    if !matches!(entry.kind.as_str(), "File" | "Directory") {
                        return Err(rejected(
                            &entry.name,
                            &format!("条目类型{}不能作为普通文件或目录提供OLE内容", entry.kind),
                        ));
                    }
                    if entry.kind == "File" && entry.encrypted == Some(true) && !authorized {
                        return Err(rejected(
                            &entry.name,
                            "加密文件需要先点击“解锁拖出”并输入密码，再重新按住拖动",
                        ));
                    }
                    if entry
                        .warning_codes
                        .iter()
                        .any(|code| code == "UnsafePath" || code == "LinkNotExtracted")
                    {
                        return Err(rejected(
                            &entry.name,
                            "读取器标记原始路径无效或条目为不提取的链接，不能授予原生文件写入路径",
                        ));
                    }
                    presentation_bytes = presentation_bytes
                        .checked_add(64)
                        .ok_or_else(|| rejected(&entry.name, "选择超过当前元数据预算"))?;
                    if !ordinals.insert(entry.ordinal) {
                        return Err(rejected(
                            &entry.name,
                            "当前快照含重复条目序号，请刷新后重试",
                        ));
                    }
                    let raw = if entry.kind == "Directory" {
                        normalized.strip_suffix('/').unwrap_or(normalized)
                    } else {
                        normalized
                    };
                    if raw.split('/').any(|part| !valid_drag_name_component(part)) {
                        return Err(rejected(
                            &entry.name,
                            "路径含空段、相对跳转、Windows保留名、非法字符或名称段过长",
                        ));
                    }
                    let name = path
                        .strip_prefix(prefix)
                        .filter(|name| !name.is_empty())
                        .ok_or_else(|| {
                            rejected(&entry.name, "条目已不在当前文件夹，请刷新后重试")
                        })?;
                    let parts = name.split('/').collect::<Vec<_>>();
                    let mut parent = String::new();
                    for end in 1..=parts.len() {
                        let original = parts[..end].join("/").to_lowercase();
                        let directory = end < parts.len() || entry.kind == "Directory";
                        let ordinal = (end == parts.len()).then_some(entry.ordinal);
                        let target = if directory {
                            directories.get(&original).cloned().unwrap_or_else(|| {
                                descriptor_name(&frozen, &parent, parts[end - 1], true)
                            })
                        } else {
                            descriptor_name(&frozen, &parent, parts[end - 1], false)
                        };
                        if target.encode_utf16().count() >= 260
                            || target
                                .split('/')
                                .any(|part| !valid_drag_name_component(part))
                        {
                            return Err(rejected(
                                &entry.name,
                                "OLE FILEDESCRIPTORW路径必须少于260个UTF-16单元，重命名后名称段仍须满足Windows输出限制",
                            ));
                        }
                        let key = target.to_lowercase();
                        if let Some(existing) = frozen.get_mut(&key) {
                            if existing.ordinal.is_none() {
                                existing.ordinal = ordinal;
                            }
                        } else {
                            presentation_bytes = presentation_bytes
                                .checked_add(256 + 2 * target.len() as u64)
                                .ok_or_else(|| rejected(&entry.name, "选择超过当前元数据预算"))?;
                            if presentation_bytes
                                .checked_add(reservation)
                                .is_none_or(|bytes| bytes > credit)
                            {
                                return Err(rejected(&entry.name, "选择超过当前元数据预算"));
                            }
                            frozen.insert(
                                key,
                                FrozenDragItem {
                                    name: target.clone(),
                                    directory,
                                    ordinal,
                                    size: if directory { None } else { entry.size },
                                },
                            );
                        }
                        if directory {
                            if !directories.contains_key(&original) {
                                presentation_bytes = presentation_bytes
                                    .checked_add(96 + original.len() as u64 + target.len() as u64)
                                    .ok_or_else(|| {
                                        rejected(&entry.name, "选择超过当前元数据预算")
                                    })?;
                                if presentation_bytes
                                    .checked_add(reservation)
                                    .is_none_or(|bytes| bytes > credit)
                                {
                                    return Err(rejected(&entry.name, "选择超过当前元数据预算"));
                                }
                            }
                            directories.insert(original, target.clone());
                        }
                        parent = target;
                    }
                    Ok(())
                })();
                rendered
                    .map_err(|message| {
                        failure = Some(message);
                        nxtools_archive::ArchiveError::UnsafePath
                    })
                    .map(|()| presentation_bytes)
            },
            || Ok(()),
            work,
        );
        expansion.map_err(|error| failure.unwrap_or_else(|| selection_failure(error)))?;
        let mut items = frozen.into_values().collect::<Vec<_>>();
        if let Some(wrapper) = wrapper {
            presentation_bytes = presentation_bytes
                .checked_add(256 + wrapper.len() as u64)
                .ok_or_else(|| selection_failure(nxtools_archive::ArchiveError::ResourceLimit))?;
            for item in &mut items {
                presentation_bytes = presentation_bytes
                    .checked_add(wrapper.len() as u64 + 1)
                    .ok_or_else(|| {
                        selection_failure(nxtools_archive::ArchiveError::ResourceLimit)
                    })?;
                if presentation_bytes > credit {
                    return Err(selection_failure(
                        nxtools_archive::ArchiveError::ResourceLimit,
                    ));
                }
                item.name = format!("{wrapper}/{}", item.name);
                if item.name.encode_utf16().count() >= 260 {
                    return Err(rejected(
                        &item.name,
                        "OLE FILEDESCRIPTORW路径必须少于260个UTF-16单元",
                    ));
                }
            }
            items.insert(
                0,
                FrozenDragItem {
                    name: wrapper.into(),
                    directory: true,
                    ordinal: None,
                    size: None,
                },
            );
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;
    fn model() -> BrowserModel {
        BrowserModel::new(Uuid::new_v4(), [(0,"top/sub/empty/","Directory"),(1,"top/sub/file.txt","File"),(2,"loose.txt","File")]
            .into_iter().map(|(ordinal,name,kind)|serde_json::from_value(serde_json::json!({
                "ordinal":ordinal,"name":name,"kind":kind,"size":3,"packed_size":null,"encrypted":false,
                "warning_codes":[],"provenance":"Utf8Flag","codepage":65001
            })).expect("entry")).collect())
    }
    #[test]
    fn ordinary_drag_collision_suffix_fits_native_name_budgets() {
        for (parent, leaf) in [
            (String::new(), format!("{}.txt", "a".repeat(251))),
            (String::new(), format!("{}.txt", "界".repeat(83))),
            ("p".repeat(250), "x.txt".to_owned()),
        ] {
            let name = if parent.is_empty() {
                leaf
            } else {
                format!("{parent}/{leaf}")
            };
            let snapshot = Uuid::new_v4();
            let mut model = BrowserModel::new(snapshot, (0..2).map(|ordinal| {
                serde_json::from_value(serde_json::json!({
                    "ordinal":ordinal,"name":name,"kind":"File","size":3,"packed_size":null,
                    "encrypted":false,"warning_codes":["OutputCollision"],"provenance":"Utf8Flag","codepage":65001
                })).expect("colliding member")
            }).collect());
            let items = if parent.is_empty() {
                model
                    .selected
                    .extend([RowId::Entry(snapshot, 0), RowId::Entry(snapshot, 1)]);
                model.freeze_drag_selection()
            } else {
                model.freeze_drag_tree(&parent, "bundle.zip", false)
            }
            .expect(
                "collision suffix must fit native presentation without rejecting original ordinals",
            );
            assert_eq!(
                items
                    .iter()
                    .filter(|item| !item.directory)
                    .filter_map(|item| item.ordinal)
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from([0, 1])
            );
            assert_eq!(
                items
                    .iter()
                    .map(|item| item.name.to_lowercase())
                    .collect::<BTreeSet<_>>()
                    .len(),
                items.len()
            );
            assert!(
                items
                    .iter()
                    .all(|item| item.name.encode_utf16().count() < 260
                        && item.name.split('/').all(valid_drag_name_component))
            );
        }
    }

    #[test]
    fn tree_freeze_is_independent_of_current_projection_and_preserves_basename() {
        let mut model = model();
        model.folder = "elsewhere".into();
        model.filter = "no matches".into();
        model.reproject();
        let items = model
            .freeze_drag_tree("top/sub", "bundle.zip", false)
            .expect("subtree");
        assert_eq!(
            items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["sub", "sub/empty", "sub/file.txt"]
        );
        assert_eq!(items[0].ordinal, None);
        assert_eq!(items[1].ordinal, Some(0));
        assert_eq!(items[2].ordinal, Some(1));
        assert!(model.selected.is_empty());
    }
    #[test]
    fn tree_root_stem_compounds_and_unsafe_names_fail_closed() {
        let model = model();
        for (name, stem) in [
            ("Bundle.TAR.GZ", "Bundle"),
            ("a.tar.bz2", "a"),
            ("a.tar.xz", "a"),
            ("a.tar.zst", "a"),
            ("a.tar.zstd", "a"),
            ("a.tgz", "a"),
            ("a.zip", "a"),
            ("a.v1.zip", "a.v1"),
            ("archive", "archive"),
        ] {
            let items = model.freeze_drag_tree("", name, false).expect("safe stem");
            assert_eq!(items[0].name, stem);
            assert!(
                items
                    .iter()
                    .skip(1)
                    .all(|item| item.name.starts_with(&format!("{stem}/")))
            );
        }
        for name in [
            "",
            ".",
            "..",
            "../escape.zip",
            "bad/name.zip",
            r"bad\name.zip",
            "NUL.zip",
            "COM1.zip",
            "COM¹.zip",
            "CONIN$.zip",
            "CON .zip",
            "bad?.zip",
            "bad.zip.",
            ".zip",
        ] {
            assert!(
                model.freeze_drag_tree("", name, false).is_err(),
                "unsafe display/stem: {name}"
            );
        }
        assert!(
            BrowserModel::new(Uuid::new_v4(), vec![])
                .freeze_drag_tree("", "empty.zip", false)
                .is_err()
        );
        assert!(
            model
                .freeze_drag_tree("missing", "bundle.zip", false)
                .is_err()
        );
    }
    #[test]
    fn tree_root_long_unicode_wrapper_preserves_name_and_member_ordinal() {
        let entry = serde_json::from_value(serde_json::json!({
            "ordinal": 7, "name": "file", "kind": "File", "size": 3,
            "packed_size": null, "encrypted": false, "warning_codes": [],
            "provenance": "Utf8Flag", "codepage": 65001
        }))
        .expect("entry");
        let model = BrowserModel::new(Uuid::new_v4(), vec![entry]);
        let wrapper = "界".repeat(86);
        let items = model
            .freeze_drag_tree("", &format!("{wrapper}.zip"), false)
            .expect("Windows UTF-16 component and FILEDESCRIPTORW path fit");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, wrapper);
        assert!(items[0].directory && items[0].ordinal.is_none());
        assert_eq!(items[1].name, format!("{wrapper}/file"));
        assert_eq!(items[1].ordinal, Some(7));
        assert!(!items[1].directory);
        assert_eq!(
            wrapper.len(),
            258,
            "transport UTF-8 bytes are not component units"
        );
    }
    #[test]
    fn normal_drag_casefolded_directory_conflict_keeps_one_shared_parent() {
        let model = BrowserModel::new(
            Uuid::new_v4(),
            ["a", "A/x", "a/y"]
                .into_iter()
                .enumerate()
                .map(|(ordinal, name)| {
                    serde_json::from_value(serde_json::json!({
                    "ordinal": ordinal, "name": name, "kind": "File", "size": 3,
                    "packed_size": null, "encrypted": null, "warning_codes": ["OutputCollision"],
                    "provenance": "Utf8Flag", "codepage": 65001
                })).expect("entry")
                })
                .collect(),
        );
        let items = model
            .freeze_drag_tree("", "bundle.zip", false)
            .expect("representable tree");
        let parents = items
            .iter()
            .filter(|item| item.directory && item.name != "bundle")
            .collect::<Vec<_>>();
        assert_eq!(
            parents.len(),
            1,
            "one case-insensitive original directory must not split after renaming"
        );
        for ordinal in [1, 2] {
            let file = items
                .iter()
                .find(|item| !item.directory && item.ordinal == Some(ordinal))
                .expect("original ordinal retained");
            assert_eq!(
                file.name.rsplit_once('/').expect("parent").0,
                parents[0].name
            );
        }
        assert_eq!(items.iter().filter(|item| !item.directory).count(), 3);
    }
    #[test]
    fn tree_root_wrapper_budget_encryption_authorization_and_path_limits() {
        let make = |count| {
            BrowserModel::new(
                Uuid::new_v4(),
                (0..count)
                    .map(|ordinal| {
                        serde_json::from_value(
                            serde_json::json!({"ordinal":ordinal,"name":format!("dir/d{ordinal}/"),
                "kind":"Directory","size":0,"packed_size":null,"encrypted":false,"warning_codes":[],
                "provenance":"Utf8Flag","codepage":65001}),
                        )
                        .expect("directory")
                    })
                    .collect(),
            )
        };
        assert_eq!(
            make(256)
                .freeze_drag_tree("", "bundle.zip", false)
                .expect("complete directory tree plus wrapper")
                .len(),
            258
        );
        let mut encrypted_model = model();
        encrypted_model.entries[1].encrypted = Some(true);
        assert!(
            encrypted_model
                .freeze_drag_tree("", "bundle.zip", false)
                .is_err()
        );
        assert!(
            encrypted_model
                .freeze_drag_tree("", "bundle.zip", true)
                .is_ok(),
            "same original preauthorization"
        );
        encrypted_model.entries[1].encrypted = None;
        assert!(
            encrypted_model
                .freeze_drag_tree("", "bundle.zip", true)
                .is_ok(),
            "optional metadata cannot veto an original source-bound request"
        );
        for name in ["top/sub/../escape", "top/sub/CON.txt"] {
            let mut model = model();
            model.entries[2].name = name.into();
            assert!(
                model
                    .freeze_drag_tree("top/sub", "bundle.zip", false)
                    .is_err(),
                "unrepresentable original path"
            );
        }
        let mut duplicate = model();
        duplicate.entries[2].name = "top/sub/file.txt".into();
        let items = duplicate
            .freeze_drag_tree("top/sub", "bundle.zip", false)
            .expect("unique descriptors for duplicate names");
        assert_eq!(items.iter().filter(|item| !item.directory).count(), 2);
    }
}

#[cfg(test)]
mod drop_tests {
    use super::*;
    fn model() -> BrowserModel {
        BrowserModel::new(Uuid::new_v4(), [(7,"top/sub/","Directory"),(9,"top/sub/empty/","Directory"),(13,"top/sub/a.txt","File"),(17,"top/other.txt","File")].into_iter().map(|(ordinal,name,kind)| {
            serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":name,"kind":kind,"size":null,"packed_size":null,"encrypted":true,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("metadata")
        }).collect())
    }
    #[test]
    fn ordinary_drop_subtree_and_list_directory_share_original_members_and_prefix() {
        let mut model = model();
        let tree = model
            .freeze_drop(Some("top/sub"), "bundle.zip")
            .expect("subtree");
        assert_eq!(tree.wrapper, "sub");
        assert_eq!(tree.ordinals.as_deref(), Some(&[7, 9, 13][..]));
        assert_eq!(tree.strip_prefix.as_deref(), Some("top/sub/"));
        model.enter("top");
        model
            .selected
            .insert(RowId::Folder(model.snapshot, "top/sub".into()));
        let list = model
            .freeze_drop(None, "bundle.zip")
            .expect("selected directory");
        assert_eq!(
            (list.wrapper, list.ordinals, list.strip_prefix),
            (tree.wrapper, tree.ordinals, tree.strip_prefix)
        );
    }
    #[test]
    fn ordinary_drop_multiple_selection_strips_only_parent_and_root_is_whole_archive() {
        let mut model = model();
        model.enter("top");
        model.select_all();
        let selected = model
            .freeze_drop(None, "bundle.tar.gz")
            .expect("complete mixed selection");
        assert_eq!(selected.wrapper, "bundle");
        assert_eq!(selected.ordinals.as_deref(), Some(&[7, 9, 13, 17][..]));
        assert_eq!(selected.strip_prefix.as_deref(), Some("top/"));
        let root = model
            .freeze_drop(Some(""), "bundle.tar.gz")
            .expect("whole root");
        assert_eq!(root.wrapper, "bundle");
        assert!(root.ordinals.is_none() && root.strip_prefix.is_none());
    }
    #[test]
    fn ordinary_drop_empty_directory_and_noncontiguous_single_file_keep_exact_intent() {
        let mut model = model();
        let empty = model
            .freeze_drop(Some("top/sub/empty"), "bundle.zip")
            .expect("empty directory");
        assert_eq!(empty.wrapper, "empty");
        assert_eq!(empty.ordinals, Some(vec![9]));
        assert_eq!(empty.strip_prefix.as_deref(), Some("top/sub/empty/"));
        model.enter("top/sub");
        model.selected.insert(RowId::Entry(model.snapshot, 13));
        let single = model.freeze_drop(None, "bundle.zip").expect("single file");
        assert_eq!(single.wrapper, "bundle");
        assert_eq!(single.ordinals, Some(vec![13]));
        assert_eq!(single.strip_prefix.as_deref(), Some("top/sub/"));
    }
    #[test]
    fn ordinary_drop_large_directory_does_not_build_or_limit_virtual_descriptors() {
        let mut model = model();
        let template = model.entries[2].clone();
        model.entries = (0..5000)
            .map(|ordinal| {
                let mut entry = template.clone();
                entry.ordinal = ordinal;
                entry.name = format!(
                    "top/sub/{}/{}.txt",
                    "long".repeat(32),
                    "long".repeat(60) + &ordinal.to_string()
                );
                entry
            })
            .collect();
        let drop = model
            .freeze_drop(Some("top/sub"), "bundle.zip")
            .expect("ordinary output, not FILEDESCRIPTORW paths");
        assert_eq!(drop.ordinals.expect("all members").len(), 5000);
        assert_eq!(drop.wrapper, "sub");
    }
    #[test]
    fn ordinary_drop_missing_stale_or_unsupported_selection_rejects_whole() {
        let mut model = model();
        assert!(model.freeze_drop(Some("missing"), "bundle.zip").is_err());
        model.selected.insert(RowId::Entry(Uuid::new_v4(), 13));
        assert!(model.freeze_drop(None, "bundle.zip").is_err());
        model.selected.clear();
        model.entries[2]
            .warning_codes
            .push("LinkNotExtracted".into());
        assert!(model.freeze_drop(Some("top/sub"), "bundle.zip").is_err());
    }
}
