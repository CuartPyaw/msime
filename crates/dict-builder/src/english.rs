//! `english.db`: the English prefix-candidate table, the bidirectional glosses derived from ECDICT, and the hand-maintained translation overrides.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{bail, Context, Result};
use regex::Regex;
use rusqlite::{params, Connection};

use crate::sqlite;
use crate::text;

const CREATE_ENGLISH_WORDS: &str = "\n            CREATE TABLE english_words (\n                word TEXT COLLATE BINARY NOT NULL,\n                display TEXT NOT NULL,\n                weight INTEGER NOT NULL DEFAULT 0,\n                PRIMARY KEY (word, display)\n            ) WITHOUT ROWID\n            ";

fn is_ascii_word(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_alphabetic())
}

/// `str.isdigit` for the weight column: every character a decimal digit.
fn is_digits(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|c| {
            unicode_general_category::get_general_category(c)
                == unicode_general_category::GeneralCategory::DecimalNumber
        })
}

/// OALDPE headwords: one lowercase ASCII word per line, no duplicates.
pub fn parse_oaldpe_words(text: &str) -> Result<BTreeSet<String>> {
    let mut words = BTreeSet::new();
    for (number, line) in text::universal_lines(text).into_iter().enumerate() {
        let word = text::strip(line);
        if !is_ascii_word(word) || word.bytes().any(|b| b.is_ascii_uppercase()) {
            bail!(
                "oaldpe_words.txt:{}: expected a lowercase ASCII word, got {word:?}",
                number + 1
            );
        }
        if !words.insert(word.to_owned()) {
            bail!("oaldpe_words.txt:{}: duplicate word {word:?}", number + 1);
        }
    }
    if words.is_empty() {
        bail!("oaldpe_words.txt: file is empty");
    }
    Ok(words)
}

/// `display input-code [weight]` lines from rime-ice's English dictionary. The input code is ignored: prefix lookup uses the display word itself. Returns the lowercase word and the display casing to show, which stays as written only when the source has a single casing for it.
pub fn parse_base_dict_words(text: &str) -> Result<BTreeMap<String, String>> {
    let mut displays: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (number, line) in text::universal_lines(text).into_iter().enumerate() {
        let stripped = text::strip(line);
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        let mut fields: Vec<&str> = text::split_whitespace(stripped).collect();
        if fields.len() < 2 {
            bail!(
                "BaseDictIceEn.txt:{}: expected display input-code [weight]",
                number + 1
            );
        }
        if fields.len() >= 3 && is_digits(fields[fields.len() - 1]) {
            fields.pop();
        }
        fields.pop();
        let display = fields.join(" ");
        if is_ascii_word(&display) {
            displays
                .entry(display.to_ascii_lowercase())
                .or_default()
                .insert(display);
        }
    }
    if displays.is_empty() {
        bail!("BaseDictIceEn.txt: no pure English words found");
    }
    Ok(displays
        .into_iter()
        .map(|(word, casings)| {
            let display = if casings.len() == 1 {
                casings.into_iter().next().unwrap_or_default()
            } else {
                word.clone()
            };
            (word, display)
        })
        .collect())
}

/// Google's unigram counts (`word<TAB>count`), which order the words once several of them match a prefix. A later line for the same word wins.
pub fn parse_google_counts(text: &str) -> HashMap<String, i64> {
    let mut counts = HashMap::new();
    for line in text::universal_lines(text) {
        let stripped = text::strip(line);
        let (word, count) = stripped.split_once('\t').unwrap_or((stripped, ""));
        if word.is_empty() || !is_digits(count) {
            continue;
        }
        if let Ok(count) = count.parse() {
            counts.insert(word.to_lowercase(), count);
        }
    }
    counts
}

