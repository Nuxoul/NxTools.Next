use crate::{InvocationIdentity, ProviderItem};
use std::collections::{HashMap, HashSet};

#[derive(Clone, PartialEq, Eq)]
pub struct MergedRow<T> {
    pub id: String,
    pub value: T,
    pub provider: Option<String>,
}

/// Preserves the original feature's normal order. Plugin rows never rerank normal rows.
pub struct ResultMerge<T> {
    generation: u64,
    keyword: bool,
    providers: HashMap<String, InvocationIdentity>,
    rows: Vec<MergedRow<T>>,
    selected: Option<String>,
}
impl<T: Clone> ResultMerge<T> {
    pub fn new(generation: u64, keyword: bool) -> Self {
        Self {
            generation,
            keyword,
            providers: HashMap::new(),
            rows: Vec::new(),
            selected: None,
        }
    }
    pub fn register(&mut self, identity: InvocationIdentity) -> bool {
        if identity.generation != self.generation || (self.keyword && !self.providers.is_empty()) {
            return false;
        }
        self.providers.insert(provider_key(&identity), identity);
        true
    }
    pub fn rows(&self) -> &[MergedRow<T>] {
        &self.rows
    }
    pub fn selected_id(&self) -> Option<&str> {
        self.selected.as_deref()
    }
    pub fn select(&mut self, id: &str) -> bool {
        if !self.rows.iter().any(|row| row.id == id) {
            return false;
        }
        self.selected = Some(id.into());
        true
    }
    pub fn set_normal(&mut self, rows: Vec<(String, T)>) {
        if self.keyword {
            return;
        }
        let previous = self.selected.clone();
        let nearest = self
            .rows
            .iter()
            .position(|row| Some(&row.id) == previous.as_ref())
            .unwrap_or(0);
        let plugins = self
            .rows
            .iter()
            .filter(|row| row.provider.is_some())
            .cloned()
            .collect::<Vec<_>>();
        self.rows = rows
            .into_iter()
            .map(|(id, value)| MergedRow {
                id,
                value,
                provider: None,
            })
            .collect();
        self.rows.extend(plugins);
        self.restore_selection(previous, nearest);
    }
    pub fn apply(
        &mut self,
        identity: &InvocationIdentity,
        replace: bool,
        items: &[ProviderItem],
        prepare: impl Fn(&ProviderItem, &str) -> T,
    ) -> bool {
        let key = provider_key(identity);
        if identity.generation != self.generation || self.providers.get(&key) != Some(identity) {
            return false;
        }
        let previous = self.selected.clone();
        let nearest = self
            .rows
            .iter()
            .position(|row| Some(&row.id) == previous.as_ref())
            .unwrap_or(0);
        let quota = if self.keyword { 200 } else { 3 };
        let mut unique = HashSet::new();
        let incoming = items
            .iter()
            .filter(|item| unique.insert(item.id.as_str()))
            .take(quota)
            .map(|item| {
                let id = format!(
                    "extension:{}/{}/{}",
                    identity.package_id, identity.handler_id, item.id
                );
                MergedRow {
                    value: prepare(item, &id),
                    id,
                    provider: Some(key.clone()),
                }
            })
            .collect::<Vec<_>>();
        if replace {
            let kept = incoming
                .iter()
                .map(|row| row.id.as_str())
                .collect::<HashSet<_>>();
            self.rows.retain(|row| {
                row.provider.as_ref() != Some(&key) || kept.contains(row.id.as_str())
            });
        }
        for row in incoming {
            if let Some(existing) = self.rows.iter_mut().find(|existing| existing.id == row.id) {
                *existing = row;
                continue;
            }
            let provider_count = self
                .rows
                .iter()
                .filter(|row| row.provider.as_ref() == Some(&key))
                .count();
            let total = self
                .rows
                .iter()
                .filter(|row| row.provider.is_some())
                .count();
            if provider_count >= quota || total >= if self.keyword { 200 } else { 12 } {
                continue;
            }
            // Append below all committed normal/plugin rows, including selected first row.
            self.rows.push(row);
        }
        self.restore_selection(previous, nearest);
        true
    }
    pub fn retire(&mut self, package_id: &str) {
        let previous = self.selected.clone();
        let nearest = self
            .rows
            .iter()
            .position(|row| Some(&row.id) == previous.as_ref())
            .unwrap_or(0);
        let keys = self
            .providers
            .iter()
            .filter(|(_, identity)| identity.package_id == package_id)
            .map(|(key, _)| key.clone())
            .collect::<HashSet<_>>();
        self.providers.retain(|key, _| !keys.contains(key));
        self.rows
            .retain(|row| row.provider.as_ref().is_none_or(|key| !keys.contains(key)));
        self.restore_selection(previous, nearest);
    }
    fn restore_selection(&mut self, previous: Option<String>, nearest: usize) {
        self.selected = previous
            .filter(|id| self.rows.iter().any(|row| &row.id == id))
            .or_else(|| {
                self.rows
                    .get(nearest.min(self.rows.len().saturating_sub(1)))
                    .map(|row| row.id.clone())
            });
    }
}
fn provider_key(identity: &InvocationIdentity) -> String {
    format!("{}/{}", identity.package_id, identity.handler_id)
}
