use crate::{Manifest, PackageError, valid_id, valid_text};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntime {
    pub program: ScriptProgram,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}
impl ScriptRuntime {
    pub fn effective_timeout_ms(&self, feature: &str, entry: &str) -> u32 {
        let (default, cap) = match (feature, entry) {
            ("quick_launch", _) => (3_000, 10_000),
            ("downloads", "resolve") => (10_000, 30_000),
            _ => (30_000, 120_000),
        };
        self.timeout_ms.unwrap_or(default).min(cap)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptProgram {
    Python,
    Powershell,
    Pwsh,
    Node,
    Exe,
}

impl ScriptProgram {
    /// Stable serde/config key. `exe` is display-only, never a configurable default.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Powershell => "powershell",
            Self::Pwsh => "pwsh",
            Self::Node => "node",
            Self::Exe => "exe",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Setting {
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(flatten)]
    pub field: SettingType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SettingType {
    Text {
        #[serde(default)]
        default: String,
    },
    Bool {
        #[serde(default)]
        default: bool,
    },
    Number {
        default: i64,
        min: i64,
        max: i64,
    },
    Choice {
        default: String,
        options: Vec<String>,
    },
    File {
        #[serde(default)]
        default: String,
    },
    Folder {
        #[serde(default)]
        default: String,
    },
    Password,
}

impl<'de> Deserialize<'de> for Setting {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let crate::strict_json::StrictValue(value) = Deserialize::deserialize(deserializer)?;
        let Value::Object(mut fields) = value else {
            return Err(D::Error::custom("setting must be an object"));
        };
        let key = fields
            .remove("key")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| D::Error::custom("setting key required"))?;
        let title = fields
            .remove("title")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| D::Error::custom("setting title required"))?;
        let description = match fields.remove("description") {
            None => String::new(),
            Some(Value::String(value)) => value,
            Some(_) => return Err(D::Error::custom("invalid setting description")),
        };
        let allowed: &[&str] = match fields.get("type").and_then(Value::as_str) {
            Some("text" | "bool" | "file" | "folder") => &["type", "default"],
            Some("number") => &["type", "default", "min", "max"],
            Some("choice") => &["type", "default", "options"],
            Some("password") => &["type"],
            _ => return Err(D::Error::custom("unsupported setting type")),
        };
        if fields.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err(D::Error::custom("unknown setting field"));
        }
        let field = serde_json::from_value(Value::Object(fields)).map_err(D::Error::custom)?;
        Ok(Self {
            key,
            title,
            description,
            field,
        })
    }
}

impl Setting {
    pub fn accepts(&self, value: &Value) -> bool {
        match &self.field {
            SettingType::Text { .. } | SettingType::File { .. } | SettingType::Folder { .. } => {
                value
                    .as_str()
                    .is_some_and(|v| v.len() <= 32_768 && !v.contains('\0'))
            }
            SettingType::Bool { .. } => value.is_boolean(),
            SettingType::Number { min, max, .. } => {
                value.as_i64().is_some_and(|v| (*min..=*max).contains(&v))
            }
            SettingType::Choice { options, .. } => value
                .as_str()
                .is_some_and(|v| options.iter().any(|option| option == v)),
            SettingType::Password => false,
        }
    }

    pub fn default_value(&self) -> Option<Value> {
        match &self.field {
            SettingType::Text { default }
            | SettingType::File { default }
            | SettingType::Folder { default }
            | SettingType::Choice { default, .. } => Some(Value::String(default.clone())),
            SettingType::Bool { default } => Some(Value::Bool(*default)),
            SettingType::Number { default, .. } => Some(Value::from(*default)),
            SettingType::Password => None,
        }
    }
}

/// Declaration matching is host-only, bounded and cannot execute plugin code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UrlMatch {
    pub host: String,
    #[serde(default)]
    pub include_subdomains: bool,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub priority: i32,
}
fn default_path() -> String {
    "*".into()
}

impl UrlMatch {
    pub fn matches(&self, url: &url::Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        let expected = self.host.to_ascii_lowercase();
        (host.eq_ignore_ascii_case(&expected)
            || (self.include_subdomains
                && host.to_ascii_lowercase().ends_with(&format!(".{expected}"))))
            && matches!(url.scheme(), "http" | "https")
            && path_matches(&self.path, url.path())
    }
}

fn path_matches(pattern: &str, path: &str) -> bool {
    let (pattern, input) = (pattern.as_bytes(), path.as_bytes());
    let (mut p, mut i, mut star, mut retry) = (0, 0, None, 0);
    while i < input.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == input[i]) {
            p += 1;
            i += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            retry = i;
        } else if let Some(s) = star {
            retry += 1;
            i = retry;
            p = s + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

pub(crate) fn validate_script_manifest(manifest: &Manifest) -> Result<(), PackageError> {
    if manifest.settings.len() > 100 || manifest.files.len() > 4096 {
        return Err(PackageError::InvalidManifest);
    }
    let mut files = HashSet::new();
    for path in &manifest.files {
        if !relative_code_path(path) || !files.insert(path.replace('\\', "/").to_lowercase()) {
            return Err(PackageError::InvalidManifest);
        }
    }
    if let Some(runtime) = &manifest.runtime {
        if runtime
            .timeout_ms
            .is_some_and(|timeout| timeout == 0 || timeout > 120_000)
            || runtime.arguments.len() > 128
            || runtime
                .arguments
                .iter()
                .any(|arg| arg.len() > 32_768 || arg.contains('\0'))
            || manifest.files.is_empty()
        {
            return Err(PackageError::InvalidManifest);
        }
    } else if !manifest.files.is_empty() {
        return Err(PackageError::InvalidManifest);
    }
    let mut settings = HashSet::new();
    for setting in &manifest.settings {
        if !valid_id(&setting.key)
            || !valid_text(&setting.title, 256)
            || setting.description.len() > 4096
            || !settings.insert(&setting.key)
        {
            return Err(PackageError::InvalidManifest);
        }
        if let SettingType::Choice { options, .. } = &setting.field
            && (options.is_empty()
                || options.len() > 100
                || options.iter().any(|v| !valid_text(v, 256)))
        {
            return Err(PackageError::InvalidManifest);
        }
        if let Some(value) = setting.default_value()
            && !setting.accepts(&value)
        {
            return Err(PackageError::InvalidManifest);
        }
    }
    Ok(())
}

pub(crate) fn relative_code_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.starts_with(['/', '\\'])
        && !path.contains([':', '\0'])
        && !path.chars().any(char::is_control)
        && !path.eq_ignore_ascii_case("extension.json")
        && path.split(['/', '\\']).all(|part| {
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || part.contains(['<', '>', '"', '|', '?', '*'])
            {
                return false;
            }
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
}
