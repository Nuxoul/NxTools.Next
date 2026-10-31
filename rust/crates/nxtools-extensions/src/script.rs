use crate::{ChoiceOption, Manifest, PackageError, TextValidation, valid_id, valid_text};
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
    #[serde(
        default = "default_protocol_version",
        skip_serializing_if = "is_legacy_protocol"
    )]
    pub protocol_version: u16,
    #[serde(default, skip_serializing_if = "RuntimeMode::is_once")]
    pub mode: RuntimeMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resident_entry_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_exit_seconds: Option<u16>,
}
impl ScriptRuntime {
    pub fn once(program: ScriptProgram) -> Self {
        Self {
            program,
            arguments: Vec::new(),
            timeout_ms: None,
            protocol_version: 1,
            mode: RuntimeMode::Once,
            resident_entry_file: None,
            idle_exit_seconds: None,
        }
    }

    pub fn effective_idle_exit_seconds(&self) -> u16 {
        self.idle_exit_seconds.unwrap_or(120)
    }

    pub fn effective_timeout_ms(&self, feature: &str, entry: &str) -> u32 {
        let (default, cap) = match (feature, entry) {
            ("quick_launch", _) => (3_000, 10_000),
            ("downloads", "resolve") => (10_000, 30_000),
            _ => (30_000, 120_000),
        };
        self.timeout_ms.unwrap_or(default).min(cap)
    }
}

fn default_protocol_version() -> u16 {
    1
}
fn is_legacy_protocol(version: &u16) -> bool {
    *version == 1
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    #[default]
    Once,
    Resident,
}
impl RuntimeMode {
    fn is_once(&self) -> bool {
        *self == Self::Once
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<TextValidation>,
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
        options: Vec<ChoiceOption>,
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
        let validation = fields
            .remove("validation")
            .map(serde_json::from_value::<TextValidation>)
            .transpose()
            .map_err(D::Error::custom)?;
        if validation.is_some()
            && !matches!(
                fields.get("type").and_then(Value::as_str),
                Some("text" | "file" | "folder")
            )
        {
            return Err(D::Error::custom(
                "validation not supported for this setting type",
            ));
        }
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
            validation,
        })
    }
}

impl Setting {
    pub fn accepts(&self, value: &Value) -> bool {
        match &self.field {
            SettingType::Text { .. } | SettingType::File { .. } | SettingType::Folder { .. } => {
                value.as_str().is_some_and(|v| {
                    v.len() <= 32_768
                        && !v.contains('\0')
                        && self
                            .validation
                            .as_ref()
                            .is_none_or(|rules| rules.accepts(v))
                })
            }
            SettingType::Bool { .. } => value.is_boolean(),
            SettingType::Number { min, max, .. } => {
                value.as_i64().is_some_and(|v| (*min..=*max).contains(&v))
            }
            SettingType::Choice { options, .. } => value
                .as_str()
                .is_some_and(|v| options.iter().any(|option| option.value() == v)),
            SettingType::Password => false,
        }
    }

    /// Import/load may retain an explicitly unconfigured required field; Save/invoke may not.
    pub(crate) fn accepts_stored(&self, value: &Value) -> bool {
        self.accepts(value)
            || (value.as_str() == Some("")
                && self
                    .validation
                    .as_ref()
                    .is_some_and(|rules| rules.required && rules.valid()))
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

impl Manifest {
    /// Readiness is stricter than declaration/load validity, including missing required values.
    pub fn settings_ready(&self, values: &std::collections::BTreeMap<String, Value>) -> bool {
        values.iter().all(|(key, value)| {
            self.settings
                .iter()
                .any(|setting| setting.key == *key && setting.accepts(value))
        }) && self.settings.iter().all(|setting| {
            matches!(setting.field, SettingType::Password)
                || values
                    .get(&setting.key)
                    .cloned()
                    .or_else(|| setting.default_value())
                    .is_some_and(|value| setting.accepts(&value))
        })
    }

    pub(crate) fn effective_settings(
        &self,
        values: &std::collections::BTreeMap<String, Value>,
    ) -> std::collections::BTreeMap<String, Value> {
        let mut effective = values.clone();
        for setting in &self.settings {
            if let Some(default) = setting.default_value() {
                effective.entry(setting.key.clone()).or_insert(default);
            }
        }
        effective
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
        if !matches!(runtime.protocol_version, 1 | 2) {
            return Err(PackageError::UnsupportedVersion);
        }
        match runtime.mode {
            RuntimeMode::Once => {
                if runtime.resident_entry_file.is_some() || runtime.idle_exit_seconds.is_some() {
                    return Err(PackageError::InvalidManifest);
                }
            }
            RuntimeMode::Resident => {
                if runtime.protocol_version != 2
                    || !runtime.resident_entry_file.as_ref().is_some_and(|entry| {
                        relative_code_path(entry) && manifest.files.contains(entry)
                    })
                    || !(30..=600).contains(&runtime.effective_idle_exit_seconds())
                {
                    return Err(PackageError::InvalidManifest);
                }
            }
        }
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
        if setting.validation.as_ref().is_some_and(|rules| {
            !matches!(
                setting.field,
                SettingType::Text { .. } | SettingType::File { .. } | SettingType::Folder { .. }
            ) || !rules.valid()
        }) {
            return Err(PackageError::InvalidManifest);
        }
        if let SettingType::Choice { options, .. } = &setting.field {
            let mut values = HashSet::new();
            if options.is_empty()
                || options.len() > 100
                || options.iter().any(|option| {
                    !valid_text(option.value(), 256)
                        || !valid_text(option.label(), 256)
                        || option.labeled() != options[0].labeled()
                        || (option.labeled() && !values.insert(option.value()))
                })
            {
                return Err(PackageError::InvalidManifest);
            }
        }
        if let Some(value) = setting.default_value()
            && !setting.accepts_stored(&value)
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
