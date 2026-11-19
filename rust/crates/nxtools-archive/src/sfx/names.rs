use super::{Error, Kind, Result};
use std::collections::BTreeMap;

pub(super) fn key(name: &str) -> String {
    name.trim_end_matches('/').to_uppercase()
}
pub(super) fn validate<'a>(
    names: impl Iterator<Item = (&'a str, bool)>,
    limit: usize,
) -> Result<()> {
    let mut catalog = BTreeMap::new();
    for (name, directory) in names {
        if name.len() > limit {
            return Err(Error::new(Kind::Limit));
        }
        if name.is_empty()
            || name.ends_with('/') != directory
            || name
                .chars()
                .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
        {
            return Err(Error::new(Kind::UnsafeName));
        }
        let path = if directory {
            &name[..name.len() - 1]
        } else {
            name
        };
        for component in path.split('/') {
            let stem = component
                .split('.')
                .next()
                .unwrap_or_default()
                .to_uppercase();
            let numbered = stem
                .strip_prefix("COM")
                .or_else(|| stem.strip_prefix("LPT"))
                .is_some_and(|n| {
                    matches!(
                        n,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                });
            if component.is_empty()
                || component == "."
                || component == ".."
                || component.ends_with(['.', ' '])
                || ["CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$"]
                    .contains(&stem.as_str())
                || numbered
            {
                return Err(Error::new(Kind::UnsafeName));
            }
        }
        if catalog.insert(key(name), directory).is_some() {
            return Err(Error::new(Kind::Collision));
        }
    }
    for name in catalog.keys() {
        for (offset, _) in name.match_indices('/') {
            if catalog.get(&name[..offset]) == Some(&false) {
                return Err(Error::new(Kind::Collision));
            }
        }
    }
    Ok(())
}
