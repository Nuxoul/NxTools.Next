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
/// the original target used by explicit copy and typed open actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedLink {
    pub display: String,
    pub copy_value: String,
    pub kind: DetectedLinkKind,
}

const MAX_DETECTED_LINKS: usize = 8;
const MAX_CANDIDATES: usize = 128;

#[derive(Clone, Copy)]
enum TargetSource {
    Standalone,
    Wrapped,
    Markdown,
    Prose,
    WholePath,
}

pub(crate) fn detect_links(text: &str) -> Vec<DetectedLink> {
    detect_links_with_path_kind(text, |path| {
        let metadata = std::path::Path::new(path).metadata().ok()?;
        Some(if metadata.is_dir() {
            DetectedLinkKind::LocalDirectory
        } else {
            DetectedLinkKind::LocalFile
        })
    })
}

fn detect_links_with_path_kind(
    text: &str,
    mut path_kind: impl FnMut(&str) -> Option<DetectedLinkKind>,
) -> Vec<DetectedLink> {
    let mut links = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut candidates = 0;
    for line in text.lines().take(64) {
        let line = line.trim();
        let mut accept = |raw: &str, source| {
            let target = match source {
                TargetSource::Prose => trim_target_wrappers(raw),
                _ => raw,
            };
            if !is_candidate(target) {
                return true;
            }
            if !seen.insert(target.to_owned()) {
                return true;
            }
            if candidates == MAX_CANDIDATES {
                return false;
            }
            candidates += 1;
            if let Some(link) = classify(target, &mut path_kind) {
                links.push(link);
            }
            links.len() < MAX_DETECTED_LINKS && candidates < MAX_CANDIDATES
        };
        let unwrapped = trim_target_wrappers(line);
        let path_line = if looks_like_drive_path(line) || looks_like_unc_path(line) {
            Some(line)
        } else if looks_like_drive_path(unwrapped) || looks_like_unc_path(unwrapped) {
            Some(unwrapped)
        } else {
            None
        };
        if let Some(path) = path_line {
            if !accept(path, TargetSource::WholePath) {
                break;
            }
        } else if !scan_line(line, &mut accept) {
            break;
        }
    }
    links
}

fn is_candidate(target: &str) -> bool {
    if looks_like_drive_path(target) || looks_like_unc_path(target) || has_http_scheme(target) {
        return true;
    }
    let authority = target.split(['/', '?', '#']).next().unwrap_or_default();
    !authority.contains('@')
        && !authority.chars().any(|ch| {
            matches!(
                ch,
                '\\' | '(' | ')' | '[' | ']' | '{' | '}' | '（' | '）' | '【' | '】'
            )
        })
        && authority.contains('.')
}

fn has_http_scheme(target: &str) -> bool {
    target
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || target
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
}

