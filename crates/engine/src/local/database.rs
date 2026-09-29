//! Read-only connections for the local mode queries. The reference opened a fresh connection per query for every generation path (local_database.cpp:14-97), which costs a file open per keystroke; this keeps one per path for the process instead, dropped by `close_cached_local_databases` before directories are replaced.
//!
//! At most one connection is kept per file name, as the reference did for the shipped dictionaries (local_database.cpp:75-80): opening the next generation's `msime.db` releases the previous one, so a long-running host never holds a deleted generation open and a test process that creates hundreds of temporary directories never accumulates descriptors. Callers hold an `Arc`, so an evicted connection stays valid until its last in-flight query ends.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

const BUSY_TIMEOUT: Duration = Duration::from_millis(1_000);

struct CachedConnection {
    path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

/// Keyed by file name; the entry remembers which full path it belongs to.
static CACHE: LazyLock<Mutex<HashMap<OsString, CachedConnection>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// READONLY, busy timeout 1000 ms. `None` when the file cannot be opened.
pub fn open_local_database(path: &Path) -> Option<Arc<Mutex<Connection>>> {
    let name = path.file_name()?.to_os_string();
    let mut cache = lock(&CACHE);
    if let Some(cached) = cache.get(&name) {
        if cached.path == path {
            return Some(Arc::clone(&cached.connection));
        }
    }
    // A missing or unreadable file is an answer, not an error: the queries turn it into their "unavailable" diagnostic (local_database.cpp:20-28).
    let connection = Arc::new(Mutex::new(open_read_only(path).ok()?));
    cache.insert(
        name,
        CachedConnection {
            path: path.to_path_buf(),
            connection: Arc::clone(&connection),
        },
    );
    Some(connection)
}

pub fn close_cached_local_databases() {
    lock(&CACHE).clear();
}

/// Never creates the file: a missing dictionary must stay missing (test_jianpin_input_session.cpp:172-176). The connection's own mutex is redundant under the `Mutex` wrapper, so SQLite's is not requested.
fn open_read_only(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(BUSY_TIMEOUT)?;
    Ok(connection)
}

/// A panic while a query held the lock leaves nothing half-written: the connection is read-only and statements reset on drop, so the poisoned value is still usable.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn fixture(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE t(x)")
            .unwrap();
        path
    }

    #[test]
    fn reuses_a_connection_until_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path(), "reuse-local.db");
        // Other tests close the process-wide cache at any moment, so a pair of opens split by such a close is retried rather than reported as a missing reuse.
        let first = (0..8)
            .find_map(|_| {
                let first = open_local_database(&path).unwrap();
                let second = open_local_database(&path).unwrap();
                Arc::ptr_eq(&first, &second).then_some(first)
            })
            .expect("a path reuses its cached connection");
        close_cached_local_databases();
        let third = open_local_database(&path).unwrap();
        assert!(!Arc::ptr_eq(&first, &third));
        // The evicted connection still answers while a caller holds it.
        let count: i64 = lock(&first)
            .query_row("SELECT count(*) FROM t", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn a_new_directory_replaces_the_same_file_name() {
        let first_dir = tempfile::tempdir().unwrap();
        let second_dir = tempfile::tempdir().unwrap();
        let first_path = fixture(first_dir.path(), "generation-local.db");
        let second_path = fixture(second_dir.path(), "generation-local.db");
        let first = open_local_database(&first_path).unwrap();
        let second = open_local_database(&second_path).unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        let cache = lock(&CACHE);
        let cached = cache.get(OsStr::new("generation-local.db"));
        assert!(cached.is_none_or(|cached| cached.path == second_path));
    }

    #[test]
    fn a_missing_file_is_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing").join("missing-local.db");
        assert!(open_local_database(&path).is_none());
        assert!(!path.exists());
        assert!(open_local_database(Path::new("")).is_none());
    }
}
