//! Blocking, off-render image loading. The caller owns worker admission and generation cancellation.
use std::{fmt, fs::File, io::BufReader, path::Path, time::Duration};

use image::{
    AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbaImage,
    codecs::gif::GifDecoder, imageops::FilterType,
};

const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_AXIS: u32 = 4096;
const MAX_PIXELS: u64 = 16_000_000;
const MAX_FRAMES: usize = 512;
const MAX_RETAINED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TARGET_SIDE: u32 = 900;
const ZERO_DELAY_FALLBACK: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MediaError {
    Io,
    NotRegularFile,
    SourceTooLarge,
    UnsupportedFormat,
    InvalidImage,
    DimensionsExceeded,
    TooManyFrames,
    RetainedBytesExceeded,
    InvalidTarget,
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Io => "Cannot read the selected image",
            Self::NotRegularFile => "Select a regular image file",
            Self::SourceTooLarge => "Image file exceeds 16 MiB",
            Self::UnsupportedFormat => "Image format is not supported",
            Self::InvalidImage => "Image data is damaged or incomplete",
            Self::DimensionsExceeded => "Image dimensions exceed the supported limit",
            Self::TooManyFrames => "GIF exceeds 512 frames",
            Self::RetainedBytesExceeded => "Decoded image exceeds the 64 MiB frame budget",
            Self::InvalidTarget => "Invalid image display size",
        })
    }
}

impl std::error::Error for MediaError {}

/// RGBA bytes in row-major order, already composited (for GIF) and downscaled.
#[derive(Debug)]
pub(crate) struct MediaFrame {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
    /// The time this frame remains visible before the following frame.
    pub(crate) duration: Duration,
}

#[derive(Debug)]
pub(crate) struct DecodedMedia {
    pub(crate) frames: Vec<MediaFrame>,
    /// Single-frame GIFs are static too; the parent does not start a timer for them.
    pub(crate) animated: bool,
}

fn checked_dimensions(width: u32, height: u32) -> Result<(), MediaError> {
    if width == 0
        || height == 0
        || width > MAX_AXIS
        || height > MAX_AXIS
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        Err(MediaError::DimensionsExceeded)
    } else {
        Ok(())
    }
}

fn target_dimensions(width: u32, height: u32, side: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= side {
        (width, height)
    } else {
        (
            ((u64::from(width) * u64::from(side)) / u64::from(longest)).max(1) as u32,
            ((u64::from(height) * u64::from(side)) / u64::from(longest)).max(1) as u32,
        )
    }
}

fn retain_frame(
    image: RgbaImage,
    duration: Duration,
    target_side: u32,
    retained: &mut u64,
) -> Result<MediaFrame, MediaError> {
    let (width, height) = target_dimensions(image.width(), image.height(), target_side);
    let bytes = u64::from(width) * u64::from(height) * 4;
    *retained = retained
        .checked_add(bytes)
        .filter(|total| *total <= MAX_RETAINED_BYTES)
        .ok_or(MediaError::RetainedBytesExceeded)?;
    let image = if (width, height) == image.dimensions() {
        image
    } else {
        image::imageops::resize(&image, width, height, FilterType::Triangle)
    };
    Ok(MediaFrame {
        width,
        height,
        rgba: image.into_raw(),
        duration,
    })
}