// A line-leading bare path is indivisible, even if a shorter prefix exists.
// All other text is scanned for Markdown, explicit wrappers and prose in order.
fn scan_line(line: &str, accept: &mut impl FnMut(&str, TargetSource) -> bool) -> bool {
    if has_http_scheme(line) && !line.chars().any(char::is_whitespace) {
        return scan_prose(line, line, accept);
    }
    let mut position = 0;
    while position < line.len() {
        let remaining = &line[position..];
        if remaining.starts_with('[')
            && let Some((target, consumed)) = markdown_destination(remaining)
        {
            if !accept(target, TargetSource::Markdown) {
                return false;
            }
            position += consumed;
            continue;
        }
        let first = remaining.chars().next().unwrap_or_default();
        let quote_start = position == 0
            || line[..position].chars().next_back().is_some_and(|ch| {
                ch.is_whitespace() || matches!(ch, '(' | '[' | '{' | '（' | '【')
            });
        if first == '<' || matches!(first, '\'' | '"') && quote_start {
            let close = if first == '<' { '>' } else { first };
            let content = &remaining[first.len_utf8()..];
            if let Some(end) = closing_wrapper(content, close) {
                let target = &content[..end];
                if is_candidate(target) {
                    if !accept(target, TargetSource::Wrapped) {
                        return false;
                    }
                } else if !scan_prose(target, line, accept) {
                    return false;
                }
                position += first.len_utf8() + end + close.len_utf8();
                continue;
            }
        }
        let next = remaining.char_indices().skip(1).find(|(offset, ch)| {
            if matches!(ch, '"' | '<') {
                return true;
            }
            if *ch == '[' && markdown_destination(&remaining[*offset..]).is_some() {
                let preceding_word = remaining[..*offset]
                    .rsplit(char::is_whitespace)
                    .next()
                    .unwrap_or_default();
                return !preceding_word
                    .char_indices()
                    .any(|(index, _)| has_http_scheme(&preceding_word[index..]));
            }
            *ch == '\''
                && remaining[..*offset]
                    .chars()
                    .next_back()
                    .is_some_and(|prev| prev.is_whitespace() || matches!(prev, '(' | '[' | '{'))
        });
        let end = next.map_or(remaining.len(), |(offset, _)| offset);
        if !scan_prose(&remaining[..end], line, accept) {
            return false;
        }
        position += end;
    }
    true
}

fn closing_wrapper(content: &str, close: char) -> Option<usize> {
    let local_path = looks_like_drive_path(content) || looks_like_unc_path(content);
    content.char_indices().find_map(|(index, ch)| {
        if ch != close {
            return None;
        }
        if close == '\'' {
            let after = &content[index + 1..];
            if after.chars().next().is_some_and(char::is_alphanumeric) {
                return None;
            }
            let following = after.trim_start_matches(|ch: char| {
                ch.is_whitespace() || matches!(ch, ',' | ';' | '，' | '；')
            });
            let next_target = following.starts_with(['\'', '"', '<'])
                || has_http_scheme(following)
                || looks_like_drive_path(following)
                || looks_like_unc_path(following);
            if local_path && !next_target && after.contains('\'') {
                return None;
            }
        }
        Some(index)
    })
}

fn markdown_destination(input: &str) -> Option<(&str, usize)> {
    let label_end = input.find("](")?;
    let rest = &input[label_end + 2..];
    let rest = rest.trim_start();
    let skipped = input.len() - rest.len();
    let (target, tail, consumed) = if let Some(angle) = rest.strip_prefix('<') {
        let end = angle.find('>')?;
        (&angle[..end], &angle[end + 1..], end + 2)
    } else {
        let mut depth = 0;
        let mut end = rest.len();
        for (index, ch) in rest.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' if depth == 0 => {
                    end = index;
                    break;
                }
                ')' => depth -= 1,
                ch if ch.is_whitespace() && depth == 0 => {
                    end = index;
                    break;
                }
                _ => {}
            }
        }
        (&rest[..end], &rest[end..], end)
    };
    if target.is_empty() {
        return None;
    }
    let spaces = tail.len() - tail.trim_start().len();
    let after_spaces = &tail[spaces..];
    let (tail, title_bytes) =
        if spaces > 0 && matches!(after_spaces.chars().next(), Some('"' | '\'' | '(')) {
            let opening = after_spaces.chars().next()?;
            let close = if opening == '(' { ')' } else { opening };
            let title = &after_spaces[opening.len_utf8()..];
            let end = title.find(close)?;
            (
                &title[end + close.len_utf8()..],
                opening.len_utf8() + end + close.len_utf8(),
            )
        } else {
            (after_spaces, 0)
        };
    if !tail.starts_with(')') {
        return None;
    }
    Some((target, skipped + consumed + spaces + title_bytes + 1))
}

