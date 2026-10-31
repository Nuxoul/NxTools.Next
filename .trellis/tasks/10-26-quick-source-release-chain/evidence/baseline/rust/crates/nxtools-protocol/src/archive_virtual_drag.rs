//! Bounded formal virtual-file drag; legacy spike wire remains compatible with the same native owner.
use crate::{
    archive_browser::{ArchiveEntryAction, ArchiveEntryCommand},
    archive_read::ArchiveReadError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ARCHIVE_DRAG_MAX_ROOTS: usize = 32;
pub const ARCHIVE_DRAG_MAX_DESCRIPTORS: usize = 128;
pub const ARCHIVE_DRAG_MAX_FILES: usize = 32;
pub const ARCHIVE_DRAG_MAX_BYTES: u64 = 512 << 20;
pub const ARCHIVE_DRAG_MAX_COMMAND_BYTES: usize = 256 << 10;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveVirtualDragCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub action: String,
    pub source_window_handle: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ArchiveVirtualDragItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_id: Option<Uuid>,
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
        if self.action != "start"
            || self.source_window_handle <= 0
            || items.is_empty()
            || items.len() > ARCHIVE_DRAG_MAX_DESCRIPTORS
        {
            return Err(invalid);
        }
        let mut total = 0u64;
        let mut names = std::collections::HashSet::new();
        let mut ordinals = std::collections::HashSet::new();
        let mut paths = std::collections::HashMap::<String, (String, bool)>::new();
        let mut files = 0usize;
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
                files += 1;
                item.entry.validate()?;
            }
            let e = &item.entry;
            if item.name.is_empty()
                || item.name.encode_utf16().count() >= 260
                || item.name.contains('\\')
                || item
                    .name
                    .split('/')
                    .any(|p| !crate::archive_job::valid_job_name(p))
                || !names.insert(item.name.to_lowercase())
                || item.size.is_some_and(|size| size > ARCHIVE_DRAG_MAX_BYTES)
            {
                return Err(invalid);
            }
            total = total.checked_add(item.size.unwrap_or(0)).ok_or(invalid)?;
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
            if total > ARCHIVE_DRAG_MAX_BYTES
                || files > ARCHIVE_DRAG_MAX_FILES
                || e.action != ArchiveEntryAction::Materialize
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
        let roots = items
            .iter()
            .filter_map(|item| item.name.split('/').next())
            .map(str::to_lowercase)
            .collect::<std::collections::HashSet<_>>();
        if roots.len() > ARCHIVE_DRAG_MAX_ROOTS {
            return Err(invalid);
        }
        for item in items {
            let parts = item.name.split('/').collect::<Vec<_>>();
            for end in 1..parts.len() {
                let parent = parts[..end].join("/");
                if !items
                    .iter()
                    .any(|item| item.directory && item.name == parent)
                {
                    return Err(invalid);
                }
            }
        }
        let payload = crate::PlatformCommand::ArchiveVirtualDrag(Box::new(self.clone()));
        let encoded = serde_json::to_string(&payload).map_err(|_| invalid)?;
        // Conservative bound for the C# default JSON encoder's Unicode/HTML escaping. Counting
        // syntax quotes too is intentional: no Rust-accepted command can expand past its receiver cap.
        let receiver_bytes = encoded
            .chars()
            .map(|c| {
                if !c.is_ascii() || "\"<>&'+`".contains(c) {
                    6 * c.len_utf16()
                } else {
                    1
                }
            })
            .sum::<usize>();
        if receiver_bytes > ARCHIVE_DRAG_MAX_COMMAND_BYTES {
            return Err(invalid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            }),
            lease_id: None,
            depth: 0,
            copy_motw: Some(true),
            password_interaction: false,
            temp_policy: None,
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
        changed.size = Some((512 << 20) + 1);
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
            assert!(command.validate().is_err(), "finite expanded count");
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
        assert!(
            command.validate().is_err(),
            "repeated immutable context cannot exceed command budget"
        );
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
            "130 implied components cannot bypass128descriptors"
        );
    }
    #[test]
    fn tree_wire_limits_distinct_roots_independently_of_files() {
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
        assert!(
            command.validate().is_err(),
            "33directoryroots exceed selected root bound"
        );
    }
}
