//! Selection advancement, segmentation getters and preedit display (core-session.md §5.8, §5.10-§5.11), including the wubi mixed routing rules that key advancement on the selected row's producer (overlays.md §3.3).

use super::input::{CreatingWordProgress, InputSession};
use crate::helpcode::compute_helpcodes;
use crate::japanese::romaji::convert_romaji;
use crate::pinyin::active_helpcode::{
    detect_active_helpcode_length, strip_active_helpcodes, strip_active_helpcodes_with_cases,
};
use crate::pinyin::autocorrect::{autocorrect_cut_detail, looks_like_syllable_with_jianpin_tail};
use crate::pinyin::segment::{is_complete_pinyin_input, join_segments, split_segments};
use crate::shuangpin::query::{
    detect_active_double_helpcode_length, is_complete_input, raw_length_for_effective_prefix,
    remove_manual_delimiters,
};
use crate::shuangpin::ShuangpinProfile;
use crate::text::count_han_chars;
use crate::types::{
    request_autocorrect_mask, CandidateSource, LocalInputMode, QueryRequest, SchemeKey, SchemeType,
};

/// What a selection did to the composition.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct SelectionTransition {
    pub continues_composition: bool,
    pub full_pure_pinyin: String,
    pub current_segmentation: String,
    pub current_segmentation_with_cases: String,
    pub selected_canonical_pinyin: String,
    pub wubi_native: bool,
}

/// A shuangpin composition split into the pinyin keys and a trailing helpcode (input_session_composition.cpp:111-163).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct ShuangpinCompositionBase {
    pub raw_input: String,
    pub raw_input_with_cases: String,
    /// Manual delimiters removed.
    pub effective_raw_input: String,
    pub effective_raw_input_with_cases: String,
    pub helpcode_length: usize,
}

pub(super) fn resolve_shuangpin_composition_base(
    request: &QueryRequest,
    profile: &ShuangpinProfile,
) -> ShuangpinCompositionBase {
    let raw_input = request.raw_input.clone();
    let raw_input_with_cases = if request.raw_input_with_cases.is_empty() {
        request.raw_input.clone()
    } else {
        request.raw_input_with_cases.clone()
    };
    let mut base = ShuangpinCompositionBase {
        effective_raw_input: remove_manual_delimiters(&raw_input),
        effective_raw_input_with_cases: remove_manual_delimiters(&raw_input_with_cases),
        raw_input,
        raw_input_with_cases,
        helpcode_length: 0,
    };
    if !request.enable_shuangpin_helpcode || base.effective_raw_input.is_empty() {
        return base;
    }
    if detect_active_double_helpcode_length(&base.raw_input, &base.raw_input_with_cases, profile)
        == 2
    {
        base.helpcode_length = 2;
        return base;
    }
    let length = base.effective_raw_input.len();
    if length % 2 == 1 && length > 1 {
        let raw_prefix_length = raw_length_for_effective_prefix(&base.raw_input, length - 1);
        // An apostrophe right before the odd letter makes it a pinyin segment the user started, not an auxiliary code.
        let separated = base.raw_input.as_bytes().get(raw_prefix_length) == Some(&b'\'');
        if !separated && is_complete_input(&base.raw_input[..raw_prefix_length], profile) {
            base.helpcode_length = 1;
        }
    }
    base
}

fn remove_delimiters(segmented: &str) -> String {
    segmented.chars().filter(|c| *c != '\'').collect()
}

/// A consumed prefix can leave the remainder starting with the separator that followed it.
fn remove_consumed_leading_separators(raw: &str) -> &str {
    raw.trim_start_matches('\'')
}

/// The canonical reading of a selected word, if it has one complete syllable per character (input_session_composition.cpp:76-96).
pub(super) fn normalize_canonical_pinyin_for_word(pinyin: &str, word: &str) -> String {
    if pinyin.is_empty() {
        return String::new();
    }
    let segments = split_segments(pinyin);
    if segments.is_empty() || segments.len() != count_han_chars(word) {
        return String::new();
    }
    if segments
        .iter()
        .any(|segment| segment.is_empty() || !is_complete_pinyin_input(segment))
    {
        return String::new();
    }
    join_segments(&segments)
}

