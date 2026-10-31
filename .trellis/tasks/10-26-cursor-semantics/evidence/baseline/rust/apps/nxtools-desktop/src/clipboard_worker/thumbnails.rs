//! Disposable list derivatives. The serialized history worker owns all disk access.
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use image::{ImageDecoder, ImageFormat, ImageReader, Limits};

pub(super) const THUMBNAIL_WIDTH: u32 = 640;
pub(super) const THUMBNAIL_HEIGHT: u32 = 288;
const CACHE_DIRECTORY: &str = "list-thumbnails-v1";
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PIXELS: u64 = 32 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_CACHE_FILES: usize = 512;

fn regular_metadata(path: &Path) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "thumbnail reparse rejected"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "thumbnail reparse rejected"
        );
    }
    Ok(metadata)
}
fn managed_file_name(original: &Path) -> Option<&std::ffi::OsStr> {
    let stem = original.file_stem()?.to_str()?;
    (original.extension()? == "png"
        && stem.len() == 64
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    .then(|| original.file_name())
    .flatten()
}
fn valid_cached_image(path: &Path) -> bool {
    if !regular_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= 2 * 1024 * 1024)
    {
        return false;
    }
    let validate = || -> Result<()> {
        let mut reader =
            ImageReader::with_format(BufReader::new(File::open(path)?), ImageFormat::Png);
        let mut limits = Limits::default();
        limits.max_image_width = Some(THUMBNAIL_WIDTH);
        limits.max_image_height = Some(THUMBNAIL_HEIGHT);
        limits.max_alloc = Some(4 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode()?;
        ensure!(
            image.width() > 0 && image.height() > 0,
            "empty cached image"
        );
        Ok(())
    };
    validate().is_ok()
}

pub(super) fn ensure_thumbnail(asset_root: &Path, original: &Path) -> Result<PathBuf> {
    let name = managed_file_name(original).context("thumbnail source is not content addressed")?;
    let root = asset_root.canonicalize()?;
    let metadata = regular_metadata(original)?;
    ensure!(
        metadata.is_file() && metadata.len() <= MAX_SOURCE_BYTES,
        "thumbnail source budget"
    );
    ensure!(
        original.canonicalize()?.parent() == Some(root.as_path()),
        "thumbnail source outside managed root"
    );
    let cache = root.join(CACHE_DIRECTORY);
    if cache.exists() {
        ensure!(
            regular_metadata(&cache)?.is_dir(),
            "thumbnail cache is not a directory"
        );
    }
    let destination = cache.join(name);
    if valid_cached_image(&destination) {
        return Ok(destination);
    }

    let input = File::open(original)?;
    let mut decoder =
        ImageReader::with_format(BufReader::new(input), ImageFormat::Png).into_decoder()?;
    let (width, height) = decoder.dimensions();
    ensure!(
        width > 0 && height > 0 && u64::from(width) * u64::from(height) <= MAX_PIXELS,
        "thumbnail pixel budget"
    );
    let mut limits = Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(192 * 1024 * 1024);
    decoder.set_limits(limits)?;
    let decoded = image::DynamicImage::from_decoder(decoder)?;
    let thumbnail = if width <= THUMBNAIL_WIDTH && height <= THUMBNAIL_HEIGHT {
        decoded
    } else {
        decoded.thumbnail(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT)
    };
    fs::create_dir_all(&cache)?;
    ensure!(
        regular_metadata(&cache)?.is_dir(),
        "thumbnail cache changed"
    );
    prune_cache(&cache)?;
    if destination.exists() {
        ensure!(
            regular_metadata(&destination)?.is_file(),
            "thumbnail destination changed"
        );
        fs::remove_file(&destination)?;
    }
    let temporary = cache.join(format!(".thumbnail-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let mut writer = BufWriter::new(output);
        thumbnail.write_to(&mut writer, ImageFormat::Png)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(destination)
}

fn prune_cache(cache: &Path) -> Result<()> {
    let mut files = Vec::new();
    let mut total = 0_u64;
    for entry in fs::read_dir(cache)? {
        let entry = entry?;
        let path = entry.path();
        if managed_file_name(&path).is_none() {
            continue;
        }
        let Ok(metadata) = regular_metadata(&path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        total = total.saturating_add(metadata.len());
        files.push((
            metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
            path,
            metadata.len(),
        ));
    }
    files.sort_by_key(|(modified, _, _)| *modified);
    let mut remaining = files.len();
    for (_, path, size) in files {
        // Reserve one maximum-sized derivative before creating the next file.
        if remaining < MAX_CACHE_FILES && total <= MAX_CACHE_BYTES - 2 * 1024 * 1024 {
            break;
        }
        fs::remove_file(path)?;
        remaining -= 1;
        total = total.saturating_sub(size);
    }
    Ok(())
}

pub(super) fn remove_thumbnail(asset_root: &Path, original: &Path) {
    if let Some(file_name) = managed_file_name(original) {
        let cache = asset_root.join(CACHE_DIRECTORY);
        if regular_metadata(&cache).is_ok_and(|metadata| metadata.is_dir()) {
            let _ = fs::remove_file(cache.join(file_name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn original(root: &Path, width: u32, height: u32) -> PathBuf {
        let path = root.join(format!("{}.png", "a".repeat(64)));
        RgbaImage::from_pixel(width, height, Rgba([19, 71, 133, 96]))
            .save_with_format(&path, ImageFormat::Png)
            .expect("save synthetic original");
        path
    }
    #[test]
    fn list_thumbnail_bounds_alpha_and_original_bytes_are_independent() {
        let directory = tempfile::tempdir().expect("temporary derivative store");
        let path = original(directory.path(), 3840, 2160);
        let before = fs::read(&path).expect("original bytes");
        let thumbnail = ensure_thumbnail(directory.path(), &path).expect("derive thumbnail");
        assert_ne!(thumbnail, path, "list must not decode the 4K original");
        let decoded = image::open(&thumbnail)
            .expect("decode derivative")
            .to_rgba8();
        assert_eq!(decoded.dimensions(), (512, 288));
        assert_eq!(decoded.get_pixel(100, 100).0, [19, 71, 133, 96]);
        assert_eq!(fs::read(&path).expect("original remains"), before);
        println!(
            "thumbnail decoded RGBA bytes: original={} derivative={}",
            3840_u64 * 2160 * 4,
            decoded.as_raw().len()
        );
    }
    #[test]
    fn thumbnail_cache_hit_is_stable_and_corrupt_derivative_is_rebuilt() {
        let directory = tempfile::tempdir().expect("temporary cache");
        let path = original(directory.path(), 120, 80);
        let thumbnail = ensure_thumbnail(directory.path(), &path).expect("first derivative");
        assert_ne!(thumbnail, path);
        assert_eq!(
            image::image_dimensions(&thumbnail).expect("small dimensions"),
            (120, 80)
        );
        let bytes = fs::read(&thumbnail).expect("thumbnail bytes");
        assert_eq!(
            ensure_thumbnail(directory.path(), &path).expect("hit"),
            thumbnail
        );
        assert_eq!(fs::read(&thumbnail).expect("unchanged hit"), bytes);
        fs::write(&thumbnail, b"broken").expect("corrupt disposable cache");
        ensure_thumbnail(directory.path(), &path).expect("repair corrupt derivative");
        assert_eq!(
            image::image_dimensions(&thumbnail).expect("repaired dimensions"),
            (120, 80)
        );
        let truncated = fs::read(&thumbnail).expect("PNG bytes");
        let pixel_header = truncated
            .windows(4)
            .position(|bytes| bytes == b"IDAT")
            .expect("encoded pixel chunk")
            + 4;
        fs::write(&thumbnail, &truncated[..pixel_header]).expect("retain valid PNG header only");
        assert!(
            image::image_dimensions(&thumbnail).is_ok(),
            "header probe alone misses corruption"
        );
        ensure_thumbnail(directory.path(), &path).expect("repair truncated pixel stream");
        assert!(
            image::open(&thumbnail).is_ok(),
            "list receives a fully decodable derivative"
        );
        remove_thumbnail(directory.path(), &path);
        assert!(!thumbnail.exists());
        assert!(
            path.is_file(),
            "derivative cleanup must never delete original"
        );
    }
    #[test]
    fn thumbnail_rejects_unmanaged_paths_and_bad_encoded_images_without_writes() {
        let directory = tempfile::tempdir().expect("managed root");
        let external = tempfile::tempdir().expect("outside root");
        let path = original(external.path(), 8, 8);
        assert!(ensure_thumbnail(directory.path(), &path).is_err());
        let bad = directory.path().join(format!("{}.png", "b".repeat(64)));
        fs::write(&bad, b"not a PNG").expect("bad synthetic image");
        assert!(ensure_thumbnail(directory.path(), &bad).is_err());
        assert!(!directory.path().join(CACHE_DIRECTORY).exists());
    }
    #[test]
    fn thumbnail_pruning_and_managed_deletion_never_remove_originals() {
        let directory = tempfile::tempdir().expect("bounded cache");
        let path = original(directory.path(), 8, 8);
        let thumbnail = ensure_thumbnail(directory.path(), &path).expect("initial derivative");
        let cache = thumbnail.parent().expect("cache root");
        let png = fs::read(&thumbnail).expect("synthetic PNG");
        for index in 0..MAX_CACHE_FILES + 8 {
            fs::write(cache.join(format!("{index:064x}.png")), &png)
                .expect("synthetic cache entry");
        }
        prune_cache(cache).expect("prune cache only");
        assert!(fs::read_dir(cache).expect("remaining cache").count() < MAX_CACHE_FILES);
        assert!(path.is_file(), "pruning never crosses original root");
        let thumbnail = ensure_thumbnail(directory.path(), &path).expect("lazy regeneration");
        assert!(super::super::remove_managed_asset(
            directory.path(),
            path.to_str().expect("fixture path")
        ));
        assert!(
            !path.exists() && !thumbnail.exists(),
            "real managed purge removes both representations"
        );
    }
}
