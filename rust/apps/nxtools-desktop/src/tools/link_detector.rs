//! 链接识别: extract URLs and existing local paths from copied text.
//! Matching is deliberately conservative — the panel appears on every
//! matching copy, so false positives are worse than misses.

/// One recognized target. `display` is what the panel shows; `copy_value` is
/// what clicking the row puts back on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedLink {
    pub display: String,
    pub copy_value: String,
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
        });
    }
    if looks_like_drive_path(candidate) && std::path::Path::new(candidate).exists() {
        return Some(DetectedLink {
            display: candidate.to_owned(),
            copy_value: candidate.to_owned(),
        });
    }
    if looks_like_unc_path(candidate) && std::path::Path::new(candidate).exists() {
        return Some(DetectedLink {
            display: candidate.to_owned(),
            copy_value: candidate.to_owned(),
        });
    }
    None
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
        assert_eq!(links[1].copy_value, "https://x.test/p");
    }


    #[test]
    fn scratch_dump() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}\\Cargo.toml", manifest.trim_end_matches('\\'));
        println!("SCRATCH path={}", path);
        println!("SCRATCH exists={}", std::path::Path::new(&path).exists());
        println!("SCRATCH links={:?}", detect_links(&format!("文件在 {}", path)));
    }

    #[test]
    fn existing_drive_paths_are_recognized() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}\\Cargo.toml", manifest.trim_end_matches('\\'));
        let links = detect_links(&format!("文件在 {}", path));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].copy_value, path);
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
