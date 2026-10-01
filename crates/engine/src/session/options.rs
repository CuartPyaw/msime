//! The public option and snapshot values of `Session` (`include/metasequoia/session.h`). Shared by the session, the nine-key session, the host facade and the golden harness.

use crate::paths::RuntimePaths;
use crate::types::{
    CandidateSource, CommandTableEntry, EnglishInputOptions, FrequencyAdjustmentOptions,
    FuzzyPinyinOptions, LocalInputMode, LocalModeOptions, MentionEntry, MixedExpressiveOptions,
    SchemeType, SentenceAssociationOptions, ShuangpinProfileKind, WordItem, WubiInputOptions,
};
use crate::vietnamese::{InputMethod as VietnameseInputMethod, ToneStyle as VietnameseToneStyle};

/// Everything a session is built with. `learning_undo` is gone with the feature.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionOptions {
    pub paths: RuntimePaths,
    pub scheme: SchemeType,
    pub shuangpin_profile: ShuangpinProfileKind,
    /// Shuangpin preedit shows the typed keys rather than the decoded quanpin.
    pub shuangpin_preedit_uses_raw: bool,
    /// How the Vietnamese scheme spells marks: Telex letters or VNI digits.
    pub vietnamese_input_method: VietnameseInputMethod,
    /// Where the Vietnamese scheme puts the tone on `oa`, `oe` and `uy`.
    pub vietnamese_tone_style: VietnameseToneStyle,
    pub helpcode_schema: String,
    /// `autocorrect_type` bits; 0 keeps the user's spelling. Either of transposition and neighbor also enables missing and extra letters, and on inputs of three or more complete syllables offers a sentence that reads one syllable as a typo. Committing the raw letters while a correction is offered turns correction off for that exact input.
    pub autocorrect_types: u32,
    pub helpcode: bool,
    pub chinese_punctuation: bool,
    pub paired_punctuation: bool,
    /// 0 follows the mode, 1 forces Chinese, 2 forces ASCII.
    pub punctuation_lock: i32,
    pub learning: bool,
    /// Hand back every whole-sentence reading instead of only the best. A host that sets this must reorder and crop them itself.
    pub sentence_alternatives: bool,
    pub fuzzy_pinyin: FuzzyPinyinOptions,
    pub frequency: FrequencyAdjustmentOptions,
    pub local_modes: LocalModeOptions,
    pub english: EnglishInputOptions,
    pub expressive: MixedExpressiveOptions,
    pub wubi: WubiInputOptions,
    /// Learn word sequences and pick pairs; also needs `learning`.
    pub personal_context: bool,
    pub sentence_association: SentenceAssociationOptions,
    /// Committed text the neural sentence models condition on; `Session::set_rescoring_context` updates it live.
    pub rescoring_context: String,
    /// The `/` mode's commands beyond the built-in ones; `Session::set_command_table` replaces it live.
    pub command_table: Vec<CommandTableEntry>,
    /// The `@` mode's names and places; `Session::set_mention_entries` replaces it live.
    pub mention_entries: Vec<MentionEntry>,
}

impl SessionOptions {
    /// The reference defaults (session.h:12-54) on the given paths.
    pub fn new(paths: RuntimePaths) -> Self {
        Self {
            paths,
            scheme: SchemeType::Quanpin,
            shuangpin_profile: ShuangpinProfileKind::Xiaohe,
            shuangpin_preedit_uses_raw: true,
            vietnamese_input_method: VietnameseInputMethod::Telex,
            vietnamese_tone_style: VietnameseToneStyle::Modern,
            helpcode_schema: "lantian".to_owned(),
            autocorrect_types: 0,
            helpcode: true,
            chinese_punctuation: true,
            paired_punctuation: true,
            punctuation_lock: 0,
            learning: true,
            sentence_alternatives: false,
            fuzzy_pinyin: FuzzyPinyinOptions::default(),
            frequency: FrequencyAdjustmentOptions::default(),
            local_modes: LocalModeOptions::default(),
            english: EnglishInputOptions::default(),
            expressive: MixedExpressiveOptions::default(),
            wubi: WubiInputOptions::default(),
            personal_context: true,
            sentence_association: SentenceAssociationOptions::default(),
            rescoring_context: String::new(),
            command_table: Vec::new(),
            mention_entries: Vec::new(),
        }
    }
}

/// A value copy of the composition; safe to hand to another thread. `candidate_sources`, `candidate_annotations` and `candidate_answers_key` are aligned with `candidates`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionSnapshot {
    pub scheme: SchemeType,
    pub local_mode: LocalInputMode,
    /// The non-letter characters `character` takes in this state: the active local mode's `spelling_symbols`, or while nothing is composed the keys that open the `/` and `@` modes. A host or runtime that would send one of these as punctuation sends it as a character instead.
    pub spelling_symbols: String,
    pub preedit: String,
    pub raw_segmentation: String,
    pub normalized_segmentation: String,
    pub candidates: Vec<WordItem>,
    pub dedicated_english: bool,
    /// The ASCII source text; `caret_position` is a byte offset into it.
    pub editing_text: String,
    pub caret_position: usize,
    pub nine_key_spellings: Vec<String>,
    /// The candidates came from the wubi mixed-pinyin fallback, not the wubi table.
    pub answered_by_pinyin_fallback: bool,
    pub wubi_unique_four_code: bool,
    pub shuangpin_profile: String,
    pub candidate_sources: Vec<CandidateSource>,
    /// Helpcodes or correction hints.
    pub candidate_annotations: Vec<String>,
    /// Whether each candidate answers the whole key rather than a prefix of it or a completion past it.
    pub candidate_answers_key: Vec<bool>,
    /// The scheme's openable candidate list is showing (the Korean Hanja list). Hosts read this instead of inferring it from the scheme and a non-empty list.
    pub candidate_list_open: bool,
}
