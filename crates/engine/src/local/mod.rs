//! Shift+letter local modes (core-session.md §10, overlays.md §8.1): Unicode code points, date and time, quick phrases, emoji, kaomoji and super jianpin, plus the emoji / kaomoji / symbol catalog the host's picker pages through. Quick phrase and jianpin read the generation's `msime.db`; emoji and kaomoji read the resource `others.db`.

pub mod catalog;
pub mod database;
pub mod date_time;
pub mod emoji;
pub mod jianpin;
pub mod quick_phrase;
pub mod unicode;

use crate::types::WordItem;

/// A local query's rows and, when the database could not be read, the diagnostic the key result carries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LocalQueryResult {
    pub candidates: Vec<WordItem>,
    pub diagnostic: Option<String>,
}

impl LocalQueryResult {
    fn failure(diagnostic: &str) -> Self {
        Self {
            candidates: Vec::new(),
            diagnostic: Some(diagnostic.to_owned()),
        }
    }
}

/// A row limit as an SQLite integer. Limits are small constants, so saturation never changes a result.
fn sql_limit(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(i64::MAX)
}
