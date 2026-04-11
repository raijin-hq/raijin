use inazuma::{AnyElement, App, Context, IntoElement, SharedString, Task, Window};

use crate::IndexPath;

/// A trait for items that can be displayed in a select.
pub trait SelectItem: Clone {
    type Value: Clone;
    fn title(&self) -> SharedString;
    /// Customize the display title used to selected item in Select Input.
    ///
    /// If return None, the title will be used.
    fn display_title(&self) -> Option<AnyElement> {
        None
    }
    /// Render the item for the select dropdown menu, default is to render the title.
    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.title().into_element()
    }
    /// Get the value of the item.
    fn value(&self) -> &Self::Value;
    /// Check if the item matches the query for search, default is to match the title.
    fn matches(&self, query: &str) -> bool {
        self.title().to_lowercase().contains(&query.to_lowercase())
    }
}

impl SelectItem for String {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        &self
    }
}

impl SelectItem for SharedString {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        &self
    }
}

impl SelectItem for &'static str {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

pub trait SelectDelegate: Sized {
    type Item: SelectItem;

    /// Returns the number of sections in the [`Select`](super::Select).
    fn sections_count(&self, _: &App) -> usize {
        1
    }

    /// Returns the section header element for the given section index.
    fn section(&self, _section: usize) -> Option<AnyElement> {
        return None;
    }

    /// Returns the number of items in the given section.
    fn items_count(&self, section: usize) -> usize;

    /// Returns the item at the given index path (Only section, row will be use).
    fn item(&self, ix: IndexPath) -> Option<&Self::Item>;

    /// Returns the index of the item with the given value, or None if not found.
    fn position<V>(&self, _value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq;

    fn perform_search(
        &mut self,
        _query: &str,
        _window: &mut Window,
        _: &mut Context<super::SelectState<Self>>,
    ) -> Task<()> {
        Task::ready(())
    }
}

impl<T: SelectItem> SelectDelegate for Vec<T> {
    type Item = T;

    fn items_count(&self, _: usize) -> usize {
        self.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.as_slice().get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        self.iter()
            .position(|v| v.value() == value)
            .map(|ix| IndexPath::default().row(ix))
    }
}
