use nxtools_archive::{
    OperationControl, ResourceLimits, Result,
    creation::{CreationOptions, CreationPlan, CreationSource, SourceEntry},
};
use uuid::Uuid;

struct SyntheticSource {
    root: SourceEntry,
    children: Vec<SourceEntry>,
}
impl CreationSource for SyntheticSource {
    fn roots(&mut self) -> Result<Vec<SourceEntry>> {
        Ok(vec![self.root.clone()])
    }
    fn next_child(&mut self, directory: Uuid, offset: u64) -> Result<Option<SourceEntry>> {
        assert_eq!(directory, self.root.id);
        Ok(self.children.get(offset as usize).cloned())
    }
    fn open_file(&mut self, _: Uuid) -> Result<()> {
        panic!("planning must not read content")
    }
    fn read_file(&mut self, _: Uuid, _: u64, _: u32) -> Result<Vec<u8>> {
        panic!("planning must not read content")
    }
    fn close_file(&mut self, _: Uuid) -> Result<()> {
        panic!("planning must not read content")
    }
    fn revalidate(&mut self) -> Result<()> {
        Ok(())
    }
}
fn entry(name: &str, directory: bool, size: u64) -> SourceEntry {
    SourceEntry {
        id: Uuid::new_v4(),
        name: name.into(),
        directory,
        size,
        dos_modified: None,
    }
}

#[test]
fn public_creation_plan_preserves_unicode_empty_entries_root_policy_and_default_exclusions() {
    for strip_single_root in [false, true] {
        let source = SyntheticSource {
            root: entry("资料", true, 0),
            children: vec![
                entry("空目录", true, 0),
                entry("空文件.txt", false, 0),
                entry("payload.bin", false, 7),
                entry("Thumbs.db", false, 10),
            ],
        };
        // Empty child directories are enumerated too; this source has exactly one such child.
        let empty = source.children[0].id;
        struct Tree(SyntheticSource, Uuid);
        impl CreationSource for Tree {
            fn roots(&mut self) -> Result<Vec<SourceEntry>> {
                self.0.roots()
            }
            fn next_child(&mut self, id: Uuid, offset: u64) -> Result<Option<SourceEntry>> {
                if id == self.1 {
                    Ok(None)
                } else {
                    self.0.next_child(id, offset)
                }
            }
            fn open_file(&mut self, id: Uuid) -> Result<()> {
                self.0.open_file(id)
            }
            fn read_file(&mut self, id: Uuid, offset: u64, length: u32) -> Result<Vec<u8>> {
                self.0.read_file(id, offset, length)
            }
            fn close_file(&mut self, id: Uuid) -> Result<()> {
                self.0.close_file(id)
            }
            fn revalidate(&mut self) -> Result<()> {
                self.0.revalidate()
            }
        }
        let mut tree = Tree(source, empty);
        let options = CreationOptions {
            strip_single_root,
            ..CreationOptions::default()
        };
        let plan = CreationPlan::build(
            &mut tree,
            &options,
            &ResourceLimits::default(),
            &OperationControl::default(),
        )
        .expect("bounded public plan");
        let names: Vec<_> = plan
            .entries()
            .iter()
            .map(|item| item.relative_path())
            .collect();
        assert_eq!(
            names,
            if strip_single_root {
                vec!["空目录/", "空文件.txt", "payload.bin"]
            } else {
                vec![
                    "资料/",
                    "资料/空目录/",
                    "资料/空文件.txt",
                    "资料/payload.bin",
                ]
            }
        );
        assert_eq!(plan.total_bytes(), 7);
        assert_eq!(plan.excluded_entries(), 1);
    }
}

#[test]
fn public_plan_rejects_case_names_unsafe_components_invalid_globs_and_finite_limits() {
    use nxtools_archive::ArchiveError;
    let control = OperationControl::default();
    for (names, expected) in [
        (vec!["same.txt", "SAME.TXT"], ArchiveError::OutputCollision),
        (vec!["CON.txt"], ArchiveError::UnsafePath),
        (vec!["../bad"], ArchiveError::UnsafePath),
        (vec!["C:ads"], ArchiveError::UnsafePath),
    ] {
        let mut source = SyntheticSource {
            root: entry("root", true, 0),
            children: names
                .into_iter()
                .map(|name| entry(name, false, 1))
                .collect(),
        };
        assert_eq!(
            CreationPlan::build(
                &mut source,
                &CreationOptions::default(),
                &ResourceLimits::default(),
                &control
            )
            .err(),
            Some(expected)
        );
    }
    for pattern in ["[", "../*", "", "!secret", "/absolute"] {
        let mut source = SyntheticSource {
            root: entry("file", false, 1),
            children: vec![],
        };
        assert_eq!(
            CreationPlan::build(
                &mut source,
                &CreationOptions {
                    exclude_patterns: vec![pattern.into()],
                    ..CreationOptions::default()
                },
                &ResourceLimits::default(),
                &control
            )
            .err(),
            Some(ArchiveError::UnsafePath)
        );
    }
    for limits in [
        ResourceLimits {
            max_entries: 1,
            ..ResourceLimits::default()
        },
        ResourceLimits {
            max_metadata_bytes: 1,
            ..ResourceLimits::default()
        },
        ResourceLimits {
            max_entry_bytes: 1,
            ..ResourceLimits::default()
        },
        ResourceLimits {
            max_output_bytes: 1,
            ..ResourceLimits::default()
        },
        ResourceLimits {
            max_working_bytes: 1,
            ..ResourceLimits::default()
        },
    ] {
        let mut source = SyntheticSource {
            root: entry("root", true, 0),
            children: vec![entry("file", false, 2)],
        };
        assert_eq!(
            CreationPlan::build(&mut source, &CreationOptions::default(), &limits, &control).err(),
            Some(ArchiveError::ResourceLimit)
        );
    }
    let cancelled = OperationControl::default();
    cancelled.cancel();
    let mut source = SyntheticSource {
        root: entry("file", false, 0),
        children: vec![],
    };
    assert_eq!(
        CreationPlan::build(
            &mut source,
            &CreationOptions::default(),
            &ResourceLimits::default(),
            &cancelled
        )
        .err(),
        Some(ArchiveError::Cancelled)
    );
}

