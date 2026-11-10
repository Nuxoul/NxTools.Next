//! Bounded formal virtual-file drag; legacy spike wire remains compatible with the same native owner.
use crate::{
    archive_browser::{ArchiveEntryAction, ArchiveEntryCommand},
    archive_read::ArchiveReadError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Windows descriptor components must agree with the native output-name policy. In particular
/// DOS devices also include superscript aliases and console handles, including before an extension.
pub fn valid_drag_name_component(name: &str) -> bool {
    if name.is_empty()
        || name.encode_utf16().count() > 255
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
    {
        return false;
    }
    let base = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_uppercase();
    !matches!(
        base.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) && !(base
        .strip_prefix("COM")
        .or_else(|| base.strip_prefix("LPT"))
        .is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        }))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveVirtualDragItem {
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub directory: bool,
    pub entry: ArchiveEntryCommand,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveVirtualDragCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub action: String,
    pub source_window_handle: i64,
    pub items: Option<Vec<ArchiveVirtualDragItem>>,
    pub authorization_id: Option<Uuid>,
}
// The in-memory entry intents retain their original owner/API. Only the wire shares source data.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DragSource {
    path: Option<String>,
    codepage: u32,
    browser: Option<crate::archive_browser::ArchiveBrowserContext>,
    depth: u32,
    copy_motw: Option<bool>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DragIdentity {
    operation_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ordinal: Option<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum DragEntryWire {
    Full(Box<ArchiveEntryCommand>),
    Identity(DragIdentity),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DragItemWire {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    directory: bool,
    entry: DragEntryWire,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DragCommandWire {
    version: u32,
    operation_id: Uuid,
    session_id: Uuid,
    generation: u64,
    action: String,
    source_window_handle: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<DragSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    items: Option<Vec<DragItemWire>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    authorization_id: Option<Uuid>,
}
impl Serialize for ArchiveVirtualDragCommand {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let first = self.items.as_ref().and_then(|items| items.first());
        let shared = self.action == "start"
            && first.is_some()
            && self.items.as_ref().is_some_and(|items| {
                items.iter().all(|item| {
                    let entry = &item.entry;
                    let first = &items[0].entry;
                    entry.version == 1
                        && entry.action == ArchiveEntryAction::Materialize
                        && entry.session_id == self.session_id
                        && entry.generation == self.generation
                        && !entry.password_interaction
                        && entry.lease_id.is_none()
                        && entry.temp_policy.is_none()
                        && entry.path == first.path
                        && entry.codepage == first.codepage
                        && entry.browser == first.browser
                        && entry.depth == first.depth
                        && entry.copy_motw == first.copy_motw
                })
            });
        let source = first.filter(|_| shared).map(|item| DragSource {
            path: item.entry.path.clone(),
            codepage: item.entry.codepage,
            browser: item.entry.browser.clone(),
            depth: item.entry.depth,
            copy_motw: item.entry.copy_motw,
        });
        let items = self.items.as_ref().map(|items| {
            items
                .iter()
                .map(|item| DragItemWire {
                    name: item.name.clone(),
                    size: item.size,
                    directory: item.directory,
                    entry: if shared {
                        DragEntryWire::Identity(DragIdentity {
                            operation_id: item.entry.operation_id,
                            ordinal: item.entry.ordinal,
                        })
                    } else {
                        DragEntryWire::Full(Box::new(item.entry.clone()))
                    },
                })
                .collect()
        });
        DragCommandWire {
            version: self.version,
            operation_id: self.operation_id,
            session_id: self.session_id,
            generation: self.generation,
            action: self.action.clone(),
            source_window_handle: self.source_window_handle,
            source,
            items,
            authorization_id: self.authorization_id,
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ArchiveVirtualDragCommand {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = DragCommandWire::deserialize(deserializer)?;
        if wire.action != "start" && wire.source.is_some() {
            return Err(serde::de::Error::custom("unexpected drag source"));
        }
        let items = wire
            .items
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| {
                        let entry = match (item.entry, wire.source.as_ref()) {
                            (DragEntryWire::Full(entry), None) => *entry,
                            (DragEntryWire::Identity(identity), Some(source)) => {
                                ArchiveEntryCommand {
                                    version: 1,
                                    operation_id: identity.operation_id,
                                    session_id: wire.session_id,
                                    generation: wire.generation,
                                    action: ArchiveEntryAction::Materialize,
                                    path: source.path.clone(),
                                    codepage: source.codepage,
                                    browser: source.browser.clone(),
                                    depth: source.depth,
                                    copy_motw: source.copy_motw,
                                    ordinal: identity.ordinal,
                                    lease_id: None,
                                    password_interaction: false,
                                    temp_policy: None,
                                    preview_options: None,
                                }
                            }
                            _ => {
                                return Err(serde::de::Error::custom(
                                    "inconsistent drag source shape",
                                ));
                            }
                        };
                        Ok(ArchiveVirtualDragItem {
                            name: item.name,
                            size: item.size,
                            directory: item.directory,
                            entry,
                        })
                    })
                    .collect::<Result<Vec<_>, D::Error>>()
            })
            .transpose()?;
        Ok(Self {
            version: wire.version,
            operation_id: wire.operation_id,
            session_id: wire.session_id,
            generation: wire.generation,
            action: wire.action,
            source_window_handle: wire.source_window_handle,
            items,
            authorization_id: wire.authorization_id,
        })
    }
}
impl ArchiveVirtualDragCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        let invalid = ArchiveReadError::InvalidRequest;
        if self.version != 1
            || self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || self.authorization_id.is_some_and(|id| id.is_nil())
        {
            return Err(invalid);
        }
        if self.action == "cancel" {
            return if self.items.is_none()
                && self.source_window_handle == 0
                && self.authorization_id.is_none()
            {
                Ok(())
            } else {
                Err(invalid)
            };
        }
        let items = self.items.as_ref().ok_or(invalid)?;
        let locate = self.action == "locate";
        if !(self.action == "start" || locate) || self.source_window_handle <= 0 || items.is_empty()
        {
            return Err(invalid);
        }
        // Destination location drags one descriptor-only wrapper; extraction runs as an ordinary
        // job after the receiver reports where that wrapper was created.
        if locate
            && (items.len() != 1
                || !items[0].directory
                || items[0].name.contains('/')
                || self.authorization_id.is_some())
        {
            return Err(invalid);
        }
        let mut names = std::collections::HashSet::new();
        let mut ordinals = std::collections::HashSet::new();
        let mut paths = std::collections::HashMap::<String, (String, bool)>::new();
        let mut operations = std::collections::HashSet::new();
        let first = &items[0].entry;
        for item in items {
            if item.directory {
                if item.size.is_some() {
                    return Err(invalid);
                }
                // Descriptor-only identity: never grants file materialization with missing ordinal.
                let mut directory = item.entry.clone();
                directory.ordinal = directory.ordinal.or(Some(0));
                directory.validate()?;
            } else {
                item.entry.validate()?;
            }
            let e = &item.entry;
            if item.name.is_empty()
                || item.name.encode_utf16().count() >= 260
                || item.name.contains('\\')
                || item.name.split('/').any(|p| !valid_drag_name_component(p))
                || !names.insert(item.name.to_lowercase())
                || item.size.is_some_and(|size| size > i64::MAX as u64)
            {
                return Err(invalid);
            }
            let parts = item.name.split('/').collect::<Vec<_>>();
            for end in 1..=parts.len() {
                let path = parts[..end].join("/");
                let directory = end < parts.len() || item.directory;
                let key = path.to_lowercase();
                if let Some((original, prior_directory)) = paths.get(&key) {
                    if original != &path || *prior_directory != directory {
                        return Err(invalid);
                    }
                } else {
                    paths.insert(key, (path, directory));
                }
            }
            if e.action != ArchiveEntryAction::Materialize
                || e.session_id != self.session_id
                || e.generation != self.generation
                || e.password_interaction
                || e.temp_policy.is_some()
                || e.lease_id.is_some()
                || e.ordinal.is_some_and(|ordinal| !ordinals.insert(ordinal))
                || !operations.insert(e.operation_id)
                || e.path != first.path
                || e.codepage != first.codepage
                || e.depth != first.depth
                || e.copy_motw != first.copy_motw
                || e.browser != first.browser
            {
                return Err(invalid);
            }
        }
        for item in items {
            let parts = item.name.split('/').collect::<Vec<_>>();
            for end in 1..parts.len() {
                let parent = parts[..end].join("/");
                if !paths
                    .get(&parent.to_lowercase())
                    .is_some_and(|(name, directory)| *directory && name == &parent)
                    || !names.contains(&parent.to_lowercase())
                {
                    return Err(invalid);
                }
            }
        }
        let mut envelope = crate::Envelope::new(crate::Message::PlatformCommand(
            crate::PlatformCommand::ArchiveVirtualDrag(Box::new(self.clone())),
        ));
        envelope.correlation_id = Some(self.operation_id);
        if serde_json::to_vec(&envelope).map_err(|_| invalid)?.len() > crate::MAX_FRAME_BYTES {
            return Err(invalid);
        }
        Ok(())
    }
}