fn scan_prose(
    fragment: &str,
    line: &str,
    accept: &mut impl FnMut(&str, TargetSource) -> bool,
) -> bool {
    for word in fragment.split_whitespace() {
        let mut remaining = word;
        loop {
            let unsupported_scheme = remaining.split_once("://").is_some_and(|(scheme, _)| {
                !has_http_scheme(remaining)
                    && !looks_like_drive_path(remaining)
                    && scheme.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
                    })
            });
            if unsupported_scheme {
                break;
            }
            let start = if is_candidate(remaining) {
                None
            } else {
                remaining
                    .char_indices()
                    .find(|(index, _)| {
                        let suffix = &remaining[*index..];
                        has_http_scheme(suffix)
                            || looks_like_drive_path(suffix)
                            || looks_like_unc_path(suffix)
                    })
                    .map(|(index, _)| index)
            };
            let target = start.map_or(remaining, |index| &remaining[index..]);
            let boundary = prose_boundary(target, target != line);
            let value = &target[..boundary.map_or(target.len(), |(index, _)| index)];
            let source = if value == line && has_http_scheme(value) {
                TargetSource::Standalone
            } else {
                TargetSource::Prose
            };
            if !accept(value, source) {
                return false;
            }
            let Some((index, separator)) = boundary else {
                break;
            };
            remaining = &target[index + separator.len_utf8()..];
        }
    }
    true
}

fn prose_boundary(target: &str, in_prose: bool) -> Option<(usize, char)> {
    let mut balance = [0_usize; 5];
    for (index, ch) in target.char_indices() {
        let opening = match ch {
            '(' => Some(0),
            '[' => Some(1),
            '{' => Some(2),
            '（' => Some(3),
            '【' => Some(4),
            _ => None,
        };
        let closing = match ch {
            ')' => Some(0),
            ']' => Some(1),
            '}' => Some(2),
            '）' => Some(3),
            '】' => Some(4),
            _ => None,
        };
        if let Some(kind) = opening {
            balance[kind] += 1;
        } else if let Some(kind) = closing {
            if in_prose && balance[kind] == 0 {
                return Some((index, ch));
            }
            balance[kind] = balance[kind].saturating_sub(1);
        } else if matches!(ch, '，' | '。' | '；' | '！' | '？')
            && index + ch.len_utf8() < target.len()
        {
            return Some((index, ch));
        }
    }
    None
}

fn classify(
    candidate: &str,
    path_kind: &mut impl FnMut(&str) -> Option<DetectedLinkKind>,
) -> Option<DetectedLink> {
    if looks_like_drive_path(candidate) || looks_like_unc_path(candidate) {
        let kind = path_kind(candidate)?;
        return Some(DetectedLink {
            display: candidate.to_owned(),
            copy_value: candidate.to_owned(),
            kind,
        });
    }
    if candidate.len() < 4 {
        return None;
    }
    if has_http_scheme(candidate) {
        parse_http_url(candidate)?;
    } else {
        let authority = candidate.split(['/', '?', '#']).next()?;
        if !authority.contains('.') || authority.contains('@') {
            return None;
        }
        let url = parse_http_url(&format!("https://{candidate}"))?;
        if !looks_like_bare_domain(&url) {
            return None;
        }
    }
    Some(DetectedLink {
        display: candidate.to_owned(),
        copy_value: candidate.to_owned(),
        kind: DetectedLinkKind::Web,
    })
}

