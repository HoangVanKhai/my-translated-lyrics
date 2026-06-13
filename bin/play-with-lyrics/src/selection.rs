//! The pure state of an interactive list or table selector.
//!
//! [`Selector`] holds the typed query, the indices of the items that match
//! it, and the cursor position. It contains no terminal handling, so its
//! behavior can be unit tested without a TTY. The [`crate::tui`] module
//! drives one of these while rendering and reading key events.

use crate::fuzzy::contains_ci;

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
    /// Indices into `items` that currently match `query`, in their
    /// original order.
    filtered: Vec<usize>,
    /// Position of the highlighted row within `filtered`.
    cursor: usize,
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
        }
    }

    /// The query typed so far.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Appends a character to the query and refilters.
    pub fn push_char(&mut self, character: char) {
        self.query.push(character);
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

    /// Recomputes the visible items for the current query and resets the
    /// cursor to the top, so the highlight never points past the end of a
    /// shortened list.
    fn refilter(&mut self) {
        self.filtered = (0..self.items.len())
            .filter(|&index| {
                self.items[index]
                    .search_keys()
                    .iter()
                    .any(|key| contains_ci(key, &self.query))
            })
            .collect();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests;
