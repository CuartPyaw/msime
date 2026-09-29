//! The host facade: the surface `crates/engine-bridge` exposed to host-api and input-runtime (api-contract.md), so the bridge can re-export this module and its callers stay unchanged. It holds the logic that lived in `bridge.cpp` rather than in the engine: the `EngineOptions` mapping and its error strings, the flattened snapshot with the helpcode annotation rule and the nine-key and Microsoft mirrors, the raw-commit learning policy and `CommitRawWithoutLearning`, and the session-free dictionary, gloss, catalog and text helpers.
//!
//! Errors are `EngineError`, whose `Display` is the C++ exception text. Callers read `cxx::Exception` through `to_string()`, except `host-api/src/lib.rs:1027`, which calls `.what()` and becomes `to_string()` when the bridge is re-pointed. Audio capture and offline handwriting recognition are not here (see modules.md).

pub mod dictionary;
pub mod glosses;
pub mod options;
pub mod session;
#[cfg(test)]
mod tests;
pub mod text;

pub use crate::local::catalog::{EmojiCatalogItem, EmojiCatalogSlice, EmojiSymbolGroup};
pub use crate::shuangpin::hints::ShuangpinKeyHint;
pub use crate::types::{CandidateEdge, SentenceAssociationOptions};
pub use dictionary::{
    dictionary_edit, dictionary_edit_bundled, dictionary_entries, dictionary_export_entries,
    dictionary_state_revision, dictionary_table_entries, dictionary_validate,
    replay_user_dictionary, reset_learned_data, stage_dictionary_state, DictionaryEntry,
    DictionaryKind, DictionaryPage, DictionaryStateRecord, DictionaryTableEntry,
    DictionaryTablePage, SnapshotReadError,
};
pub use glosses::{
    candidate_glosses, candidate_glosses_with_user, candidate_target_glosses, english_completions,
    save_candidate_gloss,
};
pub use options::{prepare_options, EngineOptions};
pub use session::{Command, EngineResult, EngineSnapshot, OnlineQuerySnapshot, Session};
pub use text::{
    emoji_catalog_filtered_page, emoji_catalog_groups, emoji_catalog_slice, emoji_symbol_groups,
    handwriting_order_candidates, hanzi_to_pinyin, normalize_full_pinyin, shuangpin_key_hints,
};
