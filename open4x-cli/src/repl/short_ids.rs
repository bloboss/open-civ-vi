//! Compact, unique ID display for REPL entities.
//!
//! Given a set of ULID-backed IDs, computes the shortest trailing substring
//! of each raw ULID string that uniquely identifies it among the set.
//! The unique suffix is highlighted (bolded) when displayed.

use std::collections::HashMap;
use std::hash::Hash;

/// Maps entity IDs to their display strings with unique suffix information.
pub struct ShortIds<T: Copy + Eq + Hash> {
    entries: HashMap<T, ShortIdEntry>,
}

struct ShortIdEntry {
    /// Raw ULID string with leading zeros stripped.
    display: String,
    /// Byte index in `display` where the unique suffix starts.
    unique_start: usize,
}

/// Minimum unique suffix length for usability.
const MIN_SUFFIX_LEN: usize = 3;

impl<T: Copy + Eq + Hash> ShortIds<T> {
    /// Build from an iterator of `(entity_id, raw_ulid_string)` pairs.
    pub fn new(ids: impl IntoIterator<Item = (T, String)>) -> Self {
        let items: Vec<(T, String)> = ids.into_iter().collect();
        let mut entries = HashMap::with_capacity(items.len());

        for (id, full) in &items {
            let display = full.trim_start_matches('0').to_string();
            let display = if display.is_empty() {
                "0".to_string()
            } else {
                display
            };

            let mut suffix_len = MIN_SUFFIX_LEN.min(full.len());
            while suffix_len < full.len() {
                let suffix = &full[full.len() - suffix_len..];
                let count = items
                    .iter()
                    .filter(|(_, other)| other.ends_with(suffix))
                    .count();
                if count == 1 {
                    break;
                }
                suffix_len += 1;
            }

            // Map the suffix boundary from the full string to the display string.
            // The unique suffix occupies the last `suffix_len` chars of both strings.
            let unique_start = display.len().saturating_sub(suffix_len);

            entries.insert(
                *id,
                ShortIdEntry {
                    display,
                    unique_start,
                },
            );
        }

        Self { entries }
    }

    /// Full display string with the unique suffix portion bolded (ANSI).
    pub fn format_bold(&self, id: T) -> String {
        match self.entries.get(&id) {
            Some(e) => {
                let prefix = &e.display[..e.unique_start];
                let suffix = &e.display[e.unique_start..];
                format!("{prefix}\x1b[1m{suffix}\x1b[0m")
            }
            None => "?".to_string(),
        }
    }

    /// Just the unique suffix (for prompts and short references).
    pub fn short(&self, id: T) -> &str {
        match self.entries.get(&id) {
            Some(e) => &e.display[e.unique_start..],
            None => "?",
        }
    }

    /// Display string length (without ANSI codes), for column alignment.
    pub fn display_len(&self, id: T) -> usize {
        match self.entries.get(&id) {
            Some(e) => e.display.len(),
            None => 1,
        }
    }

    /// Find the entity whose ULID ends with the given suffix.
    /// Returns `None` if zero or multiple entities match.
    pub fn find_by_suffix(&self, suffix: &str) -> Option<T> {
        let upper = suffix.to_uppercase();
        let mut matches = self
            .entries
            .iter()
            .filter(|(_, e)| e.display.ends_with(&upper));
        let first = matches.next()?;
        if matches.next().is_some() {
            return None; // ambiguous
        }
        Some(*first.0)
    }
}
