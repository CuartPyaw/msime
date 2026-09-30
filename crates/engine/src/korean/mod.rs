//! Korean Hangul input on the Dubeolsik (2-beolsik) layout: a syllable automaton with no dictionary, no candidates, no cloud rows and no learning. A syllable composes in the preedit and is committed as soon as the next key starts another one; the session commits the open syllable on Space, Enter, punctuation and caret keys.

pub mod dubeolsik;
pub mod scheme;

pub use scheme::KoreanScheme;
