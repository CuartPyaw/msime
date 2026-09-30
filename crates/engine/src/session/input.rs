//! `InputSession` (core-session.md §5): the platform-neutral composition and commit policy. Three mutually exclusive views drive every getter: dedicated English, a local mode, or the scheme composition. Its behaviour is split over this module's sibling files by concern; this file holds the state and the key, command and punctuation dispatch.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use super::chain::CommitChain;
use super::clock::Clock;
use super::online::OnlineRequestGuard;
use super::options::SessionOptions;
use crate::assets;
use crate::diagnostics;
use crate::error::{EngineError, Result};
use crate::helpcode::{is_supported_helpcode_schema, load_helpcode_keymap, SharedKeymap};
use crate::ime::queries::CandidateQueries;
use crate::ime::ImeSession;
use crate::local::date_time::LocalDateTime;
use crate::paths::RuntimePaths;
use crate::punctuation::PunctuationPolicy;
use crate::quanpin::QuanpinEngine;
use crate::shuangpin::profile::profile;
use crate::shuangpin::ShuangpinProfile;
use crate::types::{
    CandidateSource, Command, EnglishInputOptions, FrequencyAdjustmentOptions, KeyResult,
    LocalInputMode, LocalModeOptions, MixedExpressiveOptions, SchemeKey, SchemeType,
    ShuangpinProfileKind, WordItem, WubiInputOptions,
};
use crate::user_dictionary::ngram_store::PersonalNgramStore;
use crate::user_dictionary::removal::learn_entered_english_word;
use crate::user_dictionary::typo_profile::PersonalTypoProfile;

/// The weight an English word typed out and committed raw enters `english.db` with (user_dictionary_journal.h:137).
const ENTERED_ENGLISH_WORD_WEIGHT: i64 = 10;

/// A phrase being composed from consecutive partial selections.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct CreatingWordProgress {
    pub pinyin: String,
    pub word: String,
    pub preedit: String,
    pub completed: bool,
    pub can_store: bool,
}

pub(super) struct InputSession {
    pub paths: RuntimePaths,
    pub engine: ImeSession,
    pub queries: CandidateQueries,
    pub clock: Clock,
    pub profile: ShuangpinProfileKind,
    pub caret: Option<usize>,
    pub phrase_progress: CreatingWordProgress,
    pub pending_sequence: Option<String>,
    pub pending_sequence_with_cases: Option<String>,
    pub autocorrect_types: u32,
    pub quanpin_helpcode_enabled: bool,
    pub shuangpin_helpcode_enabled: bool,
    pub helpcode_keymap: Option<SharedKeymap>,
    pub chinese_punctuation_enabled: bool,
    pub punctuation_lock: i32,
    pub learning_enabled: bool,
    pub personal_context_enabled: bool,
    pub punctuation: PunctuationPolicy,
    pub frequency: FrequencyAdjustmentOptions,
    pub local_mode_options: LocalModeOptions,
    pub english_options: EnglishInputOptions,
    pub expressive_options: MixedExpressiveOptions,
    pub dedicated_english: bool,
    pub dedicated_english_preedit: String,
    pub dedicated_english_candidates: Vec<WordItem>,
    pub mixed_candidates: Vec<WordItem>,
    pub local_mode: LocalInputMode,
    /// The scheme to return to after temporary Japanese.
    pub temporary_original_scheme: Option<SchemeType>,
    pub local_preedit: String,
    pub local_candidates: Vec<WordItem>,
    pub online_requests: OnlineRequestGuard,
    pub chain: CommitChain,
    pub personal_context: Arc<PersonalNgramStore>,
    pub typo_profile: Arc<PersonalTypoProfile>,
    pub personal_reranked: bool,
    /// The list before the personal rerank; learning, pinning and index-0 checks use this order.
    pub ranking_candidates: Option<Vec<WordItem>>,
    /// Caret-prefix decoding (overlays.md §7.6).
    pub prefix_active: bool,
    pub prefix_query_input: String,
    pub prefix_candidates: Vec<WordItem>,
    pub shuangpin_preedit_uses_raw: bool,
    /// Writes phrases under complete quanpin keys whatever the active scheme is, so a shuangpin session never feeds a canonical key back through its profile; opened on first use.
    pub canonical_phrase_engine: Option<QuanpinEngine>,
}

