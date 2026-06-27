//! The pure state of an interactive list or table selector.
//!
//! [`Selector`] holds the typed query, the indices of the items that match
//! it, and the cursor position. It contains no terminal handling, so its
//! behavior can be unit tested without a TTY. A terminal front-end drives
//! one of these while rendering and reading key events.

use crate::fuzzy::contains_substring;
use std::cmp::Ordering;

/// A comparator that orders two items, held as a boxed closure so the selector
/// can carry any ordering.
type Comparator<'a, Item> = Box<dyn Fn(&Item, &Item) -> Ordering + 'a>;

/// An item that an interactive selector can filter by a typed query.
pub trait Searchable {
    /// The strings the query is matched against. A row matches when any
    /// of these contains the query as a case-insensitive substring.
    fn search_keys(&self) -> Vec<&str>;
}

/// The state of a selector over a borrowed slice of items.
pub struct Selector<'a, Item> {
    items: &'a [Item],
    query: String,
    /// Indices into `items` that currently match `query`, in display order:
    /// the order `order` imposes, or the original order when none is set.
    filtered: Vec<usize>,
    /// Position of the highlighted row within `filtered`.
    cursor: usize,
    /// The comparator that sorts the visible items, when one is set.
    order: Option<Comparator<'a, Item>>,
}

impl<'a, Item> Selector<'a, Item>
where
    Item: Searchable,
{
    /// Creates a selector with an empty query, so every item is visible.
    pub fn new(items: &'a [Item]) -> Self {
        let filtered = (0..items.len()).collect();
        Selector {
            items,
            query: String::new(),
            filtered,
            cursor: 0,
            order: None,
        }
    }

    /// Sets the comparator that orders the visible items and re-sorts them,
    /// keeping the highlight on the same item. The order is re-applied after
    /// every refilter, so it persists as the query changes.
    pub fn set_order(&mut self, order: impl Fn(&Item, &Item) -> Ordering + 'a) {
        let selected = self.selected_index();
        self.order = Some(Box::new(order));
        self.sort_filtered();
        if let Some(index) = selected {
            self.focus(index);
        }
    }

    /// Orders `filtered` by the current comparator, if one is set.
    fn sort_filtered(&mut self) {
        let Selector {
            items,
            filtered,
            order,
            ..
        } = self;
        if let Some(order) = order {
            let items = *items;
            let compare = &**order;
            filtered.sort_by(|&left, &right| compare(&items[left], &items[right]));
        }
    }

    /// The query typed so far.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Replaces the whole query and refilters. Used to restore a previous
    /// search; the cursor returns to the top, as after any refilter.
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.refilter();
    }

    /// Appends a character to the query and refilters.
    pub fn push_char(&mut self, char: char) {
        self.query.push(char);
        self.refilter();
    }

    /// Removes the last character of the query and refilters.
    pub fn pop_char(&mut self) {
        self.query.pop();
        self.refilter();
    }

    /// Moves the highlight one row towards the top.
    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the highlight one row towards the bottom, never past the last
    /// visible row.
    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.filtered.len() {
            self.cursor += 1;
        }
    }

    /// The indices of the currently visible items, in display order.
    pub fn filtered(&self) -> &[usize] {
        &self.filtered
    }

    /// The cursor position within the visible items.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The index, into the original slice, of the item under the cursor,
    /// if any item is visible.
    pub fn selected_index(&self) -> Option<usize> {
        self.filtered.get(self.cursor).copied()
    }

    /// Moves the highlight to the row showing the item at `index` in the
    /// original slice, when that item is currently visible. Used to restore a
    /// previous selection; an item that is filtered out leaves the cursor put.
    pub fn focus(&mut self, index: usize) {
        let position = self
            .filtered
            .iter()
            .position(|&candidate| candidate == index);
        if let Some(position) = position {
            self.cursor = position;
        }
    }

    /// Recomputes the visible items for the current query and resets the
    /// cursor to the top, so the highlight never points past the end of a
    /// shortened list.
    fn refilter(&mut self) {
        self.filtered = (0..self.items.len())
            .filter(|&index| {
                self.items[index]
                    .search_keys()
                    .iter()
                    .any(|key| contains_substring(key, &self.query))
            })
            .collect();
        self.sort_filtered();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests;