/// Native demand/cache/receiver lifecycle; never an external target-path grant.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveVirtualDragProgress {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub phase: String,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub files_done: u64,
    pub files_total: u64,
    pub current_index: Option<u32>,
    pub current_member: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_request_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_bytes_done: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_bytes_total: Option<u64>,
    /// Processed member bytes, distinct from `bytes_done` (sealed cache bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes_processed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
    pub cleanup_complete: Option<bool>,
    pub code: Option<String>,
    pub native_error_code: Option<i32>,
}
impl std::fmt::Debug for ArchiveVirtualDragProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveVirtualDragProgress([PRIVATE])")
    }
}
impl ArchiveVirtualDragProgress {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != 1
            || self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || !matches!(
                self.phase.as_str(),
                "materializing"
                    | "content_ready"
                    | "target_copying"
                    | "finished"
                    | "cancelled"
                    | "failed"
            )
            || self.files_done > self.files_total
            || self.sequence == Some(0)
            || self.current_request_id.is_some_and(|id| id.is_nil())
            || ((self.current_request_id.is_some()
                || self.current_bytes_done.is_some()
                || self.current_bytes_total.is_some()
                || self.bytes_processed.is_some())
                && (self.sequence.is_none()
                    || self.current_request_id.is_none()
                    || self.current_index.is_none()
                    || self.current_bytes_done.is_none()
                    || self.bytes_processed.is_none()))
            || self
                .bytes_processed
                .is_some_and(|done| done < self.bytes_done)
            || self
                .current_bytes_done
                .zip(self.current_bytes_total)
                .is_some_and(|(done, total)| done > total)
            || self.current_member.as_ref().is_some_and(|name| {
                name.is_empty() || name.len() > 32767 || name.chars().any(char::is_control)
            })
            || self.code.as_ref().is_some_and(|code| {
                code.len() > 64 || !code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
            })
        {
            return Err(ArchiveReadError::ProtocolViolation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r45_drag_processed_progress_wire_keeps_current_and_sealed_bytes_distinct() {
        let wire = serde_json::json!({
            "version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),
            "generation":7,"phase":"materializing","bytes_done":100,"bytes_total":500,
            "files_done":1,"files_total":2,"current_index":4,"current_member":"Root/current.bin",
            "current_request_id":Uuid::new_v4(),"current_bytes_done":50,"current_bytes_total":200,
            "bytes_processed":150,"sequence":3
        });
        let event: ArchiveVirtualDragProgress =
            serde_json::from_value(wire.clone()).expect("real drag progress wire");
        event.validate().expect("bound progress");
        let roundtrip = serde_json::to_value(&event).expect("wire roundtrip");
        for field in [
            "bytes_done",
            "current_bytes_done",
            "current_bytes_total",
            "bytes_processed",
            "sequence",
            "current_request_id",
        ] {
            assert_eq!(roundtrip[field], wire[field], "{field}");
        }
        for (field, value) in [
            ("current_request_id", serde_json::json!(Uuid::nil())),
            ("sequence", serde_json::json!(0)),
            ("current_index", serde_json::Value::Null),
            ("bytes_processed", serde_json::json!(99)),
            ("current_bytes_done", serde_json::json!(201)),
        ] {
            let mut invalid = wire.clone();
            invalid[field] = value;
            assert!(
                serde_json::from_value::<ArchiveVirtualDragProgress>(invalid)
                    .expect("typed invalid snapshot")
                    .validate()
                    .is_err(),
                "{field}"
            );
        }
        let mut unknown = wire;
        unknown["bytes_total"] = serde_json::Value::Null;
        unknown["current_bytes_total"] = serde_json::Value::Null;
        serde_json::from_value::<ArchiveVirtualDragProgress>(unknown)
            .expect("unknown totals")
            .validate()
            .expect("unknown remains valid");
    }
    #[test]
    fn tree_root_wrapper_v1_preserves_authority_and_rejects_windows_device_aliases() {
        let mut command = tree_command();
        for item in command.items.as_mut().expect("items") {
            item.name = format!("Bundle/{}", item.name);
        }
        let mut wrapper = command.items.as_ref().expect("items")[0].clone();
        wrapper.name = "Bundle".into();
        wrapper.entry.operation_id = Uuid::new_v4();
        command.items.as_mut().expect("items").insert(0, wrapper);
        command.validate().expect("v1 descriptor-only wrapper");
        assert_eq!(
            command.items.as_ref().expect("items")[3].entry.ordinal,
            Some(1)
        );
        for name in [
            "CON", "COM¹", "LPT²", "CONIN$", "CONOUT$", "CON .zip", "../bad", "bad.", "bad ",
        ] {
            assert!(
                !valid_drag_name_component(name),
                "unsafe Windows descriptor component {name}"
            );
            let mut invalid = command.clone();
            invalid.items.as_mut().expect("items")[0].name = name.into();
            assert!(invalid.validate().is_err());
        }
        for name in ["\u{7f}".to_owned(), "\u{85}".to_owned(), "界".repeat(256)] {
            assert!(
                !valid_drag_name_component(&name),
                "native/Rust component bounds agree"
            );
        }
        assert!(valid_drag_name_component(&"界".repeat(86)));
        assert!(valid_drag_name_component(&"界".repeat(255)));
        assert!(!valid_drag_name_component(&"😀".repeat(128)));
        let wire = serde_json::to_value(&command).expect("wire");
        assert_eq!(wire["version"], 1);
        assert!(wire.get("root_name").is_none(), "no new wire authority");
        assert_eq!(
            serde_json::from_value::<ArchiveVirtualDragCommand>(wire).expect("roundtrip"),
            command
        );
    }
    #[test]
    fn r45_n01_long_root_serialized_wire_fixture() {
        let mut command = tree_command();
        let wrapper = "界".repeat(86);
        for item in command.items.as_mut().expect("items") {
            item.name = format!(
                "{wrapper}{}",
                item.name.strip_prefix("root").expect("tree root")
            );
        }
        command.validate().expect("Windows-valid Root tree");
        let operation_id = command.operation_id;
        let mut envelope = crate::Envelope::new(crate::Message::PlatformCommand(
            crate::PlatformCommand::ArchiveVirtualDrag(Box::new(command)),
        ));
        envelope.correlation_id = Some(operation_id);
        let wire = envelope
            .encode_line()
            .expect("actual serialized Rust envelope");
        let decoded = crate::Envelope::decode_line(&wire).expect("serialized command roundtrip");
        assert_eq!(decoded, envelope);
        if let Some(scratch) = std::env::var_os("PI_SCRATCH_DIR") {
            let path = std::path::Path::new(&scratch).join("archive-r45-n01-long-root-wire.json");
            std::fs::write(path, wire).expect("scratch wire fixture for native admission");
        }
    }
    #[test]
    fn virtual_spike_cancel_shape_is_bounded_and_legacy_authority_is_not_reused() {
        let mut c = ArchiveVirtualDragCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            action: "cancel".into(),
            source_window_handle: 0,
            items: None,
            authorization_id: None,
        };
        assert!(c.validate().is_ok());
        c.source_window_handle = 1;
        assert!(c.validate().is_err());
        c.action = "start".into();
        c.items = Some(vec![]);
        assert!(c.validate().is_err());
        let wire = crate::Envelope::new(crate::Message::PlatformCommand(
            crate::PlatformCommand::ArchiveVirtualDragSpike(Box::new(c)),
        ));
        let value = serde_json::to_value(wire).expect("wire");
        assert_eq!(value["payload"]["command"], "archive_virtual_drag_spike");
        let formal_command =
            crate::PlatformCommand::ArchiveVirtualDrag(Box::new(ArchiveVirtualDragCommand {
                version: 1,
                operation_id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                generation: 1,
                action: "cancel".into(),
                source_window_handle: 0,
                items: None,
                authorization_id: None,
            }));
        let formal = serde_json::to_value(crate::Envelope::new(crate::Message::PlatformCommand(
            formal_command,
        )))
        .expect("formal wire");
        assert_eq!(formal["payload"]["command"], "archive_virtual_drag");
        let decoded: crate::Envelope = serde_json::from_value(formal).expect("formal roundtrip");
        assert!(matches!(
            decoded.message,
            crate::Message::PlatformCommand(crate::PlatformCommand::ArchiveVirtualDrag(_))
        ));
    }
    #[test]
    fn virtual_spike_start_preserves_source_and_rejects_duplicate_or_unbounded_selection() {
        use crate::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
        let session = Uuid::new_v4();
        let entry = ArchiveEntryCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: session,
            generation: 1,
            action: ArchiveEntryAction::Materialize,
            path: Some(r"C:\owned\source.zip".into()),
            codepage: 0,
            ordinal: Some(7),
            browser: Some(ArchiveBrowserContext {
                snapshot_id: Uuid::new_v4(),
                format: "Zip".into(),
                comment: String::new(),
                source: ArchiveSourceBinding {
                    native: "v1:test".into(),
                    digest: [1; 32],
                    size: 99,
                    modified_unix_ns: 1,
                },
                sevenz_edit: None,
                tar_edit: None,
            }),
            lease_id: None,
            depth: 0,
            copy_motw: Some(true),
            password_interaction: false,
            temp_policy: None,
            preview_options: None,
        };
        let item = ArchiveVirtualDragItem {
            name: "file.txt".into(),
            size: Some(3),
            directory: false,
            entry,
        };
        let mut command = ArchiveVirtualDragCommand {
            version: 1,
            authorization_id: None,
            operation_id: Uuid::new_v4(),
            session_id: session,
            generation: 1,
            action: "start".into(),
            source_window_handle: 42,
            items: Some(vec![item.clone()]),
        };
        assert!(command.validate().is_ok());
        command.items = Some(vec![item.clone(), item.clone()]);
        assert!(command.validate().is_err());
        let mut changed = item.clone();
        changed.entry.generation = 2;
        command.items = Some(vec![changed]);
        assert!(command.validate().is_err());
        let mut changed = item.clone();
        changed.name = "../escape.txt".into();
        command.items = Some(vec![changed]);
        assert!(command.validate().is_err());
        let mut changed = item;
        changed.size = Some(i64::MAX as u64 + 1);
        command.items = Some(vec![changed]);
        assert!(command.validate().is_err());
    }
    fn tree_command() -> ArchiveVirtualDragCommand {
        let session = Uuid::new_v4();
        let snapshot = Uuid::new_v4();
        let items=[("root",true,None::<u64>,None::<u64>),("root/empty",true,None,Some(0)),("root/file",false,None,Some(1))].into_iter().map(|(name,directory,size,ordinal)| {
            serde_json::from_value(serde_json::json!({"name":name,"directory":directory,"size":size,"entry":{"version":1,"operation_id":Uuid::new_v4(),"session_id":session,"generation":1,"action":"materialize","path":"C:\\owned\\tree.zip","ordinal":ordinal,"browser":{"snapshot_id":snapshot,"format":"Zip","comment":"","source":{"native":"v1:test","digest":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"size":99,"modified_unix_ns":1}}}})).expect("wire fixture")
        }).collect();
        ArchiveVirtualDragCommand {
            version: 1,
            authorization_id: None,
            operation_id: Uuid::new_v4(),
            session_id: session,
            generation: 1,
            action: "start".into(),
            source_window_handle: 42,
            items: Some(items),
        }
    }
    #[test]
    fn tree_unknown_wire_is_compatible_and_directory_does_not_grant_materialize_authority() {
        let command = tree_command();
        command.validate().expect("tree accepted");
        let wire = serde_json::to_value(&command).expect("wire");
        assert!(wire["items"][2]["size"].is_null());
        assert_eq!(
            serde_json::from_value::<ArchiveVirtualDragCommand>(wire).expect("roundtrip"),
            command
        );
        let mut legacy =
            serde_json::to_value(&command.items.as_ref().expect("items")[2]).expect("file");
        legacy.as_object_mut().expect("object").remove("directory");
        legacy["size"] = serde_json::json!(3);
        let item: ArchiveVirtualDragItem =
            serde_json::from_value(legacy).expect("old known file shape");
        assert!(!item.directory);
        assert_eq!(item.size, Some(3));
        assert!(
            !serde_json::to_value(item)
                .expect("legacy")
                .as_object()
                .expect("object")
                .contains_key("directory")
        );
        let mut invalid = command.clone();
        invalid.items.as_mut().expect("items")[0].directory = false;
        assert!(
            invalid.validate().is_err(),
            "missing ordinal never gains file authority"
        );
        let mut invalid = command;
        invalid.items.as_mut().expect("items")[0].size = Some(0);
        assert!(
            invalid.validate().is_err(),
            "directory cannot declare file length"
        );
    }
    #[test]
    fn locate_admits_exactly_one_unauthorized_wrapper_directory() {
        let mut command = tree_command();
        command.action = "locate".into();
        command.items.as_mut().expect("items").truncate(1);
        command.validate().expect("single wrapper locate");
        let wire = serde_json::to_value(&command).expect("wire");
        assert!(wire.get("source").is_none(), "locate carries no shared source");
        assert_eq!(
            serde_json::from_value::<ArchiveVirtualDragCommand>(wire).expect("roundtrip"),
            command
        );
        let mut file = command.clone();
        file.items = Some(vec![tree_command().items.expect("items")[2].clone()]);
        assert!(file.validate().is_err(), "locate never names a file");
        let mut nested = command.clone();
        nested.items.as_mut().expect("items")[0].name = "root/inner".into();
        assert!(nested.validate().is_err(), "locate wrapper is one component");
        let mut many = tree_command();
        many.action = "locate".into();
        assert!(many.validate().is_err(), "locate carries no member tree");
        let mut authorized = command.clone();
        authorized.authorization_id = Some(Uuid::new_v4());
        assert!(authorized.validate().is_err(), "locate never consumes unlock");
        let mut windowless = command;
        windowless.source_window_handle = 0;
        assert!(windowless.validate().is_err());
    }
    #[test]
    fn tree_wire_checks_prefix_casing_descriptor_file_and_serialized_budgets() {
        for name in [
            "ROOT/file",
            "root/empty/../bad",
            "root/empty//bad",
            "root/empty.",
            "root/empty ",
        ] {
            let mut command = tree_command();
            command.items.as_mut().expect("items")[2].name = name.into();
            assert!(
                command.validate().is_err(),
                "unsafe or inconsistent relative name {name}"
            );
        }
        let mut command = tree_command();
        command.items.as_mut().expect("items")[1].directory = false;
        command.items.as_mut().expect("items")[2].name = "root/empty/child".into();
        assert!(command.validate().is_err(), "file cannot be ancestor");
        for (count, directory) in [(129, true), (33, false)] {
            let mut command = tree_command();
            let template = command.items.as_ref().expect("items")[2].clone();
            command.items = Some(
                (0..count)
                    .map(|index| ArchiveVirtualDragItem {
                        name: format!("entry{index}"),
                        size: None,
                        directory,
                        entry: ArchiveEntryCommand {
                            operation_id: Uuid::new_v4(),
                            ordinal: Some(index),
                            ..template.entry.clone()
                        },
                    })
                    .collect(),
            );
            command.validate().expect("complete expanded count");
        }
        let mut command = tree_command();
        let template = command.items.as_ref().expect("items")[2].clone();
        command.items = Some(
            (0..5)
                .map(|index| {
                    let mut item = template.clone();
                    item.name = format!("file{index}");
                    item.entry.ordinal = Some(index);
                    item.entry.operation_id = Uuid::new_v4();
                    item.entry.browser.as_mut().expect("browser").comment = "x".repeat(65536);
                    item
                })
                .collect(),
        );
        command.validate().expect("source context serialized once");
        let wire = serde_json::to_value(&command).expect("compact source");
        assert!(wire["source"]["browser"]["comment"].is_string());
        assert!(wire["items"][0]["entry"].get("browser").is_none());
    }
    #[test]
    fn tree_missing_parent_descriptor_cannot_bypass_expanded_count() {
        let mut command = tree_command();
        command.items = Some(vec![command.items.as_ref().expect("items")[2].clone()]);
        assert!(
            command.validate().is_err(),
            "full ancestor descriptors must be explicit"
        );
        command.items.as_mut().expect("items")[0].name = vec!["a"; 130].join("/");
        assert!(
            command.validate().is_err(),
            "missing ancestor descriptors remain invalid"
        );
    }
    #[test]
    fn tree_wire_preserves_distinct_roots_independently_of_files() {
        let mut command = tree_command();
        let template = command.items.as_ref().expect("items")[0].clone();
        command.items = Some(
            (0..33)
                .map(|index| ArchiveVirtualDragItem {
                    name: format!("root{index}"),
                    entry: ArchiveEntryCommand {
                        operation_id: Uuid::new_v4(),
                        ..template.entry.clone()
                    },
                    ..template.clone()
                })
                .collect(),
        );
        command.validate().expect("all 33 directory roots");
    }
    #[test]
    fn ordinary_directory_drag_wire_preserves_256_files_and_all_descriptor_indices() {
        let mut command = tree_command();
        let seed = command.items.as_ref().expect("items")[2].clone();
        let mut root = command.items.as_ref().expect("items")[0].clone();
        root.name = "ordinary".into();
        let mut items = vec![root];
        items.extend((0..256).map(|ordinal| ArchiveVirtualDragItem {
            name: format!("ordinary/file-{ordinal:03}.txt"),
            size: Some(8),
            directory: false,
            entry: ArchiveEntryCommand {
                operation_id: Uuid::new_v4(),
                ordinal: Some(ordinal),
                ..seed.entry.clone()
            },
        }));
        command.items = Some(items);
        command
            .validate()
            .expect("ordinary directory is not a business count overflow");
        let decoded: ArchiveVirtualDragCommand = serde_json::from_slice(
            &serde_json::to_vec(&command).expect("serialize actual command"),
        )
        .expect("roundtrip");
        let items = decoded.items.as_ref().expect("complete descriptors");
        assert_eq!(items.len(), 257);
        assert_eq!(items.iter().filter(|item| !item.directory).count(), 256);
        for ordinal in [0, 128, 255] {
            let item = &items[ordinal as usize + 1];
            assert_eq!(item.entry.ordinal, Some(ordinal));
            assert_eq!(item.name, format!("ordinary/file-{ordinal:03}.txt"));
            assert_eq!(item.size, Some(8));
            assert_eq!(item.entry.path, seed.entry.path);
            assert_eq!(item.entry.browser, seed.entry.browser);
        }
    }

