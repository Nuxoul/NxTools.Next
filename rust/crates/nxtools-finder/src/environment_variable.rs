use std::collections::HashSet;
use std::fs;
use std::path::Path;

const MAX_ENVIRONMENT_VARIABLE_NAME_BYTES: usize = 128;
const MAX_ENVIRONMENT_VARIABLE_TARGETS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvironmentVariableValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvironmentVariableTargetKind {
    Directory,
    File,
    CopyValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvironmentVariableTarget {
    pub value: String,
    pub kind: EnvironmentVariableTargetKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedEnvironmentVariable {
    pub name: String,
    pub targets: Vec<EnvironmentVariableTarget>,
}

pub(crate) fn environment_variable_name(query: &str) -> Option<String> {
    let query = query.trim();
    let name = query.strip_prefix('%')?.strip_suffix('%')?;
    if name.is_empty()
        || name.len() > MAX_ENVIRONMENT_VARIABLE_NAME_BYTES
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }

    Some(name.to_ascii_uppercase())
}

pub(crate) fn environment_variable_value(query: &str) -> Option<EnvironmentVariableValue> {
    let name = environment_variable_name(query)?;

    let (_, value) = std::env::vars_os().find(|(candidate, _)| {
        candidate
            .to_str()
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(&name))
    })?;
    let value = value.to_string_lossy().into_owned();
    if value.is_empty() {
        return None;
    }

    Some(EnvironmentVariableValue { name, value })
}

pub(crate) fn environment_variable_targets(query: &str) -> Option<ResolvedEnvironmentVariable> {
    let variable = environment_variable_value(query)?;
    let targets = environment_variable_targets_from_value(&variable.value);
    Some(ResolvedEnvironmentVariable {
        name: variable.name,
        targets,
    })
}

pub(crate) fn environment_variable_targets_from_value(
    value: &str,
) -> Vec<EnvironmentVariableTarget> {
    let mut seen = HashSet::new();
    value
        .split(';')
        .filter_map(|part| {
            let part = part.trim().trim_matches('"').trim();
            if part.is_empty() || !seen.insert(part.replace('/', "\\").to_ascii_lowercase()) {
                return None;
            }
            let kind = fs::metadata(Path::new(part))
                .map(|metadata| {
                    if metadata.is_dir() {
                        EnvironmentVariableTargetKind::Directory
                    } else if metadata.is_file() {
                        EnvironmentVariableTargetKind::File
                    } else {
                        EnvironmentVariableTargetKind::CopyValue
                    }
                })
                .unwrap_or(EnvironmentVariableTargetKind::CopyValue);
            Some(EnvironmentVariableTarget {
                value: part.to_owned(),
                kind,
            })
        })
        .take(MAX_ENVIRONMENT_VARIABLE_TARGETS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parser_only_accepts_one_complete_variable_name() {
        for query in [
            "%",
            "%%",
            "%PATH",
            "PATH%",
            "%PATH%%",
            "%PATH-NAME%",
            "%PATH NAME%",
            r"C:\%PATH%\file.txt",
        ] {
            assert!(environment_variable_name(query).is_none(), "{query}");
        }
    }

    #[test]
    fn expanded_values_split_deduplicate_and_classify_filesystem_targets() {
        let root = tempdir().expect("temporary environment paths");
        let directory = root.path().join("Tools");
        let file = root.path().join("tool.exe");
        let missing = root.path().join("Missing");
        fs::create_dir(&directory).expect("create directory target");
        fs::write(&file, b"fixture").expect("create file target");
        let expanded = format!(
            "{};\"{}\";{};{}",
            directory.display(),
            file.display(),
            missing.display(),
            directory.display()
        );

        let targets = environment_variable_targets_from_value(&expanded);

        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].kind, EnvironmentVariableTargetKind::Directory);
        assert_eq!(targets[1].kind, EnvironmentVariableTargetKind::File);
        assert_eq!(targets[2].kind, EnvironmentVariableTargetKind::CopyValue);
        assert_eq!(targets[2].value, missing.to_string_lossy());
    }

    #[test]
    fn non_path_environment_value_remains_a_copy_fallback() {
        let targets = environment_variable_targets_from_value("plain-value");

        assert_eq!(
            targets,
            vec![EnvironmentVariableTarget {
                value: "plain-value".into(),
                kind: EnvironmentVariableTargetKind::CopyValue,
            }]
        );
    }

    #[test]
    fn multi_value_expansion_is_bounded() {
        let expanded = (0..80)
            .map(|index| format!("missing-{index}"))
            .collect::<Vec<_>>()
            .join(";");

        let targets = environment_variable_targets_from_value(&expanded);

        assert_eq!(targets.len(), MAX_ENVIRONMENT_VARIABLE_TARGETS);
    }
}