#[test]
fn multiple_sources_never_strip_roots_and_path_globs_do_not_match_other_directories() {
    struct Many(Vec<SourceEntry>);
    impl CreationSource for Many {
        fn roots(&mut self) -> Result<Vec<SourceEntry>> {
            Ok(self.0.clone())
        }
        fn next_child(&mut self, _: Uuid, _: u64) -> Result<Option<SourceEntry>> {
            Ok(None)
        }
        fn open_file(&mut self, _: Uuid) -> Result<()> {
            unreachable!()
        }
        fn read_file(&mut self, _: Uuid, _: u64, _: u32) -> Result<Vec<u8>> {
            unreachable!()
        }
        fn close_file(&mut self, _: Uuid) -> Result<()> {
            unreachable!()
        }
        fn revalidate(&mut self) -> Result<()> {
            Ok(())
        }
    }
    let mut source = Many(vec![entry("first", true, 0), entry("second", true, 0)]);
    let plan = CreationPlan::build(
        &mut source,
        &CreationOptions {
            strip_single_root: true,
            ..CreationOptions::default()
        },
        &ResourceLimits::default(),
        &OperationControl::default(),
    )
    .expect("multi root");
    assert_eq!(
        plan.entries()
            .iter()
            .map(|item| item.relative_path())
            .collect::<Vec<_>>(),
        ["first/", "second/"]
    );
    let mut source = SyntheticSource {
        root: entry("root", true, 0),
        children: vec![entry("remove.tmp", false, 3), entry("keep.bin", false, 7)],
    };
    let plan = CreationPlan::build(
        &mut source,
        &CreationOptions {
            exclude_patterns: vec!["root/*.tmp".into()],
            ..CreationOptions::default()
        },
        &ResourceLimits::default(),
        &OperationControl::default(),
    )
    .expect("relative exclusion");
    assert_eq!(plan.total_bytes(), 7);
    assert_eq!(plan.excluded_entries(), 1);
}

#[test]
fn archive_directory_payload_is_zero_even_when_native_directory_object_has_physical_size() {
    let mut source = SyntheticSource {
        root: entry("directory", true, 4096),
        children: vec![],
    };
    let plan = CreationPlan::build(
        &mut source,
        &CreationOptions::default(),
        &ResourceLimits::default(),
        &OperationControl::default(),
    )
    .expect("directory plan");
    assert_eq!(plan.entries().len(), 1);
    assert!(plan.entries()[0].is_directory());
    assert_eq!(
        plan.entries()[0].size(),
        0,
        "native directory allocation is not archive payload"
    );
    assert_eq!(plan.total_bytes(), 0);
}

#[test]
fn ordinary_create_duplicate_roots_still_refuse_before_traversal_or_payload() {
    struct Roots(Vec<SourceEntry>);
    impl CreationSource for Roots {
        fn roots(&mut self) -> Result<Vec<SourceEntry>> {
            Ok(self.0.clone())
        }
        fn next_child(&mut self, _: Uuid, _: u64) -> Result<Option<SourceEntry>> {
            panic!("ordinary duplicate roots must refuse before traversal")
        }
        fn open_file(&mut self, _: Uuid) -> Result<()> {
            panic!("no payload")
        }
        fn read_file(&mut self, _: Uuid, _: u64, _: u32) -> Result<Vec<u8>> {
            panic!("no payload")
        }
        fn close_file(&mut self, _: Uuid) -> Result<()> {
            panic!("no payload")
        }
        fn revalidate(&mut self) -> Result<()> {
            panic!("collision precedes final source check")
        }
    }
    for directory in [false, true] {
        for second in ["same", "SAME"] {
            let mut source = Roots(vec![
                entry("same", directory, 0),
                entry(second, directory, 0),
            ]);
            assert_eq!(
                CreationPlan::build(
                    &mut source,
                    &CreationOptions::default(),
                    &ResourceLimits::default(),
                    &OperationControl::default(),
                )
                .err(),
                Some(nxtools_archive::ArchiveError::OutputCollision)
            );
        }
    }
}
