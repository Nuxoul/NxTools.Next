use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

pub struct NxToolsAssets;

impl AssetSource for NxToolsAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "material-symbols/calculate.svg" => {
                Some(include_bytes!("../assets/material-symbols/calculate.svg"))
            }
            "material-symbols/data-object.svg" => {
                Some(include_bytes!("../assets/material-symbols/data-object.svg"))
            }
            "material-symbols/language.svg" => {
                Some(include_bytes!("../assets/material-symbols/language.svg"))
            }
            "material-symbols/terminal.svg" => {
                Some(include_bytes!("../assets/material-symbols/terminal.svg"))
            }
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
            "material-symbols/open-in-new.svg" => {
                Some(include_bytes!("../assets/material-symbols/open-in-new.svg"))
            }
            "material-symbols/search.svg" => {
                Some(include_bytes!("../assets/material-symbols/search.svg"))
            }
            "material-symbols/settings.svg" => {
                Some(include_bytes!("../assets/material-symbols/settings.svg"))
            }
            "material-symbols/history.svg" => {
                Some(include_bytes!("../assets/material-symbols/history.svg"))
            }
            "material-symbols/manage-search.svg" => Some(include_bytes!(
                "../assets/material-symbols/manage-search.svg"
            )),
            "material-symbols/download.svg" => {
                Some(include_bytes!("../assets/material-symbols/download.svg"))
            }
            "material-symbols/content-copy.svg" => Some(include_bytes!(
                "../assets/material-symbols/content-copy.svg"
            )),
            "material-symbols/content-paste.svg" => Some(include_bytes!(
                "../assets/material-symbols/content-paste.svg"
            )),
            "material-symbols/close.svg" => {
                Some(include_bytes!("../assets/material-symbols/close.svg"))
            }
            "material-symbols/info.svg" => {
                Some(include_bytes!("../assets/material-symbols/info.svg"))
            }
            "material-symbols/progress-activity.svg" => Some(include_bytes!(
                "../assets/material-symbols/progress-activity.svg"
            )),
            "material-symbols/calendar-month.svg" => Some(include_bytes!(
                "../assets/material-symbols/calendar-month.svg"
            )),
            "material-symbols/chevron-left.svg" => Some(include_bytes!(
                "../assets/material-symbols/chevron-left.svg"
            )),
            "material-symbols/check.svg" | "icons/check.svg" => {
                Some(include_bytes!("../assets/material-symbols/check.svg"))
            }
            "material-symbols/chevron-right.svg" | "icons/chevron-right.svg" => Some(
                include_bytes!("../assets/material-symbols/chevron-right.svg"),
            ),
            "material-symbols/cancel.svg" | "icons/circle-x.svg" => {
                Some(include_bytes!("../assets/material-symbols/cancel.svg"))
            }
            "brand/nxtools.png" => Some(include_bytes!(
                "../../../../browser_extension/nxtools-interceptor/icons/icon-32.png"
            )),
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
    use gpui_component::IconNamed;

    use crate::icons::MaterialIcon;

    #[test]
    fn all_material_symbols_are_embedded() {
        for icon in MaterialIcon::ALL {
            let path = icon.path();
            assert!(
                NxToolsAssets
                    .load(path.as_ref())
                    .expect("load embedded icon")
                    .is_some(),
                "missing embedded Material Symbol: {path}"
            );
        }

        for alias in [
            "icons/check.svg",
            "icons/chevron-right.svg",
            "icons/circle-x.svg",
            "brand/nxtools.png",
        ] {
            assert!(
                NxToolsAssets
                    .load(alias)
                    .expect("load icon alias")
                    .is_some()
            );
        }
    }
}