pub fn build_english_words(
    connection: &mut Connection,
    oaldpe: &BTreeSet<String>,
    base: &BTreeMap<String, String>,
    counts: &HashMap<String, i64>,
) -> Result<usize> {
    let words: BTreeSet<&String> = oaldpe.iter().chain(base.keys()).collect();
    let transaction = connection.transaction()?;
    transaction.execute_batch("DROP TABLE IF EXISTS english_words")?;
    transaction.execute_batch(CREATE_ENGLISH_WORDS)?;
    {
        let mut insert = transaction
            .prepare("INSERT INTO english_words(word, display, weight) VALUES (?, ?, ?)")?;
        for word in &words {
            let display = base.get(*word).unwrap_or(word);
            insert.execute(params![
                word,
                display,
                counts.get(*word).copied().unwrap_or(0)
            ])?;
        }
    }
    transaction.commit()?;

    // The Python verify_db.py step, including its ANALYZE: it runs before the gloss tables exist, so the shipped statistics cover english_words only.
    sqlite::integrity_check(connection)?;
    let primary_key: Vec<String> = connection
        .prepare("SELECT name FROM pragma_table_info('english_words') WHERE pk > 0 ORDER BY pk")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if primary_key != ["word", "display"] {
        bail!("english_words must use PRIMARY KEY(word, display); got {primary_key:?}");
    }
    let (rows, distinct): (i64, i64) = connection.query_row(
        "SELECT COUNT(*), COUNT(DISTINCT word) FROM english_words",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if rows == 0 || rows != distinct {
        bail!("english_words: unexpected row counts: rows={rows}, distinct_words={distinct}");
    }
    sqlite::analyze(connection, true)?;
    Ok(words.len())
}

// ---- ECDICT glosses ----

// Python's `\s` also matches the ASCII information separators, which Rust's does not.
macro_rules! ws {
    () => {
        r"[\s\x1c-\x1f]"
    };
}

static LEADING_DOMAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        "^",
        ws!(),
        r"*(?:\[[^\]]+\]|【[^】]+】)",
        ws!(),
        "*"
    ))
    .expect("valid regex")
});
static LEADING_POS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        "(?i)^",
        ws!(),
        r"*(?:(?:interj|abbr|modal|aux|adj|adv|prep|pron|conj|num|art|sing|pref|suff|vt|vi|ad|pl|int|n|v|a)\.?",
        ws!(),
        "*)+"
    ))
    .expect("valid regex")
});
static LEADING_PAREN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!("^", ws!(), r"*[（(][^）)]*[）)]", ws!(), "*")).expect("valid regex")
});
static TRAILING_PAREN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(ws!(), r"*[（(][^）)]*[）)]", ws!(), r"*\z")).expect("valid regex")
});
static INTERNAL_PAREN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[（(].*[）)]").expect("valid regex"));
static SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[,，;；、]+").expect("valid regex"));

const TRIM_PUNCTUATION: &str = " \t\r\n.:：!?！？'\"“”‘’·•-—–_/\\";

/// A reverse-index item starting with one of these is usually an explanation rather than a Chinese headword.
const REVERSE_EXPLANATION_PREFIXES: &[&str] = &[
    "表示", "用于", "用来", "用作", "指代", "即为", "一种", "一个", "某种", "某个",
];

const TAG_BONUS: &[(&str, i64)] = &[
    ("zk", 80_000),
    ("gk", 75_000),
    ("cet4", 70_000),
    ("cet6", 60_000),
    ("ky", 55_000),
    ("ielts", 50_000),
    ("toefl", 45_000),
    ("gre", 30_000),
];

fn is_cjk_term(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|c| matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}'))
}

fn is_lowercase_ascii_word(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_lowercase())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossTerm {
    pub text: String,
    pub line_index: usize,
    pub item_index: usize,
    pub domain_specific: bool,
    pub reverse_domain_allowed: bool,
}

impl GlossTerm {
    fn reverse_eligible(&self) -> bool {
        let length = self.text.chars().count();
        if (self.domain_specific && !self.reverse_domain_allowed) || !(2..=6).contains(&length) {
            return false;
        }
        !REVERSE_EXPLANATION_PREFIXES
            .iter()
            .any(|prefix| self.text.starts_with(prefix))
    }
}

fn parse_positive_int(value: &str) -> i64 {
    text::strip(value)
        .parse::<i64>()
        .map_or(0, |parsed| parsed.max(0))
}

pub struct EcdictRow<'a> {
    pub word: &'a str,
    pub translation: &'a str,
    pub collins: &'a str,
    pub oxford: &'a str,
    pub tag: &'a str,
    pub bnc: &'a str,
    pub frq: &'a str,
    pub exchange: &'a str,
}

