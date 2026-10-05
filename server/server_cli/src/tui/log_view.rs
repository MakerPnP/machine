//! Which log entries the log panel shows: filtering, following new entries and scrolling.
//!
//! Positions are tracked by entry `seq`, which is monotonic, so a locked view stays put while new entries are added and
//! old ones evicted.

use std::collections::VecDeque;
use std::ops::Range;

use regex::Regex;

use crate::logging::store::{LogEntry, LogStoreInner};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scroll {
    /// Shows the most recent entries.
    Follow,
    /// Shows the entries from `top_seq` onwards, regardless of new entries.
    Locked { top_seq: u64 },
}

pub struct LogView {
    scroll: Scroll,
    /// The seqs of the entries that pass the filters, ascending.
    filtered: VecDeque<u64>,
    /// The seq of the next entry to consider for `filtered`.
    next_seq: u64,
    /// The rules generation `filtered` was built with, `None` to rebuild.
    rules_generation: Option<u64>,
    regex: Option<Regex>,
    /// How many entries were evicted from above a locked view, since the user last scrolled.
    evicted: u64,
    /// The number of entries that fit in the panel.
    height: usize,
}

impl LogView {
    pub fn new() -> Self {
        Self {
            scroll: Scroll::Follow,
            filtered: VecDeque::new(),
            next_seq: 0,
            rules_generation: None,
            regex: None,
            evicted: 0,
            height: 1,
        }
    }

    pub fn scroll(&self) -> Scroll {
        self.scroll
    }

    pub fn evicted(&self) -> u64 {
        self.evicted
    }

    /// The number of entries that pass the filters.
    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    pub fn regex(&self) -> Option<&Regex> {
        self.regex.as_ref()
    }

    /// Sets the message filter, takes effect on the next [`Self::sync`].
    pub fn set_regex(&mut self, regex: Option<Regex>) {
        self.regex = regex;
        self.rules_generation = None;
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height.max(1);
    }

    fn passes(&self, store: &LogStoreInner, entry: &LogEntry) -> bool {
        store.is_enabled(entry)
            && self
                .regex
                .as_ref()
                .is_none_or(|regex| regex.is_match(&entry.message))
    }

    /// Updates the filtered entries with the store's new and evicted entries.
    pub fn sync(&mut self, store: &LogStoreInner) {
        let rebuild = self.rules_generation != Some(store.rules_generation());
        if rebuild {
            self.filtered.clear();
            self.next_seq = 0;
            self.rules_generation = Some(store.rules_generation());
        }

        let first_seq = store.first_seq();
        while self
            .filtered
            .front()
            .is_some_and(|&seq| seq < first_seq)
        {
            self.filtered.pop_front();
        }

        let new: Vec<u64> = store
            .entries_from(self.next_seq)
            .filter(|entry| self.passes(store, entry))
            .map(|entry| entry.seq)
            .collect();
        self.filtered.extend(new);
        self.next_seq = store.next_seq();

        if let Scroll::Locked { top_seq } = &mut self.scroll {
            if *top_seq < first_seq {
                // the entries the view was locked on are gone, stay locked on the oldest remaining entry
                self.evicted += first_seq - *top_seq;
                *top_seq = self
                    .filtered
                    .front()
                    .copied()
                    .unwrap_or(first_seq);
            } else if rebuild && self.top_index() + self.height >= self.filtered.len() {
                // after a filter change, the locked entry is on the last page, so follow instead of showing a partial page
                self.follow();
            }
        }
    }

    /// The index, in the filtered entries, of the top visible entry.
    fn top_index(&self) -> usize {
        match self.scroll {
            Scroll::Follow => self
                .filtered
                .len()
                .saturating_sub(self.height),
            Scroll::Locked { top_seq } => self
                .filtered
                .partition_point(|&seq| seq < top_seq),
        }
    }

    fn visible_range(&self) -> Range<usize> {
        let top = self.top_index();
        top..(top + self.height).min(self.filtered.len())
    }

    /// The seqs of the visible entries, top to bottom.
    pub fn visible(&self) -> impl Iterator<Item = u64> + '_ {
        self.filtered
            .range(self.visible_range())
            .copied()
    }

    /// The position of the visible entries in the filtered entries, for a scrollbar.
    pub fn position(&self) -> usize {
        self.top_index()
    }

    fn lock_at(&mut self, index: usize) {
        if let Some(&seq) = self.filtered.get(index) {
            self.scroll = Scroll::Locked { top_seq: seq };
        }
        self.evicted = 0;
    }

    pub fn up(&mut self, count: usize) {
        let top = self.top_index();
        self.lock_at(top.saturating_sub(count));
    }

    pub fn down(&mut self, count: usize) {
        if self.scroll == Scroll::Follow {
            return;
        }
        let top = self.top_index() + count;
        if top + self.height >= self.filtered.len() {
            self.follow();
        } else {
            self.lock_at(top);
        }
    }

    pub fn page_up(&mut self) {
        self.up(self.height);
    }

    pub fn page_down(&mut self) {
        self.down(self.height);
    }

    pub fn first(&mut self) {
        self.lock_at(0);
    }

    pub fn follow(&mut self) {
        self.scroll = Scroll::Follow;
        self.evicted = 0;
    }
}

#[cfg(test)]
mod tests {
    use log::{Level, LevelFilter};

    use super::*;
    use crate::logging::rules::LevelRules;
    use crate::logging::store::LogStore;
    use crate::logging::store::tests::push;

