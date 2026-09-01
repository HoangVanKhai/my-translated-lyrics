//! The pure state of an interactive list or table selector.
//!
//! [`Selector`] holds the typed query, the indices of the items that match
//! it, and the cursor position. It contains no terminal handling, so its
//! behavior can be unit tested without a TTY. A terminal front-end drives
//! one of these while rendering and reading key events.
//!
//! Two index spaces meet here: [`FilteredIndex`] names a visible row and
//! [`ItemIndex`] names an entry of the borrowed slice. [`Selector::item_at`]
//! converts one into the other.

use crate::fuzzy::contains_substring;
use pipe_trait::Pipe;
use std::cmp::Ordering;
use std::ops::Index;

/// A comparator that orders two items, held as a boxed closure so the selector
/// can carry any ordering.
type Comparator<'a, Item> = Box<dyn Fn(&Item, &Item) -> Ordering + 'a>;

/// A position within the filtered view: which visible row is meant, counting
/// from the top of what the query currently shows. The visible rows are
/// numbered from zero upwards without gaps, however many items the query
/// hides between them.
///
/// It is not an index into the borrowed slice of items;
/// [`Selector::item_at`] is the only conversion between the two spaces.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct FilteredIndex(usize);

impl FilteredIndex {
    /// The topmost visible row.
    pub const FIRST: FilteredIndex = FilteredIndex(0);

    /// The visible row `position` rows from the top of the filtered view.
    pub const fn new(position: usize) -> FilteredIndex {
        FilteredIndex(position)
    }

    /// The row's distance from the top of the filtered view.
    pub const fn get(self) -> usize {
        self.0
    }

    /// The row `rows` further down.
    pub fn down_by(self, rows: usize) -> FilteredIndex {
        FilteredIndex(self.0 + rows)
    }
}

/// An index into the borrowed slice of items, which neither a query nor an
/// ordering renumbers.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ItemIndex(usize);

impl ItemIndex {
    /// The item `index` places into the borrowed slice.
    pub const fn new(index: usize) -> ItemIndex {
        ItemIndex(index)
    }

    /// The item's position in the borrowed slice.
    pub const fn get(self) -> usize {
        self.0
    }

    /// The item before this one, or the first item when this is already the
    /// first.
    pub fn previous(self) -> ItemIndex {
        self.0.saturating_sub(1).pipe(ItemIndex)
    }

    /// The item after this one, which may be past the end of the slice.
    pub fn next(self) -> ItemIndex {
        ItemIndex(self.0 + 1)
    }
}

/// Reads the entry an [`ItemIndex`] names, panicking past the end of the
/// slice as indexing by a number does.
impl<Item> Index<ItemIndex> for [Item] {
    type Output = Item;

    fn index(&self, index: ItemIndex) -> &Item {
        &self[index.get()]
    }
}

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
    filtered: Vec<ItemIndex>,
    /// Position of the highlighted row within `filtered`.
    cursor: FilteredIndex,
    /// The comparator that sorts the visible items, when one is set.
    order: Option<Comparator<'a, Item>>,
}

impl<'a, Item> Selector<'a, Item>
where
    Item: Searchable,
{
    /// Creates a selector with an empty query, so every item is visible.
    pub fn new(items: &'a [Item]) -> Self {
        let filtered = (0..items.len()).map(ItemIndex::new).collect();
        Selector {
            items,
            query: String::new(),
            filtered,
            cursor: FilteredIndex::FIRST,
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
    pub fn set_query(&mut self, query: String) {
        self.query = query;
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

    /// Moves the highlight one row towards the top, never above the topmost
    /// visible row.
    pub fn move_up(&mut self) {
        self.cursor = self.cursor.0.saturating_sub(1).pipe(FilteredIndex);
    }

    /// Moves the highlight one row towards the bottom, never past the last
    /// visible row.
    pub fn move_down(&mut self) {
        if self.cursor.0 + 1 < self.filtered.len() {
            self.cursor = FilteredIndex(self.cursor.0 + 1);
        }
    }

    /// The currently visible items, in display order.
    pub fn filtered(&self) -> &[ItemIndex] {
        &self.filtered
    }

    /// The cursor position within the visible items.
    pub fn cursor(&self) -> FilteredIndex {
        self.cursor
    }

    /// The item shown on the visible row `position`, if the filtered view
    /// reaches that far.
    pub fn item_at(&self, position: FilteredIndex) -> Option<ItemIndex> {
        self.filtered.get(position.get()).copied()
    }

    /// The item under the cursor, if any item is visible.
    pub fn selected_index(&self) -> Option<ItemIndex> {
        self.item_at(self.cursor)
    }

    /// Moves the highlight to the row showing `index`, when that item is
    /// currently visible. Used to restore a previous selection; an item that
    /// is filtered out leaves the cursor put.
    pub fn focus(&mut self, index: ItemIndex) {
        let position = self
            .filtered
            .iter()
            .position(|&candidate| candidate == index);
        if let Some(position) = position {
            self.cursor = FilteredIndex::new(position);
        }
    }

    /// Recomputes the visible items for the current query and resets the
    /// cursor to the top, so the highlight never points past the end of a
    /// shortened list.
    fn refilter(&mut self) {
        self.filtered = (0..self.items.len())
            .map(ItemIndex::new)
            .filter(|&index| {
                self.items[index]
                    .search_keys()
                    .iter()
                    .any(|key| contains_substring(key, &self.query))
            })
            .collect();
        self.sort_filtered();
        self.cursor = FilteredIndex::FIRST;
    }
}

#[cfg(test)]
mod tests;
