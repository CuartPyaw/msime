//! Profile tables, verbatim from `shuangpin_profile.cpp` (schemes-lang.md §1.2), and the accepted syllable set (§1.3): the intact set plus `eng`, minus the historical exclusions; `yo` is accepted in every profile.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::pinyin::syllables::intact_pinyin_set;
use crate::types::ShuangpinProfileKind;

#[derive(Debug)]
pub struct ShuangpinProfile {
    pub kind: ShuangpinProfileKind,
    pub name: &'static str,
    /// Multi-letter initial to key.
    pub initials: &'static [(&'static str, &'static str)],
    /// Whole zero-initial syllable to its two-key code.
    pub zero_initials: &'static [(&'static str, &'static str)],
    /// Final to key.
    pub finals: &'static [(&'static str, &'static str)],
}

static XIAOHE: ShuangpinProfile = ShuangpinProfile {
    kind: ShuangpinProfileKind::Xiaohe,
    name: "xiaohe",
    initials: &[("sh", "u"), ("ch", "i"), ("zh", "v")],
    zero_initials: &[
        ("a", "aa"),
        ("ai", "ai"),
        ("an", "an"),
        ("ao", "ao"),
        ("ang", "ah"),
        ("e", "ee"),
        ("ei", "ei"),
        ("en", "en"),
        ("eng", "eg"),
        ("er", "er"),
        ("o", "oo"),
        ("ou", "ou"),
    ],
    finals: &[
        ("iu", "q"),
        ("ei", "w"),
        ("e", "e"),
        ("uan", "r"),
        ("ue", "t"),
        ("ve", "t"),
        ("un", "y"),
        ("u", "u"),
        ("i", "i"),
        ("uo", "o"),
        ("o", "o"),
        ("ie", "p"),
        ("a", "a"),
        ("ong", "s"),
        ("iong", "s"),
        ("ai", "d"),
        ("en", "f"),
        ("eng", "g"),
        ("ang", "h"),
        ("an", "j"),
        ("uai", "k"),
        ("ing", "k"),
        ("uang", "l"),
        ("iang", "l"),
        ("ou", "z"),
        ("ua", "x"),
        ("ia", "x"),
        ("ao", "c"),
        ("ui", "v"),
        ("v", "v"),
        ("in", "b"),
        ("iao", "n"),
        ("ian", "m"),
    ],
};

static ZIRANMA: ShuangpinProfile = ShuangpinProfile {
    kind: ShuangpinProfileKind::Ziranma,
    name: "ziranma",
    initials: &[("sh", "u"), ("ch", "i"), ("zh", "v")],
    zero_initials: &[
        ("a", "aa"),
        ("ai", "ai"),
        ("an", "an"),
        ("ao", "ao"),
        ("ang", "ah"),
        ("e", "ee"),
        ("ei", "ei"),
        ("en", "en"),
        ("eng", "eg"),
        ("er", "er"),
        ("o", "oo"),
        ("ou", "ou"),
    ],
    finals: &[
        ("iu", "q"),
        ("ia", "w"),
        ("ua", "w"),
        ("e", "e"),
        ("uan", "r"),
        ("ue", "t"),
        ("ve", "t"),
        ("ing", "y"),
        ("uai", "y"),
        ("u", "u"),
        ("i", "i"),
        ("o", "o"),
        ("uo", "o"),
        ("un", "p"),
        ("a", "a"),
        ("iong", "s"),
        ("ong", "s"),
        ("iang", "d"),
        ("uang", "d"),
        ("en", "f"),
        ("eng", "g"),
        ("ang", "h"),
        ("an", "j"),
        ("ao", "k"),
        ("ai", "l"),
        ("ei", "z"),
        ("ie", "x"),
        ("iao", "c"),
        ("ui", "v"),
        ("v", "v"),
        ("ou", "b"),
        ("in", "n"),
        ("ian", "m"),
    ],
};

static SHOUDAO: ShuangpinProfile = ShuangpinProfile {
    kind: ShuangpinProfileKind::Shoudao,
    name: "shoudao",
    initials: &[("sh", "e"), ("ch", "i"), ("zh", "v")],
    zero_initials: &[
        ("a", "aa"),
        ("ai", "ai"),
        ("an", "an"),
        ("ao", "ao"),
        ("ang", "ay"),
        // sh sits on "e", so "ee"/"ei"/"ef" would collide with she/shi/sheng.
        ("e", "ue"),
        ("ei", "ui"),
        ("en", "en"),
        ("eng", "uf"),
        ("er", "er"),
        ("o", "oo"),
        ("ou", "ou"),
    ],
    finals: &[
        ("iu", "q"),
        ("ua", "w"),
        ("e", "e"),
        ("ie", "r"),
        ("uan", "t"),
        ("ang", "y"),
        ("u", "u"),
        ("i", "i"),
        ("o", "o"),
        ("uo", "o"),
        ("iao", "p"),
        ("a", "a"),
        ("ou", "s"),
        ("ao", "d"),
        ("eng", "f"),
        ("uai", "g"),
        ("ing", "g"),
        ("ong", "h"),
        ("iong", "h"),
        ("an", "j"),
        ("en", "k"),
        ("ia", "k"),
        ("ai", "l"),
        ("ue", "l"),
        ("un", "z"),
        ("iang", "x"),
        ("uang", "x"),
        ("in", "c"),
        ("v", "v"),
        ("ui", "v"),
        ("ve", "b"),
        ("ian", "n"),
        ("ei", "m"),
    ],
};

