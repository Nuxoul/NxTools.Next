//! Actual import/stage boundary for bundled examples; never executes plugins or opens config.
use nxtools_extensions::{
    DeclarativeAction, ImportPreview, PackageError, PackageRegistry, ScriptProgram, SettingType,
};
use std::{fs, io::Write, path::Path};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const EXAMPLES: [(&str, &str, &[&str]); 3] = [
    ("github-public-proxy", "downloads", &["resolve"]),
    (
        "download-link-list",
        "downloads",
        &["copy-lines", "copy-markdown"],
    ),
    (
        "clipboard-text-tools",
        "clipboard",
        &["unique-lines", "pretty-json", "uri-decode"],
    ),
];

fn archive(path: &Path, manifest: &[u8], preview: &ImportPreview) {
    let mut writer = ZipWriter::new(fs::File::create(path).expect("owned ZIP file"));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    writer
        .start_file("extension.json", options)
        .expect("manifest entry");
    writer.write_all(manifest).expect("manifest bytes");
    for (name, bytes) in &preview.code_files {
        writer.start_file(name, options).expect("declared entry");
        writer.write_all(bytes).expect("declared bytes");
    }
    writer.finish().expect("finished owned ZIP");
}

#[test]
fn bundled_examples_preview_archive_and_immutable_disabled_untrusted_stage() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("docs/extensions/examples");
    let temp = tempfile::tempdir().expect("owned fixtures only");
    let staging = temp.path().join("staged");
    let mut registry = PackageRegistry::default();
    for (name, feature, handler_ids) in EXAMPLES {
        let source = examples.join(name);
        let preview = ImportPreview::from_directory(&source).expect("real example directory");
        let manifest = &preview.manifest;
        let package_id = format!("example.{name}");
        assert_eq!(manifest.id, package_id);
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(
            manifest.runtime.as_ref().expect("Python runtime").program,
            ScriptProgram::Python
        );
        assert_eq!(
            manifest.files,
            ["main.py", "protocol.py", "test_main.py", "README.md"]
        );
        assert_eq!(preview.code_files.len(), 4);
        assert!(
            preview.code_directory.is_none(),
            "preview performs no stage"
        );
        assert_eq!(
            manifest
                .handlers
                .iter()
                .map(|handler| handler.id.as_str())
                .collect::<Vec<_>>(),
            handler_ids
        );
        for handler in &manifest.handlers {
            assert_eq!(handler.feature, feature);
            assert_eq!(
                handler.entry,
                if name == "github-public-proxy" {
                    "resolve"
                } else {
                    "action"
                }
            );
            assert_eq!(
                handler.action,
                DeclarativeAction::Script {
                    entry_file: "main.py".into()
                }
            );
        }
        if name == "github-public-proxy" {
            assert_eq!(manifest.settings.len(), 2);
            assert_eq!(manifest.settings[0].key, "third_party_opt_in");
            assert_eq!(
                manifest.settings[0].field,
                SettingType::Bool { default: false }
            );
            assert_eq!(manifest.settings[1].key, "proxy_prefix");
            assert_eq!(
                manifest.settings[1].field,
                SettingType::Text {
                    default: "https://ghproxy.net/".into()
                }
            );
            let matches = &manifest.handlers[0].url_matches;
            assert_eq!(matches.len(), 3);
            assert_eq!(
                matches
                    .iter()
                    .map(|rule| (rule.host.as_str(), rule.path.as_str()))
                    .collect::<Vec<_>>(),
                [
                    ("github.com", "/*/*/releases/download/*"),
                    ("github.com", "/*/*/archive/*"),
                    ("raw.githubusercontent.com", "/*/*/*/*"),
                ]
            );
            assert!(matches.iter().all(|rule| !rule.include_subdomains));
        } else {
            assert!(manifest.settings.is_empty());
            assert!(
                manifest
                    .handlers
                    .iter()
                    .all(|handler| handler.url_matches.is_empty())
            );
        }
        let zip_path = temp.path().join(format!("{name}.zip"));
        archive(
            &zip_path,
            &fs::read(source.join("extension.json")).expect("example declaration"),
            &preview,
        );
        let zipped = ImportPreview::from_archive(&zip_path).expect("real ZIP import");
        assert_eq!(zipped.manifest, preview.manifest);
        assert_eq!(zipped.code_files, preview.code_files);
        assert_eq!(zipped.content_hash, preview.content_hash);
        assert!(
            !staging.join(&package_id).exists(),
            "preview must not stage"
        );
        assert_eq!(
            registry.install(preview.clone(), "fixture".into(), false),
            Err(PackageError::CodeRequired),
            "mutable source must not install directly"
        );
        let staged = zipped
            .stage_into(&staging)
            .expect("immutable content snapshot");
        let staged_directory = staged.code_directory.as_ref().expect("staged path");
        assert!(staged_directory.starts_with(fs::canonicalize(&staging).expect("stage root")));
        assert_eq!(
            staged_directory.file_name().expect("content address"),
            staged.content_hash.as_str()
        );
        // Delete owned source archive after preview; the installed package must
        // still contain/verify the snapshotted bytes, not consult the source ZIP.
        fs::remove_file(&zip_path).expect("remove owned ZIP source");
        for (relative, bytes) in &preview.code_files {
            assert_eq!(
                fs::read(staged_directory.join(relative)).expect("staged file"),
                *bytes
            );
        }
        let restaged = zipped
            .stage_into(&staging)
            .expect("same-content stage is stable");
        assert_eq!(restaged.code_directory, staged.code_directory);
        registry
            .install(staged, "owned fixture ZIP".into(), false)
            .expect("disabled install");
        let installed = registry.packages.last().expect("installed example");
        assert!(!installed.enabled);
        assert!(installed.trust.is_none());
        assert!(installed.enabled_events.is_empty());
        assert!(installed.enabled_providers.is_empty());
        installed
            .verify_current_code()
            .expect("immutable stage verifies");
        if name == "github-public-proxy" {
            assert_eq!(installed.settings.len(), 2);
            assert_eq!(
                installed.settings.get("third_party_opt_in"),
                Some(&serde_json::json!(false))
            );
            assert_eq!(
                installed.settings.get("proxy_prefix"),
                Some(&serde_json::json!("https://ghproxy.net/"))
            );
        } else {
            assert!(installed.settings.is_empty());
        }
        assert_eq!(
            registry.set_enabled(&package_id, true, &[]),
            Err(PackageError::Untrusted)
        );
    }
    assert_eq!(registry.packages.len(), 3);
    registry.validate().expect("isolated in-memory registry");
}

#[test]
fn production_embedded_packages_match_all_manifest_listed_sources() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("docs/extensions/examples");
    let previews = nxtools_extensions::bundled_previews().expect("production compiled resources");
    assert_eq!(previews.len(), 3);
    for preview in previews {
        let name = preview
            .manifest
            .id
            .strip_prefix("example.")
            .expect("bundled stable id");
        let source = ImportPreview::from_directory(&examples.join(name)).expect("source package");
        assert_eq!(preview.manifest, source.manifest);
        assert_eq!(preview.content_hash, source.content_hash);
        assert_eq!(preview.code_files, source.code_files);
        assert!(preview.code_directory.is_none());
    }
}
