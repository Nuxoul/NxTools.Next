//! Bounded, host-neutral matching over an already loaded archive catalog.
use globset::{GlobBuilder, GlobMatcher};

/// Validation errors contain no user-supplied query text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberQueryError {
    Invalid,
}

/// Compiled once per query change; callers retain the original member ordinal as authority.
pub struct MemberQuery {
    needle: String,
    pattern: Option<GlobMatcher>,
    full_path: bool,
}

impl MemberQuery {
    pub fn new(query: &str, wildcard: bool) -> Result<Self, MemberQueryError> {
        if query.len() > 1024 || query.chars().any(char::is_control) {
            return Err(MemberQueryError::Invalid);
        }
        let normalized = query.replace('\\', "/");
        let full_path = normalized.contains('/');
        let pattern = if wildcard && !normalized.is_empty() {
            Some(
                GlobBuilder::new(&normalized)
                    .literal_separator(true)
                    .case_insensitive(true)
                    .build()
                    .map_err(|_| MemberQueryError::Invalid)?
                    .compile_matcher(),
            )
        } else {
            None
        };
        Ok(Self {
            needle: normalized.to_lowercase(),
            pattern,
            full_path,
        })
    }

    pub fn matches(&self, member_path: &str) -> bool {
        if self.needle.is_empty() {
            return true;
        }
        let normalized = member_path.replace('\\', "/");
        let path = normalized.trim_end_matches('/');
        let candidate = if self.full_path {
            path
        } else {
            path.rsplit('/').next().unwrap_or(path)
        };
        if let Some(pattern) = &self.pattern {
            pattern.is_match(candidate)
        } else {
            path.to_lowercase().contains(&self.needle)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_member_matching_is_bounded_and_normalizes_both_separators() {
        assert!(
            MemberQuery::new("", true)
                .expect("empty all")
                .matches("a\\b.txt")
        );
        assert!(
            MemberQuery::new("B.Tx", false)
                .expect("substring")
                .matches("a\\b.txt")
        );
        assert!(
            MemberQuery::new("a\\b", false)
                .expect("path")
                .matches("a/b.txt")
        );
        assert!(
            MemberQuery::new("folder", false)
                .expect("path substring")
                .matches("folder/file.txt")
        );
        assert!(
            MemberQuery::new("*.TXT", true)
                .expect("basename glob")
                .matches("a\\b.txt")
        );
        assert!(
            !MemberQuery::new("*.txt", true)
                .expect("basename glob")
                .matches("a/b.log")
        );
        assert!(
            MemberQuery::new("a\\*.txt", true)
                .expect("path glob")
                .matches("a/b.txt")
        );
        assert!(
            !MemberQuery::new("a/*.txt", true)
                .expect("literal separator")
                .matches("a/b/c.txt")
        );
        assert_eq!(
            MemberQuery::new("[", true).err(),
            Some(MemberQueryError::Invalid)
        );
        assert_eq!(
            MemberQuery::new("a\0b", false).err(),
            Some(MemberQueryError::Invalid)
        );
        assert_eq!(
            MemberQuery::new(&"a".repeat(1025), false).err(),
            Some(MemberQueryError::Invalid)
        );
    }
}
