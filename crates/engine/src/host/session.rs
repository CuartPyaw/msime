//! The bridge's `Session` wrapper over the engine session (api-contract §1a, §2, §5): ASCII checks, command numbering with `CommitRawWithoutLearning`, the raw-commit learning policy, the flattened snapshot and the online query snapshot.

use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use super::options::{runtime_paths, session_options, shuangpin_profile, EngineOptions};
use crate::assets;
use crate::diagnostics;
use crate::error::{EngineError, Result};
use crate::helpcode::{compute_helpcodes, load_helpcode_keymap, SharedKeymap};
use crate::pinyin::segment::is_complete_pinyin_input;
use crate::types::{
    CandidateEdge, CandidateSource, KeyResult, LocalInputMode, OnlineQuery, SchemeType,
    ShuangpinProfileKind,
};
use crate::user_dictionary::ngram_store::flush_journal;
use crate::user_dictionary::removal::learn_entered_english_word;

/// The weight an entered English word is learned at: the C++ default argument of `learn_entered_english_word` (user_dictionary_journal.h:136-137), which the bridge relied on.
const ENTERED_ENGLISH_WORD_WEIGHT: i64 = 10;

/// The host's command numbering. `CommitRawWithoutLearning` has no engine counterpart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Backspace = 0,
    CommitCandidate = 1,
    CommitRaw = 2,
    Cancel = 3,
    MoveLeft = 4,
    MoveRight = 5,
    MoveHome = 6,
    MoveEnd = 7,
    DeleteForward = 8,
    CycleKanaVariant = 9,
    CommitReading = 10,
    /// Commit the letters as typed without learning them as an English word.
    CommitRawWithoutLearning = 11,
}

