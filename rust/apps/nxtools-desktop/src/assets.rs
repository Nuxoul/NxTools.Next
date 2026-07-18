use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

pub struct NxToolsAssets;

impl AssetSource for NxToolsAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "material-symbols/dashboard.svg" => {
                Some(include_bytes!("../assets/material-symbols/dashboard.svg"))
            }
            "material-symbols/star.svg" => {
                Some(include_bytes!("../assets/material-symbols/star.svg"))
            }
            "material-symbols/text-fields.svg" => {
                Some(include_bytes!("../assets/material-symbols/text-fields.svg"))
            }
            "material-symbols/image.svg" => {
                Some(include_bytes!("../assets/material-symbols/image.svg"))
            }
            "material-symbols/folder.svg" => {
                Some(include_bytes!("../assets/material-symbols/folder.svg"))
            }
            "material-symbols/add.svg" => {
                Some(include_bytes!("../assets/material-symbols/add.svg"))
            }
            "material-symbols/edit.svg" => {
                Some(include_bytes!("../assets/material-symbols/edit.svg"))
            }
            "material-symbols/keep.svg" => {
                Some(include_bytes!("../assets/material-symbols/keep.svg"))
            }
            "material-symbols/delete.svg" => {
                Some(include_bytes!("../assets/material-symbols/delete.svg"))
            }
            "material-symbols/arrow-upward.svg" => Some(include_bytes!(
                "../assets/material-symbols/arrow-upward.svg"
            )),
            "material-symbols/undo.svg" => {
                Some(include_bytes!("../assets/material-symbols/undo.svg"))
            }
            "material-symbols/open-in-full.svg" => Some(include_bytes!(
                "../assets/material-symbols/open-in-full.svg"
            )),
            "material-symbols/search.svg" => {
                Some(include_bytes!("../assets/material-symbols/search.svg"))
            }
            "material-symbols/check.svg" | "icons/check.svg" => {
                Some(include_bytes!("../assets/material-symbols/check.svg"))
            }
            "material-symbols/chevron-right.svg" | "icons/chevron-right.svg" => Some(
                include_bytes!("../assets/material-symbols/chevron-right.svg"),
            ),
            "material-symbols/cancel.svg" | "icons/circle-x.svg" => {
                Some(include_bytes!("../assets/material-symbols/cancel.svg"))
            }
            _ => None,
        };
        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_clipboard_material_symbols_are_embedded() {
        let paths = [
            "material-symbols/dashboard.svg",
            "material-symbols/star.svg",
            "material-symbols/text-fields.svg",
            "material-symbols/image.svg",
            "material-symbols/folder.svg",
            "material-symbols/add.svg",
            "material-symbols/edit.svg",
            "material-symbols/keep.svg",
            "material-symbols/delete.svg",
            "material-symbols/arrow-upward.svg",
            "material-symbols/undo.svg",
            "material-symbols/open-in-full.svg",
            "material-symbols/search.svg",
            "icons/check.svg",
            "icons/chevron-right.svg",
            "icons/circle-x.svg",
        ];

        for path in paths {
            assert!(
                NxToolsAssets
                    .load(path)
                    .expect("load embedded icon")
                    .is_some(),
                "missing embedded Material Symbol: {path}"
            );
        }
    }
}