/// Called only on an admitted blocking worker, never from GPUI render/callbacks.
/// `target_device_side` is the caller's bounded logical size (<=300) times DPI (<=3).
/// The decoder validates source dimensions before allocating any full-size frame.
/// An owner may discard this result if the requesting generation has retired.
pub(crate) fn decode_file(
    path: &Path,
    target_device_side: u32,
) -> Result<DecodedMedia, MediaError> {
    if !(1..=MAX_TARGET_SIDE).contains(&target_device_side) {
        return Err(MediaError::InvalidTarget);
    }
    let file = File::open(path).map_err(|_| MediaError::Io)?;
    let metadata = file.metadata().map_err(|_| MediaError::Io)?;
    if !metadata.is_file() {
        return Err(MediaError::NotRegularFile);
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(MediaError::SourceTooLarge);
    }
    let reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|_| MediaError::Io)?;
    let format = reader.format().ok_or(MediaError::UnsupportedFormat)?;
    if !matches!(
        format,
        ImageFormat::Gif
            | ImageFormat::Png
            | ImageFormat::Jpeg
            | ImageFormat::Bmp
            | ImageFormat::WebP
    ) {
        return Err(MediaError::UnsupportedFormat);
    }
    // Limit the decoder's own allocations as well as checking the canvas before frame output.
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_AXIS);
    limits.max_image_height = Some(MAX_AXIS);
    limits.max_alloc = Some(MAX_RETAINED_BYTES);
    let mut retained = 0;
    if format == ImageFormat::Gif {
        let mut decoder =
            GifDecoder::new(reader.into_inner()).map_err(|_| MediaError::InvalidImage)?;
        let (width, height) = decoder.dimensions();
        checked_dimensions(width, height)?;
        decoder
            .set_limits(limits)
            .map_err(|_| MediaError::InvalidImage)?;
        let mut frames = Vec::new();
        // AnimationDecoder composites disposal/transparency against its previous canvas.
        // Iterate incrementally: never collect_frames before checking count/retained bytes.
        for result in decoder.into_frames() {
            if frames.len() == MAX_FRAMES {
                return Err(MediaError::TooManyFrames);
            }
            let frame = result.map_err(|_| MediaError::InvalidImage)?;
            let delay = frame.delay();
            let (numerator, denominator) = delay.numer_denom_ms();
            let duration = if numerator == 0 || denominator == 0 {
                ZERO_DELAY_FALLBACK
            } else {
                Duration::from_secs_f64(f64::from(numerator) / f64::from(denominator) / 1000.0)
                    .max(Duration::from_millis(1))
            };
            frames.push(retain_frame(
                frame.into_buffer(),
                duration,
                target_device_side,
                &mut retained,
            )?);
        }
        if frames.is_empty() {
            return Err(MediaError::InvalidImage);
        }
        Ok(DecodedMedia {
            animated: frames.len() > 1,
            frames,
        })
    } else {
        let mut reader = reader;
        reader.limits(limits);
        let decoder = reader.into_decoder().map_err(|error| match error {
            image::ImageError::Limits(_) => MediaError::DimensionsExceeded,
            _ => MediaError::InvalidImage,
        })?;
        let (width, height) = decoder.dimensions();
        checked_dimensions(width, height)?;
        let image = DynamicImage::from_decoder(decoder).map_err(|_| MediaError::InvalidImage)?;
        // Avoid a second full-size RGBA allocation for RGB/JPEG/BMP/WebP sources:
        // scale in the original format first, then convert only the bounded output.
        let (out_width, out_height) = target_dimensions(width, height, target_device_side);
        let image = if (out_width, out_height) == (width, height) {
            image
        } else {
            image.resize_exact(out_width, out_height, FilterType::Triangle)
        };
        let frame = retain_frame(
            image.to_rgba8(),
            Duration::ZERO,
            target_device_side,
            &mut retained,
        )?;
        Ok(DecodedMedia {
            frames: vec![frame],
            animated: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Delay, Frame, ImageEncoder, codecs::gif::GifEncoder};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn asset(bytes: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("temporary asset");
        file.write_all(bytes).expect("write asset");
        file
    }

    fn gif(width: u32, height: u32, count: usize, pixels: Vec<u8>, delay_ms: u32) -> Vec<u8> {
        let mut encoded = Vec::new();
        {
            let mut encoder = GifEncoder::new_with_speed(&mut encoded, 30);
            for _ in 0..count {
                let rgba =
                    RgbaImage::from_raw(width, height, pixels.clone()).expect("RGBA fixture");
                encoder
                    .encode_frame(Frame::from_parts(
                        rgba,
                        0,
                        0,
                        Delay::from_numer_denom_ms(delay_ms, 1),
                    ))
                    .expect("GIF frame");
            }
        }
        encoded
    }

    #[test]
    fn static_png_is_bounded_and_stays_static() {
        let mut encoded = Vec::new();
        image::codecs::png::PngEncoder::new(&mut encoded)
            .write_image(
                &[255, 0, 0, 128, 0, 0, 255, 255],
                2,
                1,
                image::ExtendedColorType::Rgba8,
            )
            .expect("PNG fixture");
        let result = decode_file(asset(&encoded).path(), 1).expect("static PNG");
        assert!(!result.animated);
        assert_eq!((result.frames[0].width, result.frames[0].height), (1, 1));
        assert_eq!(result.frames[0].rgba.len(), 4);
        assert_eq!(result.frames[0].duration, Duration::ZERO);
    }
    #[test]
    fn static_jpeg_bmp_webp_decode_without_path_extension() {
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .encode(&[255, 0, 0], 1, 1, image::ExtendedColorType::Rgb8)
            .expect("JPEG fixture");
        let mut bmp = Vec::new();
        image::codecs::bmp::BmpEncoder::new(&mut bmp)
            .encode(&[255, 0, 0], 1, 1, image::ExtendedColorType::Rgb8)
            .expect("BMP fixture");
        let mut webp = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut webp)
            .encode(&[255, 0, 0, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .expect("WebP fixture");
        for bytes in [jpeg, bmp, webp] {
            let media = decode_file(asset(&bytes).path(), 1).expect("static image");
            assert!(!media.animated);
            assert_eq!(media.frames.len(), 1);
            assert_eq!(media.frames[0].rgba.len(), 4);
        }
    }

    #[test]
    fn gif_keep_disposal_preserves_previous_composited_pixels() {
        let mut encoded = Vec::new();
        {
            let mut encoder = GifEncoder::new_with_speed(&mut encoded, 30);
            for pixels in [
                vec![255, 0, 0, 255, 255, 0, 0, 255],
                vec![0, 0, 0, 0, 0, 0, 255, 255],
            ] {
                encoder
                    .encode_frame(Frame::from_parts(
                        RgbaImage::from_raw(2, 1, pixels).expect("frame"),
                        0,
                        0,
                        Delay::from_numer_denom_ms(50, 1),
                    ))
                    .expect("encode");
            }
        }
        let gce = encoded
            .windows(3)
            .position(|bytes| bytes == [0x21, 0xf9, 0x04])
            .expect("first graphic control extension");
        // GIF GCE packed byte: change disposal from Background (2) to Keep (1).
        encoded[gce + 3] = (encoded[gce + 3] & !0x1c) | 0x04;
        let media = decode_file(asset(&encoded).path(), 2).expect("decode");
        assert_eq!(&media.frames[1].rgba[..4], &[255, 0, 0, 255]);
        assert_eq!(&media.frames[1].rgba[4..], &[0, 0, 255, 255]);
    }

    #[test]
    fn gif_disposal_transparency_and_source_delays_are_composited() {
        let mut encoded = Vec::new();
        {
            let mut encoder = GifEncoder::new_with_speed(&mut encoded, 30);
            for (pixels, delay) in [
                (vec![255, 0, 0, 255, 255, 0, 0, 255], 50),
                (vec![0, 0, 0, 0, 0, 0, 255, 255], 200),
            ] {
                encoder
                    .encode_frame(Frame::from_parts(
                        RgbaImage::from_raw(2, 1, pixels).expect("frame"),
                        0,
                        0,
                        Delay::from_numer_denom_ms(delay, 1),
                    ))
                    .expect("encode");
            }
        }
        let media = decode_file(asset(&encoded).path(), 2).expect("decode");
        assert!(media.animated);
        assert_eq!(media.frames.len(), 2);
        assert_eq!(media.frames[0].duration, Duration::from_millis(50));
        assert_eq!(media.frames[1].duration, Duration::from_millis(200));
        // GifEncoder sets Background disposal: the transparent region reveals a cleared canvas.
        assert_eq!(&media.frames[1].rgba[..4], &[0, 0, 0, 0]);
        assert_eq!(&media.frames[1].rgba[4..], &[0, 0, 255, 255]);
    }

    #[test]
    fn zero_delay_is_normalized_and_single_frame_has_no_timer() {
        let file = asset(&gif(1, 1, 1, vec![255, 0, 0, 255], 0));
        let media = decode_file(file.path(), 1).expect("decode");
        assert!(!media.animated);
        assert_eq!(media.frames[0].duration, ZERO_DELAY_FALLBACK);
    }

    #[test]
    fn rejects_invalid_file_bytes_dimensions_and_target_without_path_leak() {
        let corrupt = asset(b"garbage");
        assert_eq!(
            decode_file(corrupt.path(), 1).expect_err("reject unsupported bytes"),
            MediaError::UnsupportedFormat
        );
        assert_eq!(
            decode_file(corrupt.path(), 901).expect_err("reject oversized display target"),
            MediaError::InvalidTarget
        );
        let mut oversized = gif(1, 1, 1, vec![255, 0, 0, 255], 10);
        oversized[6..8].copy_from_slice(&4097u16.to_le_bytes());
        assert_eq!(
            decode_file(asset(&oversized).path(), 900).expect_err("reject source width"),
            MediaError::DimensionsExceeded
        );
        oversized[6..8].copy_from_slice(&4096u16.to_le_bytes());
        oversized[8..10].copy_from_slice(&4096u16.to_le_bytes());
        assert_eq!(
            decode_file(asset(&oversized).path(), 900).expect_err("reject source pixel count"),
            MediaError::DimensionsExceeded
        );
        assert!(!MediaError::InvalidImage.to_string().contains("garbage"));
    }

    #[test]
    fn source_frame_and_retained_budgets_reject_before_unbounded_collection() {
        let file = asset(&vec![0; MAX_SOURCE_BYTES as usize + 1]);
        assert_eq!(
            decode_file(file.path(), 1).expect_err("reject source byte budget"),
            MediaError::SourceTooLarge
        );
        let many = asset(&gif(1, 1, MAX_FRAMES + 1, vec![255, 0, 0, 255], 10));
        assert_eq!(
            decode_file(many.path(), 1).expect_err("reject frame count"),
            MediaError::TooManyFrames
        );
        // GIF palette compression keeps this synthetic clip below 16 MiB encoded,
        // while its full RGBA outputs exceed the 64 MiB retained budget.
        let retained = asset(&gif(400, 400, 106, vec![255; 400 * 400 * 4], 10));
        assert_eq!(
            decode_file(retained.path(), 400).expect_err("reject retained frame budget"),
            MediaError::RetainedBytesExceeded
        );
    }
}
