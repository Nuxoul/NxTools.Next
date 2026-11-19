use super::{CONTENT_TYPE, DownloadProbe, DownloadRequest, HttpDownloadError, Response};

const PREFIX_LIMIT: usize = 1024;

pub(super) struct ContentGuard {
    pending: Vec<u8>,
    checking: bool,
}

impl ContentGuard {
    pub(super) fn new(
        request: &DownloadRequest,
        probe: &DownloadProbe,
        response: &Response,
        starts_at_zero: bool,
    ) -> Result<Self, HttpDownloadError> {
        let name = request.file_name.as_deref().or(probe.file_name.as_deref());
        let expected = expects_binary(name, probe.content_type.as_deref());
        let success = response.status().is_success();
        if expected
            && success
            && response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(is_html_type)
        {
            return Err(HttpDownloadError::UnexpectedWebPage);
        }
        Ok(Self {
            pending: Vec::new(),
            checking: expected && success && (starts_at_zero || response.status() == 200),
        })
    }

    pub(super) fn checking(&self) -> bool {
        self.checking
    }

    /// Returns the accepted prefix and how much of this chunk it already contains.
    pub(super) fn accept(
        &mut self,
        bytes: &[u8],
        ended: bool,
    ) -> Result<(Vec<u8>, usize), HttpDownloadError> {
        if !self.checking {
            return Ok((Vec::new(), 0));
        }
        let consumed = bytes.len().min(PREFIX_LIMIT - self.pending.len());
        self.pending.extend_from_slice(&bytes[..consumed]);
        let start = self
            .pending
            .strip_prefix(b"\xef\xbb\xbf")
            .unwrap_or(&self.pending);
        let start = start
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .map_or(&[][..], |at| &start[at..]);
        let markers: &[&[u8]] = &[
            b"<!doctype html",
            b"<html",
            b"<head",
            b"<body",
            b"<title",
            b"<meta",
            b"<script",
            b"<!--",
        ];
        let mut possible = start.is_empty() || b"\xef\xbb\xbf".starts_with(&self.pending);
        for marker in markers {
            let common = marker.len().min(start.len());
            if start[..common].eq_ignore_ascii_case(&marker[..common]) {
                if start.len() >= marker.len() {
                    return Err(HttpDownloadError::UnexpectedWebPage);
                }
                possible = true;
            }
        }
        if !possible || ended || self.pending.len() == PREFIX_LIMIT {
            self.checking = false;
            return Ok((std::mem::take(&mut self.pending), consumed));
        }
        Ok((Vec::new(), consumed))
    }
}

fn is_html_type(value: &str) -> bool {
    let mime = value.split(';').next().unwrap_or_default().trim();
    mime.eq_ignore_ascii_case("text/html") || mime.eq_ignore_ascii_case("application/xhtml+xml")
}

fn expects_binary(name: Option<&str>, mime: Option<&str>) -> bool {
    let extension = name
        .and_then(|name| std::path::Path::new(name).extension())
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        extension.as_str(),
        "htm" | "html" | "xhtml" | "shtml" | "doc" | "dot" | "xls" | "ppt" | "pps" | "svg"
    ) {
        return false;
    }
    if matches!(
        extension.as_str(),
        "zip"
            | "rar"
            | "7z"
            | "gz"
            | "tgz"
            | "tar"
            | "bz2"
            | "xz"
            | "zst"
            | "exe"
            | "dll"
            | "msi"
            | "apk"
            | "iso"
            | "img"
            | "dmg"
            | "pdf"
            | "epub"
            | "jar"
            | "torrent"
            | "docx"
            | "xlsx"
            | "pptx"
            | "odt"
            | "ods"
            | "odp"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "avif"
            | "bmp"
            | "ico"
            | "mp4"
            | "mkv"
            | "webm"
            | "mov"
            | "avi"
            | "mp3"
            | "m4a"
            | "aac"
            | "flac"
            | "ogg"
            | "wav"
            | "ts"
    ) {
        return true;
    }
    let mime = mime
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    (mime.starts_with("image/") && mime != "image/svg+xml")
        || mime.starts_with("audio/")
        || mime.starts_with("video/")
        || matches!(
            mime.as_str(),
            "application/zip"
                | "application/x-zip-compressed"
                | "application/vnd.rar"
                | "application/x-rar-compressed"
                | "application/x-7z-compressed"
                | "application/gzip"
                | "application/x-tar"
                | "application/pdf"
                | "application/epub+zip"
                | "application/x-msdownload"
                | "application/vnd.microsoft.portable-executable"
        )
        || mime.starts_with("application/vnd.openxmlformats-officedocument.")
        || mime.starts_with("application/vnd.oasis.opendocument.")
}