impl InputSession {
    /// Everything `Session::new` applies, in the reference order.
    pub fn new(options: &SessionOptions) -> Result<Self> {
        let paths = options.paths.clone();
        let journal = paths.user(assets::USER_JOURNAL);
        let mut engine = ImeSession::new(options.scheme, options.shuangpin_profile, &paths);
        engine.set_autocorrect_types(0);
        engine.set_quanpin_helpcode_enabled(true);
        engine.set_shuangpin_helpcode_enabled(true);
        let mut session = Self {
            queries: CandidateQueries::new(&paths, options.shuangpin_profile),
            engine,
            clock: Clock::default(),
            profile: options.shuangpin_profile,
            caret: None,
            phrase_progress: CreatingWordProgress::default(),
            pending_sequence: None,
            pending_sequence_with_cases: None,
            autocorrect_types: 0,
            quanpin_helpcode_enabled: true,
            shuangpin_helpcode_enabled: true,
            helpcode_keymap: None,
            chinese_punctuation_enabled: true,
            punctuation_lock: 0,
            learning_enabled: true,
            personal_context_enabled: true,
            punctuation: PunctuationPolicy::default(),
            frequency: FrequencyAdjustmentOptions::default(),
            local_mode_options: LocalModeOptions::default(),
            english_options: EnglishInputOptions::default(),
            expressive_options: MixedExpressiveOptions::default(),
            dedicated_english: false,
            dedicated_english_preedit: String::new(),
            dedicated_english_candidates: Vec::new(),
            mixed_candidates: Vec::new(),
            local_mode: LocalInputMode::None,
            temporary_original_scheme: None,
            local_preedit: String::new(),
            local_candidates: Vec::new(),
            online_requests: OnlineRequestGuard::new(),
            chain: CommitChain::default(),
            personal_context: PersonalNgramStore::for_journal(&journal),
            typo_profile: PersonalTypoProfile::shared(&journal),
            personal_reranked: false,
            ranking_candidates: None,
            prefix_active: false,
            prefix_query_input: String::new(),
            prefix_candidates: Vec::new(),
            shuangpin_preedit_uses_raw: true,
            canonical_phrase_engine: None,
            paths,
        };

        // session.cpp:15-29.
        session.shuangpin_preedit_uses_raw = options.shuangpin_preedit_uses_raw;
        session.set_autocorrect_types(options.autocorrect_types);
        session
            .engine
            .set_fuzzy_pinyin_options(options.fuzzy_pinyin);
        session.set_quanpin_helpcode_enabled(options.helpcode);
        session.set_shuangpin_helpcode_enabled(options.helpcode);
        if !is_supported_helpcode_schema(&options.helpcode_schema)
            || !frequency_options_valid(options.frequency)
            || !english_options_valid(options.english)
        {
            return Err(EngineError::invalid(diagnostics::INVALID_SESSION_OPTIONS));
        }
        // A missing custom table passes the name check; its load error (`UNKNOWN_HELPCODE_SCHEMA`) is what the caller sees.
        session.install_helpcode_keymap(&options.helpcode_schema)?;
        session.frequency = options.frequency;
        session.english_options = options.english;
        session.set_local_mode_options(options.local_modes);
        session.expressive_options = options.expressive;
        session.set_wubi_input_options(options.wubi);
        session.set_personal_context_enabled(options.personal_context);

        // session.cpp:61-65, plus the neural switches the patch threaded through the same place.
        session.chinese_punctuation_enabled = options.chinese_punctuation;
        session
            .punctuation
            .set_paired_enabled(options.paired_punctuation);
        session.punctuation_lock = options.punctuation_lock;
        session.learning_enabled = options.learning;
        session
            .engine
            .set_sentence_alternatives(options.sentence_alternatives);
        session
            .engine
            .set_sentence_association(options.sentence_association);
        session
            .engine
            .set_rescoring_context(&options.rescoring_context);
        session.update_mixed_candidates();
        Ok(session)
    }