/// Every `candidate_*` vector has `candidates.len()` elements; the runtime's reorderings require it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EngineSnapshot {
    pub local_mode: String,
    pub dedicated_english: bool,
    /// Mirrors `set_nine_key_enabled`; the engine snapshot has no such flag.
    pub nine_key: bool,
    pub nine_key_spellings: Vec<String>,
    pub microsoft_shuangpin: bool,
    pub shuangpin_profile: String,
    pub preedit: String,
    /// The kana reading in Japanese, else empty.
    pub reading: String,
    pub editing_text: String,
    pub caret_position: usize,
    pub segment_raw_boundaries: Vec<u64>,
    pub candidates: Vec<String>,
    pub candidate_codes: Vec<String>,
    pub scheme: u8,
    pub answered_by_pinyin_fallback: bool,
    pub wubi_unique_four_code: bool,
    pub candidate_annotations: Vec<String>,
    pub candidate_sources: Vec<u8>,
    pub candidate_positions: Vec<u8>,
    pub candidate_corrected: Vec<bool>,
    pub candidate_answers_key: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EngineResult {
    pub handled: bool,
    pub has_commit: bool,
    pub commit: String,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnlineQuerySnapshot {
    pub available: bool,
    pub scheme: u8,
    pub generation: u64,
    pub identity: String,
    pub query_text: String,
    pub cache_key: String,
    pub pinyin_segments: Vec<String>,
    pub cloud_eligible: bool,
    pub ai_eligible: bool,
    pub session_id: u64,
}

/// Thread-confined like the bridge session it replaces.
pub struct Session {
    inner: crate::session::Session,
    options: EngineOptions,
    nine_key: bool,
    microsoft_shuangpin: bool,
    shuangpin_profile: String,
    helpcode_keymap: Option<SharedKeymap>,
    helpcode_enabled: bool,
    show_helpcode: bool,
    _thread_confined: PhantomData<Rc<()>>,
}

impl Session {
    pub fn new(options: &EngineOptions) -> Result<Session> {
        // The C++ ran `options_for` a second time only to read the profile name back (bridge.cpp:430); that rerun recopied the same sidecar, so one mapping is enough.
        let inner = crate::session::Session::new(session_options(options)?)?;
        let profile = shuangpin_profile(options)?;
        // The engine loads its own copy for filtering; this one only annotates, and exists only while helpcode is on (bridge.cpp:431-435).
        let helpcode_keymap = if options.helpcode {
            Some(Arc::new(load_helpcode_keymap(
                Path::new(&options.resources),
                &options.helpcode_schema,
            )?))
        } else {
            None
        };
        Ok(Session {
            inner,
            options: options.clone(),
            nine_key: false,
            microsoft_shuangpin: options.scheme == SchemeType::Shuangpin as u8
                && profile == ShuangpinProfileKind::Microsoft,
            shuangpin_profile: profile.name().to_owned(),
            helpcode_keymap,
            helpcode_enabled: options.helpcode,
            show_helpcode: options.show_helpcode,
            _thread_confined: PhantomData,
        })
    }

    /// With the annotation rule of bridge.cpp:914-929.
    pub fn snapshot(&self) -> Result<EngineSnapshot> {
        let value = self.inner.snapshot();
        let pinyin_scheme = value.scheme.is_pinyin();
        let uppercase_all = value.scheme == SchemeType::Quanpin;
        let keymap = self
            .helpcode_keymap
            .as_deref()
            .filter(|_| self.helpcode_enabled && pinyin_scheme);
        let count = value.candidates.len();
        let mut output = EngineSnapshot {
            local_mode: value.local_mode.name().to_owned(),
            dedicated_english: value.dedicated_english,
            nine_key: self.nine_key,
            nine_key_spellings: value.nine_key_spellings,
            microsoft_shuangpin: self.microsoft_shuangpin,
            shuangpin_profile: self.shuangpin_profile.clone(),
            preedit: value.preedit,
            reading: if value.scheme == SchemeType::JapaneseRomaji {
                value.normalized_segmentation
            } else {
                String::new()
            },
            editing_text: value.editing_text,
            caret_position: value.caret_position,
            segment_raw_boundaries: self
                .inner
                .segment_raw_boundaries()
                .into_iter()
                .map(|boundary| boundary as u64)
                .collect(),
            candidates: Vec::with_capacity(count),
            candidate_codes: Vec::with_capacity(count),
            scheme: value.scheme as u8,
            answered_by_pinyin_fallback: value.answered_by_pinyin_fallback,
            wubi_unique_four_code: value.wubi_unique_four_code,
            candidate_annotations: Vec::with_capacity(count),
            candidate_sources: Vec::with_capacity(count),
            candidate_positions: Vec::with_capacity(count),
            candidate_corrected: Vec::with_capacity(count),
            candidate_answers_key: Vec::with_capacity(count),
        };
        for (index, candidate) in value.candidates.into_iter().enumerate() {
            let mut annotation = value
                .candidate_annotations
                .get(index)
                .cloned()
                .unwrap_or_else(|| candidate.corrected_from.clone());
            if let Some(keymap) = keymap {
                if !self.show_helpcode {
                    // Hidden helpcodes give their slot back to the correction hint the engine would otherwise have shown.
                    let helpcode = compute_helpcodes(&candidate.word, uppercase_all, keymap);
                    if !helpcode.is_empty() && annotation == helpcode {
                        annotation = candidate.corrected_from.clone();
                    }
                } else if annotation.is_empty() && candidate.source == CandidateSource::Generated {
                    // The engine annotates dictionary rows itself; a sentence it synthesised carries no helpcode until the host adds one.
                    annotation = compute_helpcodes(&candidate.word, uppercase_all, keymap);
                }
            }
            output.candidate_annotations.push(annotation);
            output.candidate_sources.push(candidate.source as u8);
            output
                .candidate_positions
                .push(candidate.fixed_position as u8);
            output
                .candidate_corrected
                .push(!candidate.corrected_from.is_empty());
            // A short vector would be an engine bug. False is the safe reading of one: a consumer that sees nothing answering the key declines to reorder (bridge.cpp:933-937).
            output.candidate_answers_key.push(
                value
                    .candidate_answers_key
                    .get(index)
                    .copied()
                    .unwrap_or(false),
            );
            output.candidate_codes.push(candidate.pinyin);
            output.candidates.push(candidate.word);
        }
        Ok(output)
    }

    /// `available = false` and defaults when there is no query.
    pub fn online_query(&self) -> Result<OnlineQuerySnapshot> {
        let Some(query) = self.inner.online_query() else {
            return Ok(OnlineQuerySnapshot::default());
        };
        Ok(OnlineQuerySnapshot {
            available: true,
            scheme: query.scheme as u8,
            generation: query.generation,
            identity: query.identity,
            query_text: query.query_text,
            cache_key: query.cache_key,
            pinyin_segments: query.pinyin_segments,
            cloud_eligible: query.cloud_eligible,
            ai_eligible: query.ai_eligible,
            session_id: query.session_id,
        })
    }

    pub fn reset_cache(&mut self) {
        self.inner.reset_cache();
    }

    pub fn set_caret(&mut self, caret: Option<usize>) {
        self.inner.set_caret(caret);
    }

    pub fn prefix_end(&self) -> usize {
        self.inner.prefix_end()
    }

    pub fn pending_suffix(&self) -> String {
        self.inner.pending_suffix()
    }

    pub fn reset_context(&mut self) {
        self.inner.reset_context();
    }

    /// Live update of the neural rescoring context without a rebuild.
    pub fn set_rescoring_context(&mut self, context: &str) {
        self.inner.set_rescoring_context(context);
    }

    /// False for an unavailable query or a source other than 0 (cloud) and 1 (AI).
    pub fn apply_online_candidate(
        &mut self,
        query: &OnlineQuerySnapshot,
        candidate: &str,
        source: u8,
    ) -> Result<bool> {
        let Some((request, source)) = online_request(query, source) else {
            return Ok(false);
        };
        Ok(self
            .inner
            .apply_online_candidate(&request, candidate, source))
    }

    pub fn apply_online_candidates(
        &mut self,
        query: &OnlineQuerySnapshot,
        candidates: &[String],
        source: u8,
    ) -> Result<bool> {
        let Some((request, source)) = online_request(query, source) else {
            return Ok(false);
        };
        Ok(self
            .inner
            .apply_online_candidates(&request, candidates, source))
    }

    /// `CHARACTER_MUST_BE_ASCII` above 127.
    pub fn character(&mut self, value: u8, shift: bool) -> Result<EngineResult> {
        require_ascii(value, diagnostics::CHARACTER_MUST_BE_ASCII)?;
        Ok(result_for(self.inner.character(value, shift)))
    }

    pub fn expand_initial_candidates(&mut self) -> Result<bool> {
        Ok(self.inner.expand_initial_candidates())
    }

    pub fn set_nine_key_enabled(&mut self, enabled: bool) -> Result<()> {
        self.inner.set_nine_key_enabled(enabled);
        self.nine_key = enabled;
        Ok(())
    }

    /// Out of range is unhandled.
    pub fn choose_nine_key_spelling(&mut self, index: usize) -> Result<EngineResult> {
        Ok(result_for(self.inner.choose_nine_key_spelling(index)))
    }

    /// `CommitRaw` goes through the learning policy of bridge.cpp:1332-1359 and `CommitRawWithoutLearning` through :1321-1331.
    pub fn command(&mut self, command: Command) -> Result<EngineResult> {
        match command {
            Command::CommitRaw => Ok(self.commit_raw_with_policy()),
            Command::CommitRawWithoutLearning => Ok(self.commit_raw_without_learning()),
            _ => {
                // The other host codes are the engine's ordinals one for one (bridge.cpp:1302-1319).
                let engine = crate::types::Command::from_u8(command as u8)
                    .ok_or_else(|| EngineError::invalid(diagnostics::UNSUPPORTED_INPUT_COMMAND))?;
                Ok(result_for(self.inner.command(engine)))
            }
        }
    }

    pub fn select(&mut self, index: usize) -> Result<EngineResult> {
        Ok(result_for(self.inner.select(index)))
    }

    pub fn pin_candidate(&mut self, index: usize) -> Result<EngineResult> {
        Ok(result_for(self.inner.pin(index)))
    }

    pub fn remove_candidate(&mut self, index: usize) -> Result<EngineResult> {
        Ok(result_for(self.inner.remove(index)))
    }

    /// `INVALID_CANDIDATE_POSITION` unless 1..=5.
    pub fn fix_candidate_position(&mut self, index: usize, position: u8) -> Result<EngineResult> {
        if !(1..=5).contains(&position) {
            return Err(EngineError::invalid(
                diagnostics::INVALID_CANDIDATE_POSITION,
            ));
        }
        Ok(result_for(
            self.inner.fix_position(index, i32::from(position)),
        ))
    }

    pub fn clear_candidate_position(&mut self, index: usize) -> Result<EngineResult> {
        Ok(result_for(self.inner.clear_position(index)))
    }

    pub fn select_edge(&mut self, index: usize, edge: CandidateEdge) -> Result<EngineResult> {
        Ok(result_for(self.inner.select_edge(index, edge)))
    }

    pub fn finish(&mut self, index: usize) -> Result<EngineResult> {
        Ok(result_for(self.inner.finish(index)))
    }

    /// `PUNCTUATION_MUST_BE_ASCII` above 127.
    pub fn punctuation(&mut self, value: u8) -> Result<EngineResult> {
        require_ascii(value, diagnostics::PUNCTUATION_MUST_BE_ASCII)?;
        Ok(result_for(self.inner.punctuation(value)))
    }

    pub fn balance_paired_punctuation_after_auto_close(&mut self, opening: u8) -> Result<()> {
        require_ascii(opening, diagnostics::PAIRED_OPENING_MUST_BE_ASCII)?;
        self.inner
            .balance_paired_punctuation_after_auto_close(opening);
        Ok(())
    }

    pub fn set_chinese_punctuation_enabled(&mut self, enabled: bool) -> Result<()> {
        self.inner.set_chinese_punctuation_enabled(enabled);
        Ok(())
    }

    pub fn set_punctuation_lock(&mut self, lock: u8) -> Result<()> {
        self.inner.set_punctuation_lock(i32::from(lock))
    }

    pub fn set_paired_punctuation_enabled(&mut self, enabled: bool) -> Result<()> {
        self.inner.set_paired_punctuation_enabled(enabled);
        Ok(())
    }

    pub fn set_dedicated_english(&mut self, enabled: bool) -> Result<()> {
        self.inner.set_dedicated_english(enabled);
        Ok(())
    }

    /// The letters as typed, learned as nothing. The engine's own raw commit learns the word itself in dedicated English, so that mode takes the preedit and cancels instead, stripping the trigger letter of a temporary mode the way `InputSession` does (bridge.cpp:1320-1331).
    fn commit_raw_without_learning(&mut self) -> EngineResult {
        let before = self.inner.snapshot();
        if !before.dedicated_english {
            return result_for(self.inner.command(crate::types::Command::CommitRaw));
        }
        let mut raw = before.preedit;
        if matches!(
            before.local_mode,
            LocalInputMode::TemporaryEnglish | LocalInputMode::TemporaryJapanese
        ) && !raw.is_empty()
        {
            raw.remove(0);
        }
        self.inner.command(crate::types::Command::Cancel);
        result_for(KeyResult::committed(raw))
    }

    /// Windows learns an entered word only on Enter: letters committed raw in dedicated English, a local mode, or pinyin that is not a complete syllable sequence are learned as an English word (bridge.cpp:1332-1359).
    fn commit_raw_with_policy(&mut self) -> EngineResult {
        let before = self.inner.snapshot();
        let chinese_scheme = before.scheme.is_pinyin();
        let complete_pure_pinyin = chinese_scheme && {
            let segmentation = if before.normalized_segmentation.is_empty() {
                &before.raw_segmentation
            } else {
                &before.normalized_segmentation
            };
            !segmentation.is_empty() && is_complete_pinyin_input(segmentation)
        };
        let should_learn = before.dedicated_english
            || before.local_mode != LocalInputMode::None
            || (chinese_scheme && !complete_pure_pinyin);
        let mut result = self.inner.command(crate::types::Command::CommitRaw);
        if let Some(commit) = result.commit.as_deref().filter(|_| should_learn) {
            if !commit.is_empty() {
                let word = if before.local_mode == LocalInputMode::TemporaryJapanese {
                    format!("R{commit}")
                } else {
                    commit.to_owned()
                };
                let paths = runtime_paths(&self.options);
                // The commit already happened; a word that could not be learned is reported beside it rather than undoing it.
                if learn_entered_english_word(
                    &paths.dictionary(assets::ENGLISH_DICTIONARY),
                    &paths.user(assets::USER_JOURNAL),
                    &word,
                    ENTERED_ENGLISH_WORD_WEIGHT,
                )
                .is_err()
                {
                    result.diagnostic = Some(diagnostics::ENGLISH_WORD_NOT_LEARNED.to_owned());
                }
            }
        }
        result_for(result)
    }
}

/// The C++ registered `PersonalNgramStore::flush_all` with `atexit` (personal_ngram_store.cpp:255), so context learned in the last ~2 s reached the journal when the host quit. Rust runs no destructors for statics and `atexit` needs unsafe, so the session writes its journal's queue when the host drops it, which every host does on deactivation and shutdown; a host that exits without dropping its sessions calls `flush_personal_learning` instead.
impl Drop for Session {
    fn drop(&mut self) {
        let journal = runtime_paths(&self.options).user(assets::USER_JOURNAL);
        // Nobody is left to report to: a failed write stays queued and flagged in the store, and the next record reports it and schedules another try.
        let _ = flush_journal(&journal);
        // A host thread whose sessions are gone (a quiesced IME, a closed window) holds no journal handle.
        crate::user_dictionary::journal::release_thread_journal();
    }
}

/// `result_for` (bridge.cpp:408-410): an empty commit with `has_commit = true` stays representable.
fn result_for(value: KeyResult) -> EngineResult {
    EngineResult {
        handled: value.handled,
        has_commit: value.commit.is_some(),
        commit: value.commit.unwrap_or_default(),
        diagnostic: value.diagnostic.unwrap_or_default(),
    }
}

fn require_ascii(value: u8, message: &str) -> Result<()> {
    if value > 127 {
        return Err(EngineError::invalid(message));
    }
    Ok(())
}

/// The query handed back field by field (bridge.cpp:977-1017). `None` for an unavailable query, a source other than cloud or AI, or a scheme ordinal no session could have issued, which is a stale query rather than an error (api-contract §5.3).
fn online_request(
    query: &OnlineQuerySnapshot,
    source: u8,
) -> Option<(OnlineQuery, CandidateSource)> {
    if !query.available {
        return None;
    }
    let source = match source {
        0 => CandidateSource::CloudSuggestion,
        1 => CandidateSource::AiSuggestion,
        _ => return None,
    };
    let request = OnlineQuery {
        scheme: SchemeType::from_u8(query.scheme)?,
        generation: query.generation,
        identity: query.identity.clone(),
        query_text: query.query_text.clone(),
        cache_key: query.cache_key.clone(),
        pinyin_segments: query.pinyin_segments.clone(),
        cloud_eligible: query.cloud_eligible,
        ai_eligible: query.ai_eligible,
        session_id: query.session_id,
    };
    Some((request, source))
}