/// An unknown reading anywhere makes the whole phrase unstorable, so an empty suffix empties the result.
pub(super) fn append_canonical_pinyin(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        return suffix.to_owned();
    }
    if suffix.is_empty() {
        return String::new();
    }
    format!("{prefix}'{suffix}")
}

/// Lowercased letters without delimiters, with the `v` spelling of ü folded onto `u`, so that length-preserving aliases (jv -> ju, nue -> nve) count as explained by the cut. It only compares two derived strings; the dictionary's `corrected_from` treats v and u as distinct on purpose.
pub(super) fn fold_autocorrect_letters(text: &str) -> String {
    text.bytes()
        .filter(|byte| *byte != b'\'')
        .map(|byte| match byte.to_ascii_lowercase() {
            b'v' => 'u',
            lower => lower as char,
        })
        .collect()
}

/// The preedit must always show the letters the user typed. Two layers can rewrite them into canonical pinyin: the scheme's alias table (sahng -> shang, baked into raw_segmentation) and the dictionary's correction search (shabg -> shang, which only re-separates). Both are redrawn here from the raw letters with separators at the cut positions; when the search cannot explain a rewrite (length-changing aliases such as mihng -> ming) the raw letters are shown without separators (input_session_composition.cpp:286-333).
pub(super) fn build_quanpin_autocorrect_display(request: &QueryRequest) -> String {
    let cased = if request.raw_input_with_cases.is_empty() {
        &request.raw_input
    } else {
        &request.raw_input_with_cases
    };
    let base = if request.raw_segmentation.is_empty() {
        cased
    } else {
        &request.raw_segmentation
    };
    if request.raw_input.is_empty() || cased.is_empty() {
        return base.clone();
    }
    let types = request_autocorrect_mask(
        request.enable_quanpin_autocorrect_transposition,
        request.enable_quanpin_autocorrect_neighbor,
    );
    let folded_input = fold_autocorrect_letters(cased);
    let folded_base = fold_autocorrect_letters(base);
    let letters_rewritten = folded_base != folded_input;
    // The scheme kept the typed letters and no correction can apply, so there are no other separators to draw.
    if !letters_rewritten && (types == 0 || is_complete_pinyin_input(&request.raw_input)) {
        return base.clone();
    }
    // A legal syllable plus one trailing letter is jianpin, never a typo, as in the dictionary's own guard; without this the deletion table would re-separate `zheg`.
    if looks_like_syllable_with_jianpin_tail(&request.raw_input) {
        return base.clone();
    }
    let cut = autocorrect_cut_detail(&folded_input, types).filter(|cut| !cut.is_empty());
    if let Some(cut) = cut {
        // When the scheme rewrote the letters, the query went through the alias reading, so separators may only come from the cut when both layers read the letters the same way (sahnghao -> shang'hao).
        let cut_letters = fold_autocorrect_letters(&cut.syllables().concat());
        if !letters_rewritten || cut_letters == folded_base {
            let boundary_count = cut.segments.len() - 1;
            let mut display = String::with_capacity(cased.len() + cut.segments.len());
            let mut letter_index = 0;
            let mut boundary_index = 0;
            for byte in cased.bytes().filter(|byte| *byte != b'\'') {
                display.push(byte as char);
                letter_index += 1;
                if boundary_index < boundary_count {
                    let segment = &cut.segments[boundary_index];
                    if letter_index == segment.start + segment.raw_text.len() {
                        display.push('\'');
                        boundary_index += 1;
                    }
                }
            }
            return display;
        }
    }
    if letters_rewritten {
        remove_delimiters(cased)
    } else {
        base.clone()
    }
}

