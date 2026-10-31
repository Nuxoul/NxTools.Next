//! Declarative applicability is a UX/admission contract, never an OS permission boundary.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Text,
    Files,
    Image,
    Download,
}
#[derive(Debug, Clone, Copy)]
pub struct InputSelection {
    pub kind: InputKind,
    pub count: usize,
    pub bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedInput {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<InputKind>,
    #[serde(default = "one")]
    pub min_items: usize,
    #[serde(default = "maximum")]
    pub max_items: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<usize>,
}
fn one() -> usize {
    1
}
fn maximum() -> usize {
    32
}
impl AcceptedInput {
    pub fn validate(&self) -> bool {
        self.kinds.len() <= 4
            && self
                .kinds
                .iter()
                .enumerate()
                .all(|(index, kind)| !self.kinds[..index].contains(kind))
            && self.min_items > 0
            && self.min_items <= self.max_items
            && self.max_items <= 32
            && self.max_bytes.is_none_or(|size| {
                (1..=131072).contains(&size)
                    && !self.kinds.is_empty()
                    && self
                        .kinds
                        .iter()
                        .all(|kind| matches!(kind, InputKind::Text | InputKind::Files))
            })
    }
    pub fn accepts(&self, selection: InputSelection) -> bool {
        (self.kinds.is_empty() || self.kinds.contains(&selection.kind))
            && (self.min_items..=self.max_items).contains(&selection.count)
            && self
                .max_bytes
                .is_none_or(|maximum| selection.bytes <= maximum)
    }
    pub fn accepts_context(&self, feature: &str, context: &Value) -> bool {
        let selection = if feature == "downloads" {
            context
                .get("task_ids")
                .and_then(Value::as_array)
                .map(|items| InputSelection {
                    kind: InputKind::Download,
                    count: items.len(),
                    bytes: 0,
                })
        } else if feature == "clipboard" {
            context.get("selection").and_then(|item| {
                let (kind, count, bytes) = match item.get("kind").and_then(Value::as_str)? {
                    "text" => (InputKind::Text, 1, item.get("text")?.as_str()?.len()),
                    "image" => (InputKind::Image, 1, 0),
                    "files" => {
                        let paths = item.get("paths")?.as_array()?;
                        (
                            InputKind::Files,
                            paths.len(),
                            paths
                                .iter()
                                .map(|path| path.as_str().map_or(usize::MAX, str::len))
                                .try_fold(0usize, usize::checked_add)?,
                        )
                    }
                    _ => return None,
                };
                Some(InputSelection { kind, count, bytes })
            })
        } else {
            None
        };
        selection.is_some_and(|selection| self.accepts(selection))
    }
}
