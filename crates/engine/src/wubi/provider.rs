//! The `wubi86` provider (`R/providers/wubi_candidate_provider.cpp`, with the wubi_prefix_learning overlay): exact code first, then weight, no value dedup, at most 50 rows. The provider only reads: learning and removal of a wubi row go through `session`, which journals them with the wubi kind and then resets this cache.

use std::path::{Path, PathBuf};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use crate::dictionary::pinyin::BUSY_TIMEOUT;
use crate::types::{CandidateSource, QueryRequest, SchemeType, WordItem};

const QUERY_LIMIT: i64 = 50;

/// A prefix query: the typed code's own rows lead, then every longer code it prefixes by weight. The same word reached through several codes (工 at a, aaa and aaaa) is kept once per code, because ranking and removal act on the code the row arrived with.
const QUERY_SQL: &str = "SELECT \"key\", \"value\", \"weight\" FROM wubi86 WHERE \"key\" >= ?1 AND \"key\" < ?2 ORDER BY (\"key\" = ?1) DESC, \"weight\" DESC, \"key\" ASC, rowid ASC LIMIT ?3";

pub struct WubiProvider {
    main_db: PathBuf,
    connection: Option<Connection>,
}

impl WubiProvider {
    /// Opens lazily, read-only, on the first query.
    pub fn new(main_db: &Path) -> Self {
        Self {
            main_db: main_db.to_path_buf(),
            connection: None,
        }
    }

    /// `SELECT "key","value","weight" FROM wubi86 WHERE "key" >= ?1 AND "key" < ?2 ORDER BY ("key" = ?1) DESC, "weight" DESC, "key" ASC, rowid ASC LIMIT ?3`, `?2` = the code with its last letter incremented and `?3` = 50. Rows carry `scheme = Wubi`. Any SQLite failure is an empty answer, as in the reference.
    pub fn query(&mut self, request: &QueryRequest) -> Vec<WordItem> {
        if !request.valid
            || request.scheme != SchemeType::Wubi
            || request.normalized_input.is_empty()
        {
            return Vec::new();
        }
        let Some(connection) = self.connection() else {
            return Vec::new();
        };
        match query_rows(connection, &request.normalized_input) {
            Ok(rows) => rows,
            Err(_) => {
                // The reference dropped a statement that failed and prepared it again on the next key; closing gives the same retry.
                self.connection = None;
                Vec::new()
            }
        }
    }

    /// Closes the connection, so the next query sees what learning or removal wrote since.
    pub fn reset_cache(&mut self) {
        self.connection = None;
    }

    fn connection(&mut self) -> Option<&Connection> {
        if self.connection.is_none() {
            self.connection = open_read_only(&self.main_db);
        }
        self.connection.as_ref()
    }
}

