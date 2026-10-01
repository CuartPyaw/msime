//! Cantonese input in toneless Jyutping, read against `cantonese.db` (`language_dictionary`). Output is Traditional as stored, and nothing is learned.

/// The non-letter keys the scheme spells with while composing: `'` is an explicit syllable boundary.
pub const SPELLING_SYMBOLS_COMPOSING: &str = "'";