pub fn vocabulary_quality(row: &EcdictRow) -> i64 {
    let collins = parse_positive_int(row.collins).min(5);
    let oxford = i64::from(parse_positive_int(row.oxford) != 0);
    let lowered = row.tag.to_lowercase();
    let tag_bonus = text::split_whitespace(&lowered)
        .map(|tag| {
            TAG_BONUS
                .iter()
                .find(|(name, _)| *name == tag)
                .map_or(0, |(_, bonus)| *bonus)
        })
        .max()
        .unwrap_or(0);
    let frq = parse_positive_int(row.frq);
    let bnc = parse_positive_int(row.bnc);
    let frq_bonus = if frq != 0 {
        (50_000 - frq.min(50_000)).max(0)
    } else {
        0
    };
    let bnc_bonus = if bnc != 0 {
        (25_000 - bnc.min(50_000) / 2).max(0)
    } else {
        0
    };
    collins * 100_000 + oxford * 80_000 + tag_bonus + frq_bonus + bnc_bonus
}

/// Removes leading and trailing qualifiers in parentheses; `None` when an explanation in parentheses sits inside the item.
fn strip_parenthetical_qualifiers(value: &str) -> Option<String> {
    let mut value = value.to_owned();
    loop {
        let next = TRAILING_PAREN
            .replace(&LEADING_PAREN.replace(&value, ""), "")
            .into_owned();
        if next == value {
            break;
        }
        value = next;
    }
    (!INTERNAL_PAREN.is_match(&value)).then_some(value)
}

/// Short Chinese headwords from an ECDICT translation, in reading order.
pub fn extract_gloss_terms(translation: &str) -> Vec<GlossTerm> {
    let mut terms = Vec::new();
    let mut seen = HashSet::new();
    // ECDICT stores many line breaks as the two characters `\n`.
    let translation = translation.replace("\\n", "\n");
    for (line_index, raw_line) in text::splitlines(&translation).into_iter().enumerate() {
        let mut line = text::strip(raw_line);
        if line.is_empty() {
            continue;
        }
        let mut domain_specific = false;
        let mut reverse_domain_allowed = false;
        while let Some(found) = LEADING_DOMAIN.find(line) {
            domain_specific = true;
            let label = found.as_str();
            reverse_domain_allowed =
                reverse_domain_allowed || label.contains('计') || label.contains("网络");
            line = &line[found.end()..];
        }
        let line = match LEADING_POS.find(line) {
            Some(found) => text::strip(&line[found.end()..]),
            None => text::strip(line),
        };
        for (item_index, raw_item) in SPLIT.split(line).enumerate() {
            let Some(item) =
                strip_parenthetical_qualifiers(text::strip_chars(raw_item, TRIM_PUNCTUATION))
            else {
                continue;
            };
            let item = text::strip_chars(&item, TRIM_PUNCTUATION);
            if item.chars().count() > 8 || !is_cjk_term(item) || !seen.insert(item.to_owned()) {
                continue;
            }
            terms.push(GlossTerm {
                text: item.to_owned(),
                line_index,
                item_index,
                domain_specific,
                reverse_domain_allowed,
            });
        }
    }
    terms
}

/// Up to two terms joined with `；`, general senses before domain senses, then by how common the term is as a Chinese candidate, then by position.
pub fn choose_chinese_gloss(terms: &[GlossTerm], weights: &HashMap<String, i64>) -> String {
    let texts: HashSet<&str> = terms.iter().map(|term| term.text.as_str()).collect();
    let mut ranked: Vec<&GlossTerm> = terms
        .iter()
        .filter(|term| {
            !term
                .text
                .strip_suffix('的')
                .is_some_and(|stem| texts.contains(stem))
        })
        .collect();
    ranked.sort_by(|a, b| {
        let key = |term: &GlossTerm| {
            (
                term.domain_specific,
                -weights.get(&term.text).copied().unwrap_or(0),
                term.line_index,
                term.item_index,
                term.text.chars().count(),
            )
        };
        key(a).cmp(&key(b)).then_with(|| a.text.cmp(&b.text))
    });
    ranked
        .iter()
        .take(2)
        .map(|term| term.text.as_str())
        .collect::<Vec<_>>()
        .join("；")
}

