use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            "clipboard-preview.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/clipboard-preview.svg"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec!["clipboard-preview.svg".into()])
    }
}
