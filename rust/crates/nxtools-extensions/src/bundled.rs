//! Production-owned bundled sources, embedded in the signed host executable.
//! Inspection and staging never run an interpreter or approve/enable a package.
use crate::{ImportPreview, PackageError, bundle};
use std::collections::BTreeMap;

type EmbeddedPackage = (
    &'static str,
    &'static [u8],
    [(&'static str, &'static [u8]); 4],
);

macro_rules! package {
    ($name:literal) => {
        (
            concat!("example.", $name),
            include_bytes!(concat!(
                "../../../../docs/extensions/examples/",
                $name,
                "/extension.json"
            ))
            .as_slice(),
            [
                (
                    "main.py",
                    include_bytes!(concat!(
                        "../../../../docs/extensions/examples/",
                        $name,
                        "/main.py"
                    ))
                    .as_slice(),
                ),
                (
                    "protocol.py",
                    include_bytes!(concat!(
                        "../../../../docs/extensions/examples/",
                        $name,
                        "/protocol.py"
                    ))
                    .as_slice(),
                ),
                (
                    "test_main.py",
                    include_bytes!(concat!(
                        "../../../../docs/extensions/examples/",
                        $name,
                        "/test_main.py"
                    ))
                    .as_slice(),
                ),
                (
                    "README.md",
                    include_bytes!(concat!(
                        "../../../../docs/extensions/examples/",
                        $name,
                        "/README.md"
                    ))
                    .as_slice(),
                ),
            ],
        )
    };
}

const PACKAGES: [EmbeddedPackage; 3] = [
    package!("github-public-proxy"),
    package!("download-link-list"),
    package!("clipboard-text-tools"),
];

/// Exact compiled sources, not paths under the development checkout or an online catalog.
/// Callers still use the normal immutable stage and disabled/untrusted registry import.
pub fn bundled_previews() -> Result<Vec<ImportPreview>, PackageError> {
    PACKAGES
        .iter()
        .map(|(id, manifest_bytes, files)| {
            let manifest = crate::strict_json::manifest(manifest_bytes)?;
            manifest.validate()?;
            let code_files = files
                .iter()
                .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
                .collect::<BTreeMap<_, _>>();
            if manifest.id != *id
                || manifest.files.len() != code_files.len()
                || manifest
                    .files
                    .iter()
                    .any(|name| !code_files.contains_key(name))
            {
                return Err(PackageError::InvalidManifest);
            }
            let preview = ImportPreview {
                content_hash: bundle::combined_hash(&manifest, &bundle::file_hashes(&code_files))?,
                manifest,
                code_files,
                code_directory: None,
                staged: false,
            };
            preview.validate_preview()?;
            Ok(preview)
        })
        .collect()
}
