#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchKind {
    Fuzzy,
    Exact,
    ExactWord,
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryTerm {
    text: String,
    kind: MatchKind,
    excluded: bool,
    case_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileQuery {
    alternatives: Vec<Vec<QueryTerm>>,
    required_name_masks: Vec<u64>,
    drive: Option<char>,
    path_pattern: Option<String>,
    directory_scope: Option<String>,
    parent_terms: Vec<QueryTerm>,
    bypass_exclusions: bool,
}

impl FileQuery {
    pub fn parse(query: &str) -> Self {
        let trimmed = query.trim();
        let (bypass_exclusions, trimmed) = trimmed
            .strip_prefix('*')
            .map_or((false, trimmed), |query| (true, query.trim_start()));
        if trimmed.contains(['\\', '/']) && !trimmed.contains("::") {
            let normalized_path = normalize_path(trimmed);
            let drive = normalized_path.get(..2).and_then(parse_drive_selector);
            let directory_scope = normalized_path
                .ends_with('\\')
                .then(|| normalized_path.trim_end_matches('\\').to_owned());
            return Self {
                alternatives: Vec::new(),
                required_name_masks: Vec::new(),
                drive,
                path_pattern: directory_scope.is_none().then_some(normalized_path),
                directory_scope,
                parent_terms: Vec::new(),
                bypass_exclusions,
            };
        }
        let mut parts = tokenize_query(trimmed).into_iter().peekable();
        let drive = parts
            .peek()
            .and_then(|part| parse_drive_selector(&part.text))
            .inspect(|_| {
                parts.next();
            });
        let mut alternatives = vec![Vec::new()];
        let mut parent_terms = Vec::new();
        for token in parts {
            let raw = token.text.as_str();
            if raw == "|" && !token.double_quoted && !token.word_boundary_exact {
                if alternatives.last().is_some_and(|terms| !terms.is_empty()) {
                    alternatives.push(Vec::new());
                }
            } else if let Some(filters) = raw.strip_prefix("::") {
                parent_terms.extend(
                    filters
                        .split(',')
                        .map(|filter| filter.trim_start_matches(':'))
                        .filter_map(|filter| {
                            parse_term(filter, token.double_quoted, token.word_boundary_exact)
                        }),
                );
            } else if let Some(term) =
                parse_term(raw, token.double_quoted, token.word_boundary_exact)
            {
                alternatives
                    .last_mut()
                    .expect("initial query branch")
                    .push(term);
            }
        }
        alternatives.retain(|terms| !terms.is_empty());
        let required_name_masks = alternatives
            .iter()
            .map(|terms| {
                terms
                    .iter()
                    .filter(|term| !term.excluded)
                    .fold(0, |mask, term| mask | search_char_mask(&term.text))
            })
            .collect();
        Self {
            alternatives,
            required_name_masks,
            drive,
            path_pattern: None,
            directory_scope: None,
            parent_terms,
            bypass_exclusions,
        }
    }

    pub fn bypasses_exclusions(&self) -> bool {
        self.bypass_exclusions
    }

    pub fn is_name_only(&self) -> bool {
        self.drive.is_none()
            && self.path_pattern.is_none()
            && self.directory_scope.is_none()
            && self.parent_terms.is_empty()
    }

    pub fn name_mask_may_match(&self, candidate_mask: u64) -> bool {
        self.required_name_masks.is_empty()
            || self
                .required_name_masks
                .iter()
                .any(|required| candidate_mask & required == *required)
    }

    pub fn score_name(&self, name: &str) -> Option<i64> {
        if !self.is_name_only() {
            return None;
        }
        self.alternatives
            .iter()
            .filter_map(|terms| score_terms(terms, name))
            .max()
    }

    pub fn score(&self, name: &str, path: &str) -> Option<i64> {
        if self.drive.is_some_and(|drive| {
            path.as_bytes()
                .first()
                .is_none_or(|candidate| !candidate.eq_ignore_ascii_case(&(drive as u8)))
                || path.as_bytes().get(1) != Some(&b':')
        }) {
            return None;
        }
        if let Some(pattern) = self.path_pattern.as_deref() {
            let path = normalize_path(path);
            return path.find(pattern).map(|offset| 4_000 - offset as i64);
        }
        if let Some(directory) = self.directory_scope.as_deref() {
            let path = normalize_path(path);
            return path
                .rsplit_once('\\')
                .is_some_and(|(parent, _)| parent == directory)
                .then_some(4_000);
        }
        if !self.parent_terms.is_empty() {
            score_terms(&self.parent_terms, path)?;
        }
        self.alternatives
            .iter()
            .filter_map(|terms| score_terms(terms, name))
            .max()
    }
}

pub fn search_char_mask(text: &str) -> u64 {
    text.chars().fold(0_u64, |mut mask, character| {
        for character in character.to_lowercase() {
            let bit = if character.is_ascii_lowercase() {
                character as u32 - 'a' as u32
            } else if character.is_ascii_digit() {
                26 + character as u32 - '0' as u32
            } else if character.is_ascii() {
                36 + (character as u32 % 27)
            } else {
                63
            };
            mask |= 1_u64 << bit;
        }
        mask
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryToken {
    text: String,
    double_quoted: bool,
    word_boundary_exact: bool,
}

fn tokenize_query(query: &str) -> Vec<QueryToken> {
    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut double_quoted = false;
    let mut single_quoted = false;
    let mut token_was_double_quoted = false;
    let mut token_was_word_boundary_exact = false;
    for character in query.chars() {
        match character {
            '"' if !single_quoted => {
                double_quoted = !double_quoted;
                token_was_double_quoted = true;
            }
            '\'' if !double_quoted && text.is_empty() && !single_quoted => {
                single_quoted = true;
                text.push('\'');
            }
            '\'' if single_quoted => {
                single_quoted = false;
                text.remove(0);
                token_was_word_boundary_exact = true;
            }
            character if character.is_whitespace() && !double_quoted && !single_quoted => {
                if !text.is_empty() {
                    tokens.push(QueryToken {
                        text: std::mem::take(&mut text),
                        double_quoted: token_was_double_quoted,
                        word_boundary_exact: token_was_word_boundary_exact,
                    });
                    token_was_double_quoted = false;
                    token_was_word_boundary_exact = false;
                }
            }
            _ => text.push(character),
        }
    }
    if !text.is_empty() {
        tokens.push(QueryToken {
            text,
            double_quoted: token_was_double_quoted,
            word_boundary_exact: token_was_word_boundary_exact,
        });
    }
    tokens
}

fn normalize_path(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

fn parse_drive_selector(part: &str) -> Option<char> {
    let mut characters = part.chars();
    let drive = characters.next()?;
    (drive.is_ascii_alphabetic() && characters.next() == Some(':') && characters.next().is_none())
        .then_some(drive)
}

fn score_terms(terms: &[QueryTerm], name: &str) -> Option<i64> {
    let mut score = 0_i64;
    let mut folded_name = None;
    for term in terms {
        let term_score = if term.case_sensitive {
            score_term(term.kind, &term.text, name)
        } else if term.text.is_ascii() && name.is_ascii() {
            score_ascii_case_insensitive(term.kind, &term.text, name)
        } else {
            let candidate = folded_name.get_or_insert_with(|| name.to_lowercase());
            score_term(term.kind, &term.text, candidate)
        };
        if term.excluded {
            if term_score.is_some() {
                return None;
            }
        } else {
            score += term_score?;
        }
    }
    Some(score)
}

fn score_term(kind: MatchKind, pattern: &str, candidate: &str) -> Option<i64> {
    match kind {
        MatchKind::Fuzzy => fuzzy_subsequence_score(pattern, candidate),
        MatchKind::Exact => candidate.find(pattern).map(|offset| 5_000 - offset as i64),
        MatchKind::ExactWord => exact_word_score(pattern, candidate),
        MatchKind::Prefix => candidate.starts_with(pattern).then_some(6_500),
        MatchKind::Suffix => candidate.ends_with(pattern).then_some(6_500),
    }
}

fn score_ascii_case_insensitive(kind: MatchKind, pattern: &str, candidate: &str) -> Option<i64> {
    match kind {
        MatchKind::Fuzzy => fuzzy_subsequence_score_ascii(pattern, candidate),
        MatchKind::Exact => {
            find_ascii_case_insensitive(candidate, pattern).map(|offset| 5_000 - offset as i64)
        }
        MatchKind::ExactWord => exact_word_score_ascii(pattern, candidate),
        MatchKind::Prefix => candidate
            .as_bytes()
            .get(..pattern.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(pattern.as_bytes()))
            .then_some(6_500),
        MatchKind::Suffix => candidate
            .as_bytes()
            .get(candidate.len().saturating_sub(pattern.len())..)
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case(pattern.as_bytes()))
            .then_some(6_500),
    }
}

fn find_ascii_case_insensitive(candidate: &str, pattern: &str) -> Option<usize> {
    let pattern = pattern.as_bytes();
    candidate
        .as_bytes()
        .windows(pattern.len())
        .position(|window| window.eq_ignore_ascii_case(pattern))
}

fn exact_word_score_ascii(pattern: &str, candidate: &str) -> Option<i64> {
    let bytes = candidate.as_bytes();
    let mut offset = 0;
    while let Some(relative) = find_ascii_case_insensitive(&candidate[offset..], pattern) {
        let match_offset = offset + relative;
        let before = match_offset
            .checked_sub(1)
            .and_then(|index| bytes.get(index));
        let after = bytes.get(match_offset + pattern.len());
        if before.is_none_or(|byte| !byte.is_ascii_alphanumeric())
            && after.is_none_or(|byte| !byte.is_ascii_alphanumeric())
        {
            return Some(5_500 - match_offset as i64);
        }
        offset = match_offset + 1;
    }
    None
}

fn fuzzy_subsequence_score_ascii(pattern: &str, candidate: &str) -> Option<i64> {
    if pattern.is_empty() {
        return Some(0);
    }
    if candidate.eq_ignore_ascii_case(pattern) {
        return Some(7_000);
    }
    if candidate
        .as_bytes()
        .get(..pattern.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(pattern.as_bytes()))
    {
        return Some(6_000 - candidate.len().saturating_sub(pattern.len()) as i64);
    }
    if let Some(offset) = find_ascii_case_insensitive(candidate, pattern) {
        return Some(5_000 - offset as i64);
    }
    let mut candidate = candidate.as_bytes().iter().enumerate();
    let mut first = None;
    let mut last = 0;
    for query_byte in pattern.bytes() {
        let (index, _) = candidate
            .by_ref()
            .find(|(_, byte)| byte.eq_ignore_ascii_case(&query_byte))?;
        first.get_or_insert(index);
        last = index;
    }
    Some(2_000 - last.saturating_sub(first.unwrap_or_default()) as i64)
}

fn parse_term(raw: &str, force_exact: bool, force_word_boundary_exact: bool) -> Option<QueryTerm> {
    let (excluded, raw) = raw
        .strip_prefix('!')
        .map_or((false, raw), |term| (true, term));
    if raw.is_empty() {
        return None;
    }
    let (kind, text) = if force_word_boundary_exact {
        (MatchKind::ExactWord, raw)
    } else if force_exact {
        (MatchKind::Exact, raw)
    } else if let Some(term) = raw.strip_prefix('\'') {
        (MatchKind::Exact, term)
    } else if let Some(term) = raw.strip_prefix('^') {
        (MatchKind::Prefix, term)
    } else if let Some(term) = raw.strip_suffix('$') {
        (MatchKind::Suffix, term)
    } else {
        (MatchKind::Fuzzy, raw)
    };
    // Quick launch queries are always case insensitive — an uppercase letter in
    // the query must not switch the matcher into case sensitive mode (user report).
    let case_sensitive = false;
    (!text.is_empty()).then(|| QueryTerm {
        text: text.to_lowercase(),
        kind,
        excluded,
        case_sensitive,
    })
}

fn exact_word_score(pattern: &str, candidate: &str) -> Option<i64> {
    candidate.match_indices(pattern).find_map(|(offset, _)| {
        let before = candidate[..offset].chars().next_back();
        let after = candidate[offset + pattern.len()..].chars().next();
        let bounded_before = before.is_none_or(|character| !character.is_alphanumeric());
        let bounded_after = after.is_none_or(|character| !character.is_alphanumeric());
        (bounded_before && bounded_after).then_some(5_500 - offset as i64)
    })
}

fn fuzzy_subsequence_score(pattern: &str, candidate: &str) -> Option<i64> {
    if pattern.is_empty() {
        return Some(0);
    }
    if candidate == pattern {
        return Some(7_000);
    }
    if candidate.starts_with(pattern) {
        return Some(6_000 - candidate.len().saturating_sub(pattern.len()) as i64);
    }
    if let Some(offset) = candidate.find(pattern) {
        return Some(5_000 - offset as i64);
    }
    let mut candidate_indices = candidate.char_indices();
    let mut first = None;
    let mut last = 0;
    for query_character in pattern.chars() {
        let (index, _) = candidate_indices
            .by_ref()
            .find(|(_, character)| *character == query_character)?;
        first.get_or_insert(index);
        last = index;
    }
    let span = last.saturating_sub(first.unwrap_or_default());
    Some(2_000 - span as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_mask_is_a_safe_prefilter_for_ascii_and_alternatives() {
        let query = FileQuery::parse("gram | zed");

        assert!(query.name_mask_may_match(search_char_mask("AyuGram.exe")));
        assert!(query.name_mask_may_match(search_char_mask("zed.exe")));
        assert!(!query.name_mask_may_match(search_char_mask("notes.txt")));
    }

    #[test]
    fn name_only_scoring_avoids_path_materialization_for_normal_queries() {
        let normal = FileQuery::parse("gram");
        let scoped = FileQuery::parse(r"C:\Apps\gram");

        assert!(normal.is_name_only());
        assert!(normal.score_name("AyuGram.exe").is_some());
        assert!(!scoped.is_name_only());
        assert!(scoped.score_name("AyuGram.exe").is_none());
    }

    #[test]
    fn uppercase_queries_stay_case_insensitive() {
        // The user's report: quick launch must never become case sensitive,
        // including when the query itself contains uppercase letters.
        let query = FileQuery::parse("HUM");

        assert!(query.score_name("hume").is_some());
        assert!(query.score_name("Humanoid").is_some());
        assert!(query.score_name("Helium").is_some());
        assert!(
            FileQuery::parse("HeLiUm")
                .score_name("helium.exe")
                .is_some()
        );
    }
}
