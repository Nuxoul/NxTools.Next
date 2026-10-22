//! Bounded runtime-only search data. No paths, persisted format or delta authority.
use std::collections::HashMap;

use nxtools_finder::FileQuery;

const MAX_ROWS: usize = 1_000_000;
const MAX_GROUP_BYTES: usize = 32 * 1024 * 1024;
const MAX_POSTINGS: usize = 2_000_000;
const MAX_TERMS: usize = 65_536;
pub(crate) const MAX_CANDIDATE_GROUPS: usize = 32_768;

#[derive(Debug)]
pub(crate) struct NameGroup {
    pub representative: usize,
    pub rows: Vec<u32>,
}

#[derive(Debug)]
pub(crate) struct BaseSearchData {
    pub groups: Vec<NameGroup>,
    postings: Option<HashMap<u32, Vec<u32>>>,
    non_ascii: Vec<u32>,
}

fn gram(bytes: &[u8]) -> u32 {
    u32::from(bytes[0].to_ascii_lowercase()) << 16
        | u32::from(bytes[1].to_ascii_lowercase()) << 8
        | u32::from(bytes[2].to_ascii_lowercase())
}

impl BaseSearchData {
    pub fn build<'a>(count: usize, name_at: impl Fn(usize) -> Option<&'a str>) -> Option<Self> {
        if count > MAX_ROWS {
            return None;
        }
        let mut groups = Vec::<NameGroup>::new();
        // Borrow the immutable hot name section, never clone a name/path string.
        let mut ids = HashMap::<&str, usize>::new();
        for row in 0..count {
            let name = name_at(row)?;
            let id = if let Some(id) = ids.get(name) {
                *id
            } else {
                // Conservative allowance for groups, temporary hash table and Vec growth.
                if (groups.len() + 1) * 128 + count * 8 > MAX_GROUP_BYTES {
                    return None;
                }
                let id = groups.len();
                ids.insert(name, id);
                groups.push(NameGroup {
                    representative: row,
                    rows: Vec::new(),
                });
                id
            };
            groups[id].rows.push(row as u32);
        }
        drop(ids);
        let mut postings = HashMap::<u32, Vec<u32>>::new();
        let mut posting_count = 0;
        let mut non_ascii = Vec::new();
        let mut scratch = Vec::new();
        let mut indexed = true;
        for (id, group) in groups.iter().enumerate() {
            let name = name_at(group.representative)?;
            if !name.is_ascii() {
                non_ascii.push(id as u32);
                continue;
            }
            if !indexed {
                continue;
            }
            // Bound scratch/work even for malformed synthetic long names.
            if name.len() > 1024 {
                indexed = false;
                postings.clear();
                continue;
            }
            scratch.clear();
            scratch.extend(name.as_bytes().windows(3).map(gram));
            scratch.sort_unstable();
            scratch.dedup();
            for key in &scratch {
                if posting_count >= MAX_POSTINGS
                    || (!postings.contains_key(key) && postings.len() >= MAX_TERMS)
                {
                    indexed = false;
                    postings.clear();
                    break;
                }
                postings.entry(*key).or_default().push(id as u32);
                posting_count += 1;
            }
        }
        Some(Self {
            groups,
            postings: indexed.then_some(postings),
            non_ascii,
        })
    }

    /// Allocated payload capacity, excluding allocator headers and HashMap control bytes.
    #[cfg(test)]
    pub fn payload_capacity_bytes(&self) -> usize {
        self.groups.capacity() * std::mem::size_of::<NameGroup>()
            + self
                .groups
                .iter()
                .map(|group| group.rows.capacity() * 4)
                .sum::<usize>()
            + self.non_ascii.capacity() * 4
            + self.postings.as_ref().map_or(0, |postings| {
                postings.capacity() * std::mem::size_of::<(u32, Vec<u32>)>()
                    + postings
                        .values()
                        .map(|rows| rows.capacity() * 4)
                        .sum::<usize>()
            })
    }

    /// Rarest posting is a superset; normal scoring verifies the whole query.
    /// Never allocate/visit an unbounded wide posting in the interactive path.
    pub fn candidates(&self, query: &FileQuery) -> Option<Vec<usize>> {
        let needle = query.required_ascii_substring()?;
        let postings = self.postings.as_ref()?;
        let mut rarest: &[u32] = &[];
        let mut found = false;
        for bytes in needle.as_bytes().windows(3) {
            let posting = postings.get(&gram(bytes)).map(Vec::as_slice).unwrap_or(&[]);
            if !found || posting.len() < rarest.len() {
                rarest = posting;
                found = true;
            }
        }
        let count = rarest.len().saturating_add(self.non_ascii.len());
        if count > MAX_CANDIDATE_GROUPS || count > self.groups.len() / 4 {
            return None;
        }
        let mut result = Vec::with_capacity(count);
        result.extend(rarest.iter().chain(&self.non_ascii).map(|id| *id as usize));
        result.sort_unstable();
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r8_auxiliary_candidates_are_bounded_safe_supersets_and_unsupported_queries_fall_back() {
        let mut names = (0..1000)
            .map(|id| format!("file-{id:05}.txt"))
            .collect::<Vec<_>>();
        names.extend([
            "rare-needle.txt".into(),
            "Kelvin.txt".into(),
            "re-port.txt".into(),
        ]);
        let data = BaseSearchData::build(names.len(), |row| Some(names[row].as_str()))
            .expect("bounded index");
        for raw in ["'needle", "'kelvin", "^rare", "needle$", "'not-present"] {
            let query = FileQuery::parse(raw);
            let candidates = data.candidates(&query).expect("selective query");
            assert!(candidates.len() <= MAX_CANDIDATE_GROUPS);
            for (row, name) in names.iter().enumerate() {
                if query.score_name(name).is_some() {
                    assert!(candidates.contains(&row), "lost {name} for {raw}");
                }
            }
        }
        for raw in [
            "r",
            "'re",
            "report",
            "rpt",
            "'计",
            "'file",
            "'needle | notes",
            "'needle !old",
            "'needle txt",
            r"C:\needle",
            "::Work 'needle",
        ] {
            assert!(data.candidates(&FileQuery::parse(raw)).is_none(), "{raw}");
        }
        assert!(BaseSearchData::build(MAX_ROWS + 1, |_| Some("x")).is_none());
        let duplicates = BaseSearchData::build(100, |_| Some("duplicate")).expect("duplicate data");
        assert_eq!(duplicates.groups.len(), 1);
        assert_eq!(duplicates.groups[0].rows.len(), 100);
        let long_name = "a".repeat(1025);
        let unindexed =
            BaseSearchData::build(1, |_| Some(long_name.as_str())).expect("groups retained");
        assert!(unindexed.candidates(&FileQuery::parse("'aaa")).is_none());
    }
}
