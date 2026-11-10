use super::*;

fn file(path: &str) -> DraftSource {
    DraftSource {
        path: path.into(),
        directory: false,
        bytes: Some(3),
    }
}
#[test]
fn saved_defaults_seed_real_draft_and_cancel_has_no_configuration_write() {
    let settings = ArchiveSettings {
        create_format: ArchiveCreateFormat::SevenZ,
        compression_level: ArchiveCompressionLevel::Maximum,
        test_after_create: false,
        strip_single_root: true,
        exclude_patterns: vec!["*.tmp".into()],
        ..Default::default()
    };
    let draft = CreateDraft::new(&settings, vec![file(r"C:\owned\source.txt")]);
    assert_eq!(draft.format, ArchiveCreateFormat::SevenZ);
    assert_eq!(draft.level, ArchiveCompressionLevel::Maximum);
    assert!(!draft.test_after_create);
    assert!(draft.strip_single_root);
    assert_eq!(draft.exclusions, vec!["*.tmp"]);
    drop(draft);
    assert_eq!(settings.exclude_patterns, vec!["*.tmp"]);
}
#[test]
fn format_switch_discards_inapplicable_secret_method_solid_and_level() {
    let mut draft = CreateDraft::new(&ArchiveSettings::default(), vec![file(r"C:\owned\a.txt")]);
    draft.password = "secret".into();
    draft.confirm = "secret".into();
    draft.solid = true;
    draft.encrypt_header = true;
    draft.set_format(ArchiveCreateFormat::Tar);
    assert!(draft.password.is_empty() && draft.confirm.is_empty());
    assert!(!draft.solid && !draft.encrypt_header);
    assert!(draft.options().expect("TAR").level.is_none());
    assert!(draft.options().expect("TAR").method.is_none());
    draft.set_format(ArchiveCreateFormat::Gz);
    draft.level = ArchiveCompressionLevel::Store;
    assert!(draft.options().is_err());
}
#[test]
fn single_stream_never_drops_multisource_or_accepts_a_directory() {
    let mut draft = CreateDraft::new(
        &ArchiveSettings::default(),
        vec![file(r"C:\owned\a.txt"), file(r"C:\owned\b.txt")],
    );
    draft.set_format(ArchiveCreateFormat::Xz);
    assert!(draft.options().is_err());
    draft.sources.truncate(1);
    draft.sources[0].directory = true;
    assert!(draft.options().is_err());
    draft.sources[0].directory = false;
    assert!(draft.options().is_ok());
}
#[test]
fn secret_confirmation_exclusions_name_and_native_source_bounds_are_real_validation() {
    let mut draft = CreateDraft::new(&ArchiveSettings::default(), vec![file(r"C:\owned\a.txt")]);
    draft.password = "one".into();
    draft.confirm = "two".into();
    assert!(draft.options().is_err());
    draft.confirm = "one".into();
    draft.exclusions = vec!["../bad".into()];
    assert!(draft.options().is_err());
    draft.exclusions.clear();
    draft.name = "../escape.zip".into();
    assert!(draft.options().is_err());
    draft.name = "safe.zip".into();
    draft.sources = (0..33)
        .map(|n| file(&format!(r"C:\owned\{n}.txt")))
        .collect();
    assert!(draft.options().is_err());
}
#[test]
fn compound_stem_and_default_extract_strategy_preserve_original_options() {
    assert_eq!(archive_stem("Photos.tar.gz"), "Photos");
    assert_eq!(archive_stem("Photos.tar.xz"), "Photos");
    let settings = ArchiveSettings {
        default_destination: ArchiveDestination::Smart,
        conflict_policy: ArchiveConflictPolicy::Skip,
        copy_motw: false,
        ..Default::default()
    };
    let draft = ExtractDraft::new(&settings, r"C:\owned\Photos.tar.gz".into(), None);
    assert_eq!(draft.strategy, ArchiveDestination::Smart);
    assert_eq!(draft.conflict, ArchiveConflictPolicy::Skip);
    assert!(!draft.copy_motw);
    assert!(!draft.delete_source);
}