    /// input_session.cpp:88-245: caret insertion, dedicated English, local modes and their Shift entries, the acceptance gate, then the scheme. Handled iff the preedit changed.
    pub fn handle_character(&mut self, value: u8, shift_only: bool) -> KeyResult {
        if self.caret_position() < self.editing_text().len() {
            return self.insert_at_caret(value);
        }
        self.caret = None;
        if self.dedicated_english {
            // Every other key is swallowed, even with nothing composed, so the English mode never leaks Chinese punctuation or digits into the host (core-session.md §15.2).
            if value.is_ascii_alphabetic() {
                self.dedicated_english_preedit.push(value as char);
                self.update_dedicated_english_candidates();
            }
            return KeyResult::handled();
        }
        if self.local_mode != LocalInputMode::None {
            return self.handle_local_character(value);
        }
        if self.is_korean() {
            return self.handle_korean_character(value);
        }
        if shift_only && !self.has_composition() && self.scheme().is_pinyin() {
            if let Some(mode) = self.local_mode_for_entry(value) {
                self.enter_local_mode(mode, value);
                return KeyResult::handled();
            }
        }

        let scheme = self.scheme();
        let lowercase_letter = value.is_ascii_lowercase();
        let microsoft_final = value == b';'
            && scheme == SchemeType::Shuangpin
            && self.profile == ShuangpinProfileKind::Microsoft;
        let japanese_long_vowel = value == b'-' && scheme == SchemeType::JapaneseRomaji;
        let active_helpcode = value.is_ascii_uppercase()
            && self.has_composition()
            && ((scheme == SchemeType::Quanpin && self.quanpin_helpcode_enabled)
                || (scheme == SchemeType::Shuangpin && self.shuangpin_helpcode_enabled));
        if !lowercase_letter
            && !active_helpcode
            && value != b'\''
            && !microsoft_final
            && !japanese_long_vowel
        {
            // With nothing composed the host inserts the key itself (a digit, a space, a capital), so the next word no longer follows the last one.
            if !self.has_composition() {
                self.reset_commit_context();
            }
            return KeyResult::unhandled();
        }
        if value == b'\'' && !self.has_composition() {
            self.reset_commit_context();
            return KeyResult::unhandled();
        }
        // A long pause before a new composition usually means the user moved to another field or application, which a host that never calls reset_context() does not report.
        if !self.has_composition()
            && self.chain.previous.is_some()
            && self.chain.paused(self.steady_now())
        {
            self.chain.reset();
        }

        let previous_preedit = self.preedit();
        let key = if value == b'\'' {
            SchemeKey::Apostrophe
        } else if microsoft_final {
            SchemeKey::Semicolon
        } else if japanese_long_vowel {
            SchemeKey::Minus
        } else {
            SchemeKey::Letter(value)
        };
        self.engine.handle_key(key);
        self.update_mixed_candidates();
        // A key the scheme ignores (a fifth wubi letter, a second apostrophe) leaves the preedit alone and goes back to the host.
        if self.preedit() == previous_preedit {
            return KeyResult::unhandled();
        }
        self.online_requests.invalidate();
        KeyResult::handled()
    }

    /// input_session.cpp:958-1036.
    pub fn handle_local_character(&mut self, value: u8) -> KeyResult {
        let letter = value.is_ascii_alphabetic();
        let accepted = match self.local_mode {
            // The temporary modes hand a rejected key back to the host; the others swallow it (core-session.md §15.3).
            LocalInputMode::TemporaryEnglish => {
                if !letter {
                    return KeyResult::unhandled();
                }
                true
            }
            LocalInputMode::TemporaryJapanese => {
                if !letter && value != b'\'' {
                    return KeyResult::unhandled();
                }
                let key = if value == b'\'' {
                    SchemeKey::Apostrophe
                } else {
                    SchemeKey::Letter(value)
                };
                self.engine.handle_key(key);
                self.refresh_temporary_japanese();
                return KeyResult::handled();
            }
            LocalInputMode::SuperJianpin => letter,
            LocalInputMode::Emoji | LocalInputMode::Kaomoji => letter || value == b'\'',
            LocalInputMode::QuickPhrase | LocalInputMode::DateTime => value.is_ascii_lowercase(),
            LocalInputMode::Unicode => {
                value.is_ascii_hexdigit() || (value == b'+' && self.local_preedit == "U")
            }
            LocalInputMode::None => return KeyResult::unhandled(),
        };
        if !accepted {
            return KeyResult::handled();
        }
        self.local_preedit.push(value as char);
        KeyResult::handled().with_diagnostic(self.update_local_candidates())
    }

