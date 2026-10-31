//! 链接识别: extract URLs and existing local paths from copied text.
//! Matching is deliberately conservative — the panel appears on every
//! matching copy, so false positives are worse than misses.

/// The kind determines the action taken after the row is clicked. It is kept
/// separate from the display/copy strings so local paths never enter the URL
/// construction branch in the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DetectedLinkKind {
    Web,
    LocalFile,
    LocalDirectory,
}

impl DetectedLinkKind {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::LocalFile => "local_file",
            Self::LocalDirectory => "local_directory",
        }
    }
}

/// One recognized target. `display` is what the panel shows; `copy_value` is
/// what clicking the row puts back on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedLink {
    pub display: String,
    pub copy_value: String,
    pub kind: DetectedLinkKind,
}

const MAX_DETECTED_LINKS: usize = 8;

pub(crate) fn detect_links(text: &str) -> Vec<DetectedLink> {
    let mut links: Vec<DetectedLink> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for raw_line in text.lines().take(64) {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        for candidate in split_candidates(line) {
            let Some(link) = classify(candidate) else {
                continue;
            };
            if seen.insert(link.copy_value.clone()) {
                links.push(link);
            }
            if links.len() >= MAX_DETECTED_LINKS {
                return links;
            }
        }
    }
    links
}

fn split_candidates(line: &str) -> impl Iterator<Item = &str> {
    // Quotes and angle brackets are common wrappers around pasted URLs.
    line.split(['\t', ' ', '"', '\'', '<', '>', '(', ')'])
}

fn classify(candidate: &str) -> Option<DetectedLink> {
    let candidate = candidate.trim_matches(|c: char| c == ',' || c == ';' || c == '\u{3000}');
    if candidate.len() < 4 {
        return None;
    }
    let lower = candidate.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        // Trim trailing punctuation that is almost never part of the URL.
        let trimmed = candidate.trim_end_matches(['.', ',', ';', '!', '?', ')', '。', '，']);
        return Some(DetectedLink {
            display: trimmed.to_owned(),
            copy_value: trimmed.to_owned(),
            kind: DetectedLinkKind::Web,
        });
    }
    // Bare domains ("image.routerapi.net", optionally with a path) are
    // common copies from download sites; no scheme required.
    if looks_like_bare_domain(candidate) {
        return Some(DetectedLink {
            display: candidate.to_owned(),
            copy_value: candidate.to_owned(),
            kind: DetectedLinkKind::Web,
        });
    }
    if (looks_like_drive_path(candidate) || looks_like_unc_path(candidate))
        && std::path::Path::new(candidate).exists()
    {
        let path = std::path::Path::new(candidate);
        return Some(DetectedLink {
            display: candidate.to_owned(),
            copy_value: candidate.to_owned(),
            kind: if path.is_dir() {
                DetectedLinkKind::LocalDirectory
            } else {
                DetectedLinkKind::LocalFile
            },
        });
    }
    None
}

const KNOWN_TLDS: [&str; 12] = [
    "com", "net", "org", "io", "dev", "app", "cn", "me", "tv", "cc", "xyz", "info",
];

fn looks_like_bare_domain(candidate: &str) -> bool {
    // The user's missed copy was the bare host alone
    // ("image.routerapi.net", no scheme, no path), so a path is NOT required.
    // Two labels with a known TLD is specific enough; reject all-numeric
    // hosts so IPv4 fragments and version numbers don't match.
    let candidate = candidate.trim_end_matches(['.', ',', ';', '!', '?', ')']);
    if candidate.is_empty() {
        return false;
    }
    let host = candidate.split('/').next().unwrap_or_default();
    let mut parts = host.rsplit('.');
    let Some(tld) = parts.next() else {
        return false;
    };
    if !KNOWN_TLDS.contains(&tld.to_ascii_lowercase().as_str()) {
        return false;
    }
    // A second label must exist (hosts like "com.cn" aside, this keeps
    // sentence fragments like "e.g." from matching).
    parts
        .next()
        .is_some_and(|label| !label.is_empty() && !label.bytes().all(|b| b.is_ascii_digit()))
}

fn looks_like_drive_path(candidate: &str) -> bool {
    let bytes = candidate.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn looks_like_unc_path(candidate: &str) -> bool {
    candidate.starts_with("\\\\") && candidate.len() > 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_http_urls_and_trims_punctuation() {
        let links = detect_links("看这个 https://example.com/a?b=1。 还有 https://x.test/p,");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].copy_value, "https://example.com/a?b=1");
        assert_eq!(links[0].kind, DetectedLinkKind::Web);
        assert_eq!(links[1].copy_value, "https://x.test/p");
        assert_eq!(links[1].kind, DetectedLinkKind::Web);
    }

    #[test]
    fn bare_domains_with_a_path_are_recognized_as_web_targets() {
        let links = detect_links("image.routerapi.net");
        let bare = links
            .iter()
            .find(|link| link.copy_value.contains("routerapi.net"))
            .expect("bare domain with a path must be recognized");
        assert_eq!(bare.copy_value, "image.routerapi.net");
        assert_eq!(bare.kind, DetectedLinkKind::Web);
    }

    #[test]
    fn existing_executable_and_directory_paths_keep_distinct_local_kinds() {
        let directory = tempfile::tempdir().expect("local-path fixture directory");
        let executable = directory.path().join("Throne.exe");
        std::fs::write(&executable, b"fixture").expect("local executable fixture");
        let executable_text = executable.to_string_lossy().into_owned();
        let directory_text = directory.path().to_string_lossy().into_owned();
        let links = detect_links(&format!("{executable_text} {directory_text}"));

        let file = links
            .iter()
            .find(|link| link.copy_value == executable_text)
            .expect("existing executable must be recognized");
        assert_eq!(file.kind, DetectedLinkKind::LocalFile);
        let folder = links
            .iter()
            .find(|link| link.copy_value == directory_text)
            .expect("existing directory must be recognized");
        assert_eq!(folder.kind, DetectedLinkKind::LocalDirectory);
    }

    #[test]
    fn existing_drive_paths_are_recognized() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}\\Cargo.toml", manifest.trim_end_matches('\\'));
        let links = detect_links(&format!("文件在 {}", path));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].copy_value, path);
        assert_eq!(links[0].kind, DetectedLinkKind::LocalFile);
    }

    #[test]
    fn nonexistent_paths_are_ignored() {
        assert!(detect_links("D:\\Down\\不存在的路径\\file.zip").is_empty());
    }

    #[test]
    fn plain_text_yields_nothing_and_bounded() {
        assert!(detect_links("普通一句话，没有链接").is_empty());
        let many = (0..32)
            .map(|i| format!("https://host{i}.test/x"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(detect_links(&many).len(), 8, "bounded panel rows");
    }
}