fn trim_target_wrappers(candidate: &str) -> &str {
    let mut candidate = candidate.trim_start_matches(['(', '[', '{', '（', '【']);
    let mut balance = [0_i64; 5];
    for ch in candidate.chars() {
        match ch {
            '(' => balance[0] += 1,
            ')' => balance[0] -= 1,
            '[' => balance[1] += 1,
            ']' => balance[1] -= 1,
            '{' => balance[2] += 1,
            '}' => balance[2] -= 1,
            '（' => balance[3] += 1,
            '）' => balance[3] -= 1,
            '【' => balance[4] += 1,
            '】' => balance[4] -= 1,
            _ => {}
        }
    }
    while let Some(last) = candidate.chars().next_back() {
        let bracket = match last {
            ')' => Some(0),
            ']' => Some(1),
            '}' => Some(2),
            '）' => Some(3),
            '】' => Some(4),
            _ => None,
        };
        if let Some(index) = bracket {
            if balance[index] >= 0 {
                break;
            }
            balance[index] += 1;
        } else if !matches!(
            last,
            '.' | ',' | ';' | '!' | '?' | '。' | '，' | '；' | '！' | '？'
        ) {
            break;
        }
        candidate = &candidate[..candidate.len() - last.len_utf8()];
    }
    candidate
}
fn parse_http_url(candidate: &str) -> Option<url::Url> {
    let (scheme, rest) = candidate.split_once("://")?;
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        || rest.split(['/', '?', '#']).next()?.is_empty()
        || candidate
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control() || ch == '\\')
    {
        return None;
    }
    let url = url::Url::parse(candidate).ok()?;
    if let url::Host::Domain(host) = url.host()? {
        let valid = host.trim_end_matches('.').split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        });
        if !valid {
            return None;
        }
    }
    Some(url)
}

const KNOWN_TLDS: [&str; 12] = [
    "com", "net", "org", "io", "dev", "app", "cn", "me", "tv", "cc", "xyz", "info",
];