struct EnglishEntry {
    reverse_english: String,
    quality: i64,
    terms: Vec<GlossTerm>,
}

pub struct Glosses {
    pub en_zh: BTreeMap<String, String>,
    pub zh_en: BTreeMap<String, String>,
}

fn english_candidates(connection: &Connection) -> Result<HashSet<String>> {
    let mut candidates = HashSet::new();
    let mut statement = connection.prepare("SELECT DISTINCT word FROM english_words")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if let Some(word) = row.get::<_, Option<String>>(0)? {
            let word = word.to_lowercase();
            if is_lowercase_ascii_word(&word) {
                candidates.insert(word);
            }
        }
    }
    Ok(candidates)
}

/// Every pinyin-table value in `terms`, with its best weight (floored at zero).
fn chinese_term_weights(msime: &Connection, terms: &HashSet<&str>) -> Result<HashMap<String, i64>> {
    let table_pattern = Regex::new(r"\Atbl_(?:[1-7]|others)_[a-z]\z").expect("valid regex");
    let tables: Vec<String> = msime
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name GLOB 'tbl_*_[a-z]' ORDER BY name")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?
        .into_iter()
        .filter(|name| table_pattern.is_match(name))
        .collect();
    let mut weights = HashMap::new();
    for table in tables {
        let mut statement = msime.prepare(&format!("SELECT value,weight FROM \"{table}\""))?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let Some(value) = row.get::<_, Option<String>>(0)? else {
                continue;
            };
            if terms.contains(value.as_str()) {
                let weight = row.get::<_, Option<i64>>(1)?.unwrap_or(0);
                let best = weights.entry(value).or_insert(0);
                *best = (*best).max(weight);
            }
        }
    }
    Ok(weights)
}

/// Intersects ECDICT with the English candidates and derives both gloss directions. Only general senses of words with a corpus or core-vocabulary signal feed the Chinese-to-English index, and only for Chinese terms the pinyin tables can produce.
pub fn derive_glosses(ecdict: &Path, english: &Connection, msime: &Connection) -> Result<Glosses> {
    let candidates = english_candidates(english)?;
    let bytes = std::fs::read(ecdict).with_context(|| format!("reading {}", ecdict.display()))?;
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(bytes);
    let headers = reader.headers()?.clone();
    let column = |name: &str| headers.iter().position(|header| header == name);
    let required = [
        "word",
        "translation",
        "collins",
        "oxford",
        "tag",
        "bnc",
        "frq",
    ];
    let missing: Vec<_> = required
        .iter()
        .filter(|name| column(name).is_none())
        .collect();
    if !missing.is_empty() {
        bail!("ECDICT CSV is missing columns: {missing:?}");
    }
    let [word, translation, collins, oxford, tag, bnc, frq] =
        required.map(|name| column(name).unwrap_or_default());
    let exchange = column("exchange");

    let mut entries: BTreeMap<String, EnglishEntry> = BTreeMap::new();
    let mut record = csv::StringRecord::new();
    while reader.read_record(&mut record)? {
        let field = |index: usize| record.get(index).unwrap_or("");
        let row = EcdictRow {
            word: field(word),
            translation: field(translation),
            collins: field(collins),
            oxford: field(oxford),
            tag: field(tag),
            bnc: field(bnc),
            frq: field(frq),
            exchange: exchange.map_or("", field),
        };
        let english = text::strip(row.word).to_lowercase();
        if !candidates.contains(&english) {
            continue;
        }
        let translation = text::strip(row.translation);
        if translation.is_empty() {
            continue;
        }
        let terms = extract_gloss_terms(translation);
        if terms.is_empty() {
            continue;
        }
        let quality = vocabulary_quality(&row);
        // A lemma listed in `exchange` as `0:<lemma>` stands in for its inflections in the reverse index.
        let reverse_english = row
            .exchange
            .split('/')
            .filter_map(|item| item.strip_prefix("0:"))
            .map(|lemma| text::strip(lemma).to_lowercase())
            .find(|lemma| candidates.contains(lemma) && is_lowercase_ascii_word(lemma))
            .unwrap_or_else(|| english.clone());
        let replace = entries.get(&english).is_none_or(|previous| {
            (quality, terms.len()) > (previous.quality, previous.terms.len())
        });
        if replace {
            entries.insert(
                english,
                EnglishEntry {
                    reverse_english,
                    quality,
                    terms,
                },
            );
        }
    }

    // Reverse candidates are built only after duplicate English rows are resolved, so a duplicated source row cannot distort the ranking.
    let mut reverse: HashMap<String, HashMap<String, i64>> = HashMap::new();
    for entry in entries.values() {
        if entry.quality <= 0 {
            continue;
        }
        for term in entry.terms.iter().filter(|term| term.reverse_eligible()) {
            let position_bonus =
                (30_000 - term.line_index as i64 * 2_000 - term.item_index as i64 * 500).max(0);
            let score =
                entry.quality + position_bonus - entry.reverse_english.chars().count() as i64 * 10;
            let best = reverse
                .entry(term.text.clone())
                .or_default()
                .entry(entry.reverse_english.clone())
                .or_insert(score);
            *best = (*best).max(score);
        }
    }

    let all_terms: HashSet<&str> = entries
        .values()
        .flat_map(|entry| entry.terms.iter().map(|term| term.text.as_str()))
        .collect();
    let weights = chinese_term_weights(msime, &all_terms)?;
    let en_zh = entries
        .iter()
        .map(|(english, entry)| {
            (
                english.clone(),
                choose_chinese_gloss(&entry.terms, &weights),
            )
        })
        .collect();
    let zh_en = reverse
        .into_iter()
        .filter(|(chinese, _)| weights.contains_key(chinese))
        .filter_map(|(chinese, candidates)| {
            finalize_reverse_gloss(candidates).map(|gloss| (chinese, gloss))
        })
        .collect();
    Ok(Glosses { en_zh, zh_en })
}

