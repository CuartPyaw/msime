//! SQLite housekeeping shared by the database stages.

use std::path::Path;

use anyhow::{bail, Result};
use rusqlite::Connection;

pub fn open(path: &Path) -> Result<Connection> {
    Ok(Connection::open(path)?)
}

pub fn integrity_check(connection: &Connection) -> Result<()> {
    let result: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result != "ok" {
        bail!("SQLite integrity check failed: {result}");
    }
    Ok(())
}

/// `ANALYZE` (and optionally `PRAGMA optimize`), keeping only `sqlite_stat1`. The bundled SQLite is compiled with STAT4, which the Python-built databases never carried; the runtime plans its queries from `sqlite_stat1` alone, so the extra table would only make the artifacts differ.
pub fn analyze(connection: &Connection, optimize: bool) -> Result<()> {
    connection.execute_batch("ANALYZE")?;
    if optimize {
        connection.execute_batch("PRAGMA optimize")?;
    }
    connection.execute_batch("DROP TABLE IF EXISTS sqlite_stat4")?;
    Ok(())
}

/// Ship a standalone database: no WAL header that needs sidecar files, and no free pages (dropping `sqlite_stat4` and replaced tables leaves some).
pub fn freeze(path: &Path) -> Result<()> {
    let connection = open(path)?;
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
    let mode: String = connection.query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("delete") {
        bail!(
            "{}: cannot finalize a standalone database (journal mode {mode})",
            path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_leaves_only_stat1() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.db");
        let connection = open(&path).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE t(a, b); CREATE INDEX i ON t(a); INSERT INTO t VALUES (1, 2), (1, 3), (2, 4);")
            .unwrap();
        analyze(&connection, true).unwrap();
        let tables: Vec<String> = connection
            .prepare("SELECT name FROM sqlite_master WHERE name LIKE 'sqlite_stat%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(tables, ["sqlite_stat1"]);
        drop(connection);
        freeze(&path).unwrap();
        let frozen = open(&path).unwrap();
        let mode: String = frozen
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "delete");
        let free: i64 = frozen
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))
            .unwrap();
        assert_eq!(free, 0);
        integrity_check(&open(&path).unwrap()).unwrap();
    }
}