impl InputSession {
    /// input_session_composition.cpp:714-748, with the selected row's scheme deciding the native-wubi branch.
    pub(super) fn selection_completes_composition(
        &self,
        pinyin: &str,
        word: &str,
        selected_scheme: SchemeType,
    ) -> bool {
        // Japanese and native wubi selections always finish: their advancement never continues.
        if self.is_japanese() || selected_scheme == SchemeType::Wubi {
            return true;
        }
        let request = self.engine.request();
        if self.is_shuangpin() {
            let base = resolve_shuangpin_composition_base(request, self.shuangpin_profile());
            let word_length = count_han_chars(word) * 2;
            let total = base.effective_raw_input.len();
            if base.helpcode_length > 0 {
                let required = word_length + base.helpcode_length;
                return !(required < total && word_length < total);
            }
            // With no helpcode the pure pinyin is the whole effective input.
            let mut consumed = remove_delimiters(pinyin).len();
            if consumed == 0 || consumed > total {
                consumed = word_length.min(total);
            }
            return consumed >= total;
        }
        let selected = remove_delimiters(pinyin);
        let raw_without_helpcodes =
            strip_active_helpcodes(&request.raw_input, &request.raw_input_with_cases);
        let cased_without_helpcodes =
            strip_active_helpcodes_with_cases(&request.raw_input, &request.raw_input_with_cases);
        let consumed_raw =
            raw_length_for_effective_prefix(&cased_without_helpcodes, selected.len());
        !(!selected.is_empty()
            && selected.len() < request.normalized_input.len()
            && consumed_raw < raw_without_helpcodes.len())
    }

    /// input_session_composition.cpp:799-911.
    pub(super) fn advance_composition_after_selection(
        &mut self,
        pinyin: &str,
        word: &str,
        canonical: &str,
        selected_scheme: SchemeType,
    ) -> SelectionTransition {
        let mut transition = SelectionTransition {
            selected_canonical_pinyin: canonical.to_owned(),
            wubi_native: selected_scheme == SchemeType::Wubi,
            ..SelectionTransition::default()
        };
        let request = self.engine.request().clone();
        if self.is_japanese() {
            transition.full_pure_pinyin = request.raw_input;
            transition.current_segmentation = request.segmentation;
            transition.current_segmentation_with_cases = request.raw_input_with_cases;
            return transition;
        }
        if transition.wubi_native {
            transition.full_pure_pinyin = request.normalized_input.clone();
            transition.current_segmentation = request.normalized_input;
            transition.current_segmentation_with_cases = request.raw_input;
            return transition;
        }
        transition.continues_composition =
            !self.selection_completes_composition(pinyin, word, selected_scheme);
        if self.is_shuangpin() {
            let base = resolve_shuangpin_composition_base(&request, self.shuangpin_profile());
            let word_length = count_han_chars(word) * 2;
            let total = base.effective_raw_input.len();
            transition.full_pure_pinyin =
                if base.helpcode_length > 0 && total >= base.helpcode_length {
                    base.effective_raw_input[..total - base.helpcode_length].to_owned()
                } else {
                    base.effective_raw_input.clone()
                };
            if transition.continues_composition {
                let (start, end) = if base.helpcode_length > 0 {
                    // The helpcode chose this word; the rest drops it.
                    (
                        raw_length_for_effective_prefix(&base.raw_input_with_cases, word_length),
                        raw_length_for_effective_prefix(
                            &base.raw_input_with_cases,
                            total - base.helpcode_length,
                        ),
                    )
                } else {
                    let mut consumed = remove_delimiters(pinyin).len();
                    if consumed == 0 || consumed > total {
                        consumed = word_length.min(total);
                    }
                    (
                        raw_length_for_effective_prefix(&base.raw_input_with_cases, consumed),
                        base.raw_input.len(),
                    )
                };
                let rest = remove_consumed_leading_separators(&base.raw_input[start..end]);
                let rest_with_cases =
                    remove_consumed_leading_separators(&base.raw_input_with_cases[start..end]);
                self.engine.replace_active_raw_input(rest, rest_with_cases);
                self.online_requests.invalidate();
                self.update_mixed_candidates();
            }
            transition.current_segmentation = self.pinyin_segmentation();
            transition.current_segmentation_with_cases = self.pinyin_segmentation_with_cases();
            return transition;
        }

        // Quanpin, and the quanpin rows of a wubi composition, which shorten the wubi code the same way.
        transition.full_pure_pinyin = request.normalized_input.clone();
        let selected = remove_delimiters(pinyin);
        let raw_without_helpcodes =
            strip_active_helpcodes(&request.raw_input, &request.raw_input_with_cases);
        let cased_without_helpcodes =
            strip_active_helpcodes_with_cases(&request.raw_input, &request.raw_input_with_cases);
        if transition.continues_composition {
            let consumed =
                raw_length_for_effective_prefix(&cased_without_helpcodes, selected.len());
            let rest = remove_consumed_leading_separators(&raw_without_helpcodes[consumed..]);
            let rest_with_cases =
                remove_consumed_leading_separators(&cased_without_helpcodes[consumed..]);
            if self.is_wubi() {
                // The rest of a spelling the user is still in the middle of stays pinyin; the wubi table answering it would swap schemes underneath them.
                self.engine.keep_pinyin_tail();
            }
            self.engine.replace_active_raw_input(rest, rest_with_cases);
            self.online_requests.invalidate();
            self.update_mixed_candidates();
            transition.current_segmentation = self.pinyin_segmentation();
            transition.current_segmentation_with_cases = self.pinyin_segmentation_with_cases();
            return transition;
        }
        transition.current_segmentation = if request.normalized_segmentation.is_empty() {
            request.segmentation.clone()
        } else {
            request.normalized_segmentation.clone()
        };
        transition.current_segmentation_with_cases = self.pinyin_segmentation_with_cases();
        transition
    }

