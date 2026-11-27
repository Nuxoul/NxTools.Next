use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use markdown::{ParseOptions, mdast::Node};

const MAX_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Stamp {
    modified: SystemTime,
    len: u64,
}

pub(super) fn stamp(path: &Path) -> io::Result<Stamp> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    Ok(Stamp {
        modified: metadata.modified()?,
        len: metadata.len(),
    })
}

#[derive(Clone)]
pub(super) struct Heading {
    pub title: String,
    pub level: u8,
    pub source: Range<usize>,
    pub anchor: String,
}

pub(super) struct Document {
    pub path: PathBuf,
    pub text: String,
    bom: bool,
    pub stamp: Stamp,
    pub headings: Vec<Heading>,
    pub images: Arc<HashMap<String, PathBuf>>,
}

impl Document {
    pub fn original(&self) -> Vec<u8> {
        encode(&self.text, self.bom)
    }
}

#[derive(Debug)]
pub(super) enum ReadError {
    Unsupported,
    TooLarge,
    Encoding,
    Changed,
    Io(io::Error),
}

impl From<io::Error> for ReadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl ReadError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::TooLarge => "limit",
            Self::Encoding => "encoding",
            Self::Changed => "changed",
            Self::Io(_) => "io",
        }
    }

    pub fn native_code(&self) -> Option<i32> {
        match self {
            Self::Io(error) => error.raw_os_error(),
            _ => None,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Unsupported => "请选择 .md 或 .markdown 文件。",
            Self::TooLarge => "文档超过 4 MB，无法在阅读器中打开。",
            Self::Encoding => "无法读取此编码。请将文档另存为 UTF-8 后重试。",
            Self::Changed => "文件正在被修改，请稍后重新加载。",
            Self::Io(_) => "无法读取文件，请检查文件是否存在且可以访问。",
        }
    }
}

pub(super) fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|part| part.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

pub(super) fn read(path: PathBuf) -> Result<Document, ReadError> {
    if !supported(&path) {
        return Err(ReadError::Unsupported);
    }
    let path = fs::canonicalize(path)?;
    let before = stamp(&path)?;
    if before.len > MAX_DOCUMENT_BYTES {
        return Err(ReadError::TooLarge);
    }
    let mut bytes = Vec::new();
    File::open(&path)?
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(ReadError::TooLarge);
    }
    if before != stamp(&path)? || bytes.len() as u64 != before.len {
        return Err(ReadError::Changed);
    }
    prepare(path, bytes, before)
}

fn prepare(path: PathBuf, bytes: Vec<u8>, stamp: Stamp) -> Result<Document, ReadError> {
    let mut text = String::from_utf8(bytes).map_err(|_| ReadError::Encoding)?;
    let bom = text.starts_with('\u{feff}');
    if bom {
        text.drain(..3);
    }
    let (headings, images) = project(&path, &text);
    Ok(Document {
        path,
        text,
        bom,
        stamp,
        headings,
        images: Arc::new(images),
    })
}

fn encode(text: &str, bom: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + if bom { 3 } else { 0 });
    if bom {
        bytes.extend_from_slice(b"\xef\xbb\xbf");
    }
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

pub(super) fn save(path: PathBuf, baseline: &[u8], draft: &str) -> Result<Document, ReadError> {
    if draft.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(ReadError::TooLarge);
    }
    let bytes = encode(draft, baseline.starts_with(b"\xef\xbb\xbf"));
    replace(path, baseline, bytes)
}

pub(super) fn replace(
    path: PathBuf,
    baseline: &[u8],
    bytes: Vec<u8>,
) -> Result<Document, ReadError> {
    if std::str::from_utf8(&bytes).is_err() {
        return Err(ReadError::Encoding);
    }
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(ReadError::TooLarge);
    }
    if bytes == baseline {
        let current = read(path)?;
        return if current.original() == baseline {
            Ok(current)
        } else {
            Err(ReadError::Changed)
        };
    }
    let candidate = path.with_file_name(format!(".nxtools-markdown-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(&candidate)?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.set_permissions(fs::metadata(&path)?.permissions())?;
        file.sync_all()?;
        drop(file);
        let candidate_stamp = stamp(&candidate)?;
        let mut current = Vec::with_capacity(baseline.len());
        File::open(&path)?
            .take(MAX_DOCUMENT_BYTES + 1)
            .read_to_end(&mut current)?;
        if current != baseline {
            return Err(ReadError::Changed);
        }
        fs::rename(&candidate, &path)?;
        prepare(path, bytes, candidate_stamp)
    })();
    if result.is_err() {
        let _ = fs::remove_file(candidate);
    }
    result
}