    fn store(capacity: usize, count: usize) -> LogStore {
        let store = LogStore::new(LevelRules::with_global(LevelFilter::Debug), capacity);
        for i in 0..count {
            add(&store, i);
        }
        store
    }

    fn add(store: &LogStore, i: usize) {
        // every third entry is debug, the others info
        let level = if i % 3 == 0 { Level::Debug } else { Level::Info };
        push(store, level, "a", &format!("message {}", i));
    }

    fn view(store: &LogStore, height: usize) -> LogView {
        let mut view = LogView::new();
        view.set_height(height);
        view.sync(&store.lock());
        view
    }

    fn visible(view: &LogView) -> Vec<u64> {
        view.visible().collect()
    }

    #[test]
    fn follow_shows_the_most_recent_entries() {
        let store = store(100, 10);
        let mut view = view(&store, 3);
        assert_eq!(visible(&view), [7, 8, 9]);

        add(&store, 10);
        view.sync(&store.lock());
        assert_eq!(visible(&view), [8, 9, 10]);
    }

    #[test]
    fn scrolling_up_locks_the_view() {
        let store = store(100, 10);
        let mut view = view(&store, 3);
        view.up(1);
        assert_eq!(view.scroll(), Scroll::Locked { top_seq: 6 });
        assert_eq!(visible(&view), [6, 7, 8]);

        add(&store, 10);
        add(&store, 11);
        view.sync(&store.lock());
        assert_eq!(visible(&view), [6, 7, 8]);
    }

    #[test]
    fn scrolling_down_to_the_end_follows() {
        let store = store(100, 10);
        let mut view = view(&store, 3);
        view.page_up();
        assert_eq!(visible(&view), [4, 5, 6]);
        view.down(1);
        assert_eq!(visible(&view), [5, 6, 7]);
        view.down(1);
        assert_eq!(visible(&view), [6, 7, 8]);
        // reaching the last page follows
        view.down(1);
        assert_eq!(view.scroll(), Scroll::Follow);
        assert_eq!(visible(&view), [7, 8, 9]);
    }

    #[test]
    fn first_and_page_down() {
        let store = store(100, 10);
        let mut view = view(&store, 3);
        view.first();
        assert_eq!(visible(&view), [0, 1, 2]);
        view.up(1);
        assert_eq!(visible(&view), [0, 1, 2]);
        view.page_down();
        assert_eq!(visible(&view), [3, 4, 5]);
        view.page_down();
        assert_eq!(visible(&view), [6, 7, 8]);
        view.page_down();
        assert_eq!(view.scroll(), Scroll::Follow);
        assert_eq!(visible(&view), [7, 8, 9]);
    }

    #[test]
    fn evicted_lock_moves_to_the_oldest_entry() {
        let store = store(10, 10);
        let mut view = view(&store, 3);
        view.first();
        view.down(2);
        assert_eq!(visible(&view), [2, 3, 4]);

        // evicts 0..=3, including the top visible entry
        for i in 10..14 {
            add(&store, i);
        }
        view.sync(&store.lock());
        assert_eq!(view.scroll(), Scroll::Locked { top_seq: 4 });
        assert_eq!(view.evicted(), 2);
        assert_eq!(visible(&view), [4, 5, 6]);

        // scrolling still works, from the new position
        view.down(1);
        assert_eq!(visible(&view), [5, 6, 7]);
        assert_eq!(view.evicted(), 0);
    }

    #[test]
    fn locked_view_survives_eviction_of_older_entries() {
        let store = store(10, 10);
        let mut view = view(&store, 3);
        view.up(2);
        assert_eq!(visible(&view), [5, 6, 7]);

        for i in 10..13 {
            add(&store, i);
        }
        view.sync(&store.lock());
        assert_eq!(visible(&view), [5, 6, 7]);
        view.up(1);
        assert_eq!(visible(&view), [4, 5, 6]);
    }

    #[test]
    fn level_rules_filter_the_entries() {
        let store = store(100, 10);
        let mut view = view(&store, 3);
        store
            .lock()
            .set_rule(Some("a"), Some(LevelFilter::Info));
        view.sync(&store.lock());
        assert_eq!(visible(&view), [5, 7, 8]);
        assert_eq!(view.len(), 6);
    }

    #[test]
    fn regex_filters_the_messages() {
        let store = store(100, 20);
        let mut view = view(&store, 3);
        view.set_regex(Some(Regex::new("message 1").unwrap()));
        view.sync(&store.lock());
        assert_eq!(visible(&view), [17, 18, 19]);
        view.first();
        assert_eq!(visible(&view), [1, 10, 11]);
    }

    #[test]
    fn locked_view_keeps_its_position_across_a_filter_change() {
        let store = store(100, 30);
        let mut view = view(&store, 3);
        view.first();
        view.down(10);
        assert_eq!(visible(&view), [10, 11, 12]);

        // the locked entry no longer passes, the view starts at the next one that does
        view.set_regex(Some(Regex::new("message (11|12|2)").unwrap()));
        view.sync(&store.lock());
        assert_eq!(view.scroll(), Scroll::Locked { top_seq: 10 });
        assert_eq!(visible(&view), [11, 12, 20]);
    }

    #[test]
    fn locked_view_on_the_last_page_follows_after_a_filter_change() {
        let store = store(100, 30);
        let mut view = view(&store, 3);
        view.first();
        view.down(10);

        view.set_regex(Some(Regex::new("message (11|12)").unwrap()));
        view.sync(&store.lock());
        assert_eq!(view.scroll(), Scroll::Follow);
        assert_eq!(visible(&view), [11, 12]);
    }
}