#[test]
fn separately_streams_keep_every_input_and_omit_hidden_directory_options() {
    let settings = ArchiveSettings::default();
    let mut draft = CreateDraft::new(
        &settings,
        vec![file(r"C:\owned\a.txt"), file(r"C:\owned\b.txt")],
    );
    draft.set_format(ArchiveCreateFormat::Gz);
    assert!(draft.requests(&settings).is_err());
    draft.separately = true;
    draft.strip_single_root = true;
    draft.exclusions = vec!["../hidden-invalid".into()];
    let requests = draft
        .requests(&settings)
        .expect("two distinct stream requests");
    assert_eq!(requests.len(), 2);
    for (index, request) in requests.iter().enumerate() {
        assert_eq!(
            request.paths,
            vec![draft.sources[index].path.to_str().expect("Unicode source")]
        );
        let options = request.create.as_ref().expect("create options");
        assert!(options.exclude_patterns.is_empty() && !options.strip_single_root);
        assert!(
            options.method.is_none() && options.solid.is_none() && options.encrypt_header.is_none()
        );
        request.validate().expect("exact public protocol");
    }
    draft.sources[1].path = r"C:\other\a.txt".into();
    let duplicate_outputs = draft
        .requests(&settings)
        .expect("planner resolves duplicate outputs");
    assert_eq!(duplicate_outputs.len(), 2);
    assert_eq!(
        duplicate_outputs[0].create.as_ref().expect("first").name,
        duplicate_outputs[1].create.as_ref().expect("second").name
    );
    draft.sources[1].directory = true;
    assert!(
        draft.requests(&settings).is_err(),
        "separate streams cannot serialize directories"
    );
}
#[test]
fn create_request_uses_exact_semantic_capabilities_and_native_names() {
    let settings = ArchiveSettings::default();
    let mut draft = CreateDraft::new(&settings, vec![file(r"C:\owned\a.txt")]);
    draft.level = ArchiveCompressionLevel::Store;
    let request = draft.requests(&settings).expect("stored ZIP").remove(0);
    let options = request.create.expect("ZIP options");
    assert_eq!(options.level, Some(ArchiveJobLevel::Store));
    assert_eq!(options.method, Some(ArchiveJobMethod::Store));
    assert!(
        options.password.is_none() && options.solid.is_none() && options.encrypt_header.is_none()
    );
    for name in ["NUL.zip", "bad?.zip", "trailing.zip ", "wrong.7z"] {
        draft.name = name.into();
        assert!(draft.requests(&settings).is_err(), "invalid name: {name}");
    }
    draft.name = "safe.zip".into();
    draft.destination = "relative".into();
    assert!(draft.requests(&settings).is_err());
}
#[test]
fn extract_request_preserves_selection_binding_strategy_conflict_and_no_delete_authority() {
    use nxtools_protocol::archive_browser::ArchiveSourceBinding;
    let settings = ArchiveSettings {
        filename_encoding: ArchiveFilenameEncoding::Gbk,
        ..Default::default()
    };
    let selection = DraftSelection {
        browser: ArchiveBrowserContext {
            snapshot_id: uuid::Uuid::new_v4(),
            format: "Zip".into(),
            comment: String::new(),
            source: ArchiveSourceBinding {
                native: "v1:fixture".into(),
                digest: [7; 32],
                size: 12,
                modified_unix_ns: 1,
            },
            sevenz_edit: None,
            tar_edit: None,
        },
        ordinals: vec![2, 7],
    };
    let mut draft = ExtractDraft::new(
        &settings,
        r"C:\owned\Photos.tar.gz".into(),
        Some(selection.clone()),
    );
    draft.strategy = ArchiveDestination::Name;
    draft.single_file_folder = true;
    draft.conflict = ArchiveConflictPolicy::AutoRename;
    let request = draft.request(&settings).expect("selected request");
    assert_eq!(request.codepage, 936);
    let extract = request.extract.expect("extract");
    assert_eq!(extract.strategy, ArchiveJobStrategy::Name);
    assert!(
        !extract.single_file_folder,
        "hidden smart-only option cannot leak"
    );
    let selected = extract.selection.expect("selected binding");
    assert_eq!(selected.browser, selection.browser);
    assert_eq!(selected.ordinals, selection.ordinals);
    draft.selected_only = false;
    assert!(
        draft
            .request(&settings)
            .expect("all")
            .extract
            .expect("extract")
            .selection
            .is_none()
    );
    draft.delete_source = true;
    assert!(draft.request(&settings).is_err());
}
#[test]
fn folder_name_keeps_dots_and_duplicate_roots_are_not_silently_lost() {
    let settings = ArchiveSettings::default();
    let mut folder = file(r"C:\owned\Photos.2026");
    folder.directory = true;
    folder.bytes = None;
    let mut draft = CreateDraft::new(&settings, vec![folder]);
    assert_eq!(draft.name, "Photos.2026.zip");
    draft.sources = vec![file(r"C:\first\a.txt"), file(r"C:\second\a.txt")];
    assert!(draft.requests(&settings).is_err());
}
#[test]
fn split_create_draft_maps_checked_mib_to_wire_and_rejects_other_formats() {
    let settings = ArchiveSettings::default();
    let mut draft = CreateDraft::new(&settings, vec![file(r"C:\owned\a.txt")]);
    assert_eq!(
        draft.requests(&settings).expect("legacy")[0]
            .create
            .as_ref()
            .expect("create")
            .volume_bytes,
        None
    );
    draft.volume_mib = "128".into();
    assert_eq!(
        draft.requests(&settings).expect("split")[0]
            .create
            .as_ref()
            .expect("create")
            .volume_bytes,
        Some(128 << 20)
    );
    draft.volume_mib = u64::MAX.to_string();
    assert!(draft.requests(&settings).is_err());
    draft.volume_mib = "0".into();
    assert!(draft.requests(&settings).is_err());
    draft.volume_mib = "1.5".into();
    assert!(draft.requests(&settings).is_err());
    draft.volume_mib = "64".into();
    draft.set_format(ArchiveCreateFormat::Tar);
    assert_eq!(draft.volume_mib, "64");
    assert!(draft.requests(&settings).is_err());
    draft.volume_mib.clear();
    assert!(draft.requests(&settings).is_ok());
}
#[test]
fn immutable_modify_draft_maps_all_actions_to_same_job_contract() {
    use nxtools_protocol::archive_browser::ArchiveSourceBinding;
    let mut draft = ModifyDraft {
        source: r"C:\owned\a.zip".into(),
        browser: ArchiveBrowserContext {
            snapshot_id: uuid::Uuid::new_v4(),
            format: "Zip".into(),
            comment: "old".into(),
            source: ArchiveSourceBinding {
                native: "v1:fixture".into(),
                digest: [1; 32],
                size: 7,
                modified_unix_ns: 4,
            },
            sevenz_edit: None,
            tar_edit: None,
        },
        action: ArchiveJobModifyAction::Delete,
        ordinals: vec![2],
        value: None,
        source_prefix: None,
        added: vec![],
        codepage: 65001,
    };
    let request = draft.request().expect("delete");
    assert_eq!(request.kind, ArchiveJobKind::Modify);
    assert_eq!(
        request.modify.as_ref().expect("edit").browser,
        draft.browser
    );
    draft.action = ArchiveJobModifyAction::Rename;
    draft.value = Some("folder/new.txt".into());
    assert!(draft.request().is_ok());
    draft.action = ArchiveJobModifyAction::NewDirectory;
    draft.ordinals.clear();
    draft.value = Some("folder/new/".into());
    assert!(draft.request().is_ok());
    draft.action = ArchiveJobModifyAction::Comment;
    draft.value = Some("UTF-8 注释".into());
    assert!(draft.request().is_ok());
    draft.action = ArchiveJobModifyAction::Add;
    draft.value = Some("folder/".into());
    draft.added.push(r"C:\owned\source.txt".into());
    assert_eq!(draft.request().expect("add").paths.len(), 2);
    draft.browser.format = "SevenZ".into();
    assert!(draft.request().is_err());
}

