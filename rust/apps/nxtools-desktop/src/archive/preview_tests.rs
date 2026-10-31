use super::*;
use image::ImageEncoder as _;
use std::sync::atomic::AtomicBool;

fn decoded(bytes: Vec<u8>, kind: PreviewKind) -> Result<PreviewPayload, PreviewFailure> {
    decode_bytes(bytes, kind, &AtomicBool::new(false))
}
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            &vec![255; width as usize * height as usize * 4],
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .expect("owned PNG");
    bytes
}
#[test]
fn materialized_eighty_byte_text_is_read_and_decoded() {
    let root = tempfile::tempdir().expect("owned preview file");
    let path = root.path().join("small.txt");
    std::fs::write(&path, "p".repeat(80)).expect("materialized text");
    let payload = prepare(
        &path,
        PreviewKind::Text(DocumentMode::Literal(None)),
        &AtomicBool::new(false),
    )
    .expect("actual bounded read and UTF-8 decode");
    let PreviewPayload::Text { text, .. } = payload else {
        panic!("text payload")
    };
    assert_eq!(text, "p".repeat(80));
}

#[test]
fn actual_text_bytes_utf8_controls_lines_and_markdown_projection_are_bounded() {
    let text = PreviewKind::Text(DocumentMode::Literal(None));
    assert!(decoded(vec![b'a'; 65536], text).is_ok());
    assert!(matches!(
        decoded(vec![b'a'; 65537], text),
        Err(PreviewFailure::TooLarge)
    ));
    for bytes in [vec![0xff], b"a\0b".to_vec(), b"a\x1bb".to_vec()] {
        assert!(matches!(
            decoded(bytes, text),
            Err(PreviewFailure::Unsupported)
        ));
    }
    assert!(decoded("line\n".repeat(1200).into_bytes(), text).is_ok());
    assert!(matches!(
        decoded("line\n".repeat(1201).into_bytes(), text),
        Err(PreviewFailure::TextLines)
    ));
    let source = "# Title\n<script>no()</script>\n![remote](https://invalid.test/a.png)\n![local](file:///C:/secret)";
    let payload = decoded(
        source.as_bytes().to_vec(),
        PreviewKind::Text(DocumentMode::Markdown),
    )
    .expect("passive markdown");
    let PreviewPayload::Text { text, mode } = payload else {
        panic!("text")
    };
    let prepared =
        crate::ui::markdown_document::prepared_source(&text, mode).expect("safe shared projection");
    assert!(!prepared.contains("https://invalid.test") && !prepared.contains("file:///C:/secret"));
}
#[test]
fn image_headers_encoded_bytes_axis_pixels_and_high_depth_are_bounded() {
    assert!(decoded(png(2, 3), PreviewKind::Image).is_ok());
    assert!(matches!(
        decoded(vec![0; 8 * 1024 * 1024 + 1], PreviewKind::Image),
        Err(PreviewFailure::TooLarge)
    ));
    assert!(matches!(
        decoded(b"not an image".to_vec(), PreviewKind::Image),
        Err(PreviewFailure::Corrupt)
    ));
    assert!(check_dimensions(8192, 1024).is_ok());
    assert_eq!(
        check_dimensions(8192, 1025),
        Err(PreviewFailure::ImageDimensions)
    );
    assert_eq!(
        check_dimensions(8193, 1),
        Err(PreviewFailure::ImageDimensions)
    );
    assert_eq!(check_dimensions(0, 1), Err(PreviewFailure::Corrupt));
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&[0; 8], 1, 1, image::ExtendedColorType::Rgba16)
        .expect("16 bit PNG");
    assert!(matches!(
        decoded(bytes, PreviewKind::Image),
        Err(PreviewFailure::Unsupported)
    ));
}
#[test]
fn owned_image_is_static_bgra_and_never_path_backed() {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&[255, 2, 3, 255], 1, 1, image::ExtendedColorType::Rgba8)
        .expect("red PNG");
    let PreviewPayload::Image {
        image,
        width,
        height,
    } = decoded(bytes, PreviewKind::Image).expect("owned image")
    else {
        panic!("image")
    };
    assert_eq!((width, height), (1, 1));
    assert_eq!(image.as_bytes(0).expect("first frame"), &[3, 2, 255, 255]);
}
#[test]
fn actual_read_and_cancel_precede_decode() {
    let stop = AtomicBool::new(false);
    assert!(matches!(
        read_bounded(&b"12345"[..], 4, &stop),
        Err(PreviewFailure::TooLarge)
    ));
    stop.store(true, Ordering::Release);
    assert!(matches!(
        decode_bytes(png(1, 1), PreviewKind::Image, &stop),
        Err(PreviewFailure::Cancelled)
    ));
    assert!(kind("script.ps1").is_some());
    for name in [
        "app.exe",
        "nested.zip",
        "vector.svg",
        "document.pdf",
        "movie.mp4",
    ] {
        assert!(kind(name).is_none());
    }
}
#[test]
fn enabled_image_codecs_use_only_one_owned_first_frame() {
    let rgba = image::RgbaImage::from_raw(1, 1, vec![12, 34, 56, 255]).expect("pixel");
    for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Bmp] {
        let mut encoded = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::DynamicImage::ImageRgba8(rgba.clone()).into_rgb8())
            .write_to(&mut encoded, format)
            .expect("enabled codec fixture");
        let PreviewPayload::Image {
            image,
            width,
            height,
        } = decoded(encoded.into_inner(), PreviewKind::Image).expect("supported image")
        else {
            panic!("image")
        };
        assert_eq!((width, height), (1, 1));
        assert_eq!(image.as_bytes(0).expect("owned first").len(), 4);
    }
    let mut gif = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut gif);
        encoder
            .encode_frame(image::Frame::new(rgba.clone()))
            .expect("first GIF frame");
        encoder
            .encode_frame(image::Frame::new(
                image::RgbaImage::from_raw(1, 1, vec![255, 0, 0, 255]).expect("second"),
            ))
            .expect("second GIF frame");
    }
    let PreviewPayload::Image { image, .. } =
        decoded(gif, PreviewKind::Image).expect("GIF first frame")
    else {
        panic!("image")
    };
    assert_eq!(image.as_bytes(0).expect("first"), &[56, 34, 12, 255]);
}

