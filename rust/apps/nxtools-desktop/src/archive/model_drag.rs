//! Bounded metadata-only expansion on the ORIGINAL immutable browser snapshot.
use super::*;
use nxtools_protocol::archive_virtual_drag::valid_drag_name_component;

pub(crate) struct FrozenDragItem {
    pub name: String,
    pub directory: bool,
    pub ordinal: Option<u64>,
    pub size: Option<u64>,
}

fn rejected(name: &str, reason: &str) -> String {
    format!("无法拖出“{name}”：{reason}；未拖出任何项目。")
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
    pub fn freeze_drag_selection(&self) -> Result<Vec<FrozenDragItem>, String> {
        self.freeze_drag_selection_authorized(false)
    }
    pub fn freeze_drag_selection_authorized(
        &self,
        authorized: bool,
    ) -> Result<Vec<FrozenDragItem>, String> {
        if self.selected.is_empty() {
            return Err("请选择要拖出的项目；未拖出任何项目。".into());
        }
        let mut roots = Vec::new();
        for id in &self.selected {
            let root = match id {
                RowId::Entry(snapshot, ordinal) if *snapshot == self.snapshot => {
                    let entry = self
                        .entries
                        .iter()
                        .find(|entry| entry.ordinal == *ordinal)
                        .ok_or_else(|| {
                            format!("所选条目序号{ordinal}已不在当前快照中，请刷新后重试。")
                        })?;
                    (
                        member_path(&entry.name).trim_end_matches('/').to_owned(),
                        entry.kind == "Directory",
                        Some(*ordinal),
                    )
                }
                RowId::Folder(snapshot, path) if *snapshot == self.snapshot => {
                    (path.clone(), true, None)
                }
                _ => return Err("所选条目来自已失效的快照，请刷新后重试。".into()),
            };
            roots.push(root);
        }
        let prefix = if self.folder.is_empty() {
            String::new()
        } else {
            format!("{}/", self.folder)
        };
        self.freeze_drag_roots(roots, &prefix, None, authorized)
    }
    pub fn freeze_drag_tree(
        &self,
        path: &str,
        display_name: &str,
        authorized: bool,
    ) -> Result<Vec<FrozenDragItem>, String> {
        if path.is_empty() {
            if self.entries.is_empty() {
                return Err("空压缩包没有可拖出的项目；未拖出任何项目。".into());
            }
            let stem = archive_drag_stem(display_name)?;
            self.freeze_drag_roots(
                vec![(String::new(), true, None)],
                "",
                Some(stem),
                authorized,
            )
        } else {
            let prefix = path
                .rsplit_once('/')
                .map_or(String::new(), |(parent, _)| format!("{parent}/"));
            self.freeze_drag_roots(vec![(path.into(), true, None)], &prefix, None, authorized)
        }
    }
    fn freeze_drag_roots(
        &self,
        roots: Vec<(String, bool, Option<u64>)>,
        prefix: &str,
        wrapper: Option<&str>,
        authorized: bool,
    ) -> Result<Vec<FrozenDragItem>, String> {
        let mut frozen = BTreeMap::<String, FrozenDragItem>::new();
        let mut directories = BTreeMap::<String, String>::new();
        let mut ordinals = BTreeSet::new();
        let mut resolved = vec![false; roots.len()];
        for entry in &self.entries {
            let normalized = member_path(&entry.name);
            let path = normalized.trim_end_matches('/');
            let mut wanted = false;
            for (index, (root, directory, ordinal)) in roots.iter().enumerate() {
                let matches = if *directory {
                    root.is_empty()
                        || (path == root && entry.kind == "Directory")
                        || path.starts_with(&format!("{root}/"))
                } else {
                    *ordinal == Some(entry.ordinal)
                };
                if matches {
                    resolved[index] = true;
                    wanted = true;
                }
            }
            if !wanted {
                continue;
            }
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
            if !ordinals.insert(entry.ordinal) {
                return Err(rejected(
                    &entry.name,
                    "当前快照含重复条目序号，请刷新后重试",
                ));
            }
            let raw = if entry.kind == "Directory" {
                normalized.strip_suffix('/').unwrap_or(&normalized)
            } else {
                &normalized
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
                .ok_or_else(|| rejected(&entry.name, "条目已不在当前文件夹，请刷新后重试"))?;
            let parts = name.split('/').collect::<Vec<_>>();
            let mut parent = String::new();
            for end in 1..=parts.len() {
                let original = parts[..end].join("/").to_lowercase();
                let directory = end < parts.len() || entry.kind == "Directory";
                let ordinal = (end == parts.len()).then_some(entry.ordinal);
                let target = if directory {
                    directories
                        .get(&original)
                        .cloned()
                        .unwrap_or_else(|| descriptor_name(&frozen, &parent, parts[end - 1], true))
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
                    directories.insert(original, target.clone());
                }
                parent = target;
            }
        }
        for (index, (root, _, _)) in roots.iter().enumerate() {
            if !resolved[index] {
                return Err(rejected(root, "当前快照中没有该目录或其成员，请刷新后重试"));
            }
        }
        let mut items = frozen.into_values().collect::<Vec<_>>();
        if let Some(wrapper) = wrapper {
            for item in &mut items {
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