/// The best English words for one Chinese term: at most two, each within 45% of the top score.
fn finalize_reverse_gloss(candidates: HashMap<String, i64>) -> Option<String> {
    let mut ranked: Vec<(String, i64)> = candidates.into_iter().collect();
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(a.0.chars().count().cmp(&b.0.chars().count()))
            .then_with(|| a.0.cmp(&b.0))
    });
    let top = ranked.first()?.1;
    let selected: Vec<String> = ranked
        .into_iter()
        .filter(|(_, score)| score * 100 >= top * 45)
        .take(2)
        .map(|(english, _)| english)
        .collect();
    Some(selected.join("; "))
}

const CREATE_GLOSS_TABLES: &str = "\n            DROP TABLE IF EXISTS en_zh_glosses_new;\n            DROP TABLE IF EXISTS zh_en_glosses_new;\n            CREATE TABLE en_zh_glosses_new (\n                english TEXT COLLATE BINARY PRIMARY KEY,\n                chinese_gloss TEXT NOT NULL\n            ) WITHOUT ROWID;\n            CREATE TABLE zh_en_glosses_new (\n                chinese TEXT COLLATE BINARY PRIMARY KEY,\n                english_gloss TEXT NOT NULL\n            ) WITHOUT ROWID;\n            ";
const SWAP_GLOSS_TABLES: &str = "\n            DROP TABLE IF EXISTS en_zh_glosses;\n            ALTER TABLE en_zh_glosses_new RENAME TO en_zh_glosses;\n            DROP TABLE IF EXISTS zh_en_glosses;\n            ALTER TABLE zh_en_glosses_new RENAME TO zh_en_glosses;\n            PRAGMA user_version=3;\n            ";

/// Replaces both gloss tables atomically (built under `_new` names and renamed, as the runtime may hold the file open during a local rebuild).
pub fn write_glosses(connection: &mut Connection, glosses: &Glosses) -> Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(CREATE_GLOSS_TABLES)?;
    {
        let mut insert = transaction
            .prepare("INSERT INTO en_zh_glosses_new(english,chinese_gloss) VALUES(?1,?2)")?;
        for (english, gloss) in &glosses.en_zh {
            insert.execute(params![english, gloss])?;
        }
        let mut insert = transaction
            .prepare("INSERT INTO zh_en_glosses_new(chinese,english_gloss) VALUES(?1,?2)")?;
        for (chinese, gloss) in &glosses.zh_en {
            insert.execute(params![chinese, gloss])?;
        }
    }
    transaction.execute_batch(SWAP_GLOSS_TABLES)?;
    transaction.commit()?;
    sqlite::integrity_check(connection)
}