fn looks_like_bare_domain(url: &url::Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let Some(host) = url.domain() else {
        return false;
    };
    let mut parts = host.trim_end_matches('.').rsplit('.');
    let Some(tld) = parts.next() else {
        return false;
    };
    KNOWN_TLDS.contains(&tld)
        && parts
            .next()
            .is_some_and(|label| !label.bytes().all(|byte| byte.is_ascii_digit()))
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
        let links = detect_links(&format!("{executable_text}\n{directory_text}"));

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

    #[test]
    fn existing_paths_with_spaces_and_parentheses_are_kept_whole() {
        let root = tempfile::tempdir().expect("path fixture root");
        let directory = root.path().join("Program Files (x64)");
        std::fs::create_dir(&directory).expect("spaced directory fixture");
        let file = directory.join("My App.exe");
        std::fs::write(&file, b"fixture").expect("spaced file fixture");
        for (path, kind) in [
            (&file, DetectedLinkKind::LocalFile),
            (&directory, DetectedLinkKind::LocalDirectory),
        ] {
            let value = path.to_string_lossy();
            for text in [
                value.to_string(),
                format!("\"{value}\""),
                format!("'{value}'"),
                format!("<{value}>"),
            ] {
                let links = detect_links(&text);
                assert_eq!(links.len(), 1, "whole path: {text}");
                assert_eq!(links[0].copy_value, value);
                assert_eq!(links[0].kind, kind);
            }
        }
        let links = detect_links(&format!(
            "文件 \"{}\" 目录 <{}> https://example.com/download",
            file.display(),
            directory.display()
        ));
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].copy_value, file.to_string_lossy());
        assert_eq!(links[1].copy_value, directory.to_string_lossy());
        assert_eq!(links[2].kind, DetectedLinkKind::Web);
    }

    #[test]
    fn url_parentheses_and_query_are_preserved_inside_prose_wrappers() {
        let url = "https://en.wikipedia.org/wiki/Function_(mathematics)";
        for text in [
            url.to_string(),
            format!("({url})。"),
            format!("[{url}]，"),
            format!("<{url}>"),
        ] {
            let links = detect_links(&text);
            assert_eq!(links.len(), 1, "wrapped URL: {text}");
            assert_eq!(links[0].copy_value, url, "wrapped URL: {text}");
        }
        let url = "https://example.com/search?q=(first)&next=%28second%29#section";
        assert_eq!(detect_links(url)[0].copy_value, url);
    }

    #[test]
    fn incomplete_urls_invalid_hosts_and_email_are_not_web_links() {
        for text in [
            "https://",
            "http://",
            "https:///missing-host",
            "https://?query=test",
            "https://:443/path",
            "https://example.com:bad/path",
            "https://foo..com/path",
            "https://-bad.com/path",
            "user@example.com",
            "not..example.com",
            "bad_.com",
            "ftp://example.com/file",
            "https://bad(host.com)",
        ] {
            assert!(detect_links(text).is_empty(), "not a web target: {text}");
        }
    }

    #[test]
    fn valid_web_hosts_ports_and_bare_domain_queries_keep_original_values() {
        for url in [
            "HTTP://example.com/path",
            "https://localhost:8080/path",
            "http://127.0.0.1:8080/path",
            "http://[::1]/path",
            "https://例子.com/资料",
            "example.com?download=1",
            "example.com:8443/path?download=1",
            "image.routerapi.net/file_(1).zip",
        ] {
            let links = detect_links(url);
            assert_eq!(links.len(), 1, "valid web target: {url}");
            assert_eq!(links[0].copy_value, url);
            assert_eq!(links[0].kind, DetectedLinkKind::Web);
            assert_eq!(links[0].display, url);
            assert_eq!(links[0].kind.label(), "web");
        }
        assert_eq!(
            detect_links("example.com/path),")[0].copy_value,
            "example.com/path"
        );
    }

    #[test]
    fn quoted_paths_preserve_apostrophes_and_do_not_match_a_missing_paths_prefix() {
        let root = tempfile::tempdir().expect("quoted path fixture root");
        let file = root.path().join("O'Brien report.txt");
        std::fs::write(&file, b"fixture").expect("apostrophe file fixture");
        let value = file.to_string_lossy();
        for text in [
            value.to_string(),
            format!("\"{value}\""),
            format!("<{value}>"),
            format!("({value})"),
        ] {
            let links = detect_links(&text);
            assert_eq!(links.len(), 1, "quoted path: {text}");
            assert_eq!(links[0].copy_value, value);
            assert_eq!(links[0].kind, DetectedLinkKind::LocalFile);
        }
        std::fs::write(root.path().join("report"), b"prefix").expect("existing prefix fixture");
        let missing = root.path().join("report draft.txt");
        let links = detect_links(&format!("\"{}\" https://example.com", missing.display()));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].copy_value, "https://example.com");
    }

    #[test]
    fn prose_wrappers_and_url_apostrophes_keep_the_complete_target() {
        let url = "https://example.com/wiki/Function_(math)";
        for text in [
            format!("见({url})"),
            format!("见（{url}）"),
            format!("'{url}'"),
        ] {
            let links = detect_links(&text);
            assert_eq!(links.len(), 1, "prose wrapper: {text}");
            assert_eq!(links[0].copy_value, url);
        }
        let url = "https://example.com/O'Brien?q=it's#part";
        assert_eq!(detect_links(url)[0].copy_value, url);
        let url = "https://example.com/条目（内容）";
        assert_eq!(detect_links(&format!("见（{url}）"))[0].copy_value, url);
    }

    #[test]
    fn duplicate_and_line_limits_apply_after_target_extraction() {
        let url = "https://example.com/path_(1)";
        let links = detect_links(&format!("{url}\n({url})\n<{url}>"));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].copy_value, url);
        assert!(detect_links(&format!("{}{url}", "plain text\n".repeat(64))).is_empty());
        assert_eq!(
            detect_links(&format!("{}{url}", "plain text\n".repeat(63))).len(),
            1
        );
    }
    #[test]
    fn markdown_destinations_replace_labels_and_preserve_balanced_targets() {
        for (text, target) in [
            ("[https://x.test/p](https://x.test/p),", "https://x.test/p"),
            (
                "[下载地址](https://example.com/file.zip)",
                "https://example.com/file.zip",
            ),
            (
                "[https://shown.example/a](https://target.example/b)",
                "https://target.example/b",
            ),
            (
                "[下载](https://example.com/file_(1).zip \"安装包\")",
                "https://example.com/file_(1).zip",
            ),
            (
                "[下载](<https://example.com/file_(1).zip> '安装包')",
                "https://example.com/file_(1).zip",
            ),
        ] {
            let links = detect_links(text);
            assert_eq!(links.len(), 1, "Markdown target: {text}");
            assert_eq!(links[0].copy_value, target);
        }
    }

    #[test]
    fn explicit_and_prose_url_punctuation_have_distinct_boundaries() {
        for (text, target) in [
            ("https://example.com/a!", "https://example.com/a!"),
            ("https://example.com/a;", "https://example.com/a;"),
            ("https://example.com/a?x=1,", "https://example.com/a?x=1,"),
            (
                "https://example.com/?q=你好。",
                "https://example.com/?q=你好。",
            ),
            ("<https://example.com/a!>", "https://example.com/a!"),
            ("\"https://example.com/a;\"", "https://example.com/a;"),
            ("见 https://example.com/a!", "https://example.com/a"),
            (
                "见（https://example.com/a?x=1,）",
                "https://example.com/a?x=1",
            ),
        ] {
            let links = detect_links(text);
            assert_eq!(links.len(), 1, "boundary: {text}");
            assert_eq!(links[0].copy_value, target, "boundary: {text}");
        }
    }

    #[test]
    fn single_quoted_targets_keep_internal_apostrophes() {
        assert_eq!(
            detect_links("'https://example.com/O'Brien'")[0].copy_value,
            "https://example.com/O'Brien"
        );
        let root = tempfile::tempdir().expect("quoted path fixture");
        std::fs::write(root.path().join("O"), b"prefix").expect("prefix fixture");
        let full = root.path().join("O'Brien report.txt");
        std::fs::write(&full, b"full").expect("full fixture");
        let links = detect_links(&format!("'{}'", full.display()));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].copy_value, full.to_string_lossy());
    }

    #[test]
    fn chinese_prose_and_sentence_punctuation_set_url_boundaries() {
        for (text, target) in [
            (
                "下载https://example.com/file.zip",
                "https://example.com/file.zip",
            ),
            (
                "https://example.com/file.zip，下载后解压",
                "https://example.com/file.zip",
            ),
            (
                "<https://example.com/文件，下载>",
                "https://example.com/文件，下载",
            ),
            ("https://example.com/文件，", "https://example.com/文件，"),
        ] {
            let links = detect_links(text);
            assert_eq!(links.len(), 1, "Chinese boundary: {text}");
            assert_eq!(links[0].copy_value, target);
        }
    }

    #[test]
    fn missing_complete_path_never_falls_back_to_existing_prefix() {
        let root = tempfile::tempdir().expect("whole path fixture");
        std::fs::write(root.path().join("report"), b"prefix").expect("prefix fixture");
        let missing = root.path().join("report draft.txt");
        assert!(detect_links(&missing.to_string_lossy()).is_empty());
    }

    #[test]
    fn duplicate_missing_paths_are_checked_once() {
        let mut checks = 0;
        let path = "C:\\missing\\report.txt";
        let links = detect_links_with_path_kind(&format!("{path}\n{path}\n'{path}'"), |_| {
            checks += 1;
            None
        });
        assert!(links.is_empty());
        assert_eq!(checks, 1);
    }

    #[test]
    fn candidate_budget_ignores_plain_words_and_limits_filesystem_checks() {
        let prose = format!("{} https://example.com/late", "word ".repeat(150));
        assert_eq!(
            detect_links(&prose)[0].copy_value,
            "https://example.com/late"
        );
        let paths = (0..130)
            .map(|i| format!("C:\\missing\\{i}.txt"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut checks = 0;
        let links =
            detect_links_with_path_kind(&format!("{paths}\nhttps://example.com/late"), |_| {
                checks += 1;
                None
            });
        assert!(links.is_empty());
        assert_eq!(checks, 64, "line limit also bounds metadata");
        let mut checks = 0;
        let inline = (0..130)
            .map(|i| format!("'C:\\missing\\{i}.txt'"))
            .collect::<Vec<_>>()
            .join(" ");
        let links =
            detect_links_with_path_kind(&format!("{inline} https://example.com/late"), |_| {
                checks += 1;
                None
            });
        assert!(links.is_empty());
        assert_eq!(checks, 128);
        let eight = (0..9)
            .map(|i| format!("https://example.com/{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(detect_links(&eight).len(), 8);
    }

    #[test]
    fn literal_url_brackets_and_multiple_prose_targets_keep_their_boundaries() {
        for url in [
            "http://[::1]/path",
            "https://example.com/a[1].zip",
            "https://example.com/[label](https://target.example/a)",
        ] {
            assert_eq!(detect_links(url)[0].copy_value, url);
        }
        let links =
            detect_links("下载https://example.com/one，please另一个https://example.com/two；继续");
        assert_eq!(
            links
                .iter()
                .map(|link| link.copy_value.as_str())
                .collect::<Vec<_>>(),
            vec!["https://example.com/one", "https://example.com/two"]
        );
        let url = "https://example.com/?q=你好。世界";
        assert_eq!(detect_links(&format!("<{url}>"))[0].copy_value, url);
    }

    #[test]
    fn repeated_negative_candidates_do_not_exhaust_the_distinct_budget() {
        let text = format!(
            "{} https://example.com/late",
            "'C:\\missing\\same.txt' ".repeat(200)
        );
        let mut checks = 0;
        let links = detect_links_with_path_kind(&text, |_| {
            checks += 1;
            None
        });
        assert_eq!(checks, 1);
        assert_eq!(links[0].copy_value, "https://example.com/late");
    }

    #[test]
    fn prose_closing_wrappers_end_the_target_without_cutting_balanced_url_brackets() {
        for (text, expected) in [
            (
                "请看（https://example.com/a）然后继续",
                "https://example.com/a",
            ),
            (
                "请看(https://example.com/Function_(math))then",
                "https://example.com/Function_(math)",
            ),
            ("请看[http://[::1]/a]然后继续", "http://[::1]/a"),
        ] {
            let values = detect_links(text)
                .into_iter()
                .map(|link| link.copy_value)
                .collect::<Vec<_>>();
            assert_eq!(values, vec![expected], "prose boundary: {text}");
        }
        let raw = "https://example.com/a)literal";
        assert_eq!(detect_links(raw)[0].copy_value, raw);
    }

    #[test]
    fn single_quoted_possessive_paths_are_complete_and_multiple_paths_stay_separate() {
        let root = tempfile::tempdir().expect("possessive path fixture");
        std::fs::write(root.path().join("James"), b"prefix").expect("existing prefix");
        let full = root.path().join("James' report.txt");
        let other = root.path().join("Other report.txt");
        std::fs::write(&full, b"full").expect("full path");
        std::fs::write(&other, b"other").expect("other path");
        let links = detect_links(&format!("'{}'", full.display()));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].copy_value, full.to_string_lossy());
        let links = detect_links(&format!("'{}' '{}'", full.display(), other.display()));
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].copy_value, full.to_string_lossy());
        assert_eq!(links[1].copy_value, other.to_string_lossy());
    }

    #[test]
    fn bare_url_path_and_query_at_signs_are_not_email_authorities() {
        let target = "example.com/path@part?email=user@example.org";
        let links = detect_links(target);
        assert_eq!(
            links
                .into_iter()
                .map(|link| link.copy_value)
                .collect::<Vec<_>>(),
            vec![target]
        );
        assert!(detect_links("user@example.com").is_empty());
    }

    #[test]
    fn unsupported_url_schemes_never_become_drive_path_probes() {
        let mut checks = 0;
        let links =
            detect_links_with_path_kind("ftp://example.com/file file:///C:/file.txt", |_| {
                checks += 1;
                Some(DetectedLinkKind::LocalFile)
            });
        assert_eq!(
            checks, 0,
            "unsupported schemes must not probe drive suffixes"
        );
        assert!(links.is_empty());
    }
}