    #[test]
    fn ordinary_directory_drag_progress_preserves_total_count_and_full_array_index() {
        let progress = ArchiveVirtualDragProgress {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            phase: "target_copying".into(),
            bytes_done: 34 * 8,
            bytes_total: Some(256 * 8),
            files_done: 34,
            files_total: 256,
            current_index: Some(256),
            current_member: Some("ordinary/file-255.txt".into()),
            current_request_id: None,
            current_bytes_done: None,
            current_bytes_total: None,
            bytes_processed: None,
            sequence: None,
            cleanup_complete: None,
            code: None,
            native_error_code: None,
        };
        progress
            .validate()
            .expect("logical file total and descriptor index are not active-work bounds");
        let decoded: ArchiveVirtualDragProgress =
            serde_json::from_slice(&serde_json::to_vec(&progress).expect("actual progress wire"))
                .expect("roundtrip");
        assert!(decoded == progress);
        assert_eq!(decoded.files_total, 256);
        assert_eq!(decoded.current_index, Some(256));
    }

    #[test]
    fn ordinary_drag_wire_preserves_known_64_bit_size_without_zero_substitution() {
        let mut command = tree_command();
        let known = (1u64 << 33) + 17;
        command.items.as_mut().expect("items")[2].size = Some(known);
        command
            .validate()
            .expect("64-bit descriptor metadata is representable");
        let wire = serde_json::to_value(&command).expect("actual wire");
        assert_eq!(wire["items"][2]["size"], known);
        assert!(wire["items"][0]["size"].is_null());
    }