// ---- custom translations ----

#[derive(Debug, PartialEq, Eq)]
pub struct CustomTranslation {
    pub chinese_to_english: bool,
    pub source: String,
    pub gloss: String,
}

/// `source<TAB>gloss`; a source containing a character from U+3400 up is Chinese-to-English, anything else English-to-Chinese. A later line for the same source wins.
pub fn parse_custom_translations(text: &str) -> Result<Vec<CustomTranslation>> {
    let mut entries = Vec::new();
    for (number, raw_line) in text::splitlines(text::without_bom(text))
        .into_iter()
        .enumerate()
    {
        let line = text::strip(raw_line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 {
            bail!(
                "custom/translations.txt:{}: expected source<TAB>gloss, got {raw_line:?}",
                number + 1
            );
        }
        let (source, gloss) = (text::strip(fields[0]), text::strip(fields[1]));
        if source.is_empty() || gloss.is_empty() {
            bail!(
                "custom/translations.txt:{}: empty source or gloss",
                number + 1
            );
        }
        entries.push(CustomTranslation {
            chinese_to_english: source.chars().any(|c| c >= '\u{3400}'),
            source: source.to_owned(),
            gloss: gloss.to_owned(),
        });
    }
    Ok(entries)
}

pub fn apply_custom_translations(
    connection: &mut Connection,
    entries: &[CustomTranslation],
) -> Result<()> {
    let transaction = connection.transaction()?;
    for entry in entries {
        let sql = if entry.chinese_to_english {
            "INSERT OR REPLACE INTO zh_en_glosses(chinese,english_gloss) VALUES(?1,?2)"
        } else {
            "INSERT OR REPLACE INTO en_zh_glosses(english,chinese_gloss) VALUES(?1,?2)"
        };
        transaction.execute(sql, params![entry.source, entry.gloss])?;
    }
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_words_merge_casings_and_counts() {
        let base = parse_base_dict_words("AA AA\r\naaa aaa\r\n# aac aac\r\nAaliyah Aaliyah\r\nHello hello 3\r\nhello hello\r\nice cream icecream\r\nJan jan 12\r\n").unwrap();
        assert_eq!(base.get("aa").map(String::as_str), Some("AA"));
        assert_eq!(base.get("aaliyah").map(String::as_str), Some("Aaliyah"));
        assert_eq!(
            base.get("hello").map(String::as_str),
            Some("hello"),
            "two casings fall back to lowercase"
        );
        assert_eq!(base.get("jan").map(String::as_str), Some("Jan"));
        assert!(!base.contains_key("ice cream"));

        let counts = parse_google_counts("the\t100\nHello\t7\nbad\tx\n\t5\n");
        assert_eq!(counts.get("hello"), Some(&7));
        assert!(!counts.contains_key("bad"));

        let oaldpe = parse_oaldpe_words("a\nzebra\n").unwrap();
        assert!(parse_oaldpe_words("a\na\n").is_err());
        assert!(parse_oaldpe_words("Abc\n").is_err());

        let mut connection = Connection::open_in_memory().unwrap();
        assert_eq!(
            build_english_words(&mut connection, &oaldpe, &base, &counts).unwrap(),
            7
        );
        let row: (String, i64) = connection
            .query_row(
                "SELECT display, weight FROM english_words WHERE word='hello'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(row, ("hello".into(), 7));
        let stats: Vec<String> = connection
            .prepare("SELECT tbl FROM sqlite_stat1")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(stats, ["english_words"]);
    }

    #[test]
    fn gloss_terms_drop_labels_parts_of_speech_and_explanations() {
        let terms =
            extract_gloss_terms("n. 银行, 堤(河岸)\\n[计] vt. 存款；（口）储蓄\\n[医] 一种病");
        let texts: Vec<_> = terms
            .iter()
            .map(|term| {
                (
                    term.text.as_str(),
                    term.line_index,
                    term.item_index,
                    term.domain_specific,
                )
            })
            .collect();
        assert_eq!(
            texts,
            [
                ("银行", 0, 0, false),
                ("堤", 0, 1, false),
                ("存款", 1, 0, true),
                ("储蓄", 1, 1, true),
                ("一种病", 2, 0, true)
            ]
        );
        assert!(terms[2].reverse_domain_allowed);
        assert!(terms[2].reverse_eligible());
        assert!(!terms[1].reverse_eligible(), "one character is too short");
        assert!(!terms[4].reverse_eligible());
        assert!(extract_gloss_terms("a. 很长很长很长很长的词语").is_empty());
    }

    #[test]
    fn the_chinese_gloss_prefers_common_general_terms() {
        let terms = extract_gloss_terms("n. 未来的, 未来, 前途, 期货");
        let weights = HashMap::from([("前途".to_owned(), 10), ("未来".to_owned(), 5)]);
        assert_eq!(choose_chinese_gloss(&terms, &weights), "前途；未来");
    }

    #[test]
    fn quality_combines_collins_oxford_tags_and_frequency() {
        let row = EcdictRow {
            word: "bank",
            translation: "",
            collins: "7",
            oxford: "1",
            tag: "zk GK cet4",
            bnc: "1000",
            frq: "",
            exchange: "",
        };
        assert_eq!(vocabulary_quality(&row), 500_000 + 80_000 + 80_000 + 24_500);
    }

    fn fixture_databases() -> (Connection, Connection) {
        let english = Connection::open_in_memory().unwrap();
        english.execute_batch("CREATE TABLE english_words(word, display, weight); INSERT INTO english_words VALUES ('bank','bank',0),('banks','banks',0),('run','run',0),('rare','rare',0);").unwrap();
        let msime = Connection::open_in_memory().unwrap();
        msime.execute_batch("CREATE TABLE tbl_2_y(key, jp, value, weight); INSERT INTO tbl_2_y VALUES ('yin''hang','yh','银行',900); CREATE TABLE tbl_2_p(key, jp, value, weight); INSERT INTO tbl_2_p VALUES ('pao''bu','pb','跑步',50);").unwrap();
        (english, msime)
    }

    #[test]
    fn glosses_intersect_ecdict_with_both_dictionaries() {
        let (english, msime) = fixture_databases();
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("ecdict.csv");
        std::fs::write(
            &csv,
            "\u{feff}word,phonetic,definition,translation,pos,collins,oxford,tag,bnc,frq,exchange,detail,audio\n\
             bank,,,\"n. 银行, 堤\",,3,1,zk,500,400,s:banks,,\n\
             banks,,,n. 银行,,,,,,,0:bank,,\n\
             run,,,\"v. 跑步\\nn. 运行\",,5,1,,100,100,,,\n\
             rare,,,a. 稀有的,,,,,,,,,\n\
             absent,,,n. 缺席,,5,1,,1,1,,,\n",
        )
        .unwrap();
        let glosses = derive_glosses(&csv, &english, &msime).unwrap();
        assert_eq!(
            glosses.en_zh.get("bank").map(String::as_str),
            Some("银行；堤")
        );
        assert_eq!(glosses.en_zh.get("banks").map(String::as_str), Some("银行"));
        assert!(!glosses.en_zh.contains_key("absent"));
        // 稀有 has no quality signal, 运行 and 堤 are not pinyin-table values.
        assert_eq!(glosses.zh_en.keys().collect::<Vec<_>>(), ["跑步", "银行"]);
        assert_eq!(glosses.zh_en.get("银行").map(String::as_str), Some("bank"));

        let mut english = english;
        write_glosses(&mut english, &glosses).unwrap();
        let entries = parse_custom_translations("\u{feff}# c\n华科\tHUST\nbank\t河岸\n").unwrap();
        assert!(entries[0].chinese_to_english && !entries[1].chinese_to_english);
        apply_custom_translations(&mut english, &entries).unwrap();
        let bank: String = english
            .query_row(
                "SELECT chinese_gloss FROM en_zh_glosses WHERE english='bank'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(bank, "河岸");
        let version: i64 = english
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
        let sql: String = english
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='zh_en_glosses'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(sql.starts_with("CREATE TABLE \"zh_en_glosses\" ("), "{sql}");
        assert!(parse_custom_translations("only-one-field\n").is_err());
    }
}
