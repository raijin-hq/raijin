use inazuma::{AnyElement, App, Context, IntoElement, SharedString, Task, Window};

use crate::IndexPath;

use super::traits::{SelectDelegate, SelectItem};
use super::state::SelectState;

/// A built-in searchable vector for select items.
#[derive(Debug, Clone)]
pub struct SearchableVec<T> {
    items: Vec<T>,
    matched_items: Vec<T>,
}

impl<T: Clone> SearchableVec<T> {
    pub fn push(&mut self, item: T) {
        self.items.push(item.clone());
        self.matched_items.push(item);
    }
}

impl<T: Clone> SearchableVec<T> {
    pub fn new(items: impl Into<Vec<T>>) -> Self {
        let items = items.into();
        Self {
            items: items.clone(),
            matched_items: items,
        }
    }
}

impl<T: SelectItem> From<Vec<T>> for SearchableVec<T> {
    fn from(items: Vec<T>) -> Self {
        Self {
            items: items.clone(),
            matched_items: items,
        }
    }
}

impl<I: SelectItem> SelectDelegate for SearchableVec<I> {
    type Item = I;

    fn items_count(&self, _: usize) -> usize {
        self.matched_items.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.matched_items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        for (ix, item) in self.matched_items.iter().enumerate() {
            if item.value() == value {
                return Some(IndexPath::default().row(ix));
            }
        }

        None
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _: &mut Context<SelectState<Self>>,
    ) -> Task<()> {
        self.matched_items = self
            .items
            .iter()
            .filter(|item| item.matches(query))
            .cloned()
            .collect();

        Task::ready(())
    }
}

impl<I: SelectItem> SelectDelegate for SearchableVec<SelectGroup<I>> {
    type Item = I;

    fn sections_count(&self, _: &App) -> usize {
        self.matched_items.len()
    }

    fn items_count(&self, section: usize) -> usize {
        self.matched_items
            .get(section)
            .map_or(0, |group| group.items.len())
    }

    fn section(&self, section: usize) -> Option<AnyElement> {
        Some(
            self.matched_items
                .get(section)?
                .title
                .clone()
                .into_any_element(),
        )
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        let section = self.matched_items.get(ix.section)?;

        section.items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        for (ix, group) in self.matched_items.iter().enumerate() {
            for (row_ix, item) in group.items.iter().enumerate() {
                if item.value() == value {
                    return Some(IndexPath::default().section(ix).row(row_ix));
                }
            }
        }

        None
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _: &mut Context<SelectState<Self>>,
    ) -> Task<()> {
        self.matched_items = self
            .items
            .iter()
            .filter(|item| item.matches(&query))
            .cloned()
            .map(|mut item| {
                item.items.retain(|item| item.matches(&query));
                item
            })
            .collect();

        Task::ready(())
    }
}

/// A group of select items with a title.
#[derive(Debug, Clone)]
pub struct SelectGroup<I: SelectItem> {
    pub title: SharedString,
    pub items: Vec<I>,
}

impl<I> SelectGroup<I>
where
    I: SelectItem,
{
    /// Create a new SelectGroup with the given title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            items: vec![],
        }
    }

    /// Add an item to the group.
    pub fn item(mut self, item: I) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items to the group.
    pub fn items(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.items.extend(items);
        self
    }

    pub(super) fn matches(&self, query: &str) -> bool {
        self.title.to_lowercase().contains(&query.to_lowercase())
            || self.items.iter().any(|item| item.matches(query))
    }
}