    /// input_session_composition.cpp:976-1004.
    pub(super) fn update_creating_word_progress(
        current_pinyin: &str,
        current_word: &str,
        selected_word: &str,
        transition: &SelectionTransition,
    ) -> CreatingWordProgress {
        let word = format!("{current_word}{selected_word}");
        if transition.wubi_native {
            // Wubi phrases are not composed from partial selections: a wubi code is one word.
            return CreatingWordProgress {
                pinyin: if current_pinyin.is_empty() {
                    transition.full_pure_pinyin.clone()
                } else {
                    current_pinyin.to_owned()
                },
                preedit: word.clone(),
                word,
                completed: true,
                can_store: false,
            };
        }
        let selected_canonical = normalize_canonical_pinyin_for_word(
            &transition.selected_canonical_pinyin,
            selected_word,
        );
        let prior_parts_storable = current_word.is_empty() || !current_pinyin.is_empty();
        let pinyin = if prior_parts_storable && !selected_canonical.is_empty() {
            append_canonical_pinyin(current_pinyin, &selected_canonical)
        } else {
            String::new()
        };
        let completed = !transition.continues_composition;
        let can_store =
            completed && !normalize_canonical_pinyin_for_word(&pinyin, &word).is_empty();
        CreatingWordProgress {
            preedit: format!("{word}{}", transition.current_segmentation_with_cases),
            pinyin,
            word,
            completed,
            can_store,
        }
    }

    /// The segmentation the dictionary was queried with.
    pub(super) fn pinyin_segmentation(&self) -> String {
        let request = self.engine.request();
        if request.normalized_segmentation.is_empty() {
            request.segmentation.clone()
        } else {
            request.normalized_segmentation.clone()
        }
    }

    /// input_session_composition.cpp:417-449.
    pub(super) fn pinyin_segmentation_with_cases(&self) -> String {
        let request = self.engine.request();
        let with_trailing_separator = |mut preedit: String| {
            if request.raw_input_with_cases.ends_with('\'') && !preedit.ends_with('\'') {
                preedit.push('\'');
            }
            preedit
        };
        match self.engine.current_scheme_type() {
            SchemeType::Wubi => request.raw_input.clone(),
            SchemeType::JapaneseRomaji => self.raw_with_cases().to_owned(),
            SchemeType::Shuangpin if self.shuangpin_preedit_uses_raw => {
                with_trailing_separator(if request.raw_segmentation.is_empty() {
                    request.raw_input.clone()
                } else {
                    request.raw_segmentation.clone()
                })
            }
            SchemeType::Quanpin => build_quanpin_autocorrect_display(request),
            SchemeType::Shuangpin => with_trailing_separator(self.pinyin_segmentation()),
        }
    }