pub(super) fn project(path: &Path, text: &str) -> (Vec<Heading>, HashMap<String, PathBuf>) {
    let mut headings = Vec::new();
    let mut images = HashMap::new();
    let mut used = std::collections::HashSet::new();
    let mut counters = HashMap::<String, usize>::new();
    let Ok(root) = markdown::to_mdast(text, &ParseOptions::gfm()) else {
        return (headings, images);
    };
    let mut nodes = vec![&root];
    while let Some(node) = nodes.pop() {
        match node {
            Node::Heading(heading) => {
                if let Some(position) = &heading.position {
                    let title = node.to_string();
                    let base: String = title
                        .trim()
                        .to_lowercase()
                        .chars()
                        .filter_map(|c| {
                            if c.is_whitespace() {
                                Some('-')
                            } else if c.is_alphanumeric() || matches!(c, '-' | '_') {
                                Some(c)
                            } else {
                                None
                            }
                        })
                        .collect();
                    let base = if base.is_empty() {
                        "section".to_owned()
                    } else {
                        base
                    };
                    let next = counters.entry(base.clone()).or_default();
                    let mut anchor;
                    loop {
                        anchor = if *next == 0 {
                            base.clone()
                        } else {
                            format!("{base}-{next}")
                        };
                        *next += 1;
                        if used.insert(anchor.clone()) {
                            break;
                        }
                    }
                    headings.push(Heading {
                        title,
                        level: heading.depth,
                        source: position.start.offset..position.end.offset,
                        anchor,
                    });
                }
            }
            Node::Image(image) => {
                if let Some(local) = relative_path(path, &image.url)
                    && let Ok(metadata) = fs::metadata(&local)
                    && metadata.is_file()
                {
                    images.insert(image.url.clone(), local);
                }
            }
            _ => {}
        }
        if let Some(children) = node.children() {
            nodes.extend(children.iter().rev());
        }
    }
    (headings, images)
}

pub(super) fn relative_path(document: &Path, target: &str) -> Option<PathBuf> {
    if target.is_empty() || target.starts_with(['/', '\\']) || target.contains(':') {
        return None;
    }
    #[cfg(windows)]
    let ordinary = document.to_string_lossy();
    #[cfg(windows)]
    let ordinary = ordinary
        .strip_prefix(r"\\?\UNC\")
        .map(|path| PathBuf::from(format!(r"\\{path}")))
        .or_else(|| ordinary.strip_prefix(r"\\?\").map(PathBuf::from));
    #[cfg(windows)]
    let document = ordinary.as_deref().unwrap_or(document);
    let base = url::Url::from_file_path(document).ok()?;
    let decoded = base.join(target).ok()?;
    if decoded.host_str() != base.host_str() {
        return None;
    }
    decoded.to_file_path().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_reader_reads_headings_duplicates_and_local_images() {
        let directory = tempfile::tempdir().expect("create document fixture directory");
        let path = directory.path().join("readme.md");
        let text = "# 标题\n\n## **章节**\n\n```text\n## not a heading\n```\n\n## 章节\n\n![local](sample%20image.png) ![remote](https://example.invalid/image.png)";
        fs::write(&path, text).expect("write headings document fixture");
        fs::write(directory.path().join("sample image.png"), b"image fixture")
            .expect("write local image fixture");
        let doc = read(path).expect("read headings and local images");
        assert_eq!(
            doc.headings
                .iter()
                .map(|h| (h.level, h.title.as_str(), h.anchor.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (1, "标题", "标题"),
                (2, "章节", "章节"),
                (2, "章节", "章节-1")
            ]
        );
        assert_eq!(&doc.text[doc.headings[1].source.clone()], "## **章节**");
        assert_eq!(doc.images.len(), 1);
        assert!(doc.images["sample%20image.png"].ends_with("sample image.png"));
    }

    #[test]
    fn markdown_reader_save_keeps_bom_and_refuses_external_conflict() {
        let folder = tempfile::tempdir().expect("create save fixture directory");
        let path = folder.path().join("document.md");
        let before = b"\xef\xbb\xbf# Original\r\n";
        fs::write(&path, before).expect("write original BOM document");
        let opened = read(path.clone()).expect("read original BOM document");
        assert_eq!(opened.original(), before);
        let same = save(path.clone(), before, "# Original\r\n")
            .expect("save unchanged document without altering its stamp");
        assert_eq!(same.stamp.modified, opened.stamp.modified);
        let saved =
            save(path.clone(), before, "# Edits\r\n").expect("save edited document preserving BOM");
        assert_eq!(
            fs::read(&path).expect("read saved BOM document"),
            b"\xef\xbb\xbf# Edits\r\n"
        );
        fs::write(&path, b"# External edit\n").expect("write conflicting external edit");
        assert!(matches!(
            save(path.clone(), &saved.original(), "# Draft\n"),
            Err(ReadError::Changed)
        ));
        assert_eq!(
            fs::read(&path).expect("read preserved external edit"),
            b"# External edit\n"
        );
        assert_eq!(
            fs::read_dir(folder.path())
                .expect("list save fixture directory")
                .count(),
            1
        );
    }
}
