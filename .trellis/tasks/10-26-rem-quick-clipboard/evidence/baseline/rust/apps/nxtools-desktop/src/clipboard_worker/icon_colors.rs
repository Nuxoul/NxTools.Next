//! Disposable source-icon palette metadata; all IO/decoding runs on the serialized worker.
use anyhow::{Result, ensure};
use image::{ImageFormat, ImageReader, Limits, RgbaImage};
use std::{
    collections::VecDeque,
    fs,
    io::{BufReader, Read as _},
    path::{Path, PathBuf},
};

const CACHE_CAPACITY: usize = 128;
const MAX_BYTES: u64 = 1024 * 1024;
const MAX_DIMENSION: u32 = 512;

#[derive(Default)]
pub(super) struct IconColorCache {
    entries: VecDeque<(PathBuf, Option<[u8; 3]>)>,
    root: Option<PathBuf>,
}
impl IconColorCache {
    pub(super) fn get(&mut self, root: &Path, path: &Path) -> (Option<[u8; 3]>, bool) {
        if self.root.as_deref() != Some(root) {
            self.entries.clear();
            self.root = Some(root.to_owned());
        }
        if let Some(index) = self.entries.iter().position(|(cached, _)| cached == path) {
            let entry = self.entries.remove(index).expect("located cached icon");
            let color = entry.1;
            self.entries.push_back(entry);
            return (color, true);
        }
        let color = read_icon_color(root, path).ok().flatten();
        if self.entries.len() == CACHE_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back((path.to_owned(), color));
        (color, false)
    }
}
fn regular(path: &Path) -> Result<fs::Metadata> {
    let meta = fs::symlink_metadata(path)?;
    ensure!(!meta.file_type().is_symlink(), "icon reparse rejected");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        ensure!(meta.file_attributes() & 0x400 == 0, "icon reparse rejected");
    }
    Ok(meta)
}
fn read_icon_color(root: &Path, path: &Path) -> Result<Option<[u8; 3]>> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let digest = stem.strip_prefix("source-").unwrap_or("");
    ensure!(
        path.extension().is_some_and(|ext| ext == "png")
            && digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "not a managed source icon"
    );
    ensure!(regular(root)?.is_dir(), "invalid icon root");
    let meta = regular(path)?;
    ensure!(
        meta.is_file() && meta.len() <= MAX_BYTES,
        "icon byte budget"
    );
    ensure!(
        path.canonicalize()?.parent() == Some(root.canonicalize()?.as_path()),
        "icon outside managed root"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= MAX_BYTES, "icon byte budget");
    let mut reader = ImageReader::with_format(
        BufReader::new(std::io::Cursor::new(bytes)),
        ImageFormat::Png,
    );
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(4 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?;
    // Enlarging small icons blends neutral edges into the original logo hue.
    let image = if image.width() > 64 || image.height() > 64 {
        image.thumbnail(64, 64)
    } else {
        image
    }
    .to_rgba8();
    Ok(dominant_color(&image))
}
#[derive(Clone, Copy, Default)]
struct Bucket {
    weight: u64,
    rgb: [u64; 3],
}
fn dominant_color(image: &RgbaImage) -> Option<[u8; 3]> {
    let mut buckets = vec![Bucket::default(); 4096];
    let mut chromatic = [Bucket::default(); 12];
    let mut total = 0_u64;
    let mut colored = 0_u64;
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 32 {
            continue;
        }
        let weight = u64::from(a);
        let key =
            ((usize::from(r) >> 4) << 8) | ((usize::from(g) >> 4) << 4) | (usize::from(b) >> 4);
        let add = |bucket: &mut Bucket| {
            bucket.weight += weight;
            for (sum, value) in bucket.rgb.iter_mut().zip([r, g, b]) {
                *sum += u64::from(value) * weight;
            }
        };
        add(&mut buckets[key]);
        total += weight;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        if max.saturating_sub(min) >= 40 && max > 48 {
            let delta = f32::from(max - min);
            let hue = if max == r {
                ((f32::from(g) - f32::from(b)) / delta).rem_euclid(6.0)
            } else if max == g {
                (f32::from(b) - f32::from(r)) / delta + 2.0
            } else {
                (f32::from(r) - f32::from(g)) / delta + 4.0
            };
            add(&mut chromatic[((hue * 2.0).floor() as usize).min(11)]);
            colored += weight;
        }
    }
    if total == 0 {
        return None;
    }
    // Merge neighboring hues so shaded logo pixels beat a flat minority accent.
    let bucket = if colored * 20 >= total {
        (0..12)
            .map(|center| {
                let mut merged = Bucket::default();
                for index in [(center + 11) % 12, center, (center + 1) % 12] {
                    merged.weight += chromatic[index].weight;
                    for (sum, value) in merged.rgb.iter_mut().zip(chromatic[index].rgb) {
                        *sum += value;
                    }
                }
                merged
            })
            .max_by_key(|bucket| bucket.weight)?
    } else {
        *buckets.iter().max_by_key(|bucket| bucket.weight)?
    };
    (bucket.weight > 0).then(|| bucket.rgb.map(|sum| (sum / bucket.weight) as u8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    fn save(root: &Path, key: usize, image: &RgbaImage) -> PathBuf {
        let path = root.join(format!("source-{key:064x}.png"));
        image.save(&path).expect("synthetic source icon");
        path
    }
    #[test]
    fn icon_palette_uses_actual_pixels_ignores_transparency_and_neutral_backdrop() {
        let dir = tempfile::tempdir().expect("icon fixture");
        let mut cache = IconColorCache::default();
        for (key, rgb) in [[20, 180, 70], [30, 75, 220], [190, 45, 35]]
            .into_iter()
            .enumerate()
        {
            let path = save(
                dir.path(),
                key,
                &RgbaImage::from_pixel(32, 32, Rgba([rgb[0], rgb[1], rgb[2], 255])),
            );
            assert_eq!(cache.get(dir.path(), &path), (Some(rgb), false));
        }
        let mut image = RgbaImage::from_pixel(32, 32, Rgba([255, 255, 255, 255]));
        for y in 8..24 {
            for x in 8..24 {
                image.put_pixel(x, y, Rgba([10, 170, 60, 255]));
            }
        }
        let path = save(dir.path(), 4, &image);
        assert_eq!(cache.get(dir.path(), &path).0, Some([10, 170, 60]));
        let mut image = RgbaImage::from_pixel(32, 32, Rgba([220, 10, 10, 0]));
        for y in 8..24 {
            for x in 8..24 {
                image.put_pixel(x, y, Rgba([25, 60, 210, 255]));
            }
        }
        let path = save(dir.path(), 5, &image);
        assert_eq!(cache.get(dir.path(), &path).0, Some([25, 60, 210]));
        let gray = save(
            dir.path(),
            6,
            &RgbaImage::from_pixel(32, 32, Rgba([110, 110, 110, 255])),
        );
        assert_eq!(cache.get(dir.path(), &gray).0, Some([110, 110, 110]));
        let empty = save(dir.path(), 7, &RgbaImage::new(32, 32));
        assert_eq!(cache.get(dir.path(), &empty).0, None);
    }
    #[test]
    fn icon_palette_groups_dominant_hue_across_shaded_logo_pixels() {
        let mut image = RgbaImage::new(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                image.put_pixel(
                    x,
                    y,
                    if x < 24 {
                        Rgba([
                            15 + (x % 3) as u8 * 18,
                            115 + (y % 5) as u8 * 22,
                            35 + (x % 4) as u8 * 15,
                            255,
                        ])
                    } else {
                        Rgba([220, 30, 30, 255])
                    },
                );
            }
        }
        let color = dominant_color(&image).expect("opaque logo");
        assert!(
            u16::from(color[1]) > u16::from(color[0]) * 2 && color[1] > color[2],
            "shaded green majority must beat a flat minority: {color:?}"
        );
    }
    #[test]
    fn icon_palette_cache_reuses_immutable_icons_and_remains_bounded() {
        let dir = tempfile::tempdir().expect("icon fixture");
        let mut cache = IconColorCache::default();
        let path = save(
            dir.path(),
            1,
            &RgbaImage::from_pixel(16, 16, Rgba([20, 180, 70, 255])),
        );
        assert!(!cache.get(dir.path(), &path).1);
        fs::remove_file(&path).expect("immutable fixture removed after read");
        assert_eq!(cache.get(dir.path(), &path), (Some([20, 180, 70]), true));
        for key in 2..150 {
            cache.get(
                dir.path(),
                &dir.path().join(format!("source-{key:064x}.png")),
            );
        }
        assert_eq!(cache.entries.len(), CACHE_CAPACITY);
    }
    #[test]
    fn icon_palette_storage_root_change_invalidates_cached_colors() {
        let dir = tempfile::tempdir().expect("icon fixture");
        let other = tempfile::tempdir().expect("replacement storage root");
        let mut cache = IconColorCache::default();
        let path = save(
            dir.path(),
            1,
            &RgbaImage::from_pixel(128, 128, Rgba([20, 180, 70, 255])),
        );
        assert_eq!(cache.get(dir.path(), &path), (Some([20, 180, 70]), false));
        assert_eq!(cache.get(other.path(), &path), (None, false));
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.get(dir.path(), &path), (Some([20, 180, 70]), false));
    }
    #[test]
    fn icon_palette_rejects_untrusted_corrupt_and_oversized_sources() {
        let dir = tempfile::tempdir().expect("icon fixture");
        let external = tempfile::tempdir().expect("outside root");
        let mut cache = IconColorCache::default();
        let outside = save(
            external.path(),
            1,
            &RgbaImage::from_pixel(16, 16, Rgba([0, 200, 0, 255])),
        );
        assert_eq!(cache.get(dir.path(), &outside).0, None);
        let large = save(dir.path(), 2, &RgbaImage::new(513, 513));
        assert_eq!(cache.get(dir.path(), &large).0, None);
        let corrupt = dir.path().join(format!("source-{}.png", "a".repeat(64)));
        fs::write(&corrupt, b"not png").expect("corrupt fixture");
        assert_eq!(cache.get(dir.path(), &corrupt).0, None);
        let huge = dir.path().join(format!("source-{}.png", "b".repeat(64)));
        fs::write(&huge, vec![0; MAX_BYTES as usize + 1]).expect("oversize fixture");
        assert_eq!(cache.get(dir.path(), &huge).0, None);
        assert!(
            cache.get(dir.path(), &huge).1,
            "unavailable result also cached"
        );
    }
}
