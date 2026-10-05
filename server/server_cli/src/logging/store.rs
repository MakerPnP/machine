//! Collects log records into a bounded buffer, for display by the tui.

use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Local};
use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::logging::rules::LevelRules;

pub struct LogEntry {
    /// Monotonic, never reused, so an entry can be referred to after older entries are evicted.
    pub seq: u64,
    pub timestamp: DateTime<Local>,
    pub level: Level,
    pub target: String,
    pub message: String,
}

impl LogEntry {
    pub fn format_timestamp(&self) -> impl std::fmt::Display {
        self.timestamp.format("%H:%M:%S%.3f")
    }
}

pub struct LogStore {
    inner: Mutex<LogStoreInner>,
}

pub struct LogStoreInner {
    entries: VecDeque<LogEntry>,
    /// The seq of the next entry.
    next_seq: u64,
    capacity: usize,
    rules: LevelRules,
    /// Incremented when the rules change.
    rules_generation: u64,
    /// The targets that have emitted records, or were added by the user, and how many records they emitted.
    targets: BTreeMap<String, u64>,
}

impl LogStore {
    pub fn new(rules: LevelRules, capacity: usize) -> Self {
        Self {
            inner: Mutex::new(LogStoreInner {
                entries: VecDeque::new(),
                next_seq: 0,
                capacity: capacity.max(1),
                rules,
                rules_generation: 0,
                targets: BTreeMap::new(),
            }),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, LogStoreInner> {
        // a panic while holding the lock can't leave the store inconsistent
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Installs a logger that collects records into this store.
    pub fn install(self: &Arc<Self>) -> Result<(), log::SetLoggerError> {
        let max_level = self.lock().rules.max_level();
        log::set_boxed_logger(Box::new(Collector(self.clone())))?;
        log::set_max_level(max_level);
        Ok(())
    }

    /// Writes the most recent `count` entries, that pass the level rules, to `out`.
    pub fn write_tail(&self, count: usize, out: &mut impl Write) -> std::io::Result<()> {
        let inner = self.lock();
        let entries: Vec<&LogEntry> = inner
            .entries
            .iter()
            .rev()
            .filter(|entry| inner.is_enabled(entry))
            .take(count)
            .collect();
        for entry in entries.into_iter().rev() {
            writeln!(
                out,
                "{} {:<5} {} {}",
                entry.format_timestamp(),
                entry.level,
                entry.target,
                entry.message
            )?;
        }
        Ok(())
    }
}

impl LogStoreInner {
    /// Counts a record for `target`, returning whether it should be collected.
    fn register(&mut self, target: &str, level: Level) -> bool {
        match self.targets.get_mut(target) {
            Some(count) => *count += 1,
            None => {
                self.targets
                    .insert(target.to_string(), 1);
            }
        }
        level <= self.rules.level_for(target)
    }

    pub fn push(&mut self, timestamp: DateTime<Local>, level: Level, target: &str, message: String) {
        self.entries.push_back(LogEntry {
            seq: self.next_seq,
            timestamp,
            level,
            target: target.to_string(),
            message,
        });
        self.next_seq += 1;
        self.evict();
    }

    fn evict(&mut self) {
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }

    /// The seq of the oldest entry, or of the next entry if there are none.
    pub fn first_seq(&self) -> u64 {
        self.next_seq - self.entries.len() as u64
    }

    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn get(&self, seq: u64) -> Option<&LogEntry> {
        let index = seq.checked_sub(self.first_seq())?;
        self.entries.get(index as usize)
    }

    /// The entries from `seq` onwards.
    pub fn entries_from(&self, seq: u64) -> impl Iterator<Item = &LogEntry> {
        let index = seq.saturating_sub(self.first_seq()) as usize;
        self.entries
            .range(index.min(self.entries.len())..)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Evicts the oldest entries if there are more than `capacity`.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        self.evict();
    }

    pub fn rules(&self) -> &LevelRules {
        &self.rules
    }

    pub fn rules_generation(&self) -> u64 {
        self.rules_generation
    }

    /// Sets or removes a level rule, see [`LevelRules::set`].
    ///
    /// Affects which entries are displayed, and which records are collected from now on.
    pub fn set_rule(&mut self, target: Option<&str>, level: Option<LevelFilter>) {
        self.rules.set(target, level);
        self.rules_generation += 1;
        log::set_max_level(self.rules.max_level());
    }

    /// Whether the entry passes the current level rules.
    pub fn is_enabled(&self, entry: &LogEntry) -> bool {
        entry.level <= self.rules.level_for(&entry.target)
    }

    pub fn targets(&self) -> &BTreeMap<String, u64> {
        &self.targets
    }

    /// Adds a target that hasn't emitted any records yet, so a level can be chosen for it.
    pub fn add_target(&mut self, target: &str) {
        self.targets
            .entry(target.to_string())
            .or_insert(0);
    }
}

struct Collector(Arc<LogStore>);

impl Log for Collector {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level()
            <= self
                .0
                .lock()
                .rules
                .level_for(metadata.target())
    }

    fn log(&self, record: &Record) {
        if !self
            .0
            .lock()
            .register(record.target(), record.level())
        {
            return;
        }
        let timestamp = Local::now();
        // formatted without holding the lock, in case formatting logs.
        let message = record.args().to_string();
        self.0
            .lock()
            .push(timestamp, record.level(), record.target(), message);
    }

    fn flush(&self) {}
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn push(store: &LogStore, level: Level, target: &str, message: &str) {
        let mut inner = store.lock();
        if inner.register(target, level) {
            inner.push(Local::now(), level, target, message.to_string());
        }
    }

    #[test]
    fn records_are_filtered_by_level() {
        let store = LogStore::new(LevelRules::parse("info,a=debug").rules, 10);
        push(&store, Level::Debug, "a::x", "1");
        push(&store, Level::Debug, "b", "2");
        push(&store, Level::Info, "b", "3");

        let inner = store.lock();
        let messages: Vec<&str> = inner
            .entries_from(0)
            .map(|entry| entry.message.as_str())
            .collect();
        assert_eq!(messages, ["1", "3"]);
        // targets are recorded even when their records are filtered out
        assert_eq!(inner.targets().get("b"), Some(&2));
    }

    #[test]
    fn oldest_entries_are_evicted() {
        let store = LogStore::new(LevelRules::with_global(LevelFilter::Info), 3);
        for i in 0..5 {
            push(&store, Level::Info, "a", &i.to_string());
        }

        let mut inner = store.lock();
        assert_eq!(inner.first_seq(), 2);
        assert_eq!(inner.next_seq(), 5);
        assert!(inner.get(1).is_none());
        assert_eq!(inner.get(3).unwrap().message, "3");
        assert_eq!(inner.entries_from(0).count(), 3);
        assert_eq!(inner.entries_from(4).count(), 1);
        assert_eq!(inner.entries_from(5).count(), 0);

        inner.set_capacity(1);
        assert_eq!(inner.first_seq(), 4);
        assert_eq!(inner.len(), 1);
    }

    #[test]
    fn tail_is_filtered_by_the_current_rules() {
        let store = LogStore::new(LevelRules::with_global(LevelFilter::Debug), 10);
        push(&store, Level::Info, "a", "1");
        push(&store, Level::Debug, "a", "2");
        push(&store, Level::Info, "a", "3");
        push(&store, Level::Info, "a", "4");
        store
            .lock()
            .set_rule(None, Some(LevelFilter::Info));

        let mut out = Vec::new();
        store.write_tail(2, &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        let messages: Vec<&str> = out
            .lines()
            .map(|line| line.rsplit(' ').next().unwrap())
            .collect();
        assert_eq!(messages, ["3", "4"]);
    }
}