static MICROSOFT: ShuangpinProfile = ShuangpinProfile {
    kind: ShuangpinProfileKind::Microsoft,
    name: "microsoft",
    initials: &[("sh", "u"), ("ch", "i"), ("zh", "v")],
    zero_initials: &[
        ("a", "oa"),
        ("ai", "ol"),
        ("an", "oj"),
        ("ang", "oh"),
        ("ao", "ok"),
        ("e", "oe"),
        ("ei", "oz"),
        ("en", "of"),
        ("eng", "og"),
        ("er", "or"),
        ("o", "oo"),
        ("ou", "ob"),
    ],
    finals: &[
        ("iu", "q"),
        ("ia", "w"),
        ("ua", "w"),
        ("e", "e"),
        ("uan", "r"),
        ("ue", "t"),
        ("ve", "v"),
        ("uai", "y"),
        ("v", "y"),
        ("u", "u"),
        ("i", "i"),
        ("o", "o"),
        ("uo", "o"),
        ("un", "p"),
        ("a", "a"),
        ("iong", "s"),
        ("ong", "s"),
        ("iang", "d"),
        ("uang", "d"),
        ("en", "f"),
        ("eng", "g"),
        ("ang", "h"),
        ("an", "j"),
        ("ao", "k"),
        ("ai", "l"),
        // The `ing` final is the `;` key; the scheme accepts `;` only as the second key of a chunk.
        ("ing", ";"),
        ("ei", "z"),
        ("ie", "x"),
        ("iao", "c"),
        ("ui", "v"),
        ("ou", "b"),
        ("in", "n"),
        ("ian", "m"),
    ],
};

pub fn profile(kind: ShuangpinProfileKind) -> &'static ShuangpinProfile {
    match kind {
        ShuangpinProfileKind::Xiaohe => &XIAOHE,
        ShuangpinProfileKind::Ziranma => &ZIRANMA,
        ShuangpinProfileKind::Shoudao => &SHOUDAO,
        ShuangpinProfileKind::Microsoft => &MICROSOFT,
    }
}

/// Spellings the quanpin table accepts but the historical shuangpin `pinyin.txt` did not; keeping them out preserves the ambiguous-segmentation results users are used to (shuangpin_utils.cpp:40-47). `yo` was removed from this list by the shuangpin_yo overlay.
const EXCLUDED_SYLLABLES: [&str; 13] = [
    "chua", "den", "fiao", "jve", "lo", "lue", "nou", "nue", "nun", "qve", "xve", "yve", "zhei",
];

pub fn accepted_syllables() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = intact_pinyin_set().clone();
        set.insert("eng");
        for syllable in EXCLUDED_SYLLABLES {
            set.remove(syllable);
        }
        set
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_of(table: &'static [(&'static str, &'static str)], unit: &str) -> Option<&'static str> {
        table
            .iter()
            .find(|(source, _)| *source == unit)
            .map(|(_, key)| *key)
    }

    #[test]
    fn profiles_select_by_kind() {
        for kind in [
            ShuangpinProfileKind::Xiaohe,
            ShuangpinProfileKind::Ziranma,
            ShuangpinProfileKind::Shoudao,
            ShuangpinProfileKind::Microsoft,
        ] {
            let selected = profile(kind);
            assert_eq!(selected.kind, kind);
            assert_eq!(selected.name, kind.name());
            assert_eq!(selected.zero_initials.len(), 12);
            assert_eq!(selected.finals.len(), 33);
        }
    }

    #[test]
    fn tables_keep_the_profile_specific_keys() {
        let shoudao = profile(ShuangpinProfileKind::Shoudao);
        assert_eq!(key_of(shoudao.initials, "sh"), Some("e"));
        assert_eq!(key_of(shoudao.zero_initials, "ang"), Some("ay"));
        assert_eq!(key_of(shoudao.zero_initials, "e"), Some("ue"));
        assert_eq!(key_of(shoudao.zero_initials, "ei"), Some("ui"));
        assert_eq!(key_of(shoudao.zero_initials, "eng"), Some("uf"));
        let microsoft = profile(ShuangpinProfileKind::Microsoft);
        assert_eq!(key_of(microsoft.finals, "ing"), Some(";"));
        assert_eq!(key_of(microsoft.finals, "v"), Some("y"));
        assert_eq!(key_of(microsoft.finals, "ve"), Some("v"));
        assert_eq!(key_of(microsoft.zero_initials, "a"), Some("oa"));
        let xiaohe = profile(ShuangpinProfileKind::Xiaohe);
        assert_eq!(key_of(xiaohe.initials, "zh"), Some("v"));
        assert_eq!(key_of(xiaohe.finals, "ing"), Some("k"));
    }

    #[test]
    fn accepted_set_keeps_yo_and_drops_the_historical_exclusions() {
        let set = accepted_syllables();
        assert!(set.contains("yo"));
        assert!(set.contains("eng"));
        for excluded in EXCLUDED_SYLLABLES {
            assert!(!set.contains(excluded), "{excluded} is accepted");
        }
        assert!(set.contains("shi"));
        assert!(set.contains("lve"));
    }
}
