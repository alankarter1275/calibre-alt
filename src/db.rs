//! SQLite catalog access.

use crate::models::{Book, BookFormat};
use crate::paths::{book_dir, catalog_db, ensure_data_dirs};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Process-wide DB handle (GTK app is single-threaded for UI; imports run sync on UI for P1).
pub struct Catalog {
    conn: Mutex<Connection>,
}

impl Catalog {
    pub fn open() -> Result<Self> {
        ensure_data_dirs()?;
        let path = catalog_db();
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            ",
        )?;
        let cat = Self {
            conn: Mutex::new(conn),
        };
        cat.migrate()?;
        Ok(cat)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS books (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid          TEXT    NOT NULL UNIQUE,
                title         TEXT    NOT NULL,
                sort_title    TEXT    NOT NULL,
                authors       TEXT    NOT NULL DEFAULT '',
                series        TEXT,
                description   TEXT    NOT NULL DEFAULT '',
                format        TEXT    NOT NULL,
                file_name     TEXT    NOT NULL,
                file_hash     TEXT    NOT NULL,
                cover_name    TEXT,
                added_at      TEXT    NOT NULL,
                progress      INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS tags (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE COLLATE NOCASE
            );

            CREATE TABLE IF NOT EXISTS book_tags (
                book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY (book_id, tag_id)
            );

            CREATE INDEX IF NOT EXISTS idx_books_title ON books(sort_title);
            CREATE INDEX IF NOT EXISTS idx_books_added ON books(added_at);
            CREATE INDEX IF NOT EXISTS idx_books_hash ON books(file_hash);

            CREATE TABLE IF NOT EXISTS reading_progress (
                book_id       INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
                chapter_index INTEGER NOT NULL DEFAULT 0,
                fraction      REAL    NOT NULL DEFAULT 0.0,
                updated_at    TEXT    NOT NULL
            );
            "#,
        )?;

