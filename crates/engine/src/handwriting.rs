//! The candidate policy for handwriting provider replies (`include/metasequoia/handwriting_candidates.h`), applied on every platform. Recognition itself (zinnia or its replacement) is not engine logic and is not in this crate.

use std::collections::HashSet;

pub const MAX_CANDIDATES: usize = 12;
pub const MAX_CANDIDATE_BYTES: usize = 4_096;

/// A three-byte UTF-8 character in U+3400-4DBF, U+4E00-9FFF or U+F900-FAFF.
///
/// These are the Windows `HandwritingPanel::ContainsCjk` ranges, deliberately narrower than `text::is_han` (no U+3007, no supplementary planes), because the order hosts show must not change. The reference scanned bytes for three-byte sequences; on valid UTF-8 that is exactly a scan of the characters.
pub fn contains_cjk(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}'))
}

/// Drop empty, oversized and duplicate entries, move the CJK ones first keeping order within each group, keep 12.
pub fn order_handwriting_candidates(candidates: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let (cjk, other): (Vec<&String>, Vec<&String>) = candidates
        .iter()
        .filter(|candidate| !candidate.is_empty() && candidate.len() <= MAX_CANDIDATE_BYTES)
        .filter(|candidate| seen.insert(candidate.as_str()))
        .partition(|candidate| contains_cjk(candidate));
    cjk.into_iter()
        .chain(other)
        .take(MAX_CANDIDATES)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn cjk_ranges() {
        assert!(contains_cjk("中"));
        assert!(contains_cjk("a㐀"));
        assert!(contains_cjk("\u{4DBF}"));
        assert!(contains_cjk("\u{F900}"));
        assert!(contains_cjk("\u{FAFF}"));
        assert!(!contains_cjk("〇"));
        assert!(!contains_cjk("\u{20000}"));
        assert!(!contains_cjk("abc"));
        assert!(!contains_cjk("あ"));
        assert!(!contains_cjk(""));
    }

    #[test]
    fn cjk_first_stable_deduplicated() {
        let input = strings(&["a", "中", "", "b", "中", "国", "a", "c"]);
        assert_eq!(
            order_handwriting_candidates(&input),
            strings(&["中", "国", "a", "b", "c"])
        );
    }

    #[test]
    fn drops_oversized_and_caps_at_twelve() {
        let oversized = "x".repeat(MAX_CANDIDATE_BYTES + 1);
        let exact = "y".repeat(MAX_CANDIDATE_BYTES);
        let mut input = vec![oversized, exact.clone()];
        input.extend((0..20).map(|index| format!("w{index}")));
        let ordered = order_handwriting_candidates(&input);
        assert_eq!(ordered.len(), MAX_CANDIDATES);
        assert_eq!(ordered[0], exact);
        assert_eq!(ordered[11], "w10");
    }

    #[test]
    fn cap_applies_after_partition() {
        let mut input: Vec<String> = (0..12).map(|index| format!("l{index}")).collect();
        input.push("字".to_owned());
        let ordered = order_handwriting_candidates(&input);
        assert_eq!(ordered.len(), MAX_CANDIDATES);
        assert_eq!(ordered[0], "字");
        assert_eq!(ordered[11], "l10");
    }
}