#[test]
fn decode_admission_is_globally_two_until_actual_permit_release() {
    permits().1.try_recv().expect("first global permit");
    let first = DecodePermit;
    permits().1.try_recv().expect("second global permit");
    let second = DecodePermit;
    assert!(
        permits().1.try_recv().is_err(),
        "exactly two global decode slots"
    );
    drop(first);
    permits().1.try_recv().expect("released permit");
    let replacement = DecodePermit;
    assert!(permits().1.try_recv().is_err());
    drop(second);
    drop(replacement);
}

#[test]
fn webp_rejects_unbounded_transforms_and_accepts_bounded_static_lossless() {
    let mut payload = vec![0x2f, 0, 0, 0, 0];
    // No transforms, no cache, no meta-Huffman. Five single-symbol trees G/R/B/A/D.
    let mut bits = Vec::new();
    let mut put = |value: u32, count: usize| {
        for shift in 0..count {
            bits.push(((value >> shift) & 1) as u8);
        }
    };
    put(0, 3);
    for symbol in [34, 12, 56, 255, 0] {
        put(1, 1);
        put(0, 1);
        put(1, 1);
        put(symbol, 8);
    }
    for chunk in bits.chunks(8) {
        payload.push(
            chunk
                .iter()
                .enumerate()
                .fold(0, |value, (bit, set)| value | (set << bit)),
        );
    }
    fn container(payload: &[u8]) -> Vec<u8> {
        let mut result = b"RIFF".to_vec();
        result.extend_from_slice(
            &(12 + payload.len() as u32 + (payload.len() as u32 & 1)).to_le_bytes(),
        );
        result.extend_from_slice(b"WEBPVP8L");
        result.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        result.extend_from_slice(payload);
        if payload.len() & 1 != 0 {
            result.push(0);
        }
        result
    }
    let encoded = container(&payload);
    assert!(webp_admission(&encoded).is_ok());
    let PreviewPayload::Image { image, .. } =
        decoded(encoded, PreviewKind::Image).expect("bounded static WebP")
    else {
        panic!("image")
    };
    assert_eq!(image.as_bytes(0).expect("BGRA first"), &[56, 34, 12, 255]);
    let mut bomb = payload.clone();
    bomb[1..5].copy_from_slice(&0x0fff_ffffu32.to_le_bytes());
    assert_eq!(
        webp_admission(&container(&bomb)),
        Err(PreviewFailure::ImageDimensions)
    );
    for flag in [2, 4] {
        let mut meta = payload.clone();
        meta[5] |= flag;
        assert_eq!(
            webp_admission(&container(&meta)),
            Err(PreviewFailure::Unsupported)
        );
    }
    payload[5] |= 1; // predictor transform: refused before any decoder allocation.
    assert_eq!(
        webp_admission(&container(&payload)),
        Err(PreviewFailure::Unsupported)
    );
}