        let version: Option<i64> = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        if version.is_none() {
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
        }
        Ok(())
    }

    pub fn count_books(&self) -> Result<usize> {
        let conn = self.conn.lock().expect("db lock");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM books", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn list_books(&self, sort: SortKey, query: &str) -> Result<Vec<Book>> {
        let conn = self.conn.lock().expect("db lock");
        let order = match sort {
            SortKey::Title => "sort_title COLLATE NOCASE ASC",
            SortKey::Author => "authors COLLATE NOCASE ASC, sort_title COLLATE NOCASE ASC",
            SortKey::Added => "added_at DESC",
        };

        let q = query.trim();
        let mut books = if q.is_empty() {
            let sql = format!(
                "SELECT id, uuid, title, authors, series, description, format, file_name,
                        file_hash, cover_name, added_at, progress
                 FROM books ORDER BY {order}"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], row_to_book)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            let like = format!("%{}%", escape_like(q));
            let sql = format!(
                "SELECT id, uuid, title, authors, series, description, format, file_name,
                        file_hash, cover_name, added_at, progress
                 FROM books
                 WHERE title LIKE ?1 ESCAPE '\\'
                    OR authors LIKE ?1 ESCAPE '\\'
                    OR IFNULL(series,'') LIKE ?1 ESCAPE '\\'
                 ORDER BY {order}"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![like], row_to_book)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        for book in &mut books {
            book.tags = tags_for_book(&conn, book.id)?;
            book.cover_path = book
                .cover_name
                .as_ref()
                .map(|name| book_dir(&book.uuid).join(name));
            book.file_path = book_dir(&book.uuid).join(&book.file_name);
        }
        Ok(books)
    }

    pub fn get_book(&self, id: i64) -> Result<Option<Book>> {
        let conn = self.conn.lock().expect("db lock");
        let mut book = conn
            .query_row(
                "SELECT id, uuid, title, authors, series, description, format, file_name,
                        file_hash, cover_name, added_at, progress
                 FROM books WHERE id = ?1",
                params![id],
                row_to_book,
            )
            .optional()?;
        if let Some(ref mut b) = book {
            b.tags = tags_for_book(&conn, b.id)?;
            b.cover_path = b
                .cover_name
                .as_ref()
                .map(|name| book_dir(&b.uuid).join(name));
            b.file_path = book_dir(&b.uuid).join(&b.file_name);
        }
        Ok(book)
    }

    pub fn find_by_hash(&self, hash: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().expect("db lock");
        let id = conn
            .query_row(
                "SELECT id FROM books WHERE file_hash = ?1 LIMIT 1",
                params![hash],
                |r| r.get(0),
            )
            .optional()?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_book(
        &self,
        uuid: &str,
        title: &str,
        authors: &str,
        series: Option<&str>,
        description: &str,
        format: BookFormat,
        file_name: &str,
        file_hash: &str,
        cover_name: Option<&str>,
        tags: &[String],
    ) -> Result<i64> {
        let conn = self.conn.lock().expect("db lock");
        let added = chrono_like_now();
        let sort_title = title.to_lowercase();
        conn.execute(
            "INSERT INTO books
                (uuid, title, sort_title, authors, series, description, format,
                 file_name, file_hash, cover_name, added_at, progress)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,0)",
            params![
                uuid,
                title,
                sort_title,
                authors,
                series,
                description,
                format.as_str(),
                file_name,
                file_hash,
                cover_name,
                added,
            ],
        )?;
        let id = conn.last_insert_rowid();
        for tag in tags {
            let tag = tag.trim();
            if tag.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                params![tag],
            )?;
            let tag_id: i64 = conn.query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                params![tag],
                |r| r.get(0),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )?;
        }
        Ok(id)
    }

    pub fn delete_book(&self, id: i64) -> Result<()> {
        let book = match self.get_book(id)? {
            Some(b) => b,
            None => return Ok(()),
        };
        {
            let conn = self.conn.lock().expect("db lock");
            conn.execute("DELETE FROM books WHERE id = ?1", params![id])?;
        }
        let dir = book_dir(&book.uuid);
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
        Ok(())
    }

    /// Detailed reading position (chapter + in-chapter fraction).
    pub fn get_reading_progress(&self, book_id: i64) -> Result<Option<(usize, f64)>> {
        let conn = self.conn.lock().expect("db lock");
        let row = conn
            .query_row(
                "SELECT chapter_index, fraction FROM reading_progress WHERE book_id = ?1",
                params![book_id],
                |r| Ok((r.get::<_, i64>(0)? as usize, r.get::<_, f64>(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    /// Save position and update the books.progress percent (0–100).
    pub fn set_reading_progress(
        &self,
        book_id: i64,
        chapter_index: usize,
        fraction: f64,
        chapter_count: usize,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        let frac = fraction.clamp(0.0, 1.0);
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO reading_progress (book_id, chapter_index, fraction, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(book_id) DO UPDATE SET
               chapter_index = excluded.chapter_index,
               fraction = excluded.fraction,
               updated_at = excluded.updated_at",
            params![book_id, chapter_index as i64, frac, now],
        )?;
        // Overall percent across chapters.
        let overall = if chapter_count == 0 {
            0.0
        } else {
            ((chapter_index as f64) + frac) / (chapter_count as f64) * 100.0
        };
        let pct = overall.round().clamp(0.0, 100.0) as i64;
        conn.execute(
            "UPDATE books SET progress = ?1 WHERE id = ?2",
            params![pct, book_id],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Title,
    Author,
    Added,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Title => "Title",
            SortKey::Author => "Author",
            SortKey::Added => "Added",
        }
    }

    pub const ALL: &'static [SortKey] = &[SortKey::Title, SortKey::Author, SortKey::Added];
}

fn row_to_book(row: &rusqlite::Row<'_>) -> rusqlite::Result<Book> {
    let format_str: String = row.get(6)?;
    Ok(Book {
        id: row.get(0)?,
        uuid: row.get(1)?,
        title: row.get(2)?,
        authors: row.get(3)?,
        series: row.get(4)?,
        description: row.get(5)?,
        format: BookFormat::from_str_lossy(&format_str),
        file_name: row.get(7)?,
        file_hash: row.get(8)?,
        cover_name: row.get(9)?,
        added_at: row.get(10)?,
        progress: row.get::<_, i64>(11)? as u8,
        tags: Vec::new(),
        cover_path: None,
        file_path: PathBuf::new(),
    })
}

fn tags_for_book(conn: &Connection, book_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tags t
         JOIN book_tags bt ON bt.tag_id = t.id
         WHERE bt.book_id = ?1
         ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![book_id], |r| r.get(0))?;
    let mut tags = Vec::new();
    for t in rows {
        tags.push(t?);
    }
    Ok(tags)
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn chrono_like_now() -> String {
    // RFC3339-ish local timestamp without pulling chrono crate.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Keep a sortable ISO-ish string via UTC formatting without chrono:
    // store unix for sort stability is fine; display can parse later.
    // Prefer human readable: use `date` is not portable — use simple ISO from utc.
    format_unix_utc(secs)
}

fn format_unix_utc(secs: u64) -> String {
    // Civil UTC date from days since epoch (adequate for P1).
    let days = (secs / 86400) as i64;
    let tod = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Howard Hinnant civil_from_days (UTC).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// File hash (sha256 hex) of path.
pub fn hash_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}