    /// input_session_composition.cpp:456-481.
    pub(super) fn is_all_complete_pure_pinyin(&self) -> bool {
        let request = self.engine.request();
        match self.engine.current_scheme_type() {
            SchemeType::Wubi => request.valid,
            SchemeType::JapaneseRomaji => convert_romaji(&request.raw_input).complete,
            SchemeType::Shuangpin => {
                let profile = self.shuangpin_profile();
                let base = resolve_shuangpin_composition_base(request, profile);
                if base.helpcode_length > 0
                    && base.effective_raw_input.len() >= base.helpcode_length
                {
                    let base_length = base.effective_raw_input.len() - base.helpcode_length;
                    let prefix = raw_length_for_effective_prefix(&base.raw_input, base_length);
                    return is_complete_input(&base.raw_input[..prefix], profile);
                }
                is_complete_input(&base.raw_input, profile)
            }
            SchemeType::Quanpin => {
                let segmentation = self.pinyin_segmentation();
                !segmentation.is_empty() && is_complete_pinyin_input(&segmentation)
            }
        }
    }

    /// Four native letters answered by exactly one wubi row.
    pub(super) fn wubi_unique_four_code(&self) -> bool {
        if self.dedicated_english || self.local_mode != LocalInputMode::None {
            return false;
        }
        // Only the wubi rows count: quanpin rows beside them do not make the code ambiguous (overlays.md §3.3).
        self.wubi_candidates_are_native()
            && self.engine.wubi_has_complete_code()
            && self.wubi_native_candidate_count() == 1
    }

    pub(super) fn has_active_helpcode(&self) -> bool {
        let request = self.engine.request();
        match self.engine.current_scheme_type() {
            SchemeType::Wubi | SchemeType::JapaneseRomaji => false,
            SchemeType::Shuangpin => {
                resolve_shuangpin_composition_base(request, self.shuangpin_profile())
                    .helpcode_length
                    > 0
            }
            SchemeType::Quanpin => {
                request.enable_quanpin_helpcode
                    && detect_active_helpcode_length(
                        &request.raw_input,
                        &request.raw_input_with_cases,
                    ) > 0
            }
        }
    }

    /// input_session.cpp:775-793.
    pub(super) fn candidate_annotations(&self) -> Vec<String> {
        let enabled = self.helpcode_enabled();
        let uppercase_all = self.scheme() == SchemeType::Quanpin;
        let keymap = self.helpcode_keymap.as_deref();
        self.candidates()
            .iter()
            .map(|item| {
                let helpcode = match keymap {
                    Some(keymap)
                        if enabled && item.source != CandidateSource::EnglishDictionary =>
                    {
                        compute_helpcodes(&item.word, uppercase_all, keymap)
                    }
                    _ => String::new(),
                };
                if helpcode.is_empty() {
                    item.corrected_from.clone()
                } else {
                    helpcode
                }
            })
            .collect()
    }

    /// Staged host editing applied to the scheme (input_session_composition.cpp:1037-1070).
    pub(super) fn apply_pending_sequence(&mut self) {
        self.caret = None;
        let raw = self
            .pending_sequence
            .take()
            .unwrap_or_else(|| self.engine.request().raw_input.clone());
        let raw_with_cases = self
            .pending_sequence_with_cases
            .take()
            .unwrap_or_else(|| raw.clone());
        self.engine.replace_active_raw_input(&raw, &raw_with_cases);
        self.online_requests.invalidate();
        self.update_mixed_candidates();
    }

    /// Re-query without changing the composition.
    pub(super) fn recompute_candidates(&mut self) {
        if self.pending_sequence.is_some() || self.pending_sequence_with_cases.is_some() {
            self.apply_pending_sequence();
            return;
        }
        self.engine.handle_key(SchemeKey::Requery);
        self.update_mixed_candidates();
    }

    /// The code was answered only by quanpin rows (overlays.md §3.3: read from the rows, not the list as a whole).
    pub(super) fn answered_by_pinyin_fallback(&self) -> bool {
        self.is_wubi()
            && !self.candidates().is_empty()
            && self
                .candidates()
                .iter()
                .all(|item| !Self::is_wubi_native_candidate(item))
    }

