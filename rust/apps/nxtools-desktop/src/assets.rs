use std::borrow::Cow;

use anyhow::Result;
use gpui_kit::{AssetSource, SharedString};

gpui_kit::assets::icon_assets!(DownloadIconAssets, [Link]);

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
            "material-symbols/folder-filled.svg" => Some(include_bytes!(
                "../assets/material-symbols/folder-filled.svg"
            )),
            "material-symbols/folder-zip.svg" => {
                Some(include_bytes!("../assets/material-symbols/folder-zip.svg"))
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
            "material-symbols/expand-more.svg" => {
                Some(include_bytes!("../assets/material-symbols/expand-more.svg"))
            }
            "material-symbols/expand-less.svg" => {
                Some(include_bytes!("../assets/material-symbols/expand-less.svg"))
            }
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
            "caption/minimize.svg" => Some(include_bytes!("../assets/caption/minimize.svg")),
            "caption/maximize.svg" => Some(include_bytes!("../assets/caption/maximize.svg")),
            "caption/restore.svg" => Some(include_bytes!("../assets/caption/restore.svg")),
            "caption/close.svg" => Some(include_bytes!("../assets/caption/close.svg")),
            "brand/nxtools.png" => Some(include_bytes!("../assets/brand/nxtools-32.png")),
            "brand/archive.png" => Some(include_bytes!("../assets/brand/nxtools-archive-32.png")),
            //20DIP caption/tree canvases need40source pixels at200%; reuse existing derivatives.
            "brand/nxtools-40.png" => Some(include_bytes!("../assets/brand/nxtools-40.png")),
            "brand/archive-40.png" => {
                Some(include_bytes!("../assets/brand/nxtools-archive-40.png"))
            }
            _ => None,
        };
        match bytes {
            Some(bytes) => Ok(Some(Cow::Borrowed(bytes))),
            None => match DownloadIconAssets.load(path)? {
                Some(bytes) => Ok(Some(bytes)),
                None => gpui_kit::assets::Assets.load(path),
            },
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(DownloadIconAssets.list(path)?);
        use gpui_kit::assets::IconNamed;
        for icon_path in crate::icons::MaterialIcon::ALL
            .iter()
            .map(|icon| icon.path())
            .chain(
                crate::icons::CaptionIcon::ALL
                    .iter()
                    .map(|icon| icon.path()),
            )
            .chain(
                [
                    "brand/nxtools.png",
                    "brand/archive.png",
                    "brand/nxtools-40.png",
                    "brand/archive-40.png",
                    "icons/check.svg",
                    "icons/chevron-right.svg",
                    "icons/circle-x.svg",
                ]
                .into_iter()
                .map(SharedString::from),
            )
        {
            if icon_path.starts_with(path) && !paths.contains(&icon_path) {
                paths.push(icon_path);
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::IconNamed;

    use crate::icons::{CaptionIcon, MaterialIcon};

    #[test]
    fn kit_assets_are_available_without_replacing_product_aliases() {
        for path in [
            "icons/search.svg",
            "icons/chevron-down.svg",
            "icons/close.svg",
        ] {
            assert!(
                NxToolsAssets.load(path).expect("Kit asset load").is_some(),
                "missing Kit asset {path}"
            );
        }
        let paths = NxToolsAssets
            .list("material-symbols/")
            .expect("product catalog");
        assert!(paths.contains(&SharedString::from("material-symbols/search.svg")));
        assert_eq!(
            NxToolsAssets.load("icons/check.svg").expect("alias"),
            NxToolsAssets
                .load("material-symbols/check.svg")
                .expect("product icon")
        );
    }
    #[test]
    fn download_neutral_file_fallback_is_embedded() {
        let path = gpui_kit::component::IconName::File.path();
        let bytes = NxToolsAssets
            .load(path.as_ref())
            .expect("load official file fallback")
            .expect("official file fallback is embedded");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn all_registered_icons_are_embedded() {
        for &icon in MaterialIcon::ALL {
            let path = icon.path();
            assert!(
                NxToolsAssets
                    .load(path.as_ref())
                    .expect("load embedded icon")
                    .is_some(),
                "missing embedded Material Symbol: {path}"
            );
        }

        for &icon in CaptionIcon::ALL {
            let path = icon.path();
            let bytes = NxToolsAssets
                .load(path.as_ref())
                .expect("load embedded caption icon")
                .unwrap_or_else(|| panic!("missing embedded caption icon: {path}"));
            let svg = std::str::from_utf8(bytes.as_ref()).expect("caption icon is UTF-8 SVG");
            assert!(
                svg.contains("fill=\"currentColor\""),
                "caption icon must inherit semantic text color: {path}"
            );
        }

        for alias in [
            "icons/check.svg",
            "icons/chevron-right.svg",
            "icons/circle-x.svg",
            "brand/nxtools.png",
            "brand/archive.png",
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