    #[test]
    fn ordinary_drag_wire_rejects_actual_frame_overflow_and_member_source_override() {
        let mut command = tree_command();
        let template = command.items.as_ref().expect("items")[2].clone();
        let wire = serde_json::to_value(&command).expect("compact source");
        let mut invalid = wire.clone();
        invalid["items"][2]["entry"]["path"] = serde_json::json!(r"C:\other.zip");
        assert!(serde_json::from_value::<ArchiveVirtualDragCommand>(invalid).is_err());
        let mut invalid = wire;
        invalid["source"]["browser"]["source"]
            .as_object_mut()
            .expect("source")
            .remove("size");
        assert!(serde_json::from_value::<ArchiveVirtualDragCommand>(invalid).is_err());
        command.items = Some(
            (0..10000)
                .map(|ordinal| ArchiveVirtualDragItem {
                    name: format!("{}-{ordinal}.txt", "a".repeat(96)),
                    entry: ArchiveEntryCommand {
                        operation_id: Uuid::new_v4(),
                        ordinal: Some(ordinal),
                        ..template.entry.clone()
                    },
                    ..template.clone()
                })
                .collect(),
        );
        assert!(
            serde_json::to_vec(&command)
                .expect("actual compact wire")
                .len()
                > crate::MAX_FRAME_BYTES
        );
        assert!(
            command.validate().is_err(),
            "whole selection rejected at real transport boundary"
        );
    }
}