fn open_read_only(path: &Path) -> Option<Connection> {
    // An empty path is how `RuntimePaths` spells a missing file; SQLite would open a private temporary database for it instead.
    if path.as_os_str().is_empty() {
        return None;
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    // Another session's learning write briefly holds the commit lock; waiting keeps a keystroke that lands in that window from showing an empty list.
    connection.busy_timeout(BUSY_TIMEOUT).ok()?;
    Some(connection)
}

/// The exclusive upper bound of every code `code` prefixes: its last letter incremented, so `ab` gives `ac` and `az` gives `a{`. Codes are ASCII letters, where that is the reference's last-byte increment.
fn prefix_upper_bound(code: &str) -> String {
    let mut upper = code.to_owned();
    if let Some(last) = upper.pop() {
        upper.push(char::from_u32(u32::from(last) + 1).unwrap_or(char::MAX));
    }
    upper
}

fn query_rows(connection: &Connection, code: &str) -> rusqlite::Result<Vec<WordItem>> {
    let mut statement = connection.prepare_cached(QUERY_SQL)?;
    let upper = prefix_upper_bound(code);
    let mut rows = statement.query((code, upper.as_str(), QUERY_LIMIT))?;
    let mut candidates = Vec::new();
    while let Some(row) = rows.next()? {
        // The reference skipped rows whose key or value read back as NULL.
        let (ValueRef::Text(key), ValueRef::Text(value)) = (row.get_ref(0)?, row.get_ref(1)?)
        else {
            continue;
        };
        let key = String::from_utf8_lossy(key).into_owned();
        let value = String::from_utf8_lossy(value).into_owned();
        let weight = match row.get_ref(2)? {
            ValueRef::Integer(weight) => weight,
            ValueRef::Real(weight) => weight as i64,
            ValueRef::Null | ValueRef::Text(_) | ValueRef::Blob(_) => 0,
        };
        let mut item = WordItem::new(key, value, weight, CandidateSource::Database, "");
        item.scheme = SchemeType::Wubi;
        candidates.push(item);
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _root: tempfile::TempDir,
        provider: WubiProvider,
    }

    fn fixture(sql: &str) -> Fixture {
        let root = tempfile::tempdir().expect("temporary directory");
        let main_db = root.path().join("msime.db");
        Connection::open(&main_db)
            .expect("fixture database")
            .execute_batch(sql)
            .expect("fixture SQL");
        let provider = WubiProvider::new(&main_db);
        Fixture {
            _root: root,
            provider,
        }
    }

    fn request(code: &str) -> QueryRequest {
        QueryRequest {
            scheme: SchemeType::Wubi,
            raw_input: code.to_owned(),
            raw_input_with_cases: code.to_owned(),
            normalized_input: code.to_owned(),
            valid: !code.is_empty(),
            ..QueryRequest::default()
        }
    }

    fn rows(provider: &mut WubiProvider, code: &str) -> Vec<(String, String, i64)> {
        provider
            .query(&request(code))
            .into_iter()
            .map(|item| (item.pinyin, item.word, item.weight))
            .collect()
    }

    fn row(code: &str, word: &str, weight: i64) -> (String, String, i64) {
        (code.to_owned(), word.to_owned(), weight)
    }

    // test_wubi_input_session.cpp:68-73. The pinned assertions there are stale (tests-inventory §2.1); these follow the overlay order the golden `wubi_prefix_codes` records.
    const SESSION_FIXTURE: &str = "CREATE TABLE wubi86(key TEXT,value TEXT,weight INTEGER);\
        INSERT INTO wubi86 VALUES('w','人',20);\
        INSERT INTO wubi86 VALUES('wq','你',10);\
        INSERT INTO wubi86 VALUES('wqb','爷',20);\
        INSERT INTO wubi86 VALUES('wqi','你',10);\
        INSERT INTO wubi86 VALUES('wqbb','父子',30);";

    #[test]
    fn exact_code_leads_then_weight_without_word_dedup() {
        let mut fixture = fixture(SESSION_FIXTURE);
        let provider = &mut fixture.provider;
        assert_eq!(
            rows(provider, "wq"),
            vec![
                row("wq", "你", 10),
                row("wqbb", "父子", 30),
                row("wqb", "爷", 20),
                row("wqi", "你", 10),
            ]
        );
        assert_eq!(
            rows(provider, "w"),
            vec![
                row("w", "人", 20),
                row("wqbb", "父子", 30),
                row("wqb", "爷", 20),
                row("wq", "你", 10),
                row("wqi", "你", 10),
            ]
        );
        assert_eq!(rows(provider, "wqbb"), vec![row("wqbb", "父子", 30)]);
        assert!(rows(provider, "wx").is_empty());
    }

    #[test]
    fn rows_are_tagged_wubi_with_an_empty_canonical_key() {
        let mut fixture = fixture(SESSION_FIXTURE);
        let items = fixture.provider.query(&request("w"));
        assert!(items.iter().all(|item| item.scheme == SchemeType::Wubi
            && item.source == CandidateSource::Database
            && item.canonical_pinyin.is_empty()));
    }

    // engine-bridge tests.rs:191: 55 `b..` rows reach the host as exactly 50.
    #[test]
    fn prefix_query_is_bounded() {
        let mut sql = String::from("CREATE TABLE wubi86(key TEXT,value TEXT,weight INTEGER);");
        for index in 0..55 {
            sql.push_str(&format!(
                "INSERT INTO wubi86 VALUES('b{}','合成{index:02}',{});",
                char::from(b'a' + (index % 25) as u8),
                1_000 - index
            ));
        }
        let mut fixture = fixture(&sql);
        let words: Vec<String> = fixture
            .provider
            .query(&request("b"))
            .into_iter()
            .map(|item| item.word)
            .collect();
        let expected: Vec<String> = (0..QUERY_LIMIT)
            .map(|index| format!("合成{index:02}"))
            .collect();
        assert_eq!(words, expected);
    }

    #[test]
    fn last_letter_bound_covers_z_codes() {
        let mut fixture = fixture(
            "CREATE TABLE wubi86(key TEXT,value TEXT,weight INTEGER);\
             INSERT INTO wubi86 VALUES('yz','甲',1);\
             INSERT INTO wubi86 VALUES('yzz','乙',2);\
             INSERT INTO wubi86 VALUES('z','丙',3);",
        );
        assert_eq!(prefix_upper_bound("az"), "a{");
        assert_eq!(
            rows(&mut fixture.provider, "yz"),
            vec![row("yz", "甲", 1), row("yzz", "乙", 2)]
        );
    }

    #[test]
    fn null_rows_are_skipped_and_null_weight_reads_zero() {
        let mut fixture = fixture(
            "CREATE TABLE wubi86(key TEXT,value TEXT,weight INTEGER);\
             INSERT INTO wubi86 VALUES('a',NULL,5);\
             INSERT INTO wubi86 VALUES('a','工',NULL);",
        );
        assert_eq!(rows(&mut fixture.provider, "a"), vec![row("a", "工", 0)]);
    }

    #[test]
    fn missing_table_or_invalid_request_is_empty() {
        let mut other = fixture("CREATE TABLE other(x);");
        assert!(rows(&mut other.provider, "a").is_empty());

        let mut wubi = fixture(SESSION_FIXTURE);
        assert!(rows(&mut wubi.provider, "").is_empty());
        let mut quanpin = request("w");
        quanpin.scheme = SchemeType::Quanpin;
        assert!(wubi.provider.query(&quanpin).is_empty());
    }

    #[test]
    fn missing_database_is_empty() {
        let root = tempfile::tempdir().expect("temporary directory");
        let mut provider = WubiProvider::new(&root.path().join("absent.db"));
        assert!(provider.query(&request("a")).is_empty());
        assert!(!root.path().join("absent.db").exists());
    }

    #[test]
    fn reset_cache_sees_rows_written_since() {
        let mut fixture = fixture(SESSION_FIXTURE);
        assert_eq!(rows(&mut fixture.provider, "wqbb").len(), 1);
        Connection::open(&fixture.provider.main_db)
            .expect("writer")
            .execute("UPDATE wubi86 SET weight=99 WHERE key='wqbb'", ())
            .expect("update");
        fixture.provider.reset_cache();
        assert_eq!(
            rows(&mut fixture.provider, "wqbb"),
            vec![row("wqbb", "父子", 99)]
        );
    }
}