    /// Korean keys: every letter is a jamo and always handled, carrying the syllables it finished as a commit. Any other key leaves the scheme alone. Punctuation stays unhandled without touching the composition, because the punctuation route commits the open syllable ahead of the mark; a digit, a space or another symbol commits the open syllable and stays unhandled, so the host inserts the key after the commit.
    fn handle_korean_character(&mut self, value: u8) -> KeyResult {
        if !value.is_ascii_alphabetic() {
            if !self.has_composition() {
                self.reset_commit_context();
                return KeyResult::unhandled();
            }
            if value.is_ascii_punctuation() {
                return KeyResult::unhandled();
            }
            return self.commit_korean_composition();
        }
        self.engine.handle_key(SchemeKey::Letter(value));
        let finished = self.engine.take_korean_commit();
        self.update_mixed_candidates();
        self.online_requests.invalidate();
        if finished.is_empty() {
            return KeyResult::handled();
        }
        self.chain.reset();
        KeyResult::committed(finished)
    }

    /// The open Korean syllable goes to the host and the key that ended it does not: the result is unhandled, so Space, Enter, a caret key or a digit still does its own work after the commit, as in every Korean input method.
    fn commit_korean_composition(&mut self) -> KeyResult {
        let text = self.preedit();
        self.reset_composition();
        self.chain.reset();
        KeyResult {
            handled: false,
            commit: Some(text),
            diagnostic: None,
        }
    }

    /// input_session.cpp:294-406.
    pub fn handle_command(&mut self, command: Command) -> KeyResult {
        if !self.has_composition() {
            // With nothing composed every command is left to the host, which edits the text, moves the caret or inserts a newline, so the chain no longer ends where the host's text does.
            self.chain.reset();
            return KeyResult::unhandled();
        }
        // A Korean syllable has no candidates to choose and no caret inside it: the commit and caret commands all end it and hand the key back. Backspace, Cancel and the Japanese-only variant command take the shared paths below.
        if self.is_korean()
            && self.local_mode == LocalInputMode::None
            && !self.dedicated_english
            && matches!(
                command,
                Command::CommitCandidate
                    | Command::CommitRaw
                    | Command::CommitReading
                    | Command::MoveLeft
                    | Command::MoveRight
                    | Command::MoveHome
                    | Command::MoveEnd
                    | Command::DeleteForward
            )
        {
            return self.commit_korean_composition();
        }
        match command {
            Command::MoveLeft
            | Command::MoveRight
            | Command::MoveHome
            | Command::MoveEnd
            | Command::DeleteForward => self.edit_at_caret(command),
            Command::CycleKanaVariant => {
                if !self.engine.cycle_japanese_kana_variant() {
                    return KeyResult::unhandled();
                }
                self.update_mixed_candidates();
                KeyResult::handled()
            }
            Command::Backspace => self.backspace(),
            Command::CommitCandidate => self.commit(0),
            Command::CommitRaw => self.commit_raw(),
            Command::CommitReading => {
                // Only Japanese has a reading to commit; the other schemes' normalized segmentation is pinyin, not text.
                if self.scheme() != SchemeType::JapaneseRomaji {
                    return KeyResult::unhandled();
                }
                let reading = self.normalized_segmentation();
                if reading.is_empty() {
                    return KeyResult::unhandled();
                }
                self.reset_composition();
                self.chain.reset();
                KeyResult::committed(reading)
            }
            Command::Cancel => {
                // Nothing reached the host, so the chain still ends where it did; only the composition is over.
                self.reset_composition();
                self.chain.same_composition = false;
                KeyResult::handled()
            }
        }
    }

    /// input_session.cpp:247-258.
    pub fn handle_candidate_key(&mut self, value: u8) -> KeyResult {
        if !self.has_composition() || !(b'1'..=b'9').contains(&value) {
            // The host inserts the key itself, so the next word no longer follows the last one.
            if !self.has_composition() {
                self.reset_commit_context();
            }
            return KeyResult::unhandled();
        }
        self.select_candidate(usize::from(value - b'1'))
    }

