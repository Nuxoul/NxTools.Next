//! Offline authoring validator: parses/stages nothing and never executes plugin code.
use std::io::Read;
use std::path::Path;
fn main() {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("Usage: validate_package <extension.json|package-directory|package.zip>");
        std::process::exit(2);
    };
    let path = Path::new(&path);
    let result = if path.is_dir() {
        nxtools_extensions::ImportPreview::from_directory(path)
    } else if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        nxtools_extensions::ImportPreview::from_archive(path)
    } else {
        read_manifest(path).and_then(|bytes| nxtools_extensions::ImportPreview::from_json(&bytes))
    };
    match result {
        Ok(preview) => println!(
            "VALID schema=1 handlers={} files={} (not installed, trusted or executed)",
            preview.manifest.handlers.len(),
            preview.code_files.len()
        ),
        Err(error) => {
            eprintln!("INVALID: {error}");
            std::process::exit(1);
        }
    }
}

fn read_manifest(path: &Path) -> Result<Vec<u8>, nxtools_extensions::PackageError> {
    let file =
        std::fs::File::open(path).map_err(|_| nxtools_extensions::PackageError::FileOperation)?;
    if !file
        .metadata()
        .map_err(|_| nxtools_extensions::PackageError::FileOperation)?
        .is_file()
    {
        return Err(nxtools_extensions::PackageError::FileOperation);
    }
    let mut bytes = Vec::new();
    file.take(nxtools_extensions::MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| nxtools_extensions::PackageError::FileOperation)?;
    if bytes.len() > nxtools_extensions::MAX_MANIFEST_BYTES {
        return Err(nxtools_extensions::PackageError::ManifestTooLarge);
    }
    Ok(bytes)
}