#[test]
fn hz5_reclamation_defaults_off_and_maps_explicit_start_and_continue_intents() {
    let settings = ArchiveSettings {
        default_destination: ArchiveDestination::Here,
        conflict_policy: ArchiveConflictPolicy::Ask,
        ..Default::default()
    };
    let mut start = ExtractDraft::new(&settings, r"C:\owned\stored.zip".into(), None);
    assert!(!start.reclamation && !start.continue_reclamation);
    assert!(
        start
            .request(&settings)
            .expect("ordinary extract")
            .extract
            .expect("extract")
            .reclamation
            .is_none()
    );
    start.reclamation = true;
    let start_id = Uuid::parse_str(&start.recovery_id).expect("stable generated recovery UUID");
    assert_eq!(
        start
            .request(&settings)
            .expect("explicit reclamation start")
            .extract
            .expect("extract")
            .reclamation,
        Some(ArchiveReclamationIntent {
            mode: ArchiveReclamationMode::Start,
            recovery_id: start_id,
        })
    );
    start.strategy = ArchiveDestination::Name;
    assert!(
        start
            .request(&settings)
            .expect_err("unsupported strategy must remain visible")
            .contains("当前文件夹")
    );

    let recovery_id = Uuid::new_v4();
    let mut continuation =
        ExtractDraft::continue_reclamation(&settings, r"C:\owned\stored.zip".into());
    continuation.recovery_id = recovery_id.to_string();
    let request = continuation
        .request(&settings)
        .expect("fresh-process continuation");
    assert_eq!(request.paths, vec![r"C:\owned\stored.zip"]);
    assert_eq!(request.destination.as_deref(), Some(r"C:\owned"));
    assert_eq!(
        request.extract.expect("extract").reclamation,
        Some(ArchiveReclamationIntent {
            mode: ArchiveReclamationMode::Continue,
            recovery_id,
        })
    );
}