    /// input_session.cpp:260-292.
    pub fn handle_punctuation(&mut self, value: u8) -> KeyResult {
        // Korean writes half-width ASCII punctuation whatever the Chinese punctuation switches say. With a syllable open the mark follows it in one commit; with nothing open the host inserts the key itself.
        if self.is_korean() && !self.dedicated_english && self.local_mode == LocalInputMode::None {
            if !self.has_composition() {
                self.reset_commit_context();
                return KeyResult::unhandled();
            }
            let mut result = self.finish_composition(0);
            self.chain.reset();
            result.handled = true;
            result
                .commit
                .get_or_insert_with(String::new)
                .push(char::from(value));
            return result;
        }
        // Lock 1 forces Chinese, which is what an enabled session does anyway; only lock 2 changes anything here (core-session.md §15.8).
        let mark = if self.chinese_punctuation_enabled && self.punctuation_lock != 2 {
            self.punctuation.translate(value)
        } else {
            None
        };
        let Some(mark) = mark else {
            // The host inserts the untranslated key itself, which breaks the run of words just as translated punctuation does.
            self.reset_commit_context();
            return KeyResult::unhandled();
        };
        let mut result = self.finish_composition(0);
        // Whatever follows punctuation starts a new run of words.
        self.chain.reset();
        result.handled = true;
        let mut text = result.commit.take().unwrap_or_default();
        text.push_str(mark);
        result.commit = Some(text);
        result
    }

    pub fn has_composition(&self) -> bool {
        if self.dedicated_english {
            return !self.dedicated_english_preedit.is_empty();
        }
        if self.local_mode != LocalInputMode::None {
            return !self.local_preedit.is_empty();
        }
        !self.engine.preedit().is_empty()
    }

    /// The original scheme during temporary Japanese.
    pub fn scheme(&self) -> SchemeType {
        self.temporary_original_scheme
            .unwrap_or_else(|| self.engine.current_scheme_type())
    }

    pub fn preedit(&self) -> String {
        if self.dedicated_english {
            return self.dedicated_english_preedit.clone();
        }
        if self.local_mode != LocalInputMode::None {
            return self.local_preedit.clone();
        }
        self.engine.preedit().to_owned()
    }

    pub fn raw_segmentation(&self) -> String {
        if self.dedicated_english {
            return self.dedicated_english_preedit.clone();
        }
        if self.local_mode != LocalInputMode::None {
            return self.local_preedit.clone();
        }
        self.engine.request().raw_segmentation.clone()
    }

    pub fn normalized_segmentation(&self) -> String {
        if self.dedicated_english {
            return self.dedicated_english_preedit.clone();
        }
        if self.local_mode != LocalInputMode::None {
            return self.local_preedit.clone();
        }
        self.engine.request().normalized_segmentation.clone()
    }

    /// The displayed list of whichever view is active.
    pub fn candidates(&self) -> &[WordItem] {
        if self.dedicated_english {
            return &self.dedicated_english_candidates;
        }
        if self.local_mode != LocalInputMode::None {
            return &self.local_candidates;
        }
        // Fixed positions are always on through `Session`, so the merged list is the displayed one in the reference too (input_session.cpp:765-771); a caret prefix feeds it as well.
        &self.mixed_candidates
    }

    /// Discards the composition.
    pub fn switch_scheme(&mut self, scheme: SchemeType) {
        self.reset_composition();
        self.chain.reset();
        self.engine.switch_scheme(scheme);
        self.update_mixed_candidates();
    }

    pub fn set_dedicated_english_mode(&mut self, enabled: bool) {
        if self.dedicated_english == enabled {
            return;
        }
        self.reset_composition();
        self.chain.reset();
        self.dedicated_english = enabled;
    }

    /// False for an unsupported schema.
    pub fn set_helpcode_schema(&mut self, schema: &str) -> bool {
        if !is_supported_helpcode_schema(schema) {
            return false;
        }
        // The reference threw for a custom table that went missing after the name check; the public setter reports that as false, and the old keymap stays in force.
        self.install_helpcode_keymap(schema).is_ok()
    }

    pub fn set_helpcode_enabled(&mut self, enabled: bool) {
        self.set_quanpin_helpcode_enabled(enabled);
        self.set_shuangpin_helpcode_enabled(enabled);
    }

