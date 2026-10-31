//! Immutable presentation ownership on the original panel; no search/selection state.
use super::*;
use gpui_kit::SharedString;

pub(super) struct ResultDisplayText {
    pub(super) title: SharedString,
    pub(super) subtitle: SharedString,
    pub(super) accessible: SharedString,
}
struct ResultSnapshot {
    results: Vec<SearchResult>,
    display: Vec<ResultDisplayText>,
}
#[derive(Clone)]
pub(crate) struct QuickLaunchResults(Rc<ResultSnapshot>);
impl From<Vec<SearchResult>> for QuickLaunchResults {
    fn from(results: Vec<SearchResult>) -> Self {
        let display = results
            .iter()
            .map(|result| {
                let subtitle: SharedString = result.subtitle.as_ref().map_or_else(
                    || {
                        if result.copy_text().is_some() {
                            SharedString::from("按 Enter 复制结果")
                        } else {
                            result.entry.target.clone().into()
                        }
                    },
                    |subtitle| subtitle.clone().into(),
                );
                ResultDisplayText {
                    title: result.entry.name.clone().into(),
                    subtitle,
                    accessible: format!("{}，{}", result.entry.name, result.entry.target).into(),
                }
            })
            .collect();
        Self(Rc::new(ResultSnapshot { results, display }))
    }
}
impl std::ops::Deref for QuickLaunchResults {
    type Target = Vec<SearchResult>;
    fn deref(&self) -> &Self::Target {
        &self.0.results
    }
}
impl QuickLaunchResults {
    pub(super) fn display(&self, index: usize) -> &ResultDisplayText {
        &self.0.display[index]
    }
    pub(crate) fn clear(&mut self) {
        *self = Vec::new().into();
    }
}
