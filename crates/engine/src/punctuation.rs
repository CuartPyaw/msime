//! Chinese punctuation translation (core-session.md §5.6, overlays.md §7.1, `contracts/punctuation/policy.json`): a fixed map, alternating quotes, and nested book-title marks. Quotes always alternate and book titles always nest, whether pairing is on or not; pairing only decides whether a host-side auto-close pays back the nesting count.

/// `policy.json` contract version 1, `simple` in file order.
const SIMPLE: [(u8, &str); 15] = [
    (b',', "，"),
    (b'.', "。"),
    (b'?', "？"),
    (b'!', "！"),
    (b';', "；"),
    (b':', "："),
    (b'(', "（"),
    (b')', "）"),
    (b'[', "【"),
    (b']', "】"),
    (b'\\', "、"),
    (b'`', "·"),
    (b'$', "￥"),
    (b'^', "……"),
    (b'_', "——"),
];

const DOUBLE_QUOTE: (&str, &str) = ("“", "”");
const SINGLE_QUOTE: (&str, &str) = ("‘", "’");

const NESTED_OPENING_INPUT: u8 = b'<';
const NESTED_CLOSING_INPUT: u8 = b'>';
const NESTED_OPENING: &str = "《";
const NESTED_OPENING_INNER: &str = "〈";
const NESTED_CLOSING: &str = "》";
const NESTED_CLOSING_INNER: &str = "〉";

#[derive(Debug, Clone)]
pub struct PunctuationPolicy {
    paired_enabled: bool,
    next_double_quote_is_opening: bool,
    next_single_quote_is_opening: bool,
    book_title_nesting: u32,
}

impl Default for PunctuationPolicy {
    fn default() -> Self {
        Self {
            paired_enabled: true,
            next_double_quote_is_opening: true,
            next_single_quote_is_opening: true,
            book_title_nesting: 0,
        }
    }
}

impl PunctuationPolicy {
    /// The Chinese mark for an ASCII key, advancing quote and nesting state; `None` for keys without one (`{`, `@`, `-`, ...).
    pub fn translate(&mut self, key: u8) -> Option<&'static str> {
        if let Some(&(_, output)) = SIMPLE.iter().find(|(input, _)| *input == key) {
            return Some(output);
        }
        // Without host pairing this alternation is the only way to type a closing quote, and a host that does pair rewrites the closing quote itself, so it runs either way (punctuation_policy.cpp:12).
        match key {
            b'"' => Some(alternate(
                &mut self.next_double_quote_is_opening,
                DOUBLE_QUOTE,
            )),
            b'\'' => Some(alternate(
                &mut self.next_single_quote_is_opening,
                SINGLE_QUOTE,
            )),
            NESTED_OPENING_INPUT => {
                self.book_title_nesting += 1;
                Some(if self.book_title_nesting == 1 {
                    NESTED_OPENING
                } else {
                    NESTED_OPENING_INNER
                })
            }
            // An unmatched `>` leaves the count at zero, so the next `<` still opens the outer pair.
            NESTED_CLOSING_INPUT => {
                if self.book_title_nesting == 0 {
                    return Some(NESTED_CLOSING);
                }
                self.book_title_nesting -= 1;
                Some(if self.book_title_nesting == 0 {
                    NESTED_CLOSING
                } else {
                    NESTED_CLOSING_INNER
                })
            }
            _ => None,
        }
    }

    pub fn set_paired_enabled(&mut self, enabled: bool) {
        self.paired_enabled = enabled;
    }

    /// The host emitted the closing half of `opening` itself; only `<` with pairing on and a positive nesting count changes anything.
    pub fn balance_after_auto_close(&mut self, opening: u8) {
        if self.paired_enabled && opening == NESTED_OPENING_INPUT && self.book_title_nesting > 0 {
            self.book_title_nesting -= 1;
        }
    }
}

fn alternate(
    next_is_opening: &mut bool,
    (opening, closing): (&'static str, &'static str),
) -> &'static str {
    let output = if *next_is_opening { opening } else { closing };
    *next_is_opening = !*next_is_opening;
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translate_all(policy: &mut PunctuationPolicy, keys: &str) -> Vec<Option<&'static str>> {
        keys.bytes().map(|key| policy.translate(key)).collect()
    }

    /// contracts/tests/punctuation_contract.cpp.
    #[test]
    fn contract_table() {
        assert_eq!(SIMPLE.len(), 15);
        let mut policy = PunctuationPolicy::default();
        assert_eq!(policy.translate(b'$'), Some("￥"));
        assert_eq!(policy.translate(b'_'), Some("——"));
        assert_eq!(policy.translate(b'"'), Some("“"));
        assert_eq!(policy.translate(b'"'), Some("”"));
        assert_eq!(policy.translate(b'<'), Some("《"));
        assert_eq!(policy.translate(b'<'), Some("〈"));
        for key in *b"a{@-1 " {
            assert_eq!(policy.translate(key), None);
        }
    }

    /// The idle map of test_input_session.cpp:850-885, as recorded in the qp_idle_chinese_punctuation golden.
    #[test]
    fn idle_map_in_recorded_order() {
        let mut policy = PunctuationPolicy::default();
        let expected = [
            "。", "“", "”", "‘", "’", "（", "）", "【", "】", "·", "￥", "……", "——", "《", "〈",
            "〉", "》", "、",
        ];
        let actual = translate_all(&mut policy, ".\"\"''()[]`$^_<<>>\\");
        assert_eq!(actual, expected.map(Some));
        let rest = translate_all(&mut policy, ",?!;:");
        assert_eq!(rest, ["，", "？", "！", "；", "："].map(Some));
    }

    #[test]
    fn quotes_toggle_independently() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(
            translate_all(&mut policy, "\"'\"'"),
            ["“", "‘", "”", "’"].map(Some)
        );
    }

    /// qp_paired_punctuation_disabled: alternation and nesting run with pairing off, and toggling pairing resets neither.
    #[test]
    fn pairing_off_still_alternates_and_nests() {
        let mut policy = PunctuationPolicy::default();
        policy.set_paired_enabled(false);
        assert_eq!(
            translate_all(&mut policy, "\"\"''<<>>"),
            ["“", "”", "‘", "’", "《", "〈", "〉", "》"].map(Some)
        );
        policy.translate(b'"');
        policy.set_paired_enabled(true);
        assert_eq!(policy.translate(b'"'), Some("”"));
    }

    #[test]
    fn unmatched_closing_never_goes_negative() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(
            translate_all(&mut policy, ">><"),
            ["》", "》", "《"].map(Some)
        );
        assert_eq!(policy.book_title_nesting, 1);
    }

    #[test]
    fn auto_close_balances_only_with_pairing() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(policy.translate(b'<'), Some("《"));
        policy.balance_after_auto_close(b'<');
        assert_eq!(policy.book_title_nesting, 0);
        assert_eq!(policy.translate(b'<'), Some("《"));

        policy.balance_after_auto_close(b'(');
        assert_eq!(policy.book_title_nesting, 1);

        policy.set_paired_enabled(false);
        policy.balance_after_auto_close(b'<');
        assert_eq!(policy.book_title_nesting, 1);
        assert_eq!(policy.translate(b'<'), Some("〈"));

        policy.set_paired_enabled(true);
        let mut empty = PunctuationPolicy::default();
        empty.balance_after_auto_close(b'<');
        assert_eq!(empty.book_title_nesting, 0);
    }
}