    pub(super) fn wubi_candidates_are_native(&self) -> bool {
        self.is_wubi() && self.candidates().iter().any(Self::is_wubi_native_candidate)
    }

    pub(super) fn wubi_native_candidate_count(&self) -> usize {
        self.candidates()
            .iter()
            .filter(|item| Self::is_wubi_native_candidate(item))
            .count()
    }

    /// Whether the list reads the composition as pinyin, so selections advance and learn as pinyin.
    pub(super) fn candidates_follow_pinyin(&self) -> bool {
        self.engine.current_scheme_type().is_pinyin() || !self.wubi_candidates_are_native()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_readings_need_one_complete_syllable_per_character() {
        assert_eq!(
            normalize_canonical_pinyin_for_word("ni'hao", "你好"),
            "ni'hao"
        );
        assert_eq!(normalize_canonical_pinyin_for_word("ni'hao", "你"), "");
        assert_eq!(normalize_canonical_pinyin_for_word("ni'h", "你好"), "");
        assert_eq!(normalize_canonical_pinyin_for_word("", "你"), "");
    }

    #[test]
    fn canonical_readings_append_and_empty_on_a_gap() {
        assert_eq!(append_canonical_pinyin("", "ni"), "ni");
        assert_eq!(append_canonical_pinyin("ni", "hao"), "ni'hao");
        assert_eq!(append_canonical_pinyin("ni", ""), "");
        assert_eq!(append_canonical_pinyin("shan", "shui"), "shan'shui");
    }

    #[test]
    fn consumed_separators_are_dropped_from_the_rest() {
        assert_eq!(remove_consumed_leading_separators("''hao"), "hao");
        assert_eq!(remove_consumed_leading_separators("hao'"), "hao'");
        assert_eq!(remove_delimiters("ni'hao'"), "nihao");
    }

    #[test]
    fn folding_ignores_case_delimiters_and_the_v_spelling() {
        assert_eq!(fold_autocorrect_letters("Nv'E"), "nue");
        assert_eq!(fold_autocorrect_letters("sa'Hng"), "sahng");
        assert_eq!(fold_autocorrect_letters("Nv'e"), "nue");
    }

    /// The request a quanpin session builds for `typed` under the two user switches (test_pinyin.cpp P38).
    fn display(typed: &str, transposition: bool, neighbor: bool) -> String {
        let mut scheme = crate::quanpin::QuanpinScheme::new();
        for byte in typed.bytes() {
            scheme.handle_key(if byte == b'\'' {
                SchemeKey::Apostrophe
            } else {
                SchemeKey::Letter(byte)
            });
        }
        let mut request = scheme.build_request();
        request.enable_quanpin_autocorrect_transposition = transposition;
        request.enable_quanpin_autocorrect_neighbor = neighbor;
        build_quanpin_autocorrect_display(&request)
    }

    #[test]
    fn the_preedit_shows_the_typed_letters() {
        // Both switches on; deletion and insertion ride along.
        for (typed, shown) in [
            ("sahng", "sahng"),
            ("sahnghao", "sahng'hao"),
            ("shabg", "shabg"),
            ("iandu", "ian'du"),
            ("keneng", "ke'neng"),
            ("xi'an", "xi'an"),
            ("zheg", "zhe'g"),
            ("wj", "w'j"),
            ("nv", "nv"),
            ("shng", "shng"),
            ("sshang", "sshang"),
            ("zher", "zh'er"),
        ] {
            assert_eq!(display(typed, true, true), shown, "{typed}");
        }
        assert_eq!(display("saHng", true, true), "saHng");
        assert_eq!(display("shabg", false, true), "shabg");
        // With both switches off the scheme's greedy separators stay, but an alias rewrite is still undone.
        assert_eq!(display("shabg", false, false), "sha'b'g");
        assert_eq!(display("sahng", false, false), "sahng");
    }

    #[test]
    fn display_of_an_empty_request_is_its_segmentation() {
        let request = QueryRequest {
            raw_segmentation: "x".to_owned(),
            ..QueryRequest::default()
        };
        assert_eq!(build_quanpin_autocorrect_display(&request), "x");
    }
}
