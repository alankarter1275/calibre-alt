//! SQLite catalog access — P1 books + P2 progress + P3 annotations & dictionary.
#![allow(dead_code)]

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

// ---------------------------------------------------------------------------
// Domain structs for P3
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Annotation {
    pub id: i64,
    pub book_id: i64,
    pub kind: String, // highlight | quote | note
    pub chapter_index: i64,
    pub start_path: String,
    pub start_offset: i64,
    pub end_path: String,
    pub end_offset: i64,
    pub color: String,
    pub text_excerpt: String,
    pub note: String,
    pub cfi: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SavedWord {
    pub id: i64,
    pub word: String,
    pub definition: String,
    pub dict_name: Option<String>,
    pub book_id: Option<i64>,
    pub chapter_index: Option<i64>,
    pub context_text: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Dictionary {
    pub id: i64,
    pub name: String,
    pub lang: Option<String>,
    pub entry_count: i64,
    pub added_at: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DictEntry {
    pub id: i64,
    pub dict_id: i64,
    pub word: String,
    pub definition: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightColor {
    Yellow,
    Green,
    Blue,
    Pink,
    Orange,
}

impl HighlightColor {
    pub fn as_str(self) -> &'static str {
        match self {
            HighlightColor::Yellow => "yellow",
            HighlightColor::Green => "green",
            HighlightColor::Blue => "blue",
            HighlightColor::Pink => "pink",
            HighlightColor::Orange => "orange",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "green" => HighlightColor::Green,
            "blue" => HighlightColor::Blue,
            "pink" | "rose" => HighlightColor::Pink,
            "orange" => HighlightColor::Orange,
            _ => HighlightColor::Yellow,
        }
    }

    pub const ALL: &'static [HighlightColor] = &[
        HighlightColor::Yellow,
        HighlightColor::Green,
        HighlightColor::Blue,
        HighlightColor::Pink,
        HighlightColor::Orange,
    ];
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

            -- P3 tables
            CREATE TABLE IF NOT EXISTS annotations (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                book_id       INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                kind          TEXT    NOT NULL,
                chapter_index INTEGER NOT NULL,
                start_path    TEXT    NOT NULL,
                start_offset  INTEGER NOT NULL,
                end_path      TEXT    NOT NULL,
                end_offset    INTEGER NOT NULL,
                color         TEXT    NOT NULL DEFAULT 'yellow',
                text_excerpt  TEXT    NOT NULL DEFAULT '',
                note          TEXT    NOT NULL DEFAULT '',
                cfi           TEXT,
                created_at    TEXT    NOT NULL,
                updated_at    TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_annotations_book ON annotations(book_id);
            CREATE INDEX IF NOT EXISTS idx_annotations_book_chapter ON annotations(book_id, chapter_index);
            CREATE INDEX IF NOT EXISTS idx_annotations_kind ON annotations(kind);

            CREATE TABLE IF NOT EXISTS saved_words (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                word          TEXT    NOT NULL,
                definition    TEXT    NOT NULL,
                dict_name     TEXT,
                book_id       INTEGER REFERENCES books(id) ON DELETE SET NULL,
                chapter_index INTEGER,
                context_text  TEXT,
                created_at    TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_saved_words_word ON saved_words(word COLLATE NOCASE);

            CREATE TABLE IF NOT EXISTS dictionaries (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL UNIQUE,
                lang        TEXT,
                entry_count INTEGER NOT NULL DEFAULT 0,
                added_at    TEXT    NOT NULL
            );

            CREATE TABLE IF NOT EXISTS dict_entries (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                dict_id    INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
                word       TEXT    NOT NULL COLLATE NOCASE,
                definition TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_dict_entries_word ON dict_entries(word COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_dict_entries_dict ON dict_entries(dict_id);
            "#,
        )?;

        let version: Option<i64> = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        if version.is_none() {
            conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])?;
        } else if let Some(v) = version {
            if v < 3 {
                conn.execute("UPDATE schema_version SET version = 3", [])?;
            }
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

    // -----------------------------------------------------------------------
    // P3: Annotations
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_annotation(
        &self,
        book_id: i64,
        kind: &str,
        chapter_index: i64,
        start_path: &str,
        start_offset: i64,
        end_path: &str,
        end_offset: i64,
        color: &str,
        text_excerpt: &str,
        note: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().expect("db lock");
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO annotations
                (book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                 color, text_excerpt, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                book_id,
                kind,
                chapter_index,
                start_path,
                start_offset,
                end_path,
                end_offset,
                color,
                text_excerpt,
                note,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_annotations_for_book(&self, book_id: i64) -> Result<Vec<Annotation>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare(
            "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                    color, text_excerpt, note, cfi, created_at, updated_at
             FROM annotations WHERE book_id = ?1 ORDER BY chapter_index ASC, created_at ASC",
        )?;
        let rows = stmt.query_map(params![book_id], row_to_annotation)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_annotations_for_chapter(
        &self,
        book_id: i64,
        chapter_index: i64,
    ) -> Result<Vec<Annotation>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare(
            "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                    color, text_excerpt, note, cfi, created_at, updated_at
             FROM annotations
             WHERE book_id = ?1 AND chapter_index = ?2
             ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![book_id, chapter_index], row_to_annotation)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_all_quotes(&self, query: &str) -> Result<Vec<Annotation>> {
        let conn = self.conn.lock().expect("db lock");
        let q = query.trim();
        let mut stmt = if q.is_empty() {
            conn.prepare(
                "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                        color, text_excerpt, note, cfi, created_at, updated_at
                 FROM annotations WHERE kind IN ('quote','highlight')
                 ORDER BY created_at DESC LIMIT 500",
            )?
        } else {
            conn.prepare(
                "SELECT id, book_id, kind, chapter_index, start_path, start_offset, end_path, end_offset,
                        color, text_excerpt, note, cfi, created_at, updated_at
                 FROM annotations
                 WHERE kind IN ('quote','highlight')
                   AND (text_excerpt LIKE ?1 ESCAPE '\\' OR note LIKE ?1 ESCAPE '\\')
                 ORDER BY created_at DESC LIMIT 500",
            )?
        };
        let like = format!("%{}%", escape_like(q));
        let rows = if q.is_empty() {
            stmt.query_map([], row_to_annotation)?
        } else {
            stmt.query_map(params![like], row_to_annotation)?
        };
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete_annotation(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute("DELETE FROM annotations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_annotation_note(&self, id: i64, note: &str) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        let now = chrono_like_now();
        conn.execute(
            "UPDATE annotations SET note = ?1, updated_at = ?2 WHERE id = ?3",
            params![note, now, id],
        )?;
        Ok(())
    }

    pub fn update_annotation_color(&self, id: i64, color: &str) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        let now = chrono_like_now();
        conn.execute(
            "UPDATE annotations SET color = ?1, updated_at = ?2 WHERE id = ?3",
            params![color, now, id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P3: Saved words
    // -----------------------------------------------------------------------

    pub fn insert_saved_word(
        &self,
        word: &str,
        definition: &str,
        dict_name: Option<&str>,
        book_id: Option<i64>,
        chapter_index: Option<i64>,
        context_text: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().expect("db lock");
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO saved_words (word, definition, dict_name, book_id, chapter_index, context_text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                word,
                definition,
                dict_name,
                book_id,
                chapter_index,
                context_text,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_saved_words(&self, query: &str) -> Result<Vec<SavedWord>> {
        let conn = self.conn.lock().expect("db lock");
        let q = query.trim();
        if q.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT id, word, definition, dict_name, book_id, chapter_index, context_text, created_at
                 FROM saved_words ORDER BY created_at DESC LIMIT 500",
            )?;
            let rows = stmt.query_map([], row_to_saved_word)?;
            rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
        } else {
            let like = format!("%{}%", escape_like(q));
            let mut stmt = conn.prepare(
                "SELECT id, word, definition, dict_name, book_id, chapter_index, context_text, created_at
                 FROM saved_words
                 WHERE word LIKE ?1 ESCAPE '\\' OR definition LIKE ?1 ESCAPE '\\'
                 ORDER BY created_at DESC LIMIT 500",
            )?;
            let rows = stmt.query_map(params![like], row_to_saved_word)?;
            rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
        }
    }

    pub fn delete_saved_word(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute("DELETE FROM saved_words WHERE id = ?1", params![id])?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P3: Dictionaries
    // -----------------------------------------------------------------------

    pub fn list_dictionaries(&self) -> Result<Vec<Dictionary>> {
        let conn = self.conn.lock().expect("db lock");
        let mut stmt = conn.prepare(
            "SELECT id, name, lang, entry_count, added_at FROM dictionaries ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Dictionary {
                id: r.get(0)?,
                name: r.get(1)?,
                lang: r.get(2)?,
                entry_count: r.get(3)?,
                added_at: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn insert_dictionary(
        &self,
        name: &str,
        lang: Option<&str>,
        entry_count: i64,
    ) -> Result<i64> {
        let conn = self.conn.lock().expect("db lock");
        let now = chrono_like_now();
        conn.execute(
            "INSERT INTO dictionaries (name, lang, entry_count, added_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(name) DO UPDATE SET lang=excluded.lang, entry_count=excluded.entry_count",
            params![name, lang, entry_count, now],
        )?;
        let id = conn.query_row(
            "SELECT id FROM dictionaries WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn delete_dictionary(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute("DELETE FROM dictionaries WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_dictionary_entry_count(&self, id: i64, count: i64) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "UPDATE dictionaries SET entry_count = ?1 WHERE id = ?2",
            params![count, id],
        )?;
        Ok(())
    }

    pub fn insert_dict_entry(
        &self,
        dict_id: i64,
        word: &str,
        definition: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "INSERT INTO dict_entries (dict_id, word, definition) VALUES (?1, ?2, ?3)",
            params![dict_id, word, definition],
        )?;
        Ok(())
    }

    pub fn batch_insert_dict_entries(
        &self,
        dict_id: i64,
        entries: &[(String, String)],
    ) -> Result<()> {
        let mut conn = self.conn.lock().expect("db lock");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO dict_entries (dict_id, word, definition) VALUES (?1, ?2, ?3)",
            )?;
            for (w, d) in entries {
                stmt.execute(params![dict_id, w, d])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn clear_dict_entries(&self, dict_id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("db lock");
        conn.execute(
            "DELETE FROM dict_entries WHERE dict_id = ?1",
            params![dict_id],
        )?;
        Ok(())
    }

    pub fn search_dict(&self, word: &str, limit: usize) -> Result<Vec<DictEntry>> {
        let conn = self.conn.lock().expect("db lock");
        let clean = word.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }
        // Exact match first, then prefix, then LIKE fallback
        let mut stmt = conn.prepare(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word = ?1 COLLATE NOCASE
             ORDER BY word ASC LIMIT ?2",
        )?;
        let mut out = Vec::new();
        let lim = limit as i64;
        for r in stmt.query_map(params![clean, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        if !out.is_empty() {
            return Ok(out);
        }

        // Prefix search
        let like = format!("{}%", escape_like(clean));
        let mut stmt2 = conn.prepare(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY LENGTH(word) ASC, word ASC LIMIT ?2",
        )?;
        for r in stmt2.query_map(params![like, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        if !out.is_empty() {
            return Ok(out);
        }

        // Substring fallback
        let like2 = format!("%{}%", escape_like(clean));
        let mut stmt3 = conn.prepare(
            "SELECT id, dict_id, word, definition FROM dict_entries
             WHERE word LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                OR definition LIKE ?1 ESCAPE '\\'
             ORDER BY LENGTH(word) ASC LIMIT ?2",
        )?;
        for r in stmt3.query_map(params![like2, lim], |r| {
            Ok(DictEntry {
                id: r.get(0)?,
                dict_id: r.get(1)?,
                word: r.get(2)?,
                definition: r.get(3)?,
            })
        })? {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn dict_entry_count(&self) -> Result<i64> {
        let conn = self.conn.lock().expect("db lock");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM dict_entries", [], |r| r.get(0))?;
        Ok(n)
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

fn row_to_annotation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Annotation> {
    Ok(Annotation {
        id: row.get(0)?,
        book_id: row.get(1)?,
        kind: row.get(2)?,
        chapter_index: row.get(3)?,
        start_path: row.get(4)?,
        start_offset: row.get(5)?,
        end_path: row.get(6)?,
        end_offset: row.get(7)?,
        color: row.get(8)?,
        text_excerpt: row.get(9)?,
        note: row.get(10)?,
        cfi: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn row_to_saved_word(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedWord> {
    Ok(SavedWord {
        id: row.get(0)?,
        word: row.get(1)?,
        definition: row.get(2)?,
        dict_name: row.get(3)?,
        book_id: row.get(4)?,
        chapter_index: row.get(5)?,
        context_text: row.get(6)?,
        created_at: row.get(7)?,
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
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_utc(secs)
}

fn format_unix_utc(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let tod = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

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
