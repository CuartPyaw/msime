//! `@` mode: the names and places of the host's mention list, filtered by the letters after `@`. The list is the user's own and stays on the device; the engine neither reads contacts nor asks any service where a place is.

use super::command::TEXT_UTF16_LIMIT;
use crate::types::{CandidateSource, MentionEntry, WordItem};

/// Two pages of the nine-row Windows candidate window; typing more of a key narrows the list.
pub const RESULT_LIMIT: usize = 18;
/// Entries kept from a host list, as many as one dictionary import takes; the rest are ignored.
pub const LIST_LIMIT: usize = 1000;
pub const KEY_LIMIT: usize = 64;

/// The entries that are usable of a host list: non-empty text within the candidate text bound, a key of lowercase letters and single apostrophes between them, the first entry of a text, at most `LIST_LIMIT`.
pub fn usable_mentions(entries: &[MentionEntry]) -> Vec<MentionEntry> {
    let mut usable: Vec<MentionEntry> = Vec::new();
    for entry in entries {
        if usable.len() == LIST_LIMIT {
            break;
        }
        let text_valid =
            !entry.text.trim().is_empty() && entry.text.encode_utf16().count() <= TEXT_UTF16_LIMIT;
        let key_valid = entry.key.len() <= KEY_LIMIT
            && entry
                .key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'\'')
            && (entry.key.is_empty() || entry.key.split('\'').all(|syllable| !syllable.is_empty()));
        if text_valid && key_valid && !usable.iter().any(|kept| kept.text == entry.text) {
            usable.push(entry.clone());
        }
    }
    usable
}

/// Generated rows for the letters after `@`, weight `count - index`, at most `RESULT_LIMIT`: entries the input spells out completely, then entries it begins, each group in list order. A key matches by its letters (`zhangsan`) or its initials (`zs`); an entry without a key matches by its own text in lowercase. `pinyin` holds the key. `entries` must have gone through `usable_mentions`.
pub fn query_mentions(code: &str, entries: &[MentionEntry]) -> Vec<WordItem> {
    let spellings = |entry: &MentionEntry| -> [String; 2] {
        if entry.key.is_empty() {
            return [entry.text.to_ascii_lowercase(), String::new()];
        }
        [
            entry.key.replace('\'', ""),
            entry
                .key
                .split('\'')
                .filter_map(|syllable| syllable.chars().next())
                .collect(),
        ]
    };
    let mut rows: Vec<&MentionEntry> = Vec::new();
    for exact in [true, false] {
        for entry in entries {
            let matched = spellings(entry).iter().any(|spelling| {
                !spelling.is_empty() && spelling.starts_with(code) && (spelling == code) == exact
            });
            if matched && !rows.iter().any(|kept| std::ptr::eq(*kept, entry)) {
                rows.push(entry);
            }
        }
    }
    let count = rows.len().min(RESULT_LIMIT);
    rows.into_iter()
        .take(count)
        .enumerate()
        .map(|(index, entry)| {
            WordItem::new(
                entry.key.clone(),
                entry.text.clone(),
                (count - index) as i64,
                CandidateSource::Generated,
                "",
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mention(text: &str, key: &str) -> MentionEntry {
        MentionEntry {
            text: text.to_owned(),
            key: key.to_owned(),
        }
    }

    fn list() -> Vec<MentionEntry> {
        usable_mentions(&[
            mention("张三", "zhang'san"),
            mention("张珊珊", "zhang'shan'shan"),
            mention("深圳市", "shen'zhen'shi"),
            mention("Alice", ""),
        ])
    }

    fn words(code: &str) -> Vec<String> {
        query_mentions(code, &list())
            .into_iter()
            .map(|row| row.word)
            .collect()
    }

    #[test]
    fn a_bare_at_lists_the_whole_list_in_order() {
        assert_eq!(words(""), ["张三", "张珊珊", "深圳市", "Alice"]);
        let rows = query_mentions("", &list());
        assert_eq!(rows[0].pinyin, "zhang'san");
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.source, CandidateSource::Generated);
            assert_eq!(row.weight, (rows.len() - index) as i64);
        }
    }

    #[test]
    fn keys_match_by_letters_and_by_initials() {
        assert_eq!(words("zhang"), ["张三", "张珊珊"]);
        assert_eq!(words("zs"), ["张三", "张珊珊"]);
        assert_eq!(words("zss"), ["张珊珊"]);
        assert_eq!(words("szs"), ["深圳市"]);
        assert_eq!(words("shenzhen"), ["深圳市"]);
        assert_eq!(words("al"), ["Alice"]);
        assert!(words("x").is_empty());
    }

    #[test]
    fn complete_spellings_come_first() {
        let entries =
            usable_mentions(&[mention("张珊", "zhang'shan"), mention("张三", "zhang'san")]);
        let rows: Vec<String> = query_mentions("zs", &entries)
            .into_iter()
            .map(|row| row.word)
            .collect();
        assert_eq!(rows, ["张珊", "张三"]);
        let rows: Vec<String> = query_mentions("zhangsan", &entries)
            .into_iter()
            .map(|row| row.word)
            .collect();
        assert_eq!(rows, ["张三"]);
    }

    #[test]
    fn unusable_entries_are_dropped() {
        let long = "名".repeat(TEXT_UTF16_LIMIT + 1);
        let usable = usable_mentions(&[
            mention("", "a"),
            mention("  ", "a"),
            mention(&long, "a"),
            mention("大写", "Da"),
            mention("数字", "a1"),
            mention("空音节", "a''b"),
            mention("尾撇", "a'"),
            mention("李四", "li'si"),
            mention("李四", "lisi"),
        ]);
        assert_eq!(usable, [mention("李四", "li'si")]);
        let many: Vec<MentionEntry> = (0..LIST_LIMIT + 5)
            .map(|index| mention(&index.to_string(), ""))
            .collect();
        assert_eq!(usable_mentions(&many).len(), LIST_LIMIT);
    }
}