    pub fn set_wubi_mixed_pinyin(&mut self, enabled: bool) {
        self.set_wubi_input_options(WubiInputOptions {
            mixed_pinyin: enabled,
        });
    }

    pub fn set_personal_context_enabled(&mut self, enabled: bool) {
        if self.personal_context_enabled == enabled {
            return;
        }
        self.personal_context_enabled = enabled;
        if !enabled {
            self.reset_commit_context();
        }
    }

    pub fn set_rescoring_context(&mut self, context: &str) {
        self.engine.set_rescoring_context(context);
    }

    /// Clears caret, phrase progress, pending sequence, local mode, dedicated preedit and caches; returns from temporary Japanese.
    pub fn reset_composition(&mut self) {
        self.caret = None;
        self.prefix_candidates.clear();
        self.prefix_query_input.clear();
        self.prefix_active = false;
        self.phrase_progress = CreatingWordProgress::default();
        self.pending_sequence = None;
        self.pending_sequence_with_cases = None;
        self.online_requests.invalidate();
        let original_scheme = self.temporary_original_scheme.take();
        self.local_mode = LocalInputMode::None;
        self.local_preedit.clear();
        self.local_candidates.clear();
        self.dedicated_english_preedit.clear();
        self.dedicated_english_candidates.clear();
        self.mixed_candidates.clear();
        self.personal_reranked = false;
        self.ranking_candidates = None;
        self.engine.reset();
        if let Some(original) = original_scheme {
            if self.engine.current_scheme_type() != original {
                self.engine.switch_scheme(original);
            }
        }
    }

    /// Resets the chain and recomputes when the list was personally reranked.
    pub fn reset_commit_context(&mut self) {
        self.chain.reset();
        // A list reordered for the forgotten context must not stay on screen.
        if self.personal_reranked {
            self.update_mixed_candidates();
        }
    }

    pub fn reset_cache(&mut self) {
        self.engine.reset_cache();
        if let Some(engine) = self.canonical_phrase_engine.as_mut() {
            engine.reset_cache();
        }
        // The visible prefix list stays until the next refresh, but its query must run again against the invalidated provider caches.
        self.prefix_query_input.clear();
    }

    /// input_session_composition.cpp:379-385. While a caret prefix is decoded the prefix list is the one on screen, so it is the one widened; the reference's caret-prefix overlay widened only the hidden whole-input list and reported growth the host could not see.
    pub(super) fn expand_initial_candidates(&mut self) -> bool {
        self.refresh_prefix_candidates();
        let grew = if self.prefix_active {
            let raw_with_cases = self.raw_with_cases()[..self.prefix_end()].to_owned();
            self.engine.expand_raw_initial_candidates(
                &self.prefix_query_input,
                &raw_with_cases,
                &mut self.prefix_candidates,
            )
        } else {
            self.engine.expand_initial_candidates()
        };
        if !grew {
            return false;
        }
        // The prefix is unchanged, so the refresh keeps the widened prefix list instead of querying it again.
        self.update_mixed_candidates();
        true
    }

    pub(super) fn steady_now(&self) -> Instant {
        (self.clock.steady)()
    }

    pub(super) fn local_now(&self) -> LocalDateTime {
        (self.clock.local)()
    }

