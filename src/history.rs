//! Flight history persistence for quick re-tracking.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY_SIZE: usize = 20;
const CONFIG_DIR: &str = "flight-tracker-tui";
const HISTORY_FILE: &str = "history.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub flight_number: String,
    /// Route info for display (e.g., "SFO→LHR")
    #[serde(default)]
    pub route: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    entries: VecDeque<HistoryEntry>,
}

impl History {
    /// Load history from the config file, or return empty history if not found.
    pub fn load() -> Self {
        let path = Self::config_path();

        if let Some(path) = path {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(history) = serde_json::from_str(&contents) {
                    return history;
                }
            }
        }

        Self::default()
    }

    /// Save history to the config file.
    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            // Ensure config directory exists
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if let Ok(contents) = serde_json::to_string_pretty(self) {
                let _ = fs::write(&path, contents);
            }
        }
    }

    /// Add a flight to history, moving it to the front if already present.
    pub fn add(&mut self, flight_number: String, route: Option<String>) {
        // Remove if already exists (we'll re-add at front)
        self.entries.retain(|e| e.flight_number != flight_number);

        // Add to front
        self.entries.push_front(HistoryEntry {
            flight_number,
            route,
        });

        // Trim to max size
        while self.entries.len() > MAX_HISTORY_SIZE {
            self.entries.pop_back();
        }
    }

    /// Get all history entries.
    pub fn entries(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter()
    }

    /// Get entries that match a prefix (for autocomplete suggestions).
    #[allow(dead_code)]
    pub fn matching(&self, prefix: &str) -> Vec<&HistoryEntry> {
        let prefix_upper = prefix.to_uppercase();
        self.entries
            .iter()
            .filter(|e| e.flight_number.starts_with(&prefix_upper))
            .collect()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of entries.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get the config file path.
    fn config_path() -> Option<PathBuf> {
        dirs_config_dir().map(|mut p| {
            p.push(CONFIG_DIR);
            p.push(HISTORY_FILE);
            p
        })
    }
}

/// Get the user's config directory.
fn dirs_config_dir() -> Option<PathBuf> {
    // Try XDG_CONFIG_HOME first, then fall back to ~/.config
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg));
    }

    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_add() {
        let mut history = History::default();

        history.add("UA123".to_string(), Some("SFO→LHR".to_string()));
        history.add("BA285".to_string(), None);

        assert_eq!(history.len(), 2);

        let entries: Vec<_> = history.entries().collect();
        assert_eq!(entries[0].flight_number, "BA285"); // Most recent first
        assert_eq!(entries[1].flight_number, "UA123");
    }

    #[test]
    fn test_history_add_duplicate_moves_to_front() {
        let mut history = History::default();

        history.add("UA123".to_string(), None);
        history.add("BA285".to_string(), None);
        history.add("UA123".to_string(), Some("SFO→LHR".to_string())); // Re-add with route

        assert_eq!(history.len(), 2);

        let entries: Vec<_> = history.entries().collect();
        assert_eq!(entries[0].flight_number, "UA123");
        assert_eq!(entries[0].route, Some("SFO→LHR".to_string()));
    }

    #[test]
    fn test_history_max_size() {
        let mut history = History::default();

        for i in 0..25 {
            history.add(format!("FL{:03}", i), None);
        }

        assert_eq!(history.len(), MAX_HISTORY_SIZE);

        // Most recent should be at front
        let entries: Vec<_> = history.entries().collect();
        assert_eq!(entries[0].flight_number, "FL024");
    }

    #[test]
    fn test_history_matching() {
        let mut history = History::default();

        history.add("UA123".to_string(), None);
        history.add("UA456".to_string(), None);
        history.add("BA285".to_string(), None);

        let matches = history.matching("UA");
        assert_eq!(matches.len(), 2);

        let matches = history.matching("ba"); // Case insensitive
        assert_eq!(matches.len(), 1);

        let matches = history.matching("XX");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_history_serialization() {
        let mut history = History::default();
        history.add("UA123".to_string(), Some("SFO→LHR".to_string()));

        let json = serde_json::to_string(&history).unwrap();
        let restored: History = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 1);
        let entries: Vec<_> = restored.entries().collect();
        assert_eq!(entries[0].flight_number, "UA123");
        assert_eq!(entries[0].route, Some("SFO→LHR".to_string()));
    }
}