    pub(super) fn shuangpin_profile(&self) -> &'static ShuangpinProfile {
        profile(self.profile)
    }

    pub(super) fn journal_path(&self) -> PathBuf {
        self.paths.user(assets::USER_JOURNAL)
    }

    /// The live scheme, which is Japanese during temporary Japanese.
    pub(super) fn is_wubi(&self) -> bool {
        self.engine.current_scheme_type() == SchemeType::Wubi
    }

    pub(super) fn is_shuangpin(&self) -> bool {
        self.engine.current_scheme_type() == SchemeType::Shuangpin
    }

    pub(super) fn is_japanese(&self) -> bool {
        self.engine.current_scheme_type() == SchemeType::JapaneseRomaji
    }

    pub(super) fn is_korean(&self) -> bool {
        self.engine.current_scheme_type() == SchemeType::Korean
    }

    /// The helpcode switch of the session's scheme; other schemes have none.
    pub(super) fn helpcode_enabled(&self) -> bool {
        match self.scheme() {
            SchemeType::Quanpin => self.quanpin_helpcode_enabled,
            SchemeType::Shuangpin => self.shuangpin_helpcode_enabled,
            SchemeType::Wubi | SchemeType::JapaneseRomaji | SchemeType::Korean => false,
        }
    }

    /// The pinyin context pins and the personal rerank are keyed on.
    pub(super) fn pinyin_ranking_context(&self) -> String {
        let request = self.engine.request();
        if request.normalized_segmentation.is_empty() {
            request.segmentation.clone()
        } else {
            request.normalized_segmentation.clone()
        }
    }

    /// The phrase would be lost in progress when nothing is left to compose: backspacing the remaining pinyin away, or a continuing selection whose leftover collapses. It must not be glued onto the next composition's selections and stored as a phrase nobody typed.
    pub(super) fn discard_abandoned_phrase_progress(&mut self) {
        if !self.has_composition() {
            self.phrase_progress = CreatingWordProgress::default();
            self.chain.same_composition = false;
        }
    }

    fn install_helpcode_keymap(&mut self, schema: &str) -> Result<()> {
        let keymap = Arc::new(load_helpcode_keymap(&self.paths.resources, schema)?);
        self.helpcode_keymap = Some(keymap.clone());
        self.engine.set_helpcode_keymap(Some(keymap));
        self.update_mixed_candidates();
        Ok(())
    }

    fn set_autocorrect_types(&mut self, types: u32) {
        // Hosts may re-apply their configuration before every key; an unchanged value must not rebuild the list on that hot path.
        if self.autocorrect_types == types {
            return;
        }
        self.autocorrect_types = types;
        self.engine.set_autocorrect_types(types);
        self.update_mixed_candidates();
    }

    fn set_quanpin_helpcode_enabled(&mut self, enabled: bool) {
        if self.quanpin_helpcode_enabled == enabled {
            return;
        }
        self.quanpin_helpcode_enabled = enabled;
        self.engine.set_quanpin_helpcode_enabled(enabled);
        // The engine reads the flag only while querying, so a live composition has to be asked again.
        self.recompute_candidates();
        self.online_requests.invalidate();
    }

    fn set_shuangpin_helpcode_enabled(&mut self, enabled: bool) {
        if self.shuangpin_helpcode_enabled == enabled {
            return;
        }
        self.shuangpin_helpcode_enabled = enabled;
        self.engine.set_shuangpin_helpcode_enabled(enabled);
        self.recompute_candidates();
        self.online_requests.invalidate();
    }

    fn set_local_mode_options(&mut self, options: LocalModeOptions) {
        self.local_mode_options = options;
        if self.local_mode != LocalInputMode::None && !local_mode_enabled(options, self.local_mode)
        {
            self.reset_composition();
        }
    }

    fn set_wubi_input_options(&mut self, options: WubiInputOptions) {
        self.engine.set_wubi_input_options(options);
        // The setting decides which dictionary answers the code in hand, so a live composition has to be asked again; otherwise the fallback rows outlive the switch being turned off.
        if self.is_wubi() && !self.dedicated_english && self.local_mode == LocalInputMode::None {
            self.recompute_candidates();
        }
    }

    fn local_mode_for_entry(&self, value: u8) -> Option<LocalInputMode> {
        let options = self.local_mode_options;
        let (mode, enabled) = match value {
            b'U' => (LocalInputMode::Unicode, options.unicode),
            b'T' => (LocalInputMode::DateTime, options.date_time),
            b'K' => (LocalInputMode::QuickPhrase, options.quick_phrase),
            b'E' => (LocalInputMode::Emoji, options.emoji),
            b'M' => (LocalInputMode::Kaomoji, options.kaomoji),
            b'J' => (LocalInputMode::SuperJianpin, options.super_jianpin),
            b'Y' => (LocalInputMode::TemporaryEnglish, options.temporary_english),
            b'R' => (
                LocalInputMode::TemporaryJapanese,
                options.temporary_japanese,
            ),
            _ => return None,
        };
        enabled.then_some(mode)
    }

    fn enter_local_mode(&mut self, mode: LocalInputMode, letter: u8) {
        if mode == LocalInputMode::TemporaryJapanese {
            self.temporary_original_scheme = Some(self.scheme());
            self.engine.switch_scheme(SchemeType::JapaneseRomaji);
        }
        self.local_mode = mode;
        self.local_preedit = (letter as char).to_string();
        self.local_candidates.clear();
        self.chain.reset();
        self.add_local_fallback_candidate();
    }

    /// The `R` preedit and rows follow the Japanese engine after every edit.
    pub(super) fn refresh_temporary_japanese(&mut self) {
        self.local_preedit = format!("R{}", self.engine.preedit());
        self.local_candidates = self.engine.candidates().to_vec();
        self.add_local_fallback_candidate();
    }

    fn backspace(&mut self) -> KeyResult {
        if self.caret_position() < self.editing_text().len() {
            return self.edit_at_caret(Command::Backspace);
        }
        self.caret = None;
        if self.dedicated_english {
            self.dedicated_english_preedit.pop();
            self.update_dedicated_english_candidates();
            return KeyResult::handled();
        }
        if self.local_mode != LocalInputMode::None {
            // Backspacing the bare prefix letter leaves the mode.
            if self.local_preedit.len() <= 1 {
                self.reset_composition();
                return KeyResult::handled();
            }
            if self.local_mode == LocalInputMode::TemporaryJapanese {
                self.engine.handle_key(SchemeKey::Backspace);
                self.refresh_temporary_japanese();
                return KeyResult::handled();
            }
            self.local_preedit.pop();
            return KeyResult::handled().with_diagnostic(self.update_local_candidates());
        }
        self.engine.handle_key(SchemeKey::Backspace);
        self.update_mixed_candidates();
        self.discard_abandoned_phrase_progress();
        self.online_requests.invalidate();
        KeyResult::handled()
    }

    fn commit_raw(&mut self) -> KeyResult {
        let mut raw = self.preedit();
        // The temporary modes' prefix letter is a mode marker, not text; every other local mode commits it (core-session.md §15.5).
        if matches!(
            self.local_mode,
            LocalInputMode::TemporaryEnglish | LocalInputMode::TemporaryJapanese
        ) && !raw.is_empty()
        {
            raw.remove(0);
        }
        let mut diagnostic = None;
        if self.dedicated_english
            && learn_entered_english_word(
                &self.paths.dictionary(assets::ENGLISH_DICTIONARY),
                &self.journal_path(),
                &raw,
                ENTERED_ENGLISH_WORD_WEIGHT,
            )
            .is_err()
        {
            diagnostic = Some(diagnostics::ENGLISH_WORD_NOT_LEARNED.to_owned());
        }
        if diagnostic.is_none() {
            diagnostic = self.learn_rejected_correction();
        }
        self.reset_composition();
        self.chain.reset();
        KeyResult::committed(raw).with_diagnostic(diagnostic)
    }

    /// Rows in a mixed list decide where learning goes by the scheme that produced them (overlays.md §3.3).
    pub(super) fn is_wubi_native_candidate(item: &WordItem) -> bool {
        item.scheme == SchemeType::Wubi
    }

    /// Whether a row came from a dictionary a pin, fixed position or removal can write to.
    pub(super) fn is_editable_source(&self, item: &WordItem) -> bool {
        item.source == CandidateSource::EnglishDictionary
            || (item.source.is_dictionary() && self.scheme() != SchemeType::JapaneseRomaji)
    }
}

fn frequency_options_valid(options: FrequencyAdjustmentOptions) -> bool {
    (1..=10).contains(&options.trigger_count) && (1..=10).contains(&options.linear_step)
}

fn english_options_valid(options: EnglishInputOptions) -> bool {
    (1..=8).contains(&options.minimum_prefix)
}

fn local_mode_enabled(options: LocalModeOptions, mode: LocalInputMode) -> bool {
    match mode {
        LocalInputMode::None => true,
        LocalInputMode::Unicode => options.unicode,
        LocalInputMode::DateTime => options.date_time,
        LocalInputMode::QuickPhrase => options.quick_phrase,
        LocalInputMode::Emoji => options.emoji,
        LocalInputMode::Kaomoji => options.kaomoji,
        LocalInputMode::SuperJianpin => options.super_jianpin,
        LocalInputMode::TemporaryEnglish => options.temporary_english,
        LocalInputMode::TemporaryJapanese => options.temporary_japanese,
    }
}
